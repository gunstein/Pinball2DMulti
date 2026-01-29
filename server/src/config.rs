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
            min_age_for_capture: 3.0,       // seconds
            min_age_for_reroute: 2.0,       // seconds
            reroute_arrival_time_min: 4.0,  // seconds
            reroute_arrival_time_max: 10.0, // seconds (4.0 + 6.0)
        }
    }
}

/// Server configuration
pub struct ServerConfig {
    pub listen_addr: String,
    pub tick_rate_hz: u32,
    pub broadcast_rate_hz: u32,
    pub cell_count: usize,
    pub rng_seed: u64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            listen_addr: "0.0.0.0:9001".to_string(),
            tick_rate_hz: 60,
            broadcast_rate_hz: 15,
            cell_count: 2048,
            rng_seed: 42,
        }
    }
}
