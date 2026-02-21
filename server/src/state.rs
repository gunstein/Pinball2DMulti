use crate::bot::BotManager;
use crate::config::{DeepSpaceConfig, ServerConfig};
use crate::deep_space::{CaptureEvent, SphereDeepSpace};
use crate::player::{color_from_id, Player};
use crate::protocol::{ball_to_wire, player_to_wire, PlayersStateMsg, SpaceStateMsg};
use crate::sphere::PortalPlacement;
use rand_chacha::ChaCha8Rng;
use std::collections::HashMap;

/// How long (seconds) since last activity before a player is considered inactive.
const ACTIVITY_TIMEOUT: f64 = 30.0;

/// Central game state owned by the game loop task.
pub struct GameState {
    pub deep_space: SphereDeepSpace,
    pub placement: PortalPlacement,
    pub players: HashMap<u32, Player>,
    pub config: DeepSpaceConfig,
    pub rng: ChaCha8Rng,
    pub bots: BotManager,
    next_player_id: u32,
    /// Global maximum balls in deep space
    max_balls_global: usize,
    /// Elapsed server time in seconds (incremented each tick)
    elapsed: f64,
    /// Whether there were active players last tick (used to detect reactivation)
    was_active: bool,
}

impl GameState {
    pub fn new(
        server_config: &ServerConfig,
        deep_space_config: DeepSpaceConfig,
        capture_speed: f64,
    ) -> Self {
        use rand::SeedableRng;
        let mut rng = ChaCha8Rng::seed_from_u64(server_config.rng_seed);
        let placement = PortalPlacement::new(server_config.cell_count, &mut rng);
        let deep_space = SphereDeepSpace::new(deep_space_config.clone(), capture_speed);

        let mut state = Self {
            deep_space,
            placement,
            players: HashMap::new(),
            config: deep_space_config,
            rng,
            bots: BotManager::new(),
            next_player_id: 1,
            max_balls_global: server_config.max_balls_global,
            elapsed: 0.0,
            was_active: false,
        };

        // Spawn bots
        for _ in 0..server_config.bot_count {
            state.add_bot();
        }

        state
    }

    /// Add a bot player. Returns the player ID if successful.
    pub fn add_bot(&mut self) -> Option<u32> {
        let (id, _) = self.add_player_internal(true)?;
        let player = self.players.get(&id)?;
        self.bots.add_bot(player, &mut self.rng);
        Some(id)
    }

    /// Add a new player, returns (player_id, Player)
    pub fn add_player(&mut self) -> Option<(u32, Player)> {
        self.add_player_internal(false)
    }

    /// Internal: Add a new player with is_bot flag
    fn add_player_internal(&mut self, is_bot: bool) -> Option<(u32, Player)> {
        let cell_index = self.placement.allocate(None)?;
        let id = self.next_player_id;
        self.next_player_id += 1;

        let player = Player {
            id,
            cell_index: cell_index as u32,
            portal_pos: self.placement.portal_pos(cell_index),
            color: color_from_id(id),
            paused: false,
            balls_produced: 0,
            is_bot,
            last_activity: 0.0,
        };

        self.players.insert(id, player.clone());
        self.sync_players_to_deep_space();
        Some((id, player))
    }

    /// Remove a player
    pub fn remove_player(&mut self, id: u32) {
        if let Some(player) = self.players.remove(&id) {
            self.placement.release(player.cell_index as usize);
            self.sync_players_to_deep_space();
        }
    }

    /// Set a player's paused state. Returns true if player exists and state changed.
    pub fn set_player_paused(&mut self, id: u32, paused: bool) -> bool {
        if let Some(player) = self.players.get_mut(&id) {
            if player.paused != paused {
                player.paused = paused;
                self.sync_players_to_deep_space();
                return true;
            }
        }
        false
    }

    /// Record player activity (called when server receives an activity heartbeat).
    pub fn player_activity(&mut self, id: u32) {
        if let Some(player) = self.players.get_mut(&id) {
            player.last_activity = self.elapsed;
        }
    }

    /// Check if any real (non-bot) player has been active recently.
    pub fn has_active_players(&self) -> bool {
        self.players
            .values()
            .any(|p| !p.is_bot && !p.paused && (self.elapsed - p.last_activity) < ACTIVITY_TIMEOUT)
    }

