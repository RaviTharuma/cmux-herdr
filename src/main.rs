//! cmux-herdr — Herdr/cmux integration plugin (Rust port).
//!
//! Behavioral port of the Python bridge. Modules mirror `bridge/cmux_herdr_*`.

mod api;
mod layout;
mod model;
mod socket;
mod status;

fn main() {
    // CLI dispatch is wired in a later phase (CliAndUpdate). This binary
    // currently exposes the ported library modules; `cargo test` exercises
    // socket + api parity.
    eprintln!("cmux-herdr: CLI dispatch not yet wired");
    std::process::exit(2);
}
