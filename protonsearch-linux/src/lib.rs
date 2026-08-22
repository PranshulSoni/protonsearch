//! Linux-only ProtonSearch providers.
//!
//! This crate deliberately does not depend on the existing Windows-specific
//! crate. It is an additive Linux implementation boundary with its own GTK4
//! launcher entry point.

pub mod actions;
pub mod agent;
pub mod calculator;
pub mod capabilities;
pub mod clipboard;
pub mod desktop;
pub mod gui;
pub mod hermes;
pub mod hyprland;
pub mod icons;
pub mod performance;
pub mod platform;
pub mod providers;
pub mod search;
pub mod settings;
pub mod system;
pub mod tray;
pub mod update;
pub mod xdg;
