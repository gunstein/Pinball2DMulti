use bevy::prelude::*;
use bevy_prototype_lyon::prelude::*;
use bevy_rapier2d::prelude::*;

use crate::board::geometry::{
    escape_slot_bounds, guide_walls, launcher_stop, launcher_wall, wall_segments, Segment,
    BOTTOM_WALL_INDEX, WALL_COLLIDER_THICKNESS,
};
use crate::constants::{color_from_hex, px_to_world, Colors};

pub struct WallsPlugin;
const WALL_FRICTION: f32 = 0.2;

#[derive(Component)]
pub(crate) struct Drain;

#[derive(Component)]
pub(crate) struct EscapeSlot;

impl Plugin for WallsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_walls);
    }
}

fn spawn_walls(mut commands: Commands) {
    let wall_color = color_from_hex(Colors::WALL);

    let walls = wall_segments();
    for (idx, seg) in walls.iter().enumerate() {
        spawn_wall(
            &mut commands,
            *seg,
            wall_color,
            3.0,
            idx == BOTTOM_WALL_INDEX,
        );
    }
    for seg in guide_walls() {
        spawn_wall(&mut commands, seg, wall_color, 3.0, false);
    }
    spawn_wall(&mut commands, launcher_wall(), wall_color, 6.0, false);
    spawn_wall(&mut commands, launcher_stop(), wall_color, 6.0, false);
    spawn_escape_sensor(&mut commands);
}

fn spawn_wall(commands: &mut Commands, seg: Segment, color: Color, width: f32, drain: bool) {
    let a_world = px_to_world(seg.from.x, seg.from.y, 0.0);
    let b_world = px_to_world(seg.to.x, seg.to.y, 0.0);
    let mid_world = (a_world + b_world) * 0.5;
    let d = b_world - a_world;
    let len = d.truncate().length();
    let angle = d.y.atan2(d.x);

    let mut entity = commands.spawn((
        RigidBody::Fixed,
        Collider::cuboid(len * 0.5, WALL_COLLIDER_THICKNESS),
        Friction {
            coefficient: WALL_FRICTION,
            combine_rule: CoefficientCombineRule::Min,
        },
        Restitution::coefficient(0.3),
        Transform::from_xyz(mid_world.x, mid_world.y, 0.0)
            .with_rotation(Quat::from_rotation_z(angle)),
    ));

    if drain {
        entity.insert((Sensor, ActiveEvents::COLLISION_EVENTS, Drain));
    }

    // Visual: line in world coordinates
    let line = shapes::Line(a_world.truncate(), b_world.truncate());
    commands.spawn((
        ShapeBuilder::with(&line).stroke((color, width)).build(),
        Transform::from_xyz(0.0, 0.0, 2.0),
    ));
}

fn spawn_escape_sensor(commands: &mut Commands) {
    let bounds = escape_slot_bounds();
    let center_x = (bounds.x_min + bounds.x_max) * 0.5;
    let center_y = (bounds.y_top + bounds.y_bottom) * 0.5;
    let width = (bounds.x_max - bounds.x_min).max(1.0);
    let height = (bounds.y_bottom - bounds.y_top).max(1.0);
    let world = px_to_world(center_x, center_y, 0.0);

    commands.spawn((
        RigidBody::Fixed,
        Collider::cuboid(width * 0.5, height * 0.5),
        Sensor,
        ActiveEvents::COLLISION_EVENTS,
        Transform::from_xyz(world.x, world.y, 0.0),
        EscapeSlot,
    ));
}
