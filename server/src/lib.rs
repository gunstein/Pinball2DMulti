//! Multiplayer pinball server.
//!
//! Players each have a 2D pinball board. When a ball escapes the top of a
//! board it enters **deep space** — a shared 3D unit sphere where balls
//! travel along great circles. When a ball reaches another player's portal
//! on the sphere it drops into that player's board from above.
//!
//! # Architecture
//!
//! - **`ws`** — WebSocket handler: one connection per player, validates
//!   input, enforces rate limits and connection caps.
//! - **`game_loop`** — Single async task that owns all mutable game state
//!   and runs the simulation tick at a fixed rate.
//! - **`state`** — `GameState`: player registry, bot management, ball
//!   production tracking, and the deep-space simulation.
//! - **`deep_space`** — `SphereDeepSpace`: the core simulation. Balls
//!   rotate on great circles (Rodrigues rotation), get rerouted toward
//!   portals via smooth slerp transitions, and are captured when they
//!   enter a portal's angular threshold.
//! - **`sphere`** — `PortalPlacement`: distributes player portals evenly
//!   on the sphere using a Fibonacci lattice.
//! - **`bot`** — AI players with personalities (Eager, Relaxed, Chaotic)
//!   that receive captured balls and send them back after a delay.
//! - **`vec3`** / **`player`** / **`protocol`** / **`config`** — shared
//!   types, serialization, and configuration.

pub mod bot;
pub mod config;
pub mod deep_space;
pub mod game_loop;
pub mod player;
pub mod protocol;
pub mod sphere;
pub mod state;
pub mod vec3;
pub mod ws;