    /// Tick the deep-space simulation and bots.
    /// Returns captures for real players only (bot captures are handled internally).
    pub fn tick(&mut self, dt: f64) -> Vec<CaptureEvent> {
        self.elapsed += dt;

        let all_captures = self.deep_space.tick(dt, &mut self.rng);

        // Detect transition from inactive → active: flush stale pending bot balls
        let has_active = self.has_active_players();
        if has_active && !self.was_active {
            self.bots.clear_pending();
        }
        self.was_active = has_active;
        let mut real_captures = Vec::new();
        for cap in all_captures {
            if self.bots.is_bot(cap.player_id) {
                // Only queue bot captures when there are active players.
                // During inactivity we discard the ball so pending queues don't
                // accumulate and flood deep-space the moment a player returns.
                if has_active {
                    self.bots
                        .handle_capture(cap.player_id, cap.vx, cap.vy, &mut self.rng);
                }
            } else {
                // Real player - return the capture event
                real_captures.push(cap);
            }
        }

        // Tick bots - they may send balls (only when active players exist)
        let real_player_count = self
            .players
            .values()
            .filter(|p| {
                !p.is_bot && !p.paused && (self.elapsed - p.last_activity) < ACTIVITY_TIMEOUT
            })
            .count();
        let bot_balls = self
            .bots
            .tick(dt, &mut self.rng, real_player_count, has_active);
        for (bot_id, vx, vy) in bot_balls {
            self.ball_escaped(bot_id, vx, vy);
        }

        real_captures
    }

    /// Add a ball escaped from a player's board.
    /// Returns None if player not found or global ball cap reached.
    pub fn ball_escaped(&mut self, owner_id: u32, vx: f64, vy: f64) -> Option<u32> {
        // Check global ball cap
        if self.deep_space.ball_count() >= self.max_balls_global {
            return None;
        }

        let player = self.players.get_mut(&owner_id)?;
        let portal_pos = player.portal_pos;
        player.balls_produced += 1;
        Some(
            self.deep_space
                .add_ball(owner_id, portal_pos, vx, vy, &mut self.rng),
        )
    }

    /// Get space state for broadcasting
    pub fn get_space_state(&self) -> SpaceStateMsg {
        SpaceStateMsg {
            server_time: self.elapsed,
            balls: self.deep_space.get_ball_iter().map(ball_to_wire).collect(),
        }
    }

    /// Get players state for broadcasting
    pub fn get_players_state(&self) -> PlayersStateMsg {
        // Count balls in flight per player
        let mut balls_in_flight: HashMap<u32, u32> = HashMap::new();
        for ball in self.deep_space.get_ball_iter() {
            *balls_in_flight.entry(ball.owner_id).or_insert(0) += 1;
        }

        PlayersStateMsg {
            players: self
                .players
                .values()
                .map(|p| player_to_wire(p, *balls_in_flight.get(&p.id).unwrap_or(&0)))
                .collect(),
        }
    }

    fn sync_players_to_deep_space(&mut self) {
        let players: Vec<Player> = self.players.values().cloned().collect();
        self.deep_space.set_players(players);
    }

    /// Get current ball count in deep space
    pub fn deep_space_ball_count(&self) -> usize {
        self.deep_space.ball_count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DeepSpaceConfig, ServerConfig};

    fn test_state() -> GameState {
        let server_config = ServerConfig {
            cell_count: 100,
            rng_seed: 12345,
            bot_count: 0, // No bots in unit tests for predictable behavior
            ..Default::default()
        };
        let mut deep_space_config = DeepSpaceConfig::default();
        // Use shorter times for faster tests
        deep_space_config.min_age_for_capture = 2.0;
        deep_space_config.reroute_after = 5.0;
        GameState::new(&server_config, deep_space_config, 3.0)
    }

    #[test]
    fn balls_produced_starts_at_zero() {
        let mut state = test_state();
        let (_, player) = state.add_player().unwrap();
        assert_eq!(player.balls_produced, 0);
    }

    #[test]
    fn balls_produced_increments_on_escape() {
        let mut state = test_state();
        let (id, _) = state.add_player().unwrap();

        // Initial count is 0
        assert_eq!(state.players.get(&id).unwrap().balls_produced, 0);

        // First escape
        state.ball_escaped(id, 1.0, 2.0);
        assert_eq!(state.players.get(&id).unwrap().balls_produced, 1);

        // Second escape
        state.ball_escaped(id, -1.0, 1.5);
        assert_eq!(state.players.get(&id).unwrap().balls_produced, 2);

        // Third escape
        state.ball_escaped(id, 0.5, 3.0);
        assert_eq!(state.players.get(&id).unwrap().balls_produced, 3);
    }

