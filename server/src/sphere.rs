use crate::vec3::{self, Vec3};
use rand::seq::SliceRandom;
use rand::Rng;
use std::collections::HashMap;

/// Golden angle in radians: PI * (3 - sqrt(5)) ≈ 2.39996...
/// Pre-computed since sqrt is not const fn.
const GOLDEN_ANGLE: f64 = 2.399963229728653;

/// Generate M evenly-distributed points on a unit sphere using Fibonacci spiral.
pub fn fibonacci_sphere(m: usize) -> Vec<Vec3> {
    let mut points = Vec::with_capacity(m);

    for i in 0..m {
        let y = 1.0 - (2.0 * (i as f64 + 0.5)) / m as f64;
        let r = (1.0 - y * y).sqrt();
        let phi = i as f64 * GOLDEN_ANGLE;

        let x = phi.cos() * r;
        let z = phi.sin() * r;

        points.push(vec3::normalize(Vec3::new(x, y, z)));
    }

    points
}

/// Portal placement manager.
/// Manages cell allocation for players on the sphere.
pub struct PortalPlacement {
    pub cell_centers: Vec<Vec3>,
    free_cells: Vec<usize>,
    token_to_cell: HashMap<String, usize>,
}

impl PortalPlacement {
    pub fn new(cell_count: usize, rng: &mut impl Rng) -> Self {
        let cell_centers = fibonacci_sphere(cell_count);

        let mut free_cells: Vec<usize> = (0..cell_count).collect();
        free_cells.shuffle(rng);

        Self {
            cell_centers,
            free_cells,
            token_to_cell: HashMap::new(),
        }
    }

    /// Allocate a cell for a player.
    pub fn allocate(&mut self, resume_token: Option<&str>) -> Option<usize> {
        // Try to resume previous cell
        if let Some(token) = resume_token {
            if let Some(&prev_cell) = self.token_to_cell.get(token) {
                if let Some(free_idx) = self.free_cells.iter().position(|&c| c == prev_cell) {
                    self.free_cells.swap_remove(free_idx);
                    return Some(prev_cell);
                }
            }
        }

        let cell_index = self.free_cells.pop()?;

        if let Some(token) = resume_token {
            self.token_to_cell.insert(token.to_string(), cell_index);
        }

        Some(cell_index)
    }

    /// Release a cell back to the pool.
    pub fn release(&mut self, cell_index: usize) {
        if !self.free_cells.contains(&cell_index) {
            self.free_cells.push(cell_index);
        }
    }

    /// Get portal position for a cell.
    pub fn portal_pos(&self, cell_index: usize) -> Vec3 {
        self.cell_centers[cell_index]
    }

    /// Number of available cells
    pub fn available_count(&self) -> usize {
        self.free_cells.len()
    }

    /// Total cell count
    pub fn total_count(&self) -> usize {
        self.cell_centers.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vec3::{dot, length};
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    fn test_rng() -> ChaCha8Rng {
        ChaCha8Rng::seed_from_u64(42)
    }

    #[test]
    fn fibonacci_generates_correct_count() {
        let points = fibonacci_sphere(100);
        assert_eq!(points.len(), 100);
    }

    #[test]
    fn all_points_are_unit_vectors() {
        let points = fibonacci_sphere(50);
        for p in &points {
            assert!((length(*p) - 1.0).abs() < 1e-9);
        }
    }

    #[test]
    fn points_are_reasonably_distributed() {
        let points = fibonacci_sphere(100);
        let min_expected_dist = 0.1;

        for i in 0..points.len() {
            for j in (i + 1)..points.len() {
                let d = dot(points[i], points[j]).clamp(-1.0, 1.0);
                let angular_dist = d.acos();
                assert!(
                    angular_dist > min_expected_dist,
                    "Points {} and {} too close: {}",
                    i,
                    j,
                    angular_dist
                );
            }
        }
    }

    #[test]
    fn covers_both_hemispheres() {
        let points = fibonacci_sphere(100);
        let has_positive_z = points.iter().any(|p| p.z > 0.5);
        let has_negative_z = points.iter().any(|p| p.z < -0.5);
        assert!(has_positive_z);
        assert!(has_negative_z);
    }

    #[test]
    fn allocates_unique_cell_indices() {
        let mut rng = test_rng();
        let mut placement = PortalPlacement::new(100, &mut rng);
        let mut allocated = std::collections::HashSet::new();

        for _ in 0..50 {
            let idx = placement.allocate(None).unwrap();
            assert!(!allocated.contains(&idx));
            allocated.insert(idx);
        }
    }

    #[test]
    fn returns_none_when_all_allocated() {
        let mut rng = test_rng();
        let mut placement = PortalPlacement::new(10, &mut rng);

        for _ in 0..10 {
            assert!(placement.allocate(None).is_some());
        }
        assert!(placement.allocate(None).is_none());
    }

    #[test]
    fn portal_pos_returns_unit_vector() {
        let mut rng = test_rng();
        let mut placement = PortalPlacement::new(100, &mut rng);
        let idx = placement.allocate(None).unwrap();
        let pos = placement.portal_pos(idx);
        assert!((length(pos) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn available_count_decreases() {
        let mut rng = test_rng();
        let mut placement = PortalPlacement::new(100, &mut rng);
        assert_eq!(placement.available_count(), 100);
        placement.allocate(None);
        assert_eq!(placement.available_count(), 99);
        placement.allocate(None);
        assert_eq!(placement.available_count(), 98);
    }

    #[test]
    fn total_count_returns_cell_count() {
        let mut rng = test_rng();
        let placement = PortalPlacement::new(200, &mut rng);
        assert_eq!(placement.total_count(), 200);
    }

    #[test]
    fn shuffle_distributes_across_sphere() {
        let mut rng = test_rng();
        let mut placement = PortalPlacement::new(1000, &mut rng);
        let mut z_values = Vec::new();

        for _ in 0..10 {
            let idx = placement.allocate(None).unwrap();
            z_values.push(placement.portal_pos(idx).z);
        }

        let min_z = z_values.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_z = z_values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        assert!(max_z - min_z > 0.5);
    }

    #[test]
    fn resume_token_reclaims_released_cell() {
        let mut rng = test_rng();
        let mut placement = PortalPlacement::new(100, &mut rng);
        let idx1 = placement.allocate(Some("player-123")).unwrap();
        placement.release(idx1);
        let idx2 = placement.allocate(Some("player-123")).unwrap();
        assert_eq!(idx1, idx2);
    }

    #[test]
    fn different_tokens_get_different_indices() {
        let mut rng = test_rng();
        let mut placement = PortalPlacement::new(100, &mut rng);
        let idx1 = placement.allocate(Some("player-1")).unwrap();
        let idx2 = placement.allocate(Some("player-2")).unwrap();
        assert_ne!(idx1, idx2);
    }
}
