//! Bot players that simulate human behavior in the pinball game.
//!
//! Bots are lightweight state machines that:
//! - Receive balls via `receive_ball()`
//! - Decide when to send them back based on personality
//! - Return escape velocities via `tick()`

use crate::player::Player;
use rand::Rng;

/// Bot personality affects timing and behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BotPersonality {
    /// Sends ball back quickly (0.3-0.8s delay)
    Eager,
    /// Takes time before sending (1.5-4.0s delay)
    Relaxed,
    /// Unpredictable timing and velocity (0.2-6.0s delay)
    Chaotic,
}

impl BotPersonality {
    /// Get delay range for this personality (min, max) in seconds
    fn delay_range(&self) -> (f64, f64) {
        match self {
            BotPersonality::Eager => (0.3, 0.8),
            BotPersonality::Relaxed => (1.5, 4.0),
            BotPersonality::Chaotic => (0.2, 6.0),
        }
    }

    /// Generate a random delay for this personality
    fn random_delay(&self, rng: &mut impl Rng) -> f64 {
        let (min, max) = self.delay_range();
        min + rng.gen::<f64>() * (max - min)
    }

    /// Generate velocity modification factor for this personality
    fn velocity_factor(&self, rng: &mut impl Rng) -> f64 {
        match self {
            BotPersonality::Eager => 0.9 + rng.gen::<f64>() * 0.2, // 0.9-1.1
            BotPersonality::Relaxed => 0.8 + rng.gen::<f64>() * 0.3, // 0.8-1.1
            BotPersonality::Chaotic => 0.5 + rng.gen::<f64>() * 1.0, // 0.5-1.5
        }
    }

    /// Select a random personality
    pub fn random(rng: &mut impl Rng) -> Self {
        match rng.gen_range(0..3) {
            0 => BotPersonality::Eager,
            1 => BotPersonality::Relaxed,
            _ => BotPersonality::Chaotic,
        }
    }
}

/// A pending ball waiting to be sent back
#[derive(Debug, Clone)]
struct PendingBall {
    /// Original velocity from capture
    vx: f64,
    vy: f64,
    /// Time remaining before sending
    delay: f64,
}

/// A bot player that automatically plays the game
#[derive(Debug)]
pub struct BotPlayer {
    /// The player ID (same as in GameState.players)
    pub player_id: u32,
    /// Bot's personality
    pub personality: BotPersonality,
    /// Queue of balls waiting to be sent
    pending_balls: Vec<PendingBall>,
    /// Time until bot sends an initial ball (to seed the game)
    pub initial_ball_delay: Option<f64>,
}

impl BotPlayer {
    /// Create a new bot with the given player ID and personality
    pub fn new(player_id: u32, personality: BotPersonality, rng: &mut impl Rng) -> Self {
        // Bots send an initial ball after a random delay (2-8 seconds)
        let initial_delay = 2.0 + rng.gen::<f64>() * 6.0;

        Self {
            player_id,
            personality,
            pending_balls: Vec::new(),
            initial_ball_delay: Some(initial_delay),
        }
    }

    /// Called when a ball is captured by this bot's portal
    pub fn receive_ball(&mut self, vx: f64, vy: f64, rng: &mut impl Rng) {
        let delay = self.personality.random_delay(rng);
        self.pending_balls.push(PendingBall { vx, vy, delay });
    }

    /// Tick the bot. Returns Some((vx, vy)) if the bot wants to send a ball.
    pub fn tick(&mut self, dt: f64, rng: &mut impl Rng) -> Option<(f64, f64)> {
        // Check initial ball
        if let Some(ref mut delay) = self.initial_ball_delay {
            *delay -= dt;
            if *delay <= 0.0 {
                self.initial_ball_delay = None;
                // Send a ball with random velocity
                let vx = rng.gen_range(-2.0..2.0);
                let vy = rng.gen_range(1.0..3.0);
                return Some((vx, vy));
            }
        }

        // Check pending balls
        for ball in &mut self.pending_balls {
            ball.delay -= dt;
        }

        // Find first ball ready to send
        if let Some(idx) = self.pending_balls.iter().position(|b| b.delay <= 0.0) {
            let ball = self.pending_balls.remove(idx);

            // Apply personality-based velocity modification
            let factor = self.personality.velocity_factor(rng);

            // Add some randomness to direction for Chaotic bots
            let (vx, vy) = if self.personality == BotPersonality::Chaotic {
                let angle_offset = rng.gen_range(-0.5..0.5); // radians
                let speed = (ball.vx * ball.vx + ball.vy * ball.vy).sqrt() * factor;
                let base_angle = ball.vy.atan2(ball.vx);
                let new_angle = base_angle + angle_offset;
                (speed * new_angle.cos(), speed * new_angle.sin().abs())
            } else {
                (ball.vx * factor, ball.vy.abs() * factor)
            };

            // Ensure vy is positive (ball goes into deep space)
            return Some((vx, vy.abs().max(0.5)));
        }

        None
    }

