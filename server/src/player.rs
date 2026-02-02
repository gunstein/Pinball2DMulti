use crate::vec3::Vec3;

/// Player/Portal on the sphere
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Player {
    pub id: u32,
    pub cell_index: u32,
    pub portal_pos: Vec3,
    pub color: u32,
    /// Whether the player is paused (e.g., tab not visible). Paused players don't capture balls.
    #[serde(default)]
    pub paused: bool,
    /// Total number of balls this player has produced (sent to deep space)
    #[serde(default)]
    pub balls_produced: u32,
}

/// Generate a color from player ID using golden angle hue distribution.
pub fn color_from_id(id: u32) -> u32 {
    let hue = ((id as u32).wrapping_mul(137)) % 360;
    hsv_to_rgb(hue as f64, 0.55, 0.95)
}

fn hsv_to_rgb(h: f64, s: f64, v: f64) -> u32 {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;

    let (r, g, b) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    let ri = ((r + m) * 255.0).round() as u32;
    let gi = ((g + m) * 255.0).round() as u32;
    let bi = ((b + m) * 255.0).round() as u32;

    (ri << 16) | (gi << 8) | bi
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_from_id_produces_valid_rgb() {
        for id in 1..=100 {
            let color = color_from_id(id);
            assert!(
                color <= 0xFFFFFF,
                "Color {:#x} out of range for id {}",
                color,
                id
            );
        }
    }

    #[test]
    fn different_ids_give_different_colors() {
        let c1 = color_from_id(1);
        let c2 = color_from_id(2);
        let c3 = color_from_id(3);
        assert_ne!(c1, c2);
        assert_ne!(c2, c3);
    }
}
