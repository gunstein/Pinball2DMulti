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

    /// Add a ball escaped from a player's board
    pub fn ball_escaped(&mut self, owner_id: u32, vx: f64, vy: f64) -> Option<u32> {
        let player = self.players.get(&owner_id)?;
        let portal_pos = player.portal_pos;
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
        PlayersStateMsg {
            players: self.players.values().map(PlayerWire::from_player).collect(),
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
