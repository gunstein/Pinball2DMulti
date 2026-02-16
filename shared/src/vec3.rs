/// 3D vector utilities for sphere-based deep-space.
/// All vectors are assumed to be unit vectors (on the sphere surface) unless noted.

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vec3 {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }
}

/// Shorthand constructor matching TypeScript vec3()
pub fn vec3(x: f64, y: f64, z: f64) -> Vec3 {
    Vec3::new(x, y, z)
}

/// Dot product
pub fn dot(a: Vec3, b: Vec3) -> f64 {
    a.x * b.x + a.y * b.y + a.z * b.z
}

/// Cross product
pub fn cross(a: Vec3, b: Vec3) -> Vec3 {
    Vec3 {
        x: a.y * b.z - a.z * b.y,
        y: a.z * b.x - a.x * b.z,
        z: a.x * b.y - a.y * b.x,
    }
}

/// Vector length
pub fn length(v: Vec3) -> f64 {
    (v.x * v.x + v.y * v.y + v.z * v.z).sqrt()
}

/// Normalize vector to unit length
pub fn normalize(v: Vec3) -> Vec3 {
    let len = length(v);
    if len < 1e-10 {
        return Vec3::new(1.0, 0.0, 0.0);
    }
    Vec3::new(v.x / len, v.y / len, v.z / len)
}

/// Scale vector by scalar
pub fn scale(v: Vec3, s: f64) -> Vec3 {
    Vec3::new(v.x * s, v.y * s, v.z * s)
}

/// Add two vectors
pub fn add(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(a.x + b.x, a.y + b.y, a.z + b.z)
}

/// Subtract vectors (a - b). Only used in tests.
#[cfg(test)]
pub fn sub(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(a.x - b.x, a.y - b.y, a.z - b.z)
}

/// Rotate vector around axis by angle (Rodrigues' rotation formula).
pub fn rotate_around_axis(v: Vec3, axis: Vec3, angle: f64) -> Vec3 {
    let cos_a = angle.cos();
    let sin_a = angle.sin();
    let one_minus_cos = 1.0 - cos_a;

    let cross_av = cross(axis, v);
    let dot_av = dot(axis, v);

    Vec3 {
        x: v.x * cos_a + cross_av.x * sin_a + axis.x * dot_av * one_minus_cos,
        y: v.y * cos_a + cross_av.y * sin_a + axis.y * dot_av * one_minus_cos,
        z: v.z * cos_a + cross_av.z * sin_a + axis.z * dot_av * one_minus_cos,
    }
}

/// Rotate pos around axis by angle, normalize, and write result back to pos.
/// Rodrigues' rotation + normalize in one pass, zero allocations.
pub fn rotate_normalize_in_place(pos: &mut Vec3, axis: Vec3, angle: f64) {
    let cos_a = angle.cos();
    let sin_a = angle.sin();
    let one_minus_cos = 1.0 - cos_a;

    let cx = axis.y * pos.z - axis.z * pos.y;
    let cy = axis.z * pos.x - axis.x * pos.z;
    let cz = axis.x * pos.y - axis.y * pos.x;

    let d = axis.x * pos.x + axis.y * pos.y + axis.z * pos.z;

    let rx = pos.x * cos_a + cx * sin_a + axis.x * d * one_minus_cos;
    let ry = pos.y * cos_a + cy * sin_a + axis.y * d * one_minus_cos;
    let rz = pos.z * cos_a + cz * sin_a + axis.z * d * one_minus_cos;

    let len = (rx * rx + ry * ry + rz * rz).sqrt();
    if len < 1e-10 {
        pos.x = 1.0;
        pos.y = 0.0;
        pos.z = 0.0;
    } else {
        let inv = 1.0 / len;
        pos.x = rx * inv;
        pos.y = ry * inv;
        pos.z = rz * inv;
    }
}

/// Get angular distance between two unit vectors (in radians).
pub fn angular_distance(a: Vec3, b: Vec3) -> f64 {
    let d = dot(a, b);
    d.clamp(-1.0, 1.0).acos()
}

