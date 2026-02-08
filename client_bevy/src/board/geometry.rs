use bevy::prelude::Vec2;

use crate::constants::{BOARD_CENTER_X, BOARD_CENTER_Y, BOARD_HALF_HEIGHT, BOARD_HALF_WIDTH};

#[derive(Clone, Copy)]
pub struct Segment {
    pub from: Vec2,
    pub to: Vec2,
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

pub fn in_escape_slot(px: f32, py: f32) -> bool {
    let cx = BOARD_CENTER_X;
    let cy = BOARD_CENTER_Y;
    let hh = BOARD_HALF_HEIGHT;
    let x_min = cx - 90.0;
    let x_max = cx + 90.0;
    let y_top = cy - hh - 5.0;
    let y_bottom = cy - hh + 20.0;

    px >= x_min && px <= x_max && py >= y_top && py <= y_bottom
}
