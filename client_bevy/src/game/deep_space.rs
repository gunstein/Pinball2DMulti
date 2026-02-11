use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::window::PrimaryWindow;
use bevy_prototype_lyon::prelude::*;

use crate::board::geometry::playfield_center_x;
use crate::constants::{color_from_hex, px_to_world, Colors, CANVAS_HEIGHT, CANVAS_WIDTH};
use crate::shared::connection::ServerConnection;

use super::UpdateSet;

pub struct DeepSpacePlugin;

const STAR_COUNT: usize = 150;
const MAX_PORTAL_DOTS: usize = 60;
const MAX_BALL_DOTS: usize = 60;
const THETA_MAX: f64 = 0.8;
const PIXELS_PER_RADIAN: f32 = 400.0;
const STAR_MIN_RADIUS: f32 = 0.5;
const STAR_MAX_RADIUS: f32 = 2.0;
const BALL_RADIUS: f32 = 10.0;
const TAIL_SEGMENTS: usize = 3;
const TAIL_TIME_STEP: f64 = 0.05;
const TAIL_START_ALPHA: f32 = 0.3;
const TAIL_END_ALPHA: f32 = 0.05;

#[derive(Resource)]
struct DeepSpaceState {
    center_px: Vec2,
    time: f32,
    self_marker_ring: Entity,
    self_marker_core: Entity,
    last_window_size: Vec2,
    dot_image: Handle<Image>,
}

#[derive(Component)]
struct DeepSpaceStar {
    base_alpha: f32,
    twinkle_speed: f32,
    twinkle_offset: f32,
}

#[derive(Component)]
struct DeepSpacePortalDot {
    index: usize,
}

#[derive(Component)]
struct DeepSpaceBallDot {
    index: usize,
}

#[derive(Component)]
struct DeepSpaceBallTailDot {
    ball_index: usize,
    segment: usize,
}

#[derive(Component)]
struct SelfMarkerRing;

#[derive(Component)]
struct SelfMarkerCore;

impl Plugin for DeepSpacePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_deep_space).add_systems(
            Update,
            (
                regenerate_stars_on_resize,
                animate_stars,
                update_portal_dots,
                update_ball_dots,
                update_ball_trails,
                update_self_marker,
            )
                .in_set(UpdateSet::Visuals),
        );
    }
}

fn create_soft_circle_texture(images: &mut Assets<Image>) -> Handle<Image> {
    let size = 16_u32;
    let mut data = vec![0_u8; (size * size * 4) as usize];
    let center = (size as f32 - 1.0) * 0.5;
    let radius = center.max(1.0);

    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            let dist = (dx * dx + dy * dy).sqrt() / radius;
            let alpha = if dist >= 1.0 {
                0.0
            } else {
                (1.0 - dist).powf(2.2)
            };
            let idx = ((y * size + x) * 4) as usize;
            data[idx] = 255;
            data[idx + 1] = 255;
            data[idx + 2] = 255;
            data[idx + 3] = (alpha * 255.0).round().clamp(0.0, 255.0) as u8;
        }
    }

    images.add(Image::new_fill(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &data,
        TextureFormat::Rgba8UnormSrgb,
        bevy::asset::RenderAssetUsages::default(),
    ))
}

/// Hash-based pseudo-random for deterministic but well-distributed star placement.
fn hash_f(seed: u32) -> f32 {
    // PCG-style hash: good bit avalanche for sequential seeds
    let mut s = seed;
    s = s.wrapping_mul(747796405).wrapping_add(2891336453);
    s = ((s >> ((s >> 28).wrapping_add(4))) ^ s).wrapping_mul(277803737);
    s = (s >> 22) ^ s;
    (s as f32) / (u32::MAX as f32)
}

fn spawn_stars(commands: &mut Commands, window_w: f32, window_h: f32, dot_image: Handle<Image>) {
    // Stars need to cover the visible area. The camera scales CANVAS into window,
    // so the visible world extent may be larger than CANVAS if aspect ratios differ.
    let scale_x = CANVAS_WIDTH / window_w;
    let scale_y = CANVAS_HEIGHT / window_h;
    let cam_scale = scale_x.max(scale_y).max(0.0001);
    let visible_w = window_w * cam_scale;
    let visible_h = window_h * cam_scale;

    for i in 0..STAR_COUNT {
        let seed = i as u32;
        let fx = hash_f(seed * 7);
        let fy = hash_f(seed * 7 + 1);
        let fv = hash_f(seed * 7 + 2);

        let wx = (fx - 0.5) * visible_w;
        let wy = (fy - 0.5) * visible_h;

        let base_alpha = 0.10 + fv * 0.34;
        let twinkle_speed = 0.5 + hash_f(seed * 7 + 3) * 2.0;
        let twinkle_offset = hash_f(seed * 7 + 5) * std::f32::consts::TAU;
        let size = STAR_MIN_RADIUS + hash_f(seed * 7 + 4) * (STAR_MAX_RADIUS - STAR_MIN_RADIUS);

        commands.spawn((
            Sprite {
                image: dot_image.clone(),
                color: color_from_hex(Colors::STAR).with_alpha(base_alpha),
                custom_size: Some(Vec2::splat(size * 2.0)),
                ..default()
            },
            Transform::from_xyz(wx, wy, 0.1),
            DeepSpaceStar {
                base_alpha,
                twinkle_speed,
                twinkle_offset,
            },
        ));
    }
}

