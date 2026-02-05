use crate::config::DeepSpaceConfig;
use crate::player::Player;
use crate::vec3::{
    angular_distance, arbitrary_orthogonal, build_tangent_basis, cross, dot,
    get_velocity_direction, length, map_2d_to_tangent, map_tangent_to_2d, normalize,
    rotate_normalize_in_place, slerp, Vec3,
};
use rand::Rng;
use std::collections::HashMap;

/// Duration of smooth reroute transition (seconds)
const REROUTE_TRANSITION_DURATION: f64 = 4.0;

/// Deep-space ball moving on sphere surface
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpaceBall3D {
    pub id: u32,
    pub owner_id: u32,
    pub pos: Vec3,
    pub axis: Vec3,
    pub omega: f64,
    pub age: f64,
    pub time_since_hit: f64,
    pub reroute_cooldown: f64,
    /// Target axis for smooth reroute (None = no transition in progress)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reroute_target_axis: Option<Vec3>,
    /// Progress of reroute transition (0.0 to 1.0)
    #[serde(skip_serializing_if = "is_zero")]
    pub reroute_progress: f64,
    /// Target omega for smooth reroute
    #[serde(skip_serializing_if = "is_zero")]
    pub reroute_target_omega: f64,
}

fn is_zero(v: &f64) -> bool {
    *v == 0.0
}

/// Event when a ball enters a portal.
/// Contains only the essential data: player ID and computed 2D velocity.
#[derive(Debug, Clone)]
pub struct CaptureEvent {
    pub ball_id: u32,
    pub player_id: u32,
    /// Original owner of the ball (for color)
    pub ball_owner_id: u32,
    /// Color of the ball (from original owner)
    pub ball_color: u32,
    /// 2D velocity for TransferIn (pre-computed, no need for ball/player clones)
    pub vx: f64,
    pub vy: f64,
}

/// Sphere deep space simulation.
pub struct SphereDeepSpace {
    config: DeepSpaceConfig,
    cos_portal_alpha: f64,
    balls: HashMap<u32, SpaceBall3D>,
    players: Vec<Player>,
    next_ball_id: u32,
    capture_buffer: Vec<CaptureEvent>,
    /// Speed at which captured balls enter the board (m/s)
    capture_speed: f64,
}

impl SphereDeepSpace {
    pub fn new(config: DeepSpaceConfig, capture_speed: f64) -> Self {
        let cos_portal_alpha = config.portal_alpha.cos();
        Self {
            config,
            cos_portal_alpha,
            balls: HashMap::new(),
            players: Vec::new(),
            next_ball_id: 1,
            capture_buffer: Vec::new(),
            capture_speed,
        }
    }

    /// Update player list
    pub fn set_players(&mut self, players: Vec<Player>) {
        self.players = players;
    }

    /// Add a ball to deep space from an escape.
    pub fn add_ball(
        &mut self,
        owner_id: u32,
        portal_pos: Vec3,
        vx: f64,
        vy: f64,
        rng: &mut impl Rng,
    ) -> u32 {
        let id = self.next_ball_id;
        self.next_ball_id = self.next_ball_id.wrapping_add(1);

        let (e1, e2) = build_tangent_basis(portal_pos);
        let tangent = map_2d_to_tangent(vx, vy, e1, e2);

        let cross_vec = cross(portal_pos, tangent);
        let cross_len = length(cross_vec);

        let axis = if cross_len < 0.01 {
            arbitrary_orthogonal(portal_pos)
        } else {
            Vec3::new(
                cross_vec.x / cross_len,
                cross_vec.y / cross_len,
                cross_vec.z / cross_len,
            )
        };

        let omega = self.config.omega_min
            + rng.gen::<f64>() * (self.config.omega_max - self.config.omega_min);

        let pos = normalize(portal_pos);

        let ball = SpaceBall3D {
            id,
            owner_id,
            pos,
            axis,
            omega,
            age: 0.0,
            time_since_hit: 0.0,
            reroute_cooldown: 0.0,
            reroute_target_axis: None,
            reroute_progress: 0.0,
            reroute_target_omega: 0.0,
        };

        self.balls.insert(id, ball);
        id
    }

    /// Get all balls (allocates a new vec)
    pub fn get_balls(&self) -> Vec<&SpaceBall3D> {
        self.balls.values().collect()
    }

