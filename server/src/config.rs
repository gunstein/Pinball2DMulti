/// Deep-space configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeepSpaceConfig {
    pub portal_alpha: f64,
    pub omega_min: f64,
    pub omega_max: f64,
    pub reroute_after: f64,
    pub reroute_cooldown: f64,
    pub min_age_for_capture: f64,
    /// Minimum age before ball can be rerouted (seconds)
    pub min_age_for_reroute: f64,
    /// Minimum arrival time for reroute omega calculation (seconds)
    pub reroute_arrival_time_min: f64,
    /// Maximum arrival time for reroute omega calculation (seconds)
    pub reroute_arrival_time_max: f64,
}

impl Default for DeepSpaceConfig {
    fn default() -> Self {
        Self {
            portal_alpha: 0.15,             // ~8.6 degrees
            omega_min: 0.5,                 // rad/s (~12.6s per full orbit)
            omega_max: 1.0,                 // rad/s (~6.3s per full orbit)
            reroute_after: 12.0,            // seconds
            reroute_cooldown: 6.0,          // seconds
            min_age_for_capture: 15.0,      // seconds - ball must travel before capture
            min_age_for_reroute: 2.0,       // seconds
            reroute_arrival_time_min: 4.0,  // seconds
            reroute_arrival_time_max: 10.0, // seconds (4.0 + 6.0)
        }
    }
}

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
            max_balls_global: 10000,
            allowed_origins: vec![], // Empty = allow all origins (open game server)
            bot_count: 3,            // Default to 3 bot players
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

impl DeepSpaceConfig {
    /// Validate configuration. Returns Err with description if invalid.
    pub fn validate(&self) -> Result<(), String> {
        if !self.portal_alpha.is_finite() || self.portal_alpha <= 0.0 {
            return Err("portal_alpha must be finite and > 0".to_string());
        }
        if self.portal_alpha > std::f64::consts::PI {
            return Err("portal_alpha must be <= PI".to_string());
        }
        if !self.omega_min.is_finite() || self.omega_min < 0.0 {
            return Err("omega_min must be finite and >= 0".to_string());
        }
        if !self.omega_max.is_finite() || self.omega_max < self.omega_min {
            return Err("omega_max must be finite and >= omega_min".to_string());
        }
        if !self.min_age_for_capture.is_finite() || self.min_age_for_capture < 0.0 {
            return Err("min_age_for_capture must be finite and >= 0".to_string());
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
    fn default_deep_space_config_is_valid() {
        let config = DeepSpaceConfig::default();
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

    #[test]
    fn deep_space_config_omega_max_less_than_min_invalid() {
        let mut config = DeepSpaceConfig::default();
        config.omega_min = 2.0;
        config.omega_max = 1.0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn deep_space_config_portal_alpha_too_large_invalid() {
        let mut config = DeepSpaceConfig::default();
        config.portal_alpha = 4.0; // > PI
        assert!(config.validate().is_err());
    }
}
