//! cmux-herdr — Herdr/cmux integration plugin (Rust port).
//!
//! Behavioral port of the Python bridge. Modules mirror `bridge/cmux_herdr_*`.
mod api;
mod bridge;
mod cli;
mod control;
mod engine;
mod handoff;
mod host;
mod impose;
mod io;
mod layout;
mod lifecycle;
mod live;
mod model;
mod mirror;
mod pump;
mod session;
mod sidebar;
mod socket;
mod state;
mod status;
mod update;
mod version;

fn main() {
    std::process::exit(cli::run());
}
