pub use pinball_shared::config::DeepSpaceConfig;

/// Server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub listen_addr: String,
    pub tick_rate_hz: u32,
    pub broadcast_rate_hz: u32,
    pub cell_count: usize,
    pub rng_seed: u64,
    /// Maximum velocity component magnitude for ball_escaped (m/s)
    pub max_velocity: f64,
    /// Maximum ball_escaped messages per second per client
    pub max_ball_escaped_per_sec: u32,
    /// Maximum concurrent connections
    pub max_connections: usize,
    /// Global maximum balls in deep space (prevents memory exhaustion)
    pub max_balls_global: usize,
    /// Allowed origins for WebSocket connections (empty = allow all)
    pub allowed_origins: Vec<String>,
    /// Number of bot players to spawn on server start
    pub bot_count: usize,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            listen_addr: "0.0.0.0:9001".to_string(),
            tick_rate_hz: 60,
            broadcast_rate_hz: 10,
            cell_count: 2048,
            rng_seed: 42,
            max_velocity: 10.0,
            max_ball_escaped_per_sec: 30,
            max_connections: 1000,
            max_balls_global: 1000,
            allowed_origins: vec![],
            bot_count: 3,
        }
    }
}

impl ServerConfig {
    /// Validate configuration. Returns Err with description if invalid.
    pub fn validate(&self) -> Result<(), String> {
        if self.tick_rate_hz == 0 {
            return Err("tick_rate_hz must be > 0".to_string());
        }
        if self.broadcast_rate_hz == 0 {
            return Err("broadcast_rate_hz must be > 0".to_string());
        }
        if self.cell_count == 0 {
            return Err("cell_count must be > 0".to_string());
        }
        if !self.max_velocity.is_finite() || self.max_velocity <= 0.0 {
            return Err("max_velocity must be finite and > 0".to_string());
        }
        if self.max_connections == 0 {
            return Err("max_connections must be > 0".to_string());
        }
        if self.max_balls_global == 0 {
            return Err("max_balls_global must be > 0".to_string());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_server_config_is_valid() {
        let config = ServerConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn server_config_zero_tick_rate_invalid() {
        let mut config = ServerConfig::default();
        config.tick_rate_hz = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn server_config_zero_broadcast_rate_invalid() {
        let mut config = ServerConfig::default();
        config.broadcast_rate_hz = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn server_config_zero_cell_count_invalid() {
        let mut config = ServerConfig::default();
        config.cell_count = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn server_config_nan_max_velocity_invalid() {
        let mut config = ServerConfig::default();
        config.max_velocity = f64::NAN;
        assert!(config.validate().is_err());
    }

    #[test]
    fn server_config_negative_max_velocity_invalid() {
        let mut config = ServerConfig::default();
        config.max_velocity = -1.0;
        assert!(config.validate().is_err());
    }
}
