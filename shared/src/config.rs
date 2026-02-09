/// Deep-space configuration
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../client/src/shared/generated/")]
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
            portal_alpha: 0.15,
            omega_min: 0.5,
            omega_max: 1.0,
            reroute_after: 12.0,
            reroute_cooldown: 6.0,
            min_age_for_capture: 15.0,
            min_age_for_reroute: 2.0,
            reroute_arrival_time_min: 4.0,
            reroute_arrival_time_max: 10.0,
        }
    }
}

impl DeepSpaceConfig {
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
    fn default_deep_space_config_is_valid() {
        let config = DeepSpaceConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn omega_max_less_than_min_invalid() {
        let mut config = DeepSpaceConfig::default();
        config.omega_min = 2.0;
        config.omega_max = 1.0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn portal_alpha_too_large_invalid() {
        let mut config = DeepSpaceConfig::default();
        config.portal_alpha = 4.0;
        assert!(config.validate().is_err());
    }
}