/// Spherical linear interpolation between two unit vectors.
/// t=0 returns a, t=1 returns b.
pub fn slerp(a: Vec3, b: Vec3, t: f64) -> Vec3 {
    let d = dot(a, b).clamp(-1.0, 1.0);

    // If vectors are very close, use linear interpolation to avoid division by zero
    if d > 0.9995 {
        return normalize(Vec3::new(
            a.x + t * (b.x - a.x),
            a.y + t * (b.y - a.y),
            a.z + t * (b.z - a.z),
        ));
    }

    // If vectors are nearly opposite, choose a deterministic great-circle plane.
    if d < -0.9995 {
        let axis = arbitrary_orthogonal(a);
        return normalize(rotate_around_axis(a, axis, std::f64::consts::PI * t));
    }

    let theta = d.acos();
    let sin_theta = theta.sin();

    let s0 = ((1.0 - t) * theta).sin() / sin_theta;
    let s1 = (t * theta).sin() / sin_theta;

    Vec3::new(
        s0 * a.x + s1 * b.x,
        s0 * a.y + s1 * b.y,
        s0 * a.z + s1 * b.z,
    )
}

/// Find an arbitrary vector orthogonal to v.
pub fn arbitrary_orthogonal(v: Vec3) -> Vec3 {
    let reference = if v.y.abs() < 0.9 {
        Vec3::new(0.0, 1.0, 0.0)
    } else {
        Vec3::new(1.0, 0.0, 0.0)
    };
    normalize(cross(reference, v))
}

/// Build a local tangent basis (e1, e2) for a point on the sphere.
pub fn build_tangent_basis(u: Vec3) -> (Vec3, Vec3) {
    let reference = if dot(u, Vec3::new(0.0, 1.0, 0.0)).abs() < 0.9 {
        Vec3::new(0.0, 1.0, 0.0)
    } else {
        Vec3::new(1.0, 0.0, 0.0)
    };

    let e1 = normalize(cross(reference, u));
    let e2 = cross(u, e1);

    (e1, e2)
}

/// Map a 2D direction to a 3D tangent direction on the sphere.
pub fn map_2d_to_tangent(dx: f64, dy: f64, e1: Vec3, e2: Vec3) -> Vec3 {
    let len = (dx * dx + dy * dy).sqrt();
    if len < 1e-10 {
        return e1;
    }
    let nx = dx / len;
    let ny = dy / len;
    normalize(add(scale(e1, nx), scale(e2, ny)))
}

/// Map a 3D tangent direction back to 2D components.
pub fn map_tangent_to_2d(tangent: Vec3, e1: Vec3, e2: Vec3) -> (f64, f64) {
    (dot(tangent, e1), dot(tangent, e2))
}

