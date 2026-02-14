use bevy::prelude::Vec2;

use crate::constants::{BOARD_CENTER_X, BOARD_CENTER_Y, BOARD_HALF_HEIGHT, BOARD_HALF_WIDTH};

#[derive(Clone, Copy)]
pub struct Segment {
    pub from: Vec2,
    pub to: Vec2,
}

#[derive(Clone, Copy)]
pub struct EscapeSlotBounds {
    pub x_min: f32,
    pub x_max: f32,
    pub y_top: f32,
    pub y_bottom: f32,
}

#[derive(Clone, Copy)]
pub struct CircleDef {
    pub center: Vec2,
    pub radius: f32,
}

#[derive(Clone, Copy)]
pub enum FlipperSide {
    Left,
    Right,
}

#[derive(Clone, Copy)]
pub struct FlipperDef {
    pub pivot: Vec2,
    pub length: f32,
    pub width: f32,
    pub pivot_radius: f32,
    pub tip_radius: f32,
    pub side: FlipperSide,
}

pub const WALL_COLLIDER_THICKNESS: f32 = 5.0;

pub const BOTTOM_WALL_INDEX: usize = 5;

pub fn wall_segments() -> Vec<Segment> {
    let cx = BOARD_CENTER_X;
    let cy = BOARD_CENTER_Y;
    let hw = BOARD_HALF_WIDTH;
    let hh = BOARD_HALF_HEIGHT;

    let chamfer_size = 60.0;
    let escape_slot_width = 180.0;
    let escape_slot_left_x = cx - escape_slot_width * 0.5;
    let escape_slot_right_x = cx + escape_slot_width * 0.5;

    let p0 = Vec2::new(cx - hw, cy - hh);
    let p1 = Vec2::new(cx + hw - chamfer_size, cy - hh);
    let p2 = Vec2::new(cx + hw, cy - hh + chamfer_size);
    let p3 = Vec2::new(cx + hw, cy + hh);
    let p4 = Vec2::new(cx - hw, cy + hh);

    vec![
        Segment { from: p0, to: p4 },
        Segment {
            from: p0,
            to: Vec2::new(escape_slot_left_x, cy - hh),
        },
        Segment {
            from: Vec2::new(escape_slot_right_x, cy - hh),
            to: p1,
        },
        Segment { from: p1, to: p2 },
        Segment { from: p2, to: p3 },
        Segment { from: p4, to: p3 },
    ]
}

pub fn guide_walls() -> Vec<Segment> {
    let cx = BOARD_CENTER_X;
    let cy = BOARD_CENTER_Y;
    let hw = BOARD_HALF_WIDTH;
    let hh = BOARD_HALF_HEIGHT;
    let flipper_y = cy + hh - 70.0;
    let lane_inner_x = cx + hw - 35.0;

    vec![
        Segment {
            from: Vec2::new(cx - hw, flipper_y - 80.0),
            to: Vec2::new(cx - 90.0, flipper_y - 14.0),
        },
        Segment {
            from: Vec2::new(lane_inner_x, flipper_y - 80.0),
            to: Vec2::new(cx + 90.0, flipper_y - 14.0),
        },
    ]
}

pub fn launcher_wall() -> Segment {
    let cx = BOARD_CENTER_X;
    let cy = BOARD_CENTER_Y;
    let hw = BOARD_HALF_WIDTH;
    let hh = BOARD_HALF_HEIGHT;
    let lane_inner_x = cx + hw - 35.0;
    let bottom_y = cy + hh;

    Segment {
        from: Vec2::new(lane_inner_x, cy),
        to: Vec2::new(lane_inner_x, bottom_y),
    }
}

pub fn launcher_stop() -> Segment {
    let cx = BOARD_CENTER_X;
    let cy = BOARD_CENTER_Y;
    let hw = BOARD_HALF_WIDTH;
    let hh = BOARD_HALF_HEIGHT;
    let lane_inner_x = cx + hw - 35.0;
    let bottom_y = cy + hh;
    let launcher_stop_y = bottom_y - 20.0;

    Segment {
        from: Vec2::new(lane_inner_x, launcher_stop_y),
        to: Vec2::new(cx + hw, launcher_stop_y),
    }
}

pub fn bumpers() -> Vec<CircleDef> {
    let cx = BOARD_CENTER_X;
    let cy = BOARD_CENTER_Y;

    vec![
        CircleDef {
            center: Vec2::new(cx - 70.0, cy - 120.0),
            radius: 25.0,
        },
        CircleDef {
            center: Vec2::new(cx + 50.0, cy - 150.0),
            radius: 25.0,
        },
        CircleDef {
            center: Vec2::new(cx - 20.0, cy + 10.0),
            radius: 25.0,
        },
    ]
}

pub fn flippers() -> [FlipperDef; 2] {
    let cx = BOARD_CENTER_X;
    let cy = BOARD_CENTER_Y;
    let hh = BOARD_HALF_HEIGHT;
    let flipper_y = cy + hh - 70.0;

    [
        FlipperDef {
            pivot: Vec2::new(cx - 90.0, flipper_y),
            length: 78.0,
            width: 12.0,
            pivot_radius: 10.0,
            tip_radius: 4.0,
            side: FlipperSide::Left,
        },
        FlipperDef {
            pivot: Vec2::new(cx + 90.0, flipper_y),
            length: 78.0,
            width: 12.0,
            pivot_radius: 10.0,
            tip_radius: 4.0,
            side: FlipperSide::Right,
        },
    ]
}