    /// Number of balls waiting to be sent
    pub fn pending_count(&self) -> usize {
        self.pending_balls.len()
    }
}

/// Configuration for bots
#[derive(Debug, Clone)]
pub struct BotConfig {
    /// Number of bots to spawn
    pub count: usize,
    /// Whether bots send initial balls to seed the game
    pub send_initial_balls: bool,
}

impl Default for BotConfig {
    fn default() -> Self {
        Self {
            count: 3,
            send_initial_balls: true,
        }
    }
}

/// Manages all bot players
#[derive(Debug)]
pub struct BotManager {
    /// List of bot players (public for testing)
    pub bots: Vec<BotPlayer>,
}

impl BotManager {
    pub fn new() -> Self {
        Self { bots: Vec::new() }
    }

    /// Add a bot for the given player
    pub fn add_bot(&mut self, player: &Player, rng: &mut impl Rng) {
        let personality = BotPersonality::random(rng);
        let bot = BotPlayer::new(player.id, personality, rng);
        tracing::info!(
            "Bot {} created with {:?} personality",
            player.id,
            personality
        );
        self.bots.push(bot);
    }

    /// Remove a bot by player ID
    pub fn remove_bot(&mut self, player_id: u32) {
        self.bots.retain(|b| b.player_id != player_id);
    }

    /// Called when a ball is captured. Routes to the appropriate bot if target is a bot.
    pub fn handle_capture(&mut self, player_id: u32, vx: f64, vy: f64, rng: &mut impl Rng) {
        if let Some(bot) = self.bots.iter_mut().find(|b| b.player_id == player_id) {
            bot.receive_ball(vx, vy, rng);
        }
    }

    /// Check if a player ID belongs to a bot
    pub fn is_bot(&self, player_id: u32) -> bool {
        self.bots.iter().any(|b| b.player_id == player_id)
    }

    /// Tick all bots. Returns list of (player_id, vx, vy) for balls to send.
    pub fn tick(&mut self, dt: f64, rng: &mut impl Rng) -> Vec<(u32, f64, f64)> {
        let mut results = Vec::new();
        for bot in &mut self.bots {
            if let Some((vx, vy)) = bot.tick(dt, rng) {
                results.push((bot.player_id, vx, vy));
            }
        }
        results
    }

    /// Get number of active bots
    pub fn bot_count(&self) -> usize {
        self.bots.len()
    }

    /// Get bot player IDs
    pub fn bot_ids(&self) -> Vec<u32> {
        self.bots.iter().map(|b| b.player_id).collect()
    }
}