fn spawn_deep_space(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let center_px = Vec2::new(playfield_center_x(), CANVAS_HEIGHT * 0.5);
    let center_world = px_to_world(center_px.x, center_px.y, 0.0).truncate();
    let radius = THETA_MAX as f32 * PIXELS_PER_RADIAN;
    let dot_image = create_soft_circle_texture(&mut images);

    // Boundary circle
    commands.spawn((
        ShapeBuilder::with(&shapes::Circle {
            radius,
            center: Vec2::ZERO,
        })
        .fill(Color::srgba(0.13, 0.27, 0.66, 0.05))
        .stroke((Color::srgba(0.4, 0.65, 1.0, 0.25), 2.0))
        .build(),
        Transform::from_xyz(center_world.x, center_world.y, 0.5),
    ));

    // Stars — initial spawn assuming default window size
    spawn_stars(&mut commands, 700.0, 760.0, dot_image.clone());

    // Portal dots (pre-allocated, hidden)
    for i in 0..MAX_PORTAL_DOTS {
        commands.spawn((
            Sprite {
                image: dot_image.clone(),
                color: color_from_hex(Colors::WALL).with_alpha(0.8),
                custom_size: Some(Vec2::splat(12.0)),
                ..default()
            },
            Transform::from_xyz(center_world.x, center_world.y, 1.5),
            Visibility::Hidden,
            DeepSpacePortalDot { index: i },
        ));
    }

    // Ball dots (pre-allocated, hidden)
    for i in 0..MAX_BALL_DOTS {
        commands.spawn((
            Sprite {
                image: dot_image.clone(),
                color: color_from_hex(Colors::BALL_GLOW).with_alpha(0.8),
                custom_size: Some(Vec2::splat(BALL_RADIUS * 2.0)),
                ..default()
            },
            Transform::from_xyz(center_world.x, center_world.y, 1.8),
            Visibility::Hidden,
            DeepSpaceBallDot { index: i },
        ));
    }

    // Ball tail dots (pre-allocated, hidden)
    for ball_index in 0..MAX_BALL_DOTS {
        for segment in 1..=TAIL_SEGMENTS {
            let t = segment as f32;
            let alpha =
                TAIL_START_ALPHA + ((TAIL_END_ALPHA - TAIL_START_ALPHA) * t) / TAIL_SEGMENTS as f32;
            let radius = (BALL_RADIUS * (1.0 - t * 0.15)).max(1.0);
            commands.spawn((
                Sprite {
                    image: dot_image.clone(),
                    color: color_from_hex(Colors::BALL_GLOW).with_alpha(alpha),
                    custom_size: Some(Vec2::splat(radius * 2.0)),
                    ..default()
                },
                Transform::from_xyz(center_world.x, center_world.y, 1.7),
                Visibility::Hidden,
                DeepSpaceBallTailDot {
                    ball_index,
                    segment,
                },
            ));
        }
    }

    // Self marker
    let self_marker_ring = commands
        .spawn((
            ShapeBuilder::with(&shapes::Circle {
                radius: 14.0,
                center: Vec2::ZERO,
            })
            .stroke((color_from_hex(Colors::BALL_GLOW).with_alpha(0.7), 2.0))
            .build(),
            Transform::from_xyz(center_world.x, center_world.y, 2.0),
            SelfMarkerRing,
        ))
        .id();

    let self_marker_core = commands
        .spawn((
            ShapeBuilder::with(&shapes::Circle {
                radius: 5.0,
                center: Vec2::ZERO,
            })
            .fill(color_from_hex(Colors::BALL_GLOW).with_alpha(0.8))
            .build(),
            Transform::from_xyz(center_world.x, center_world.y, 2.1),
            SelfMarkerCore,
        ))
        .id();

    commands.insert_resource(DeepSpaceState {
        center_px,
        time: 0.0,
        self_marker_ring,
        self_marker_core,
        last_window_size: Vec2::new(700.0, 760.0),
        dot_image,
    });
}