pub fn playfield_center_x() -> f32 {
    let cx = BOARD_CENTER_X;
    let hw = BOARD_HALF_WIDTH;
    let lane_inner_x = cx + hw - 35.0;
    (cx - hw + lane_inner_x) * 0.5 + 14.0
}

pub fn ball_spawn() -> Vec2 {
    let cx = BOARD_CENTER_X;
    let cy = BOARD_CENTER_Y;
    let hw = BOARD_HALF_WIDTH;
    let hh = BOARD_HALF_HEIGHT;
    let lane_width = 35.0;
    let bottom_y = cy + hh;
    let launcher_stop_y = bottom_y - 20.0;

    Vec2::new(cx + hw - lane_width * 0.5, launcher_stop_y - 12.0)
}

pub fn escape_slot_bounds() -> EscapeSlotBounds {
    let cx = BOARD_CENTER_X;
    let cy = BOARD_CENTER_Y;
    let hh = BOARD_HALF_HEIGHT;
    EscapeSlotBounds {
        x_min: cx - 90.0,
        x_max: cx + 90.0,
        y_top: cy - hh - 5.0,
        y_bottom: cy - hh,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flippers_are_symmetric_around_board_center_x() {
        let [left, right] = flippers();

        assert_eq!(left.pivot.y, right.pivot.y);
        let left_offset = left.pivot.x - BOARD_CENTER_X;
        let right_offset = right.pivot.x - BOARD_CENTER_X;
        assert_eq!(left_offset, -right_offset);

        assert_eq!(left.length, right.length);
        assert_eq!(left.width, right.width);
        assert_eq!(left.pivot_radius, right.pivot_radius);
        assert_eq!(left.tip_radius, right.tip_radius);
    }

    #[test]
    fn guide_walls_are_symmetric() {
        let guides = guide_walls();
        assert_eq!(guides.len(), 2);
        let left = &guides[0];
        let right = &guides[1];

        assert_eq!(left.from.y, right.from.y);
        assert_eq!(left.to.y, right.to.y);

        let [left_flipper, right_flipper] = flippers();
        assert_eq!(left.to.x, left_flipper.pivot.x);
        assert_eq!(right.to.x, right_flipper.pivot.x);
    }

    #[test]
    fn guide_walls_do_not_overlap_with_flippers() {
        let guides = guide_walls();
        let [left_flipper, right_flipper] = flippers();

        assert!(guides[0].to.y < left_flipper.pivot.y - left_flipper.pivot_radius);
        assert!(guides[1].to.y < right_flipper.pivot.y - right_flipper.pivot_radius);
    }

    #[test]
    fn escape_slot_is_centered_on_board() {
        let segs = wall_segments();
        // Bottom wall segments are indices 1 and 2 (left and right of escape slot)
        let left_seg = &segs[1];
        let right_seg = &segs[2];
        let slot_left = left_seg.to.x;
        let slot_right = right_seg.from.x;
        let slot_center = (slot_left + slot_right) / 2.0;
        assert_eq!(slot_center, BOARD_CENTER_X);
    }

    #[test]
    fn playfield_center_x_is_between_walls() {
        let left_wall = BOARD_CENTER_X - BOARD_HALF_WIDTH;
        let pfx = playfield_center_x();
        assert!(pfx > left_wall);
        assert!(pfx < BOARD_CENTER_X + BOARD_HALF_WIDTH);
    }

    #[test]
    fn wall_segments_have_no_zero_length() {
        for seg in wall_segments() {
            let d = seg.to - seg.from;
            let len = d.length();
            assert!(len > 0.0, "wall segment has zero length");
        }
    }

    #[test]
    fn guide_wall_segments_have_no_zero_length() {
        for seg in guide_walls() {
            let d = seg.to - seg.from;
            let len = d.length();
            assert!(len > 0.0, "guide wall segment has zero length");
        }
    }

    #[test]
    fn flipper_pivot_radius_larger_than_tip() {
        for f in flippers() {
            assert!(f.pivot_radius > f.tip_radius);
        }
    }

    #[test]
    fn ball_spawn_is_in_launcher_lane() {
        let spawn = ball_spawn();
        let lane_inner_x = BOARD_CENTER_X + BOARD_HALF_WIDTH - 35.0;
        let right_wall_x = BOARD_CENTER_X + BOARD_HALF_WIDTH;
        assert!(spawn.x > lane_inner_x);
        assert!(spawn.x < right_wall_x);
    }

    #[test]
    fn in_escape_slot_center_is_inside() {
        let b = escape_slot_bounds();
        let px = (b.x_min + b.x_max) * 0.5;
        let py = (b.y_top + b.y_bottom) * 0.5;
        assert!(px >= b.x_min && px <= b.x_max && py >= b.y_top && py <= b.y_bottom);
    }

    #[test]
    fn in_escape_slot_far_away_is_outside() {
        let b = escape_slot_bounds();
        let px = BOARD_CENTER_X;
        let py = BOARD_CENTER_Y;
        assert!(!(px >= b.x_min && px <= b.x_max && py >= b.y_top && py <= b.y_bottom));
    }
}