impl Default for BotManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    fn test_rng() -> ChaCha8Rng {
        ChaCha8Rng::seed_from_u64(42)
    }

    #[test]
    fn bot_receives_and_sends_ball() {
        let mut rng = test_rng();
        let mut bot = BotPlayer::new(1, BotPersonality::Eager, &mut rng);

        // Disable initial ball for this test
        bot.initial_ball_delay = None;

        // Receive a ball
        bot.receive_ball(1.0, 2.0, &mut rng);
        assert_eq!(bot.pending_count(), 1);

        // Tick until ball is sent (eager bots are fast)
        let mut sent = false;
        for _ in 0..100 {
            if bot.tick(0.1, &mut rng).is_some() {
                sent = true;
                break;
            }
        }
        assert!(sent, "Bot should send ball within 10 seconds");
        assert_eq!(bot.pending_count(), 0);
    }

    #[test]
    fn eager_bot_sends_quickly() {
        let mut rng = test_rng();
        let mut bot = BotPlayer::new(1, BotPersonality::Eager, &mut rng);
        bot.initial_ball_delay = None;

        bot.receive_ball(1.0, 2.0, &mut rng);

        // Tick for 1 second in small steps
        let mut ticks = 0;
        while bot.tick(0.05, &mut rng).is_none() && ticks < 20 {
            ticks += 1;
        }
        // Eager bots should send within 0.8 seconds
        assert!(ticks <= 16, "Eager bot took {} ticks (0.8s max)", ticks);
    }

    #[test]
    fn relaxed_bot_waits_longer() {
        let mut rng = test_rng();
        let mut bot = BotPlayer::new(1, BotPersonality::Relaxed, &mut rng);
        bot.initial_ball_delay = None;

        bot.receive_ball(1.0, 2.0, &mut rng);

        // Should not send within first second
        for _ in 0..10 {
            assert!(bot.tick(0.1, &mut rng).is_none());
        }

        // But should send eventually
        let mut sent = false;
        for _ in 0..50 {
            if bot.tick(0.1, &mut rng).is_some() {
                sent = true;
                break;
            }
        }
        assert!(sent, "Relaxed bot should eventually send");
    }

    #[test]
    fn bot_sends_initial_ball() {
        let mut rng = test_rng();
        let bot = BotPlayer::new(1, BotPersonality::Eager, &mut rng);
        assert!(bot.initial_ball_delay.is_some());
    }

    #[test]
    fn bot_manager_routes_captures() {
        let mut rng = test_rng();
        let mut manager = BotManager::new();

        let player = Player {
            id: 1,
            cell_index: 0,
            portal_pos: crate::vec3::Vec3::new(1.0, 0.0, 0.0),
            color: 0xff0000,
            paused: false,
            balls_produced: 0,
        };

        manager.add_bot(&player, &mut rng);
        assert!(manager.is_bot(1));
        assert!(!manager.is_bot(2));

        // Disable initial ball
        manager.bots[0].initial_ball_delay = None;

        manager.handle_capture(1, 1.0, 2.0, &mut rng);
        assert_eq!(manager.bots[0].pending_count(), 1);
    }

    #[test]
    fn bot_manager_tick_returns_balls() {
        let mut rng = test_rng();
        let mut manager = BotManager::new();

        let player = Player {
            id: 1,
            cell_index: 0,
            portal_pos: crate::vec3::Vec3::new(1.0, 0.0, 0.0),
            color: 0xff0000,
            paused: false,
            balls_produced: 0,
        };

        manager.add_bot(&player, &mut rng);
        manager.bots[0].initial_ball_delay = None;
        manager.handle_capture(1, 1.0, 2.0, &mut rng);

        // Tick until ball is returned
        let mut results = Vec::new();
        for _ in 0..100 {
            results = manager.tick(0.1, &mut rng);
            if !results.is_empty() {
                break;
            }
        }

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, 1); // player_id
    }

    #[test]
    fn velocity_is_always_valid() {
        let mut rng = test_rng();

        for personality in [
            BotPersonality::Eager,
            BotPersonality::Relaxed,
            BotPersonality::Chaotic,
        ] {
            let mut bot = BotPlayer::new(1, personality, &mut rng);
            bot.initial_ball_delay = None;
            bot.receive_ball(1.0, 2.0, &mut rng);

            // Tick until sent
            let mut velocity = None;
            for _ in 0..200 {
                if let Some(v) = bot.tick(0.1, &mut rng) {
                    velocity = Some(v);
                    break;
                }
            }

            let (vx, vy) = velocity.expect("Should send ball");
            assert!(!vx.is_nan(), "vx is NaN for {:?}", personality);
            assert!(!vy.is_nan(), "vy is NaN for {:?}", personality);
            assert!(vy >= 0.5, "vy should be positive for {:?}", personality);
        }
    }

    #[test]
    fn chaotic_bot_has_variable_timing() {
        // Run multiple trials to verify chaotic behavior varies
        let mut send_times = Vec::new();

        for seed in 0..5 {
            let mut trial_rng = ChaCha8Rng::seed_from_u64(seed);
            let mut bot = BotPlayer::new(1, BotPersonality::Chaotic, &mut trial_rng);
            bot.initial_ball_delay = None;
            bot.receive_ball(1.0, 2.0, &mut trial_rng);

            let mut ticks = 0;
            while bot.tick(0.1, &mut trial_rng).is_none() && ticks < 100 {
                ticks += 1;
            }
            send_times.push(ticks);
        }

        // Chaotic bots should have varying send times (not all the same)
        let all_same = send_times.iter().all(|&t| t == send_times[0]);
        assert!(
            !all_same,
            "Chaotic bot should have variable timing, got {:?}",
            send_times
        );
    }

    #[test]
    fn chaotic_bot_modifies_velocity_direction() {
        let mut rng = test_rng();
        let mut bot = BotPlayer::new(1, BotPersonality::Chaotic, &mut rng);
        bot.initial_ball_delay = None;

        // Send multiple balls and collect velocities
        let mut velocities = Vec::new();
        for _ in 0..5 {
            bot.receive_ball(1.0, 2.0, &mut rng);

            // Tick until sent
            for _ in 0..100 {
                if let Some((vx, vy)) = bot.tick(0.1, &mut rng) {
                    velocities.push((vx, vy));
                    break;
                }
            }
        }

        // Chaotic bots should produce varying vx values (direction changes)
        let all_same_vx = velocities
            .iter()
            .all(|(vx, _)| (*vx - velocities[0].0).abs() < 0.01);
        assert!(
            !all_same_vx,
            "Chaotic bot should vary velocity direction, got {:?}",
            velocities
        );
    }

    #[test]
    fn remove_bot_works() {
        let mut rng = test_rng();
        let mut manager = BotManager::new();

        let player1 = Player {
            id: 1,
            cell_index: 0,
            portal_pos: crate::vec3::Vec3::new(1.0, 0.0, 0.0),
            color: 0xff0000,
            paused: false,
            balls_produced: 0,
        };
        let player2 = Player {
            id: 2,
            cell_index: 1,
            portal_pos: crate::vec3::Vec3::new(0.0, 1.0, 0.0),
            color: 0x00ff00,
            paused: false,
            balls_produced: 0,
        };

        manager.add_bot(&player1, &mut rng);
        manager.add_bot(&player2, &mut rng);

        assert_eq!(manager.bot_count(), 2);
        assert!(manager.is_bot(1));
        assert!(manager.is_bot(2));

        // Remove bot 1
        manager.remove_bot(1);

        assert_eq!(manager.bot_count(), 1);
        assert!(!manager.is_bot(1));
        assert!(manager.is_bot(2));

        // Remove bot 2
        manager.remove_bot(2);

        assert_eq!(manager.bot_count(), 0);
        assert!(!manager.is_bot(2));
    }

    #[test]
    fn remove_nonexistent_bot_is_safe() {
        let mut manager = BotManager::new();

        // Should not panic
        manager.remove_bot(999);
        assert_eq!(manager.bot_count(), 0);
    }

    #[test]
    fn bot_handles_multiple_pending_balls() {
        let mut rng = test_rng();
        let mut bot = BotPlayer::new(1, BotPersonality::Eager, &mut rng);
        bot.initial_ball_delay = None;

        // Queue multiple balls
        bot.receive_ball(1.0, 2.0, &mut rng);
        bot.receive_ball(2.0, 3.0, &mut rng);
        bot.receive_ball(3.0, 4.0, &mut rng);

        assert_eq!(bot.pending_count(), 3);

        // Tick until all balls are sent
        let mut sent_count = 0;
        for _ in 0..200 {
            if bot.tick(0.1, &mut rng).is_some() {
                sent_count += 1;
            }
            if bot.pending_count() == 0 {
                break;
            }
        }

        assert_eq!(sent_count, 3, "All 3 balls should be sent");
        assert_eq!(bot.pending_count(), 0);
    }

    #[test]
    fn bot_initial_ball_fires_after_delay() {
        let mut rng = test_rng();
        let mut bot = BotPlayer::new(1, BotPersonality::Eager, &mut rng);

        // Initial delay should be set (2-8 seconds)
        assert!(bot.initial_ball_delay.is_some());
        let delay = bot.initial_ball_delay.unwrap();
        assert!(
            delay >= 2.0 && delay <= 8.0,
            "Initial delay should be 2-8s, got {}",
            delay
        );

        // Should not fire immediately
        assert!(bot.tick(0.1, &mut rng).is_none());

        // Tick past the delay
        let mut fired = false;
        for _ in 0..100 {
            if bot.tick(0.1, &mut rng).is_some() {
                fired = true;
                break;
            }
        }

        assert!(fired, "Initial ball should fire after delay");
        assert!(
            bot.initial_ball_delay.is_none(),
            "Delay should be cleared after firing"
        );
    }

    #[test]
    fn bot_ids_returns_all_bot_ids() {
        let mut rng = test_rng();
        let mut manager = BotManager::new();

        for id in [5, 10, 15] {
            let player = Player {
                id,
                cell_index: 0,
                portal_pos: crate::vec3::Vec3::new(1.0, 0.0, 0.0),
                color: 0xff0000,
                paused: false,
                balls_produced: 0,
            };
            manager.add_bot(&player, &mut rng);
        }

        let ids = manager.bot_ids();
        assert_eq!(ids.len(), 3);
        assert!(ids.contains(&5));
        assert!(ids.contains(&10));
        assert!(ids.contains(&15));
    }
}
