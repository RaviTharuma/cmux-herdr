//! cmux-herdr runtime library.
//!
//! Modules mirror the former `bridge/cmux_herdr_*.py` responsibilities. The
//! binary target is intentionally a thin exit-code adapter around [`cli::run`].

pub mod api;
pub mod bridge;
pub mod cli;
pub mod control;
pub mod engine;
pub mod handoff;
pub mod host;
pub mod impose;
pub mod io;
pub mod layout;
pub mod lifecycle;
pub mod live;
pub mod mirror;
pub mod model;
pub mod pump;
pub mod session;
pub mod sidebar;
pub mod socket;
pub mod state;
pub mod status;
pub mod update;
pub mod version;
