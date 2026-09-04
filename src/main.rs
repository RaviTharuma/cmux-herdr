//! cmux-herdr — Herdr/cmux integration plugin (Rust port).
//!
//! Behavioral port of the Python bridge. Modules mirror `bridge/cmux_herdr_*`.
mod api;
mod handoff;
mod impose;
mod layout;
mod model;
mod socket;
mod state;
mod status;
mod version;

use std::sync::LazyLock;

use clap::Command;

/// Process-lifetime resolved version so clap can borrow a `&'static str`.
/// Mirrors Python's `_read_version()` (runtime `VERSION` file, embedded
/// fallback), evaluated once at first parser build.
static VERSION: LazyLock<String> = LazyLock::new(version::read_version);

/// Build the top-level argument parser (`build_parser`).
///
/// Subcommands are registered here as the CLI layer is ported (CliAndUpdate
/// phase). The parser prog, description, and `--version` behavior mirror the
/// Python `argparse` setup in `bin/cmux-herdr`.
fn build_parser() -> Command {
    Command::new("cmux-herdr")
        .about(
            "cmux plugin for Herdr — cmux is the official UI, with status pills, \
             tab/pane mirroring, and inner-mux control.",
        )
        .version(VERSION.as_str())
        .subcommand_required(true)
        .arg_required_else_help(true)
}

fn main() {
    let matches = build_parser().get_matches();
    match matches.subcommand() {
        // Subcommand handlers are wired in the CliAndUpdate phase. Until then a
        // recognized-but-unimplemented command exits 2, distinct from clap's
        // usage-error exit code 2 for unknown commands (clap prints its own
        // message before we reach here).
        Some((name, _sub)) => {
            eprintln!("cmux-herdr: subcommand '{name}' not yet wired");
            std::process::exit(2);
        }
        None => {
            eprintln!("cmux-herdr: no subcommand");
            std::process::exit(2);
        }
    }
}