    /// Get an iterable view of balls
    pub fn get_ball_iter(&self) -> impl Iterator<Item = &SpaceBall3D> {
        self.balls.values()
    }

    /// Get a specific ball
    pub fn get_ball(&self, id: u32) -> Option<&SpaceBall3D> {
        self.balls.get(&id)
    }

    /// Get a mutable reference to a specific ball (for testing)
    pub fn get_ball_mut(&mut self, id: u32) -> Option<&mut SpaceBall3D> {
        self.balls.get_mut(&id)
    }

    /// Simulate one tick.
    pub fn tick(&mut self, dt: f64, rng: &mut impl Rng) -> Vec<CaptureEvent> {
        // Take buffer out of self to avoid borrow conflicts
        let mut captures = std::mem::take(&mut self.capture_buffer);
        captures.clear();

        let cos_portal_alpha = self.cos_portal_alpha;
        let min_age = self.config.min_age_for_capture;
        let min_age_reroute = self.config.min_age_for_reroute;
        let reroute_after = self.config.reroute_after;
        let reroute_cd = self.config.reroute_cooldown;
        let omega_min = self.config.omega_min;
        let omega_max = self.config.omega_max;
        let arrival_time_min = self.config.reroute_arrival_time_min;
        let arrival_time_max = self.config.reroute_arrival_time_max;
        let capture_speed = self.capture_speed;
        let players = &self.players;

        for ball in self.balls.values_mut() {
            // Update position in-place
            rotate_normalize_in_place(&mut ball.pos, ball.axis, ball.omega * dt);

            // Update timers
            ball.age += dt;
            ball.time_since_hit += dt;
            ball.reroute_cooldown = (ball.reroute_cooldown - dt).max(0.0);

            // Check portal hits (only if old enough)
            // Select portal with highest dot product to avoid bias toward first player
            // Skip paused players - they don't capture balls
            let mut captured = false;
            if ball.age >= min_age {
                let mut best_match: Option<(&Player, f64)> = None;
                for player in players {
                    // Skip paused players
                    if player.paused {
                        continue;
                    }
                    // Bots don't capture their own balls (so balls can reach other players)
                    if player.is_bot && player.id == ball.owner_id {
                        continue;
                    }
                    let p = player.portal_pos;
                    let d = ball.pos.x * p.x + ball.pos.y * p.y + ball.pos.z * p.z;
                    if d >= cos_portal_alpha {
                        if best_match.map_or(true, |(_, best_d)| d > best_d) {
                            best_match = Some((player, d));
                        }
                    }
                }
                if let Some((player, _)) = best_match {
                    // Compute 2D velocity at capture (no cloning needed)
                    let vel_dir = get_velocity_direction(ball.pos, ball.axis, ball.omega);
                    let (e1, e2) = build_tangent_basis(player.portal_pos);
                    let (dx, dy) = map_tangent_to_2d(vel_dir, e1, e2);
                    let len = (dx * dx + dy * dy).sqrt();
                    let (vx, vy) = if len < 0.01 {
                        (0.0, capture_speed)
                    } else {
                        // vy must always be positive (downward into the board)
                        // The ball enters from the top, so it always moves down
                        ((dx / len) * capture_speed, (dy / len).abs() * capture_speed)
                    };

                    // Find original owner's color
                    let ball_color = players
                        .iter()
                        .find(|p| p.id == ball.owner_id)
                        .map(|p| p.color)
                        .unwrap_or(0xffffff);

                    captures.push(CaptureEvent {
                        ball_id: ball.id,
                        player_id: player.id,
                        ball_owner_id: ball.owner_id,
                        ball_color,
                        vx,
                        vy,
                    });
                    captured = true;
                }
            }

            // Process ongoing reroute transition (smooth interpolation)
            if let Some(target_axis) = ball.reroute_target_axis {
                ball.reroute_progress += dt / REROUTE_TRANSITION_DURATION;

                if ball.reroute_progress >= 1.0 {
                    // Transition complete
                    ball.axis = target_axis;
                    ball.omega = ball.reroute_target_omega;
                    ball.reroute_target_axis = None;
                    ball.reroute_progress = 0.0;
                    ball.reroute_target_omega = 0.0;
                } else {
                    // Smoothly interpolate axis using slerp
                    // Use quintic smoothstep for very gradual easing: 6t⁵ - 15t⁴ + 10t³
                    let t = ball.reroute_progress;
                    let smooth_t = t * t * t * (t * (t * 6.0 - 15.0) + 10.0);

                    // Blend very gradually - small incremental changes each frame
                    // The blend factor increases slowly, making the curve bend gently
                    let blend = smooth_t * 0.03;
                    ball.axis = slerp(ball.axis, target_axis, blend);
                    ball.axis = normalize(ball.axis);

                    // Smoothly interpolate omega (also very gradual)
                    ball.omega = ball.omega + (ball.reroute_target_omega - ball.omega) * blend;
                }
            }

            // Start new reroute if not captured, no transition in progress, and conditions met
            if !captured
                && ball.reroute_target_axis.is_none()
                && ball.age >= min_age_reroute
                && ball.time_since_hit >= reroute_after
                && ball.reroute_cooldown <= 0.0
                && !players.is_empty()
            {
                let target_idx = rng.gen_range(0..players.len());
                let target_pos = players[target_idx].portal_pos;

                let dot_pos_target = dot(ball.pos, target_pos);
                if dot_pos_target > 0.99 {
                    // Already very close to target, just set cooldown
                    ball.reroute_cooldown = reroute_cd;
                } else {
                    // Calculate target axis (direction to rotate toward target)
                    let cross_vec = cross(ball.pos, target_pos);
                    let cross_len = length(cross_vec);

                    let new_axis = if cross_len < 0.01 {
                        arbitrary_orthogonal(ball.pos)
                    } else {
                        Vec3::new(
                            cross_vec.x / cross_len,
                            cross_vec.y / cross_len,
                            cross_vec.z / cross_len,
                        )
                    };

                    // Calculate target omega
                    let delta = angular_distance(ball.pos, target_pos);
                    let t =
                        arrival_time_min + rng.gen::<f64>() * (arrival_time_max - arrival_time_min);
                    let new_omega = (delta / t).clamp(omega_min, omega_max);

                    // Start smooth transition
                    ball.reroute_target_axis = Some(new_axis);
                    ball.reroute_target_omega = new_omega;
                    ball.reroute_progress = 0.0;

                    ball.time_since_hit = 0.0;
                    ball.reroute_cooldown = reroute_cd;
                }
            }
        }

        // Remove captured balls
        for cap in &captures {
            self.balls.remove(&cap.ball_id);
        }

        // Return buffer for reuse next tick
        let result = captures.clone();
        self.capture_buffer = captures;
        result
    }

