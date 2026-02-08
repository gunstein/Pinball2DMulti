use bevy::prelude::*;
use bevy_prototype_lyon::prelude::*;
use bevy_rapier2d::prelude::*;

use crate::board::geometry::{
    guide_walls, launcher_stop, launcher_wall, wall_segments, Segment, BOTTOM_WALL_INDEX,
    WALL_COLLIDER_THICKNESS,
};
use crate::constants::{color_from_hex, Colors, PPM};

use super::to_world2;

pub struct WallsPlugin;

#[derive(Component)]
pub(crate) struct Drain;

impl Plugin for WallsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_walls);
    }
}

fn spawn_walls(mut commands: Commands) {
    let body = commands
        .spawn((
            RigidBody::Fixed,
            Transform::default(),
            GlobalTransform::default(),
        ))
        .id();

    let walls = wall_segments();
    for (idx, seg) in walls.iter().enumerate() {
        spawn_segment_collider(&mut commands, body, *seg, idx == BOTTOM_WALL_INDEX);
    }

    for seg in guide_walls() {
        spawn_segment_collider(&mut commands, body, seg, false);
    }

    spawn_segment_collider(&mut commands, body, launcher_wall(), false);
    spawn_segment_collider(&mut commands, body, launcher_stop(), false);

    let wall_color = color_from_hex(Colors::WALL);
    for seg in wall_segments() {
        spawn_line_visual(&mut commands, seg, wall_color, 3.0, 2.0);
    }
    for seg in guide_walls() {
        spawn_line_visual(&mut commands, seg, wall_color, 3.0, 2.0);
    }
    spawn_line_visual(&mut commands, launcher_wall(), wall_color, 6.0, 2.0);
    spawn_line_visual(&mut commands, launcher_stop(), wall_color, 6.0, 2.0);
}

fn spawn_segment_collider(commands: &mut Commands, parent: Entity, seg: Segment, drain: bool) {
    let mid = (seg.from + seg.to) * 0.5;
    let d = seg.to - seg.from;
    let len = d.length();
    let angle = d.y.atan2(d.x);

    let mut entity = commands.spawn((
        Collider::cuboid((len * 0.5) / PPM, WALL_COLLIDER_THICKNESS / PPM),
        Restitution::coefficient(0.3),
        Transform::from_xyz(mid.x / PPM, mid.y / PPM, 0.0)
            .with_rotation(Quat::from_rotation_z(angle)),
        GlobalTransform::default(),
    ));

    if drain {
        entity.insert((Sensor, ActiveEvents::COLLISION_EVENTS, Drain));
    }

    let child = entity.id();
    commands.entity(parent).add_child(child);
}

fn spawn_line_visual(commands: &mut Commands, seg: Segment, color: Color, width: f32, z: f32) {
    let a = to_world2(seg.from.x, seg.from.y);
    let b = to_world2(seg.to.x, seg.to.y);
    let line = shapes::Line(a, b);

    commands.spawn((
        ShapeBuilder::with(&line).stroke((color, width)).build(),
        Transform::from_xyz(0.0, 0.0, z),
    ));
}
