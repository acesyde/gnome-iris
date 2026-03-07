//! Core domain logic for gnome-iris — ReShade management for Wine/Proton games.
//!
//! This module is GTK-free. All types here are pure Rust.

pub mod app_state;
pub mod cache;
pub mod config;
pub mod game;
pub mod install;
pub mod reshade;
pub mod shaders;
pub mod steam;
