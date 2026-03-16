//! gnome-iris domain library — pure-Rust `ReShade` management for Wine/Proton games.
//!
//! This crate exposes the GTK-free domain layer. The GTK/Relm4 UI lives in the
//! binary crate only.

/// Domain logic — `ReShade` installation, game tracking, shader sync, and more.
pub mod reshade;