/// Get the velocity direction of a ball moving on a great circle.
pub fn get_velocity_direction(pos: Vec3, axis: Vec3, omega: f64) -> Vec3 {
    let dir = normalize(cross(axis, pos));
    if omega >= 0.0 {
        dir
    } else {
        scale(dir, -1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    fn assert_vec3_close(actual: Vec3, expected: Vec3) {
        assert!(
            (actual.x - expected.x).abs() < 1e-6
                && (actual.y - expected.y).abs() < 1e-6
                && (actual.z - expected.z).abs() < 1e-6,
            "Expected {:?} to be close to {:?}",
            actual,
            expected
        );
    }

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1e-6,
            "Expected {} to be close to {}",
            actual,
            expected
        );
    }

    #[test]
    fn vec3_creates_vector() {
        let v = vec3(1.0, 2.0, 3.0);
        assert_eq!(v.x, 1.0);
        assert_eq!(v.y, 2.0);
        assert_eq!(v.z, 3.0);
    }

    #[test]
    fn dot_orthogonal_is_zero() {
        assert_eq!(dot(vec3(1.0, 0.0, 0.0), vec3(0.0, 1.0, 0.0)), 0.0);
    }

    #[test]
    fn dot_parallel_is_product_of_lengths() {
        assert_eq!(dot(vec3(2.0, 0.0, 0.0), vec3(3.0, 0.0, 0.0)), 6.0);
    }

    #[test]
    fn dot_antiparallel_is_negative() {
        assert_eq!(dot(vec3(1.0, 0.0, 0.0), vec3(-1.0, 0.0, 0.0)), -1.0);
    }

    #[test]
    fn cross_x_and_y_is_z() {
        assert_vec3_close(
            cross(vec3(1.0, 0.0, 0.0), vec3(0.0, 1.0, 0.0)),
            vec3(0.0, 0.0, 1.0),
        );
    }

    #[test]
    fn cross_parallel_is_zero() {
        assert_vec3_close(
            cross(vec3(1.0, 0.0, 0.0), vec3(2.0, 0.0, 0.0)),
            vec3(0.0, 0.0, 0.0),
        );
    }

    #[test]
    fn length_of_unit_vectors() {
        assert_eq!(length(vec3(1.0, 0.0, 0.0)), 1.0);
        assert_eq!(length(vec3(0.0, 1.0, 0.0)), 1.0);
        assert_eq!(length(vec3(0.0, 0.0, 1.0)), 1.0);
    }

    #[test]
    fn length_of_3_4_0_is_5() {
        assert_eq!(length(vec3(3.0, 4.0, 0.0)), 5.0);
    }

    #[test]
    fn normalize_returns_unit_vector() {
        let v = normalize(vec3(3.0, 4.0, 0.0));
        assert_close(length(v), 1.0);
        assert_vec3_close(v, vec3(0.6, 0.8, 0.0));
    }

    #[test]
    fn normalize_zero_returns_arbitrary_unit() {
        let v = normalize(vec3(0.0, 0.0, 0.0));
        assert_close(length(v), 1.0);
    }

    #[test]
    fn scale_multiplies() {
        assert_vec3_close(scale(vec3(1.0, 2.0, 3.0), 2.0), vec3(2.0, 4.0, 6.0));
    }

    #[test]
    fn add_sums() {
        assert_vec3_close(
            add(vec3(1.0, 2.0, 3.0), vec3(4.0, 5.0, 6.0)),
            vec3(5.0, 7.0, 9.0),
        );
    }

    #[test]
    fn sub_subtracts() {
        assert_vec3_close(
            sub(vec3(4.0, 5.0, 6.0), vec3(1.0, 2.0, 3.0)),
            vec3(3.0, 3.0, 3.0),
        );
    }

    #[test]
    fn rotate_x_around_z_by_90_gives_y() {
        assert_vec3_close(
            rotate_around_axis(vec3(1.0, 0.0, 0.0), vec3(0.0, 0.0, 1.0), PI / 2.0),
            vec3(0.0, 1.0, 0.0),
        );
    }

    #[test]
    fn rotate_x_around_z_by_180_gives_neg_x() {
        assert_vec3_close(
            rotate_around_axis(vec3(1.0, 0.0, 0.0), vec3(0.0, 0.0, 1.0), PI),
            vec3(-1.0, 0.0, 0.0),
        );
    }

    #[test]
    fn rotate_x_around_z_by_360_gives_x() {
        assert_vec3_close(
            rotate_around_axis(vec3(1.0, 0.0, 0.0), vec3(0.0, 0.0, 1.0), 2.0 * PI),
            vec3(1.0, 0.0, 0.0),
        );
    }

    #[test]
    fn rotate_around_own_axis_does_nothing() {
        assert_vec3_close(
            rotate_around_axis(vec3(1.0, 0.0, 0.0), vec3(1.0, 0.0, 0.0), PI / 2.0),
            vec3(1.0, 0.0, 0.0),
        );
    }

    #[test]
    fn rotate_preserves_length() {
        let v = normalize(vec3(1.0, 1.0, 1.0));
        let axis = normalize(vec3(1.0, 2.0, 3.0));
        assert_close(length(rotate_around_axis(v, axis, 1.234)), 1.0);
    }

    #[test]
    fn angular_distance_same_point() {
        assert_close(
            angular_distance(vec3(1.0, 0.0, 0.0), vec3(1.0, 0.0, 0.0)),
            0.0,
        );
    }

    #[test]
    fn angular_distance_orthogonal() {
        assert_close(
            angular_distance(vec3(1.0, 0.0, 0.0), vec3(0.0, 1.0, 0.0)),
            PI / 2.0,
        );
    }

    #[test]
    fn angular_distance_opposite() {
        assert_close(
            angular_distance(vec3(1.0, 0.0, 0.0), vec3(-1.0, 0.0, 0.0)),
            PI,
        );
    }

    #[test]
    fn slerp_returns_endpoints_for_t0_t1() {
        let a = vec3(1.0, 0.0, 0.0);
        let b = vec3(0.0, 1.0, 0.0);
        assert_vec3_close(slerp(a, b, 0.0), a);
        assert_vec3_close(slerp(a, b, 1.0), b);
    }

    #[test]
    fn slerp_midpoint_between_orthogonal_vectors() {
        let a = vec3(1.0, 0.0, 0.0);
        let b = vec3(0.0, 1.0, 0.0);
        let mid = slerp(a, b, 0.5);
        assert_close(length(mid), 1.0);
        assert!((mid.x - std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-6);
        assert!((mid.y - std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-6);
    }

    #[test]
    fn slerp_nearly_opposite_vectors_stays_finite() {
        let a = vec3(1.0, 0.0, 0.0);
        let b = normalize(vec3(-1.0, 1e-8, 0.0));
        let p = slerp(a, b, 0.5);
        assert!(p.x.is_finite() && p.y.is_finite() && p.z.is_finite());
        assert_close(length(p), 1.0);
    }

    #[test]
    fn arbitrary_orthogonal_is_orthogonal() {
        let v = normalize(vec3(1.0, 2.0, 3.0));
        assert_close(dot(v, arbitrary_orthogonal(v)), 0.0);
    }

    #[test]
    fn arbitrary_orthogonal_is_unit() {
        let v = normalize(vec3(1.0, 2.0, 3.0));
        assert_close(length(arbitrary_orthogonal(v)), 1.0);
    }

    #[test]
    fn arbitrary_orthogonal_works_for_axis_aligned() {
        for v in [
            vec3(1.0, 0.0, 0.0),
            vec3(0.0, 1.0, 0.0),
            vec3(0.0, 0.0, 1.0),
        ] {
            let orth = arbitrary_orthogonal(v);
            assert_close(dot(v, orth), 0.0);
            assert_close(length(orth), 1.0);
        }
    }

    #[test]
    fn tangent_basis_orthonormal() {
        let u = normalize(vec3(1.0, 2.0, 3.0));
        let (e1, e2) = build_tangent_basis(u);
        assert_close(length(e1), 1.0);
        assert_close(length(e2), 1.0);
        assert_close(dot(u, e1), 0.0);
        assert_close(dot(u, e2), 0.0);
        assert_close(dot(e1, e2), 0.0);
    }

    #[test]
    fn tangent_basis_north_pole() {
        let u = vec3(0.0, 1.0, 0.0);
        let (e1, e2) = build_tangent_basis(u);
        assert_close(dot(u, e1), 0.0);
        assert_close(dot(u, e2), 0.0);
        assert_close(dot(e1, e2), 0.0);
    }

    #[test]
    fn map_2d_tangent_round_trip() {
        let u = normalize(vec3(1.0, 2.0, 3.0));
        let (e1, e2) = build_tangent_basis(u);
        let dx = 0.6_f64;
        let dy = 0.8_f64;
        let tangent = map_2d_to_tangent(dx, dy, e1, e2);
        let (dx2, dy2) = map_tangent_to_2d(tangent, e1, e2);
        let len = (dx * dx + dy * dy).sqrt();
        assert_close(dx2, dx / len);
        assert_close(dy2, dy / len);
    }

    #[test]
    fn tangent_is_unit_vector() {
        let u = normalize(vec3(1.0, 2.0, 3.0));
        let (e1, e2) = build_tangent_basis(u);
        assert_close(length(map_2d_to_tangent(3.0, 4.0, e1, e2)), 1.0);
    }

    #[test]
    fn tangent_is_orthogonal_to_u() {
        let u = normalize(vec3(1.0, 2.0, 3.0));
        let (e1, e2) = build_tangent_basis(u);
        let tangent = map_2d_to_tangent(1.0, 1.0, e1, e2);
        assert!(dot(u, tangent).abs() < 1e-6);
    }
}