    #[test]
    fn balls_in_flight_calculated_correctly() {
        let mut state = test_state();
        let (id1, _) = state.add_player().unwrap();
        let (id2, _) = state.add_player().unwrap();

        // Initially no balls in flight
        let players_state = state.get_players_state();
        for p in &players_state.players {
            assert_eq!(p.balls_in_flight, 0);
        }

        // Player 1 sends 3 balls
        state.ball_escaped(id1, 1.0, 2.0);
        state.ball_escaped(id1, 1.5, 2.5);
        state.ball_escaped(id1, 2.0, 3.0);

        // Player 2 sends 1 ball
        state.ball_escaped(id2, 0.5, 1.0);

        let players_state = state.get_players_state();
        let p1 = players_state.players.iter().find(|p| p.id == id1).unwrap();
        let p2 = players_state.players.iter().find(|p| p.id == id2).unwrap();

        assert_eq!(p1.balls_in_flight, 3);
        assert_eq!(p1.balls_produced, 3);
        assert_eq!(p2.balls_in_flight, 1);
        assert_eq!(p2.balls_produced, 1);
    }

    #[test]
    fn balls_in_flight_decreases_after_capture() {
        let mut state = test_state();
        let (id, _) = state.add_player().unwrap();

        // Send a ball
        state.ball_escaped(id, 1.0, 2.0);
        assert_eq!(state.deep_space_ball_count(), 1);

        // balls_produced stays the same even after capture
        let initial_produced = state.players.get(&id).unwrap().balls_produced;
        assert_eq!(initial_produced, 1);

        // Tick until ball is captured (simulate time passing)
        // Real players CAN capture their own balls
        for _ in 0..1000 {
            state.tick(0.1);
        }

        // Ball should be captured by now
        assert_eq!(state.deep_space_ball_count(), 0);

        // balls_in_flight should be 0, but balls_produced stays at 1
        let players_state = state.get_players_state();
        let p = players_state.players.iter().find(|p| p.id == id).unwrap();
        assert_eq!(p.balls_in_flight, 0);
        assert_eq!(p.balls_produced, 1);
    }

    // --- Bot integration tests ---

    fn test_state_with_bots(bot_count: usize) -> GameState {
        let server_config = ServerConfig {
            cell_count: 100,
            rng_seed: 12345,
            bot_count,
            ..Default::default()
        };
        let mut deep_space_config = DeepSpaceConfig::default();
        deep_space_config.min_age_for_capture = 2.0;
        deep_space_config.reroute_after = 5.0;
        GameState::new(&server_config, deep_space_config, 3.0)
    }

    /// Add a real player and mark them active so bots will produce balls.
    fn add_active_player(state: &mut GameState) -> u32 {
        let (id, _) = state.add_player().unwrap();
        state.player_activity(id);
        id
    }

    #[test]
    fn bots_are_created_on_startup() {
        let state = test_state_with_bots(3);

        assert_eq!(state.bots.bot_count(), 3);
        assert_eq!(state.players.len(), 3);

        // All players should be bots
        for player in state.players.values() {
            assert!(state.bots.is_bot(player.id));
        }
    }

    #[test]
    fn bots_send_initial_balls_to_deep_space() {
        let mut state = test_state_with_bots(3);
        add_active_player(&mut state);

        // Initially no balls
        assert_eq!(state.deep_space_ball_count(), 0);

        // Tick for 10 seconds (bots send initial ball after 2-8 seconds)
        for _ in 0..600 {
            state.tick(1.0 / 60.0);
        }

        // Should have some balls in deep space from bots
        assert!(
            state.deep_space_ball_count() > 0,
            "Bots should have sent initial balls to deep space"
        );
    }

