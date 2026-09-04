//! Package version resolution.
//!
//! Port of `_read_version` / `__version__` from `bin/cmux-herdr`. The `VERSION`
//! file at the install root is the source of truth; when it is unreadable the
//! binary falls back to the value embedded at build time (the same `VERSION`
//! file, wired through `Cargo.toml`), then to the legacy literal.
//!
//! Python read `VERSION` at runtime so a shipped tree could be re-versioned
//! without a rebuild. A compiled binary keeps that behavior by preferring a
//! runtime `VERSION` next to the executable, and only embeds a fallback so a
//! standalone binary still reports a correct version.

use std::path::PathBuf;

/// Legacy fallback matching Python's `__version__` when nothing else resolves.
const LEGACY_VERSION: &str = "0.5.0";

/// Version embedded at build time from `Cargo.toml` (kept in sync with the
/// repo `VERSION` file).
const EMBEDDED_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Return the package version (`_read_version`).
///
/// Resolution order:
/// 1. A `VERSION` file located at the install root (exe's parent's parent, then
///    exe's parent) — the runtime source of truth, mirroring Python's `_ROOT`.
/// 2. The build-time embedded version.
/// 3. The legacy literal `0.5.0`.
pub fn read_version() -> String {
    if let Some(v) = runtime_version() {
        return v;
    }
    if !EMBEDDED_VERSION.is_empty() {
        return EMBEDDED_VERSION.to_string();
    }
    LEGACY_VERSION.to_string()
}

/// Try to read `VERSION` from the install tree relative to the executable.
fn runtime_version() -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    // Python: `_ROOT = __file__.parent.parent`; the launcher lives in `bin/`.
    // Probe the grandparent (repo root) then the parent, matching an installed
    // layout of `<root>/bin/<exe>` or `<root>/<exe>`.
    for root in candidate_roots(&exe) {
        let candidate = root.join("VERSION");
        if let Ok(text) = std::fs::read_to_string(&candidate) {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

fn candidate_roots(exe: &std::path::Path) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(parent) = exe.parent() {
        if let Some(grandparent) = parent.parent() {
            roots.push(grandparent.to_path_buf());
        }
        roots.push(parent.to_path_buf());
    }
    roots
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_version_matches_cargo() {
        // CARGO_PKG_VERSION is the crate version (kept synced with VERSION).
        assert_eq!(EMBEDDED_VERSION, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn read_version_is_non_empty() {
        let v = read_version();
        assert!(!v.is_empty());
        // A dotted semver-ish string in every resolution branch.
        assert!(v.contains('.'), "version {v:?} should look like x.y.z");
    }
}
