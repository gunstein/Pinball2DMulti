use bevy::prelude::*;
use bevy_prototype_lyon::prelude::*;

use crate::board::geometry::playfield_center_x;
use crate::constants::{color_from_hex, Colors, CANVAS_HEIGHT};
use crate::shared::connection::ServerConnection;

use super::{to_world2, UpdateSet};

pub struct DeepSpacePlugin;

const STAR_COUNT: usize = 150;
const MAX_PORTAL_DOTS: usize = 60;
const MAX_BALL_DOTS: usize = 60;
const THETA_MAX: f64 = 0.8;
const PIXELS_PER_RADIAN: f32 = 400.0;

#[derive(Resource)]
struct DeepSpaceVisualState {
    center_px: Vec2,
    time: f32,
    self_marker_ring: Entity,
    self_marker_core: Entity,
}

#[derive(Component)]
struct DeepSpaceBoundary;

#[derive(Component)]
struct DeepSpaceSelfMarkerRing;

#[derive(Component)]
struct DeepSpaceSelfMarkerCore;

#[derive(Component)]
struct DeepSpaceStar {
    base_alpha: f32,
    twinkle_speed: f32,
}

#[derive(Component)]
struct DeepSpacePortalDot {
    index: usize,
}

#[derive(Component)]
struct DeepSpaceBallDot {
    index: usize,
}

impl Plugin for DeepSpacePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_deep_space)
            .add_systems(Update, animate_deep_space_stars.in_set(UpdateSet::Visuals))
            .add_systems(Update, update_deep_space_visuals.in_set(UpdateSet::Visuals));
    }
}

fn spawn_deep_space(mut commands: Commands) {
    let center_px = Vec2::new(playfield_center_x(), CANVAS_HEIGHT * 0.5);
    let center_world = to_world2(center_px.x, center_px.y);
    let radius = THETA_MAX as f32 * PIXELS_PER_RADIAN;

    let boundary = shapes::Circle {
        radius,
        center: Vec2::ZERO,
    };

    commands.spawn((
        ShapeBuilder::with(&boundary)
            .fill(Color::srgba(0.13, 0.27, 0.66, 0.05))
            .stroke((Color::srgba(0.4, 0.65, 1.0, 0.25), 2.0))
            .build(),
        Transform::from_xyz(center_world.x, center_world.y, 0.5),
        DeepSpaceBoundary,
    ));

    for i in 0..STAR_COUNT {
        let t = i as f32;
        let x = ((t * 17.13).sin() * 0.5 + 0.5) * 400.0;
        let y = ((t * 31.77).cos() * 0.5 + 0.5) * 700.0;
        let base_alpha = 0.1 + ((i % 10) as f32) * 0.04;
        let twinkle_speed = 0.5 + ((i % 7) as f32) * 0.3;
        let size = 0.5 + ((i % 3) as f32) * 0.5;

        let star = shapes::Circle {
            radius: size,
            center: Vec2::ZERO,
        };

        let pos = to_world2(x, y);
        commands.spawn((
            ShapeBuilder::with(&star)
                .fill(color_from_hex(Colors::STAR).with_alpha(base_alpha))
                .build(),
            Transform::from_xyz(pos.x, pos.y, 0.1),
            DeepSpaceStar {
                base_alpha,
                twinkle_speed,
            },
        ));
    }

    let dot_shape = shapes::Circle {
        radius: 6.0,
        center: Vec2::ZERO,
    };
    for i in 0..MAX_PORTAL_DOTS {
        commands.spawn((
            ShapeBuilder::with(&dot_shape)
                .fill(color_from_hex(Colors::WALL).with_alpha(0.8))
                .build(),
            Transform::from_xyz(center_world.x, center_world.y, 1.5),
            Visibility::Hidden,
            DeepSpacePortalDot { index: i },
        ));
    }

    let ball_dot_shape = shapes::Circle {
        radius: 5.0,
        center: Vec2::ZERO,
    };
    for i in 0..MAX_BALL_DOTS {
        commands.spawn((
            ShapeBuilder::with(&ball_dot_shape)
                .fill(color_from_hex(Colors::BALL_GLOW).with_alpha(0.8))
                .build(),
            Transform::from_xyz(center_world.x, center_world.y, 1.8),
            Visibility::Hidden,
            DeepSpaceBallDot { index: i },
        ));
    }

    let self_marker_ring_shape = shapes::Circle {
        radius: 14.0,
        center: Vec2::ZERO,
    };
    let self_marker_core_shape = shapes::Circle {
        radius: 5.0,
        center: Vec2::ZERO,
    };

    let self_marker_ring = commands
        .spawn((
            ShapeBuilder::with(&self_marker_ring_shape)
                .stroke((color_from_hex(Colors::BALL_GLOW).with_alpha(0.7), 2.0))
                .build(),
            Transform::from_xyz(center_world.x, center_world.y, 2.0),
            DeepSpaceSelfMarkerRing,
        ))
        .id();

    let self_marker_core = commands
        .spawn((
            ShapeBuilder::with(&self_marker_core_shape)
                .fill(color_from_hex(Colors::BALL_GLOW).with_alpha(0.8))
                .build(),
            Transform::from_xyz(center_world.x, center_world.y, 2.1),
            DeepSpaceSelfMarkerCore,
        ))
        .id();

    commands.insert_resource(DeepSpaceVisualState {
        center_px,
        time: 0.0,
        self_marker_ring,
        self_marker_core,
    });
}

