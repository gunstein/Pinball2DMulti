use bevy::prelude::*;
use bevy_prototype_lyon::prelude::*;
use bevy_rapier2d::prelude::*;

use crate::board::geometry::{
    guide_walls, launcher_stop, launcher_wall, wall_segments, Segment, BOTTOM_WALL_INDEX,
    WALL_COLLIDER_THICKNESS,
};
use crate::constants::{color_from_hex, px_to_world, Colors};

pub struct WallsPlugin;

#[derive(Component)]
pub(crate) struct Drain;

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