fn regenerate_stars_on_resize(
    mut commands: Commands,
    mut deep: ResMut<DeepSpaceState>,
    q_window: Query<&Window, With<PrimaryWindow>>,
    q_stars: Query<Entity, With<DeepSpaceStar>>,
) {
    let Ok(window) = q_window.single() else {
        return;
    };
    let size = Vec2::new(window.width(), window.height());
    if size.x <= 0.0 || size.y <= 0.0 {
        return;
    }
    if (size - deep.last_window_size).length() < 1.0 {
        return;
    }
    deep.last_window_size = size;

    // Despawn old stars
    for entity in &q_stars {
        commands.entity(entity).despawn();
    }

    // Spawn new stars filling the window
    spawn_stars(&mut commands, size.x, size.y, deep.dot_image.clone());
}

fn animate_stars(
    time: Res<Time>,
    mut deep: ResMut<DeepSpaceState>,
    mut q_stars: Query<(&DeepSpaceStar, &mut Sprite)>,
) {
    deep.time += time.delta_secs();
    for (star, mut sprite) in &mut q_stars {
        let twinkle = (deep.time * star.twinkle_speed + star.twinkle_offset).sin() * 0.3 + 0.7;
        sprite.color = color_from_hex(Colors::STAR).with_alpha(star.base_alpha * twinkle);
    }
}

fn update_portal_dots(
    conn: Res<ServerConnection>,
    deep: Res<DeepSpaceState>,
    mut q_dots: Query<(
        &DeepSpacePortalDot,
        &mut Transform,
        &mut Visibility,
        &mut Sprite,
    )>,
) {
    let self_pos = conn
        .players
        .iter()
        .find(|p| p.id == conn.self_id)
        .map(|p| p.portal_pos)
        .unwrap_or(crate::shared::vec3::Vec3::new(1.0, 0.0, 0.0));

    let (e1, e2) = crate::shared::vec3::build_tangent_basis(self_pos);
    let cos_theta_max = THETA_MAX.cos();

    for (dot, mut tf, mut vis, mut sprite) in &mut q_dots {
        if dot.index >= conn.players.len() {
            if *vis != Visibility::Hidden {
                *vis = Visibility::Hidden;
            }
            continue;
        }

        let p = &conn.players[dot.index];
        if let Some((sx, sy)) = project(
            self_pos,
            p.portal_pos,
            e1,
            e2,
            deep.center_px,
            cos_theta_max,
        ) {
            let world = px_to_world(sx, sy, 0.0);
            tf.translation.x = world.x;
            tf.translation.y = world.y;
            if *vis != Visibility::Visible {
                *vis = Visibility::Visible;
            }
            let alpha = if p.paused { 0.2 } else { 0.6 };
            let new_color = color_from_hex(p.color).with_alpha(alpha);
            if sprite.color != new_color {
                sprite.color = new_color;
            }
        } else if *vis != Visibility::Hidden {
            *vis = Visibility::Hidden;
        }
    }
}

fn player_color_signature(players: &[crate::shared::types::Player]) -> u64 {
    let mut sig: u64 = 0;
    for p in players {
        sig = sig
            .wrapping_mul(0x9e3779b185ebca87)
            .wrapping_add(p.id as u64)
            .wrapping_add((p.color as u64) << 8);
    }
    sig
}

fn update_ball_dots(
    conn: Res<ServerConnection>,
    deep: Res<DeepSpaceState>,
    mut owner_colors: Local<Vec<(u32, u32)>>,
    mut last_sig: Local<u64>,
    mut q_dots: Query<(
        &DeepSpaceBallDot,
        &mut Transform,
        &mut Visibility,
        &mut Sprite,
    )>,
) {
    let self_pos = conn
        .players
        .iter()
        .find(|p| p.id == conn.self_id)
        .map(|p| p.portal_pos)
        .unwrap_or(crate::shared::vec3::Vec3::new(1.0, 0.0, 0.0));

    let (e1, e2) = crate::shared::vec3::build_tangent_basis(self_pos);
    let cos_theta_max = THETA_MAX.cos();

    let sig = player_color_signature(&conn.players);
    if sig != *last_sig {
        owner_colors.clear();
        owner_colors.extend(conn.players.iter().map(|p| (p.id, p.color)));
        *last_sig = sig;
    }

    for (dot, mut tf, mut vis, mut sprite) in &mut q_dots {
        if dot.index >= conn.interpolated_balls.len() {
            if *vis != Visibility::Hidden {
                *vis = Visibility::Hidden;
            }
            continue;
        }

        let b = &conn.interpolated_balls[dot.index];
        if let Some((sx, sy)) = project(self_pos, b.pos, e1, e2, deep.center_px, cos_theta_max) {
            let world = px_to_world(sx, sy, 0.0);
            tf.translation.x = world.x;
            tf.translation.y = world.y;
            if *vis != Visibility::Visible {
                *vis = Visibility::Visible;
            }

            let color = owner_colors
                .iter()
                .find(|(id, _)| *id == b.owner_id)
                .map(|(_, color)| *color)
                .unwrap_or(Colors::BALL_GLOW);
            let new_color = color_from_hex(color).with_alpha(0.8);
            if sprite.color != new_color {
                sprite.color = new_color;
            }
        } else if *vis != Visibility::Hidden {
            *vis = Visibility::Hidden;
        }
    }
}

