//! Leash engine. Watches chosen Windows services and applies the user policy.
//!
//! This crate holds no Tauri code. Only `scm.rs` and `elevation.rs` may call
//! the Windows API.

pub mod config;
pub mod elevation;
pub mod log;
pub mod safety;
pub mod scm;