    #[test]
    fn bots_receive_and_return_balls() {
        let mut state = test_state_with_bots(2);
        add_active_player(&mut state);

        // Disable initial balls for predictable test
        for bot in &mut state.bots.bots {
            bot.initial_ball_delay = None;
        }

        // Get both bot IDs - we need one to send, another to receive
        // (players don't capture their own balls)
        let bot_ids = state.bots.bot_ids();
        let sender_bot_id = bot_ids[0];
        let receiver_bot_id = bot_ids[1];

        // Get the receiver bot's portal position
        let receiver_portal_pos = state.players.get(&receiver_bot_id).unwrap().portal_pos;

        // Add a ball from sender bot near the receiver's portal
        state.deep_space.add_ball(
            sender_bot_id,
            receiver_portal_pos,
            0.1,
            0.1,
            &mut state.rng.clone(),
        );

        // Manually age the ball so it can be captured
        if let Some(ball) = state.deep_space.get_ball_mut(1) {
            ball.age = 10.0; // Old enough to capture
            ball.pos = receiver_portal_pos; // At receiver's portal
        }

        // Tick - ball should be captured by receiver bot
        state.tick(0.01);

        // Ball was captured (removed from deep space)
        assert_eq!(state.deep_space_ball_count(), 0);

        // Receiver bot should have a pending ball
        let bot = state
            .bots
            .bots
            .iter()
            .find(|b| b.player_id == receiver_bot_id)
            .unwrap();
        assert_eq!(bot.pending_count(), 1, "Bot should have received the ball");

        // Tick until bot sends the ball back
        let mut ball_returned = false;
        for _ in 0..100 {
            state.tick(0.1);
            if state.deep_space_ball_count() > 0 {
                ball_returned = true;
                break;
            }
        }

        assert!(ball_returned, "Bot should return ball to deep space");
    }

    #[test]
    fn real_player_captures_not_routed_to_bots() {
        let mut state = test_state_with_bots(1);

        // Disable initial balls
        for bot in &mut state.bots.bots {
            bot.initial_ball_delay = None;
        }

        // Add a real player
        let (real_player_id, _) = state.add_player().unwrap();
        assert!(!state.bots.is_bot(real_player_id));

        // Get the real player's portal position
        let real_player_portal = state.players.get(&real_player_id).unwrap().portal_pos;

        // Add a ball from the real player (real players CAN capture their own balls)
        state.deep_space.add_ball(
            real_player_id,
            real_player_portal,
            0.1,
            0.1,
            &mut state.rng.clone(),
        );

        // Age the ball and position it at the real player's portal
        if let Some(ball) = state.deep_space.get_ball_mut(1) {
            ball.age = 10.0;
            ball.pos = real_player_portal;
        }

        // Tick - should return a capture event for the real player
        let captures = state.tick(0.01);

        assert_eq!(captures.len(), 1, "Should have one capture for real player");
        assert_eq!(captures[0].player_id, real_player_id);
    }

    #[test]
    fn bot_balls_produced_increments() {
        let mut state = test_state_with_bots(1);
        add_active_player(&mut state);

        let bot_id = state.bots.bot_ids()[0];

        // Initially 0 balls produced
        assert_eq!(state.players.get(&bot_id).unwrap().balls_produced, 0);

        // Tick until bot sends initial ball (2-8 seconds)
        for _ in 0..600 {
            state.tick(1.0 / 60.0);
            if state.players.get(&bot_id).unwrap().balls_produced > 0 {
                break;
            }
        }

        assert!(
            state.players.get(&bot_id).unwrap().balls_produced > 0,
            "Bot should have produced at least one ball"
        );
    }

    #[test]
    fn bot_does_not_capture_own_ball() {
        let mut state = test_state_with_bots(1);

        // Disable initial balls for predictable test
        for bot in &mut state.bots.bots {
            bot.initial_ball_delay = None;
        }

        let bot_id = state.bots.bot_ids()[0];
        let bot_portal_pos = state.players.get(&bot_id).unwrap().portal_pos;

        // Add a ball owned by the bot, positioned at the bot's portal
        state
            .deep_space
            .add_ball(bot_id, bot_portal_pos, 0.1, 0.1, &mut state.rng.clone());

        // Age the ball so it can be captured
        if let Some(ball) = state.deep_space.get_ball_mut(1) {
            ball.age = 10.0;
            ball.pos = bot_portal_pos;
        }

        // Tick - ball should NOT be captured by bot's own portal (bots skip own balls)
        state.tick(0.01);

        // Ball should still exist in deep space (not captured by own portal)
        assert_eq!(
            state.deep_space_ball_count(),
            1,
            "Ball should pass through bot's own portal without being captured"
        );

        // Bot should NOT have a pending ball
        let bot = state
            .bots
            .bots
            .iter()
            .find(|b| b.player_id == bot_id)
            .unwrap();
        assert_eq!(
            bot.pending_count(),
            0,
            "Bot should NOT receive its own ball"
        );
    }

