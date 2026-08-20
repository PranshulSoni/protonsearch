//! Linux-only ProtonSearch providers.
//!
//! This crate deliberately does not depend on the existing Windows-specific
//! crate. It is an additive Linux implementation boundary with its own GTK4
//! launcher entry point.

pub mod actions;
pub mod calculator;
pub mod capabilities;
pub mod desktop;
pub mod gui;
pub mod hyprland;
pub mod icons;
pub mod providers;
pub mod search;
pub mod settings;
pub mod system;
pub mod tray;
pub mod xdg;
