use crate::config::{DeepSpaceConfig, ServerConfig};
use crate::deep_space::{CaptureEvent, SphereDeepSpace};
use crate::player::{color_from_id, Player};
use crate::protocol::{BallWire, PlayerWire, PlayersStateMsg, SpaceStateMsg};
use crate::sphere::PortalPlacement;
use rand_chacha::ChaCha8Rng;
use std::collections::HashMap;

/// Central game state owned by the game loop task.
pub struct GameState {
    pub deep_space: SphereDeepSpace,
    pub placement: PortalPlacement,
    pub players: HashMap<u32, Player>,
    pub config: DeepSpaceConfig,
    pub rng: ChaCha8Rng,
    next_player_id: u32,
    /// Global maximum balls in deep space
    max_balls_global: usize,
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

        Self {
            deep_space,
            placement,
            players: HashMap::new(),
            config: deep_space_config,
            rng,
            next_player_id: 1,
            max_balls_global: server_config.max_balls_global,
        }
    }

    /// Add a new player, returns (player_id, Player)
    pub fn add_player(&mut self) -> Option<(u32, Player)> {
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

    /// Tick the deep-space simulation
    pub fn tick(&mut self, dt: f64) -> Vec<CaptureEvent> {
        self.deep_space.tick(dt, &mut self.rng)
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
            balls: self
                .deep_space
                .get_ball_iter()
                .map(BallWire::from_ball)
                .collect(),
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
                .map(|p| PlayerWire::from_player(p, *balls_in_flight.get(&p.id).unwrap_or(&0)))
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
        // With min_flight_seconds=2.0 and capture_threshold=0.1, we need to tick enough
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
}