    /// Place a ball from `sender_id` at `receiver_id`'s portal, aged so it can
    /// be captured immediately. Returns the ball ID.
    fn place_ball_at_bot_portal(state: &mut GameState, sender_id: u32, receiver_id: u32) -> u32 {
        let receiver_portal = state.players.get(&receiver_id).unwrap().portal_pos;
        let ball_id =
            state
                .deep_space
                .add_ball(sender_id, receiver_portal, 0.1, 0.1, &mut state.rng.clone());
        if let Some(ball) = state.deep_space.get_ball_mut(ball_id) {
            ball.age = 10.0;
            ball.pos = receiver_portal;
        }
        ball_id
    }

    /// During inactivity, a ball captured by a bot must NOT be queued in
    /// pending_balls — otherwise it would be sent back the moment a real player
    /// returns, flooding deep-space.
    ///
    /// Before the fix this test failed because `handle_capture` was called
    /// unconditionally; now it is only called when `has_active_players()` is true.
    #[test]
    fn bot_capture_during_inactivity_is_discarded() {
        let mut state = test_state_with_bots(2);

        // No real players → has_active_players() is false from the start.
        // Disable initial balls so bots don't spontaneously add to deep-space.
        for bot in &mut state.bots.bots {
            bot.initial_ball_delay = None;
        }

        let bot_ids = state.bots.bot_ids();
        let sender_bot = bot_ids[0];
        let receiver_bot = bot_ids[1];

        // Manually place a ball at the receiver bot's portal.
        place_ball_at_bot_portal(&mut state, sender_bot, receiver_bot);
        assert_eq!(state.deep_space_ball_count(), 1);

        // One tick: ball is captured by receiver bot's portal.
        state.tick(0.01);

        // Ball should be gone from deep-space.
        assert_eq!(state.deep_space_ball_count(), 0, "Ball should be captured");

        // But the bot's pending queue must be empty — the capture was discarded.
        let bot = state
            .bots
            .bots
            .iter()
            .find(|b| b.player_id == receiver_bot)
            .unwrap();
        assert_eq!(
            bot.pending_count(),
            0,
            "Capture during inactivity must not queue a pending ball"
        );
    }

    /// When a real player returns after inactivity, any pending balls that were
    /// queued just before the player went inactive must be flushed so they are
    /// not dumped into deep-space all at once.
    ///
    /// Before the fix `clear_pending` was never called, so all accumulated
    /// pending balls would be sent in a burst on reactivation.
    #[test]
    fn pending_balls_flushed_on_reactivation() {
        let mut state = test_state_with_bots(2);

        // Add an active real player so has_active_players() returns true.
        let real_player_id = add_active_player(&mut state);

        // Disable spontaneous/initial bot production.
        for bot in &mut state.bots.bots {
            bot.initial_ball_delay = None;
        }

        let bot_ids = state.bots.bot_ids();
        let sender_bot = bot_ids[0];
        let receiver_bot = bot_ids[1];

        // Place and capture a ball while active — this queues a pending ball.
        place_ball_at_bot_portal(&mut state, sender_bot, receiver_bot);
        state.tick(0.01); // capture happens, pending_count becomes 1

        let bot = state
            .bots
            .bots
            .iter()
            .find(|b| b.player_id == receiver_bot)
            .unwrap();
        assert_eq!(bot.pending_count(), 1, "Should have one pending ball");

        // Now remove the real player → has_active_players() drops to false.
        state.remove_player(real_player_id);

        // One tick to register the inactive state (was_active → false).
        state.tick(0.01);

        // Re-add an active real player — triggers inactive→active transition
        // and clear_pending() should fire.
        let new_player_id = add_active_player(&mut state);
        state.tick(0.01); // transition tick

        let bot = state
            .bots
            .bots
            .iter()
            .find(|b| b.player_id == receiver_bot)
            .unwrap();
        assert_eq!(
            bot.pending_count(),
            0,
            "Pending balls must be flushed on reactivation"
        );

        state.remove_player(new_player_id);
    }
}