fn animate_deep_space_stars(
    time: Res<Time>,
    mut deep: ResMut<DeepSpaceVisualState>,
    mut q_stars: Query<(&DeepSpaceStar, &mut Shape)>,
) {
    deep.time += time.delta_secs();

    for (star, mut shape) in &mut q_stars {
        let twinkle = (deep.time * star.twinkle_speed).sin() * 0.3 + 0.7;
        if let Some(fill) = shape.fill.as_mut() {
            fill.color = color_from_hex(Colors::STAR).with_alpha(star.base_alpha * twinkle);
        }
    }
}

fn update_deep_space_visuals(
    conn: Res<ServerConnection>,
    deep: Res<DeepSpaceVisualState>,
    mut q_portal: Query<
        (
            &DeepSpacePortalDot,
            &mut Transform,
            &mut Visibility,
            &mut Shape,
        ),
        (
            With<DeepSpacePortalDot>,
            Without<DeepSpaceBallDot>,
            Without<DeepSpaceSelfMarkerRing>,
            Without<DeepSpaceSelfMarkerCore>,
        ),
    >,
    mut q_ball: Query<
        (
            &DeepSpaceBallDot,
            &mut Transform,
            &mut Visibility,
            &mut Shape,
        ),
        (
            With<DeepSpaceBallDot>,
            Without<DeepSpacePortalDot>,
            Without<DeepSpaceSelfMarkerRing>,
            Without<DeepSpaceSelfMarkerCore>,
        ),
    >,
    mut q_self_ring: Query<
        &mut Shape,
        (
            With<DeepSpaceSelfMarkerRing>,
            Without<DeepSpacePortalDot>,
            Without<DeepSpaceBallDot>,
            Without<DeepSpaceSelfMarkerCore>,
        ),
    >,
    mut q_self_core: Query<
        &mut Shape,
        (
            With<DeepSpaceSelfMarkerCore>,
            Without<DeepSpacePortalDot>,
            Without<DeepSpaceBallDot>,
            Without<DeepSpaceSelfMarkerRing>,
        ),
    >,
) {
    let self_pos = conn
        .players
        .iter()
        .find(|p| p.id == conn.self_id)
        .map(|p| p.portal_pos)
        .unwrap_or(crate::shared::vec3::Vec3::new(1.0, 0.0, 0.0));

    let (e1, e2) = crate::shared::vec3::build_tangent_basis(self_pos);
    let cos_theta_max = THETA_MAX.cos();

    let self_color = conn
        .players
        .iter()
        .find(|p| p.id == conn.self_id)
        .map(|p| p.color)
        .unwrap_or(Colors::BALL_GLOW);

    if let Ok(mut self_shape) = q_self_ring.get_mut(deep.self_marker_ring) {
        if let Some(stroke) = self_shape.stroke.as_mut() {
            stroke.color = color_from_hex(self_color).with_alpha(0.7);
        }
    }
    if let Ok(mut self_shape) = q_self_core.get_mut(deep.self_marker_core) {
        if let Some(fill) = self_shape.fill.as_mut() {
            fill.color = color_from_hex(self_color).with_alpha(0.8);
        }
    }

    for (_dot, _tf, mut vis, _) in &mut q_portal {
        *vis = Visibility::Hidden;
    }
    for (_dot, _tf, mut vis, _) in &mut q_ball {
        *vis = Visibility::Hidden;
    }

    for (dot, mut tf, mut vis, mut shape) in &mut q_portal {
        if dot.index >= conn.players.len() {
            continue;
        }
        let p = &conn.players[dot.index];
        if let Some((sx, sy)) = project_deep_space(
            self_pos,
            p.portal_pos,
            e1,
            e2,
            deep.center_px,
            cos_theta_max,
        ) {
            let world = to_world2(sx, sy);
            tf.translation.x = world.x;
            tf.translation.y = world.y;
            *vis = Visibility::Visible;
            if let Some(fill) = shape.fill.as_mut() {
                let alpha = if p.paused { 0.2 } else { 0.6 };
                fill.color = color_from_hex(p.color).with_alpha(alpha);
            }
        }
    }

    for (dot, mut tf, mut vis, mut shape) in &mut q_ball {
        if dot.index >= conn.interpolated_balls.len() {
            continue;
        }

        let b = &conn.interpolated_balls[dot.index];
        if let Some((sx, sy)) =
            project_deep_space(self_pos, b.pos, e1, e2, deep.center_px, cos_theta_max)
        {
            let world = to_world2(sx, sy);
            tf.translation.x = world.x;
            tf.translation.y = world.y;
            *vis = Visibility::Visible;

            let color = conn
                .players
                .iter()
                .find(|p| p.id == b.owner_id)
                .map(|p| p.color)
                .unwrap_or(Colors::BALL_GLOW);
            if let Some(fill) = shape.fill.as_mut() {
                fill.color = color_from_hex(color).with_alpha(0.8);
            }
        }
    }
}

fn project_deep_space(
    self_pos: crate::shared::vec3::Vec3,
    pos: crate::shared::vec3::Vec3,
    e1: crate::shared::vec3::Vec3,
    e2: crate::shared::vec3::Vec3,
    center_px: Vec2,
    cos_theta_max: f64,
) -> Option<(f32, f32)> {
    let d = crate::shared::vec3::dot(self_pos, pos);
    if d < cos_theta_max {
        return None;
    }

    let theta = d.clamp(-1.0, 1.0).acos();

    let vx = pos.x - self_pos.x * d;
    let vy = pos.y - self_pos.y * d;
    let vz = pos.z - self_pos.z * d;
    let vlen = (vx * vx + vy * vy + vz * vz).sqrt();
    if vlen < 1e-6 {
        return Some((center_px.x, center_px.y));
    }

    let dirx = vx / vlen;
    let diry = vy / vlen;
    let dirz = vz / vlen;
    let dx = dirx * e1.x + diry * e1.y + dirz * e1.z;
    let dy = dirx * e2.x + diry * e2.y + dirz * e2.z;

    let r = theta as f32 * PIXELS_PER_RADIAN;
    Some((center_px.x + dx as f32 * r, center_px.y + dy as f32 * r))
}