fn update_ball_trails(
    conn: Res<ServerConnection>,
    deep: Res<DeepSpaceState>,
    mut owner_colors: Local<Vec<(u32, u32)>>,
    mut last_sig: Local<u64>,
    mut q_tails: Query<(
        &DeepSpaceBallTailDot,
        &mut Transform,
        &mut Visibility,
        &mut Sprite,
    )>,
) {
    let self_pos = conn
        .players
        .iter()
        .find(|p| p.id == conn.self_id)
        .map(|p| p.portal_pos)
        .unwrap_or(crate::shared::vec3::Vec3::new(1.0, 0.0, 0.0));

    let (e1, e2) = crate::shared::vec3::build_tangent_basis(self_pos);
    let cos_theta_max = THETA_MAX.cos();

    let sig = player_color_signature(&conn.players);
    if sig != *last_sig {
        owner_colors.clear();
        owner_colors.extend(conn.players.iter().map(|p| (p.id, p.color)));
        *last_sig = sig;
    }

    for (tail, mut tf, mut vis, mut sprite) in &mut q_tails {
        if tail.ball_index >= conn.interpolated_balls.len() {
            if *vis != Visibility::Hidden {
                *vis = Visibility::Hidden;
            }
            continue;
        }

        let ball = &conn.interpolated_balls[tail.ball_index];
        let mut tail_pos = ball.pos;
        crate::shared::vec3::rotate_normalize_in_place(
            &mut tail_pos,
            ball.axis,
            -ball.omega * tail.segment as f64 * TAIL_TIME_STEP,
        );

        if let Some((sx, sy)) = project(self_pos, tail_pos, e1, e2, deep.center_px, cos_theta_max) {
            let world = px_to_world(sx, sy, 0.0);
            tf.translation.x = world.x;
            tf.translation.y = world.y;
            if *vis != Visibility::Visible {
                *vis = Visibility::Visible;
            }

            let color = owner_colors
                .iter()
                .find(|(id, _)| *id == ball.owner_id)
                .map(|(_, color)| *color)
                .unwrap_or(Colors::BALL_GLOW);
            let t = tail.segment as f32;
            let alpha =
                TAIL_START_ALPHA + ((TAIL_END_ALPHA - TAIL_START_ALPHA) * t) / TAIL_SEGMENTS as f32;
            let new_color = color_from_hex(color).with_alpha(alpha);
            if sprite.color != new_color {
                sprite.color = new_color;
            }
        } else if *vis != Visibility::Hidden {
            *vis = Visibility::Hidden;
        }
    }
}

fn update_self_marker(
    conn: Res<ServerConnection>,
    deep: Res<DeepSpaceState>,
    mut last_color: Local<u32>,
    mut q_ring: Query<&mut Shape, (With<SelfMarkerRing>, Without<SelfMarkerCore>)>,
    mut q_core: Query<&mut Shape, (With<SelfMarkerCore>, Without<SelfMarkerRing>)>,
) {
    let self_color = conn
        .players
        .iter()
        .find(|p| p.id == conn.self_id)
        .map(|p| p.color)
        .unwrap_or(Colors::BALL_GLOW);

    if self_color == *last_color && *last_color != 0 {
        return;
    }
    *last_color = self_color;

    let ring_color = color_from_hex(self_color).with_alpha(0.7);
    if let Ok(mut shape) = q_ring.get_mut(deep.self_marker_ring) {
        if let Some(stroke) = shape.stroke.as_mut() {
            stroke.color = ring_color;
        }
    }
    let core_color = color_from_hex(self_color).with_alpha(0.8);
    if let Ok(mut shape) = q_core.get_mut(deep.self_marker_core) {
        if let Some(fill) = shape.fill.as_mut() {
            fill.color = core_color;
        }
    }
}

fn project(
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