    /// Ball count
    pub fn ball_count(&self) -> usize {
        self.balls.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vec3::vec3;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    fn test_config() -> DeepSpaceConfig {
        DeepSpaceConfig {
            portal_alpha: 0.1,
            omega_min: 1.0,
            omega_max: 1.0,
            reroute_after: 10.0,
            reroute_cooldown: 5.0,
            min_age_for_capture: 0.5,
            min_age_for_reroute: 2.0,
            reroute_arrival_time_min: 4.0,
            reroute_arrival_time_max: 10.0,
        }
    }

    fn test_rng() -> ChaCha8Rng {
        ChaCha8Rng::seed_from_u64(42)
    }

    fn create_test_players() -> Vec<Player> {
        vec![
            Player {
                id: 1,
                cell_index: 0,
                portal_pos: vec3(1.0, 0.0, 0.0),
                color: 0xff0000,
                paused: false,
                balls_produced: 0,
                is_bot: false,
            },
            Player {
                id: 2,
                cell_index: 1,
                portal_pos: vec3(0.0, 1.0, 0.0),
                color: 0x00ff00,
                paused: false,
                balls_produced: 0,
                is_bot: false,
            },
            Player {
                id: 3,
                cell_index: 2,
                portal_pos: vec3(0.0, 0.0, 1.0),
                color: 0x0000ff,
                paused: false,
                balls_produced: 0,
                is_bot: false,
            },
            Player {
                id: 4,
                cell_index: 3,
                portal_pos: vec3(-1.0, 0.0, 0.0),
                color: 0xffff00,
                paused: false,
                balls_produced: 0,
                is_bot: false,
            },
        ]
    }

    const TEST_CAPTURE_SPEED: f64 = 1.5;

    fn setup() -> (SphereDeepSpace, ChaCha8Rng) {
        let mut ds = SphereDeepSpace::new(test_config(), TEST_CAPTURE_SPEED);
        ds.set_players(create_test_players());
        (ds, test_rng())
    }

    // --- addBall ---

    #[test]
    fn add_ball_correct_owner() {
        let (mut ds, mut rng) = setup();
        let id = ds.add_ball(1, vec3(1.0, 0.0, 0.0), 1.0, 0.0, &mut rng);
        assert_eq!(ds.get_ball(id).unwrap().owner_id, 1);
    }

    #[test]
    fn add_ball_pos_is_unit() {
        let (mut ds, mut rng) = setup();
        let id = ds.add_ball(1, vec3(1.0, 0.0, 0.0), 1.0, 0.0, &mut rng);
        assert!((length(ds.get_ball(id).unwrap().pos) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn add_ball_axis_is_unit() {
        let (mut ds, mut rng) = setup();
        let id = ds.add_ball(1, vec3(1.0, 0.0, 0.0), 1.0, 0.0, &mut rng);
        assert!((length(ds.get_ball(id).unwrap().axis) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn add_ball_starts_with_age_zero() {
        let (mut ds, mut rng) = setup();
        let id = ds.add_ball(1, vec3(1.0, 0.0, 0.0), 1.0, 0.0, &mut rng);
        assert_eq!(ds.get_ball(id).unwrap().age, 0.0);
    }

    #[test]
    fn add_ball_starts_at_portal() {
        let (mut ds, mut rng) = setup();
        let portal_pos = vec3(1.0, 0.0, 0.0);
        let id = ds.add_ball(1, portal_pos, 1.0, 0.0, &mut rng);
        assert!((dot(ds.get_ball(id).unwrap().pos, portal_pos) - 1.0).abs() < 1e-6);
    }

    // --- tick - movement ---

    #[test]
    fn ball_moves_on_great_circle() {
        let (mut ds, mut rng) = setup();
        let id = ds.add_ball(1, vec3(1.0, 0.0, 0.0), 0.0, 1.0, &mut rng);
        let initial_x = ds.get_ball(id).unwrap().pos.x;
        ds.tick(0.1, &mut rng);
        let ball = ds.get_ball(id).unwrap();
        assert!((ball.pos.x - initial_x).abs() > 0.001);
        assert!((length(ball.pos) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn ball_age_increases() {
        let (mut ds, mut rng) = setup();
        let id = ds.add_ball(1, vec3(1.0, 0.0, 0.0), 1.0, 0.0, &mut rng);
        ds.tick(0.5, &mut rng);
        assert!((ds.get_ball(id).unwrap().age - 0.5).abs() < 1e-9);
    }

    #[test]
    fn multiple_ticks_accumulate_age() {
        let (mut ds, mut rng) = setup();
        let id = ds.add_ball(1, vec3(1.0, 0.0, 0.0), 1.0, 0.0, &mut rng);
        ds.tick(0.1, &mut rng);
        ds.tick(0.1, &mut rng);
        ds.tick(0.1, &mut rng);
        assert!((ds.get_ball(id).unwrap().age - 0.3).abs() < 1e-9);
    }

    // --- tick - capture ---

    #[test]
    fn not_captured_before_min_age() {
        let (mut ds, mut rng) = setup();
        let id = ds.add_ball(1, vec3(0.0, 1.0, 0.0), 0.01, 0.0, &mut rng);
        ds.get_ball_mut(id).unwrap().pos = normalize(vec3(0.0, 1.0, 0.0));
        let captures = ds.tick(0.1, &mut rng);
        assert!(captures.is_empty());
        assert!(ds.get_ball(id).unwrap().age < test_config().min_age_for_capture);
    }

    #[test]
    fn captured_when_at_portal_and_old_enough() {
        let (mut ds, mut rng) = setup();
        let id = ds.add_ball(1, vec3(1.0, 0.0, 0.0), 1.0, 0.0, &mut rng);
        {
            let ball = ds.get_ball_mut(id).unwrap();
            ball.age = test_config().min_age_for_capture + 0.1;
            ball.pos = normalize(vec3(0.0, 1.0, 0.0));
        }
        let captures = ds.tick(0.01, &mut rng);
        assert_eq!(captures.len(), 1);
        assert_eq!(captures[0].player_id, 2);
        assert_eq!(captures[0].ball_id, id);
    }

    #[test]
    fn paused_player_does_not_capture() {
        let mut ds = SphereDeepSpace::new(test_config(), TEST_CAPTURE_SPEED);
        // Create players where player 2 is paused
        let mut players = create_test_players();
        players[1].paused = true; // Player 2 at (0, 1, 0) is paused
        ds.set_players(players);
        let mut rng = test_rng();

        let id = ds.add_ball(1, vec3(1.0, 0.0, 0.0), 1.0, 0.0, &mut rng);
        {
            let ball = ds.get_ball_mut(id).unwrap();
            ball.age = test_config().min_age_for_capture + 0.1;
            ball.pos = normalize(vec3(0.0, 1.0, 0.0)); // At player 2's portal
        }
        let captures = ds.tick(0.01, &mut rng);
        // Ball should NOT be captured because player 2 is paused
        assert!(captures.is_empty());
        // Ball should still exist
        assert!(ds.get_ball(id).is_some());
    }

    #[test]
    fn captured_ball_is_removed() {
        let (mut ds, mut rng) = setup();
        let id = ds.add_ball(1, vec3(1.0, 0.0, 0.0), 1.0, 0.0, &mut rng);
        {
            let ball = ds.get_ball_mut(id).unwrap();
            ball.age = test_config().min_age_for_capture + 0.1;
            ball.pos = normalize(vec3(0.0, 1.0, 0.0));
        }
        ds.tick(0.01, &mut rng);
        assert!(ds.get_ball(id).is_none());
    }

    #[test]
    fn capture_event_contains_ball_data() {
        let (mut ds, mut rng) = setup();
        let id = ds.add_ball(1, vec3(1.0, 0.0, 0.0), 1.0, 0.0, &mut rng);
        {
            let ball = ds.get_ball_mut(id).unwrap();
            ball.age = test_config().min_age_for_capture + 0.1;
            ball.pos = normalize(vec3(0.0, 0.0, 1.0));
        }
        let captures = ds.tick(0.01, &mut rng);
        // Ball owned by player 1, captured by player 3 (portal at z=1)
        assert_eq!(captures[0].ball_id, id);
        assert_eq!(captures[0].player_id, 3);
    }

    // --- captured balls not rerouted ---

    #[test]
    fn captured_ball_axis_not_mutated_by_reroute() {
        // This test verified that the ball's axis wasn't mutated by reroute before capture.
        // Now that CaptureEvent doesn't contain ball data, we just verify capture happens.
        let (mut ds, mut rng) = setup();
        let id = ds.add_ball(1, vec3(1.0, 0.0, 0.0), 1.0, 0.0, &mut rng);
        {
            let ball = ds.get_ball_mut(id).unwrap();
            ball.age = test_config().min_age_for_capture + 0.1;
            ball.pos = normalize(vec3(0.0, 1.0, 0.0));
            ball.time_since_hit = test_config().reroute_after + 1.0;
            ball.reroute_cooldown = 0.0;
        }
        let captures = ds.tick(0.01, &mut rng);
        assert_eq!(captures.len(), 1);
        assert_eq!(captures[0].ball_id, id);
        assert_eq!(captures[0].player_id, 2); // portal at y=1
    }

    // --- reroute ---

    #[test]
    fn ball_is_rerouted_after_reroute_after_seconds() {
        let (mut ds, mut rng) = setup();
        let id = ds.add_ball(1, vec3(1.0, 0.0, 0.0), 1.0, 0.0, &mut rng);
        {
            let ball = ds.get_ball_mut(id).unwrap();
            ball.age = test_config().reroute_after + 1.0;
            ball.time_since_hit = test_config().reroute_after + 1.0;
            ball.reroute_cooldown = 0.0;
            ball.pos = normalize(vec3(1.0, 1.0, 1.0));
        }
        // First tick starts the transition
        ds.tick(0.01, &mut rng);
        let ball = ds.get_ball(id).unwrap();
        // Reroute now starts a smooth transition instead of instant change
        assert!(
            ball.reroute_target_axis.is_some(),
            "Reroute should start a smooth transition"
        );

        // Second tick should advance progress
        ds.tick(0.01, &mut rng);
        let ball = ds.get_ball(id).unwrap();
        assert!(ball.reroute_progress > 0.0, "Progress should have advanced");
    }

    #[test]
    fn reroute_sets_cooldown() {
        let (mut ds, mut rng) = setup();
        let id = ds.add_ball(1, vec3(1.0, 0.0, 0.0), 1.0, 0.0, &mut rng);
        {
            let ball = ds.get_ball_mut(id).unwrap();
            ball.pos = normalize(vec3(1.0, 1.0, 1.0));
            ball.age = test_config().reroute_after + 1.0;
            ball.time_since_hit = test_config().reroute_after + 1.0;
            ball.reroute_cooldown = 0.0;
        }
        ds.tick(0.01, &mut rng);
        assert!(ds.get_ball(id).unwrap().reroute_cooldown > 0.0);
    }

    #[test]
    fn reroute_resets_time_since_hit() {
        let (mut ds, mut rng) = setup();
        let id = ds.add_ball(1, vec3(1.0, 0.0, 0.0), 1.0, 0.0, &mut rng);
        {
            let ball = ds.get_ball_mut(id).unwrap();
            ball.pos = normalize(vec3(1.0, 1.0, 1.0));
            ball.age = test_config().reroute_after + 1.0;
            ball.time_since_hit = test_config().reroute_after + 1.0;
            ball.reroute_cooldown = 0.0;
        }
        ds.tick(0.01, &mut rng);
        assert!(ds.get_ball(id).unwrap().time_since_hit < 1.0);
    }

    // --- getBalls ---

    #[test]
    fn get_balls_returns_all() {
        let (mut ds, mut rng) = setup();
        ds.add_ball(1, vec3(1.0, 0.0, 0.0), 1.0, 0.0, &mut rng);
        ds.add_ball(2, vec3(0.0, 1.0, 0.0), 0.0, 1.0, &mut rng);
        assert_eq!(ds.get_balls().len(), 2);
    }

    #[test]
    fn get_balls_empty() {
        let (ds, _) = setup();
        assert!(ds.get_balls().is_empty());
    }

    // --- capture velocity ---

    #[test]
    fn capture_velocity_correct_magnitude() {
        // Create deep space with specific capture speed
        let capture_speed = 2.5;
        let mut ds = SphereDeepSpace::new(test_config(), capture_speed);
        ds.set_players(create_test_players());
        let mut rng = test_rng();

        let id = ds.add_ball(1, vec3(1.0, 0.0, 0.0), 1.0, 0.0, &mut rng);
        {
            let ball = ds.get_ball_mut(id).unwrap();
            // Move ball to player 2's portal (y=1) and make it old enough
            ball.pos = normalize(vec3(0.0, 1.0, 0.0));
            ball.age = test_config().min_age_for_capture + 0.1;
        }

        let captures = ds.tick(0.01, &mut rng);
        assert_eq!(captures.len(), 1);

        let cap = &captures[0];
        let actual_speed = (cap.vx * cap.vx + cap.vy * cap.vy).sqrt();
        assert!((actual_speed - capture_speed).abs() < 1e-6);
    }

    // --- edge cases ---

    #[test]
    fn reroute_handles_near_antiparallel() {
        // Use high min_age to prevent capture during reroute test
        let mut config = test_config();
        config.min_age_for_capture = 999.0;
        let mut ds = SphereDeepSpace::new(config, TEST_CAPTURE_SPEED);
        ds.set_players(vec![
            Player {
                id: 1,
                cell_index: 0,
                portal_pos: vec3(1.0, 0.0, 0.0),
                color: 0xff0000,
                paused: false,
                balls_produced: 0,
                is_bot: false,
            },
            Player {
                id: 2,
                cell_index: 1,
                portal_pos: vec3(-1.0, 0.0, 0.0),
                color: 0x00ff00,
                paused: false,
                balls_produced: 0,
                is_bot: false,
            },
        ]);
        let mut rng = test_rng();
        let id = ds.add_ball(1, vec3(1.0, 0.0, 0.0), 1.0, 0.0, &mut rng);
        {
            let ball = ds.get_ball_mut(id).unwrap();
            ball.pos = normalize(vec3(0.999, 0.01, 0.01));
            ball.age = test_config().reroute_after + 1.0;
            ball.time_since_hit = test_config().reroute_after + 1.0;
            ball.reroute_cooldown = 0.0;
        }
        ds.tick(0.01, &mut rng);
        let ball = ds.get_ball(id).unwrap();
        assert!((length(ball.pos) - 1.0).abs() < 1e-6);
        assert!((length(ball.axis) - 1.0).abs() < 1e-6);
        assert!(!ball.pos.x.is_nan());
        assert!(!ball.axis.x.is_nan());
    }

    #[test]
    fn reroute_handles_ball_close_to_target() {
        // Use high min_age to prevent capture during reroute test
        let mut config = test_config();
        config.min_age_for_capture = 999.0;
        let mut ds = SphereDeepSpace::new(config, TEST_CAPTURE_SPEED);
        ds.set_players(create_test_players());
        let mut rng = test_rng();
        let id = ds.add_ball(1, vec3(1.0, 0.0, 0.0), 1.0, 0.0, &mut rng);
        {
            let ball = ds.get_ball_mut(id).unwrap();
            ball.pos = normalize(vec3(0.001, 0.9999, 0.001));
            ball.age = test_config().reroute_after + 1.0;
            ball.time_since_hit = test_config().reroute_after + 1.0;
            ball.reroute_cooldown = 0.0;
        }
        ds.tick(0.01, &mut rng);
        let ball = ds.get_ball(id).unwrap();
        assert!(!ball.pos.x.is_nan());
        assert!(!ball.axis.x.is_nan());
        assert!((length(ball.pos) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn capture_at_exact_threshold() {
        let (mut ds, mut rng) = setup();
        let cos_alpha = test_config().portal_alpha.cos();
        let id = ds.add_ball(1, vec3(1.0, 0.0, 0.0), 1.0, 0.0, &mut rng);
        {
            let ball = ds.get_ball_mut(id).unwrap();
            ball.age = test_config().min_age_for_capture + 0.1;
            let sin_alpha = (1.0 - cos_alpha * cos_alpha).sqrt();
            ball.pos = normalize(vec3(sin_alpha, cos_alpha, 0.0));
        }
        let captures = ds.tick(0.001, &mut rng);
        assert_eq!(captures.len(), 1);
        assert_eq!(captures[0].player_id, 2);
    }

    #[test]
    fn no_capture_outside_threshold() {
        let (mut ds, mut rng) = setup();
        let outside_angle = test_config().portal_alpha + 0.05;
        let cos_outside = outside_angle.cos();
        let sin_outside = (1.0 - cos_outside * cos_outside).sqrt();
        let id = ds.add_ball(1, vec3(1.0, 0.0, 0.0), 1.0, 0.0, &mut rng);
        {
            let ball = ds.get_ball_mut(id).unwrap();
            ball.age = test_config().min_age_for_capture + 0.1;
            ball.pos = normalize(vec3(sin_outside, cos_outside, 0.0));
        }
        let captures = ds.tick(0.001, &mut rng);
        assert!(captures.is_empty());
    }

    #[test]
    fn add_ball_zero_velocity_valid() {
        let (mut ds, mut rng) = setup();
        let id = ds.add_ball(1, vec3(1.0, 0.0, 0.0), 0.0, 0.0, &mut rng);
        let ball = ds.get_ball(id).unwrap();
        assert!((length(ball.pos) - 1.0).abs() < 1e-6);
        assert!((length(ball.axis) - 1.0).abs() < 1e-6);
        assert!(!ball.omega.is_nan());
    }

    #[test]
    fn ball_stays_on_sphere_after_many_ticks() {
        let (mut ds, mut rng) = setup();
        let id = ds.add_ball(1, vec3(1.0, 0.0, 0.0), 1.0, 1.0, &mut rng);
        for _ in 0..1000 {
            ds.tick(0.016, &mut rng);
        }
        if let Some(ball) = ds.get_ball(id) {
            assert!((length(ball.pos) - 1.0).abs() < 1e-6);
            assert!(!ball.pos.x.is_nan());
        }
    }

    // --- end-to-end pipeline ---

    #[test]
    fn escape_travel_capture_velocity() {
        let speed_2d = 2.0;
        let config = DeepSpaceConfig {
            portal_alpha: 0.1,
            omega_min: 1.0,
            omega_max: 1.0,
            reroute_after: 100.0,
            reroute_cooldown: 100.0,
            min_age_for_capture: 0.1,
            min_age_for_reroute: 2.0,
            reroute_arrival_time_min: 4.0,
            reroute_arrival_time_max: 10.0,
        };
        let mut ds = SphereDeepSpace::new(config, speed_2d);
        let mut rng = test_rng();

        let p1_pos = vec3(1.0, 0.0, 0.0);
        let p2_pos = vec3(-1.0, 0.0, 0.0);
        ds.set_players(vec![
            Player {
                id: 1,
                cell_index: 0,
                portal_pos: p1_pos,
                color: 0xff0000,
                paused: false,
                balls_produced: 0,
                is_bot: false,
            },
            Player {
                id: 2,
                cell_index: 1,
                portal_pos: p2_pos,
                color: 0x00ff00,
                paused: false,
                balls_produced: 0,
                is_bot: false,
            },
        ]);

        ds.add_ball(1, p1_pos, 0.0, 1.0, &mut rng);
        assert_eq!(ds.ball_count(), 1);

        let mut capture_event = None;
        for _ in 0..10000 {
            let captures = ds.tick(1.0 / 60.0, &mut rng);
            if !captures.is_empty() {
                capture_event = Some(captures.into_iter().next().unwrap());
                break;
            }
        }

        let cap = capture_event.expect("Ball should be captured");
        assert_eq!(ds.ball_count(), 0);

        // vx/vy are now pre-computed in the capture event with speed_2d
        let actual_speed = (cap.vx * cap.vx + cap.vy * cap.vy).sqrt();
        assert!((actual_speed - speed_2d).abs() < 1e-4);
        assert!(!cap.vx.is_nan());
        assert!(!cap.vy.is_nan());
    }

    // --- sanity long-run ---

    #[test]
    fn sanity_300_players_200_balls_60s() {
        let config = DeepSpaceConfig {
            portal_alpha: 0.15,
            omega_min: 0.5,
            omega_max: 1.0,
            reroute_after: 12.0,
            reroute_cooldown: 6.0,
            min_age_for_capture: 3.0,
            min_age_for_reroute: 2.0,
            reroute_arrival_time_min: 4.0,
            reroute_arrival_time_max: 10.0,
        };
        let mut ds = SphereDeepSpace::new(config, TEST_CAPTURE_SPEED);
        let mut rng = ChaCha8Rng::seed_from_u64(123);

        let mut placement = crate::sphere::PortalPlacement::new(2048, &mut rng);
        let mut players = Vec::new();
        for i in 1..=300u32 {
            let cell_index = placement.allocate(None).unwrap();
            players.push(Player {
                id: i,
                cell_index: cell_index as u32,
                portal_pos: placement.portal_pos(cell_index),
                color: 0xffffff,
                paused: false,
                balls_produced: 0,
                is_bot: false,
            });
        }
        ds.set_players(players.clone());

        for i in 0..200 {
            let owner = &players[i % players.len()];
            let vx = rng.gen::<f64>() * 2.0 - 1.0;
            let vy = rng.gen::<f64>() * 2.0 - 1.0;
            ds.add_ball(owner.id, owner.portal_pos, vx, vy, &mut rng);
        }
        assert_eq!(ds.ball_count(), 200);

        let mut total_captures = 0;
        let dt = 1.0 / 60.0;
        let total_ticks = 60 * 60;
        let start = std::time::Instant::now();

        for _ in 0..total_ticks {
            let captures = ds.tick(dt, &mut rng);
            for cap in &captures {
                // Verify capture velocities are valid
                assert!(!cap.vx.is_nan());
                assert!(!cap.vy.is_nan());
            }
            total_captures += captures.len();
        }

        let elapsed = start.elapsed();

        for ball in ds.get_ball_iter() {
            assert!(!ball.pos.x.is_nan());
            assert!(!ball.pos.y.is_nan());
            assert!(!ball.pos.z.is_nan());
            assert!((length(ball.pos) - 1.0).abs() < 1e-3);
            assert!(!ball.axis.x.is_nan());
            assert!(!ball.omega.is_nan());
        }

        assert!(total_captures > 0);
        assert_eq!(ds.ball_count() + total_captures, 200);
        assert!(elapsed.as_millis() < 5000, "Took too long: {:?}", elapsed);
    }
}
