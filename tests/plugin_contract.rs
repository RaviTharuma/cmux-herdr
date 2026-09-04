//! Official cmux plugin-manager and launcher contract.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

fn executable(path: &Path) -> bool {
    fs::metadata(path)
        .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[test]
fn manifest_uses_fetch_build_and_sidebar_launcher() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let text = fs::read_to_string(root.join("cmux-plugin.toml")).unwrap();
    let version = fs::read_to_string(root.join("VERSION")).unwrap();
    let version = version.trim();
    assert_eq!(env!("CARGO_PKG_VERSION"), version);
    let doc = text.parse::<toml_edit::DocumentMut>().unwrap();
    assert_eq!(doc["plugin"]["name"].as_str(), Some("cmux-herdr"));
    assert_eq!(doc["plugin"]["kind"].as_str(), Some("sidebar"));
    assert_eq!(doc["plugin"]["version"].as_str(), Some(version));
    assert_eq!(
        doc["run"]["command"]
            .as_array()
            .unwrap()
            .get(0)
            .unwrap()
            .as_str(),
        Some("bin/cmux-herdr-sidebar")
    );
    assert_eq!(
        doc["build"]["command"]
            .as_array()
            .unwrap()
            .get(0)
            .unwrap()
            .as_str(),
        Some("bin/cmux-herdr-fetch")
    );
    for name in ["cmux-herdr", "cmux-herdr-sidebar", "cmux-herdr-fetch"] {
        let path = root.join("bin").join(name);
        assert!(executable(&path), "{} must be executable", path.display());
        let first = fs::read_to_string(path).unwrap();
        assert_eq!(first.lines().next(), Some("#!/bin/sh"));
    }
}

fn copy_executable(from: &Path, to: &Path) {
    fs::copy(from, to).unwrap();
    fs::set_permissions(to, fs::Permissions::from_mode(0o755)).unwrap();
}

#[test]
fn launchers_exec_cached_binary_and_resolve_symlink() {
    // Keep direct and symlink execution in one test. Some overlay filesystems
    // transiently return ETXTBSY when separate tests execute freshly copied
    // scripts concurrently.
    let source = Path::new(env!("CARGO_MANIFEST_DIR"));
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("checkout");
    fs::create_dir_all(root.join("bin")).unwrap();
    fs::create_dir_all(root.join(".cmux-herdr/bin")).unwrap();
    for name in ["cmux-herdr", "cmux-herdr-sidebar", "cmux-herdr-fetch"] {
        copy_executable(&source.join("bin").join(name), &root.join("bin").join(name));
    }
    let runtime = root.join(".cmux-herdr/bin/cmux-herdr");
    fs::write(&runtime, "#!/bin/sh\nprintf '%s\\n' \"$*\"\n").unwrap();
    fs::set_permissions(&runtime, fs::Permissions::from_mode(0o755)).unwrap();

    let cli = Command::new(root.join("bin/cmux-herdr"))
        .args(["status", "--json"])
        .output()
        .unwrap();
    assert!(cli.status.success());
    assert_eq!(String::from_utf8(cli.stdout).unwrap(), "status --json\n");

    let sidebar = Command::new(root.join("bin/cmux-herdr-sidebar"))
        .arg("--once")
        .output()
        .unwrap();
    assert!(sidebar.status.success());
    assert_eq!(
        String::from_utf8(sidebar.stdout).unwrap(),
        "sidebar --once\n"
    );

    let link_dir = tmp.path().join("home/.local/bin");
    fs::create_dir_all(&link_dir).unwrap();
    std::os::unix::fs::symlink(root.join("bin/cmux-herdr"), link_dir.join("cmux-herdr")).unwrap();
    let linked = Command::new(link_dir.join("cmux-herdr"))
        .arg("tree")
        .output()
        .unwrap();
    assert!(linked.status.success());
    assert_eq!(String::from_utf8(linked.stdout).unwrap(), "tree\n");
}

#[test]
fn installer_copy_fallback_installs_a_runnable_cli() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR"));
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("checkout");
    let home = tmp.path().join("home");
    let fake_bin = tmp.path().join("fake-bin");
    fs::create_dir_all(root.join("bin")).unwrap();
    fs::create_dir_all(root.join("scripts")).unwrap();
    fs::create_dir_all(&fake_bin).unwrap();
    copy_executable(
        &source.join("scripts/install.sh"),
        &root.join("scripts/install.sh"),
    );
    copy_executable(&source.join("bin/cmux-herdr"), &root.join("bin/cmux-herdr"));
    let original_launcher = fs::read(root.join("bin/cmux-herdr")).unwrap();
    fs::create_dir_all(home.join(".local/bin")).unwrap();
    std::os::unix::fs::symlink(
        root.join("bin/cmux-herdr"),
        home.join(".local/bin/cmux-herdr"),
    )
    .unwrap();
    let fetch = root.join("bin/cmux-herdr-fetch");
    fs::write(
        &fetch,
        r#"#!/bin/sh
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
mkdir -p "$root/.cmux-herdr/bin"
cat > "$root/.cmux-herdr/bin/cmux-herdr" <<'EOF'
#!/bin/sh
printf 'runtime %s\n' "$*"
EOF
chmod +x "$root/.cmux-herdr/bin/cmux-herdr"
"#,
    )
    .unwrap();
    fs::set_permissions(&fetch, fs::Permissions::from_mode(0o755)).unwrap();
    let fake_ln = fake_bin.join("ln");
    fs::write(&fake_ln, "#!/bin/sh\nexit 1\n").unwrap();
    fs::set_permissions(&fake_ln, fs::Permissions::from_mode(0o755)).unwrap();

    let path = format!(
        "{}:{}",
        fake_bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let install = Command::new(root.join("scripts/install.sh"))
        .env("HOME", &home)
        .env("PATH", path)
        .output()
        .unwrap();
    assert!(
        install.status.success(),
        "install failed: {}",
        String::from_utf8_lossy(&install.stderr)
    );
    assert_eq!(
        fs::read(root.join("bin/cmux-herdr")).unwrap(),
        original_launcher,
        "copy fallback overwrote the launcher through the existing symlink"
    );
    assert!(!fs::symlink_metadata(home.join(".local/bin/cmux-herdr"))
        .unwrap()
        .file_type()
        .is_symlink());

    let cli = Command::new(home.join(".local/bin/cmux-herdr"))
        .arg("status")
        .output()
        .unwrap();
    assert!(
        cli.status.success(),
        "copied CLI failed: {}",
        String::from_utf8_lossy(&cli.stderr)
    );
    assert_eq!(String::from_utf8(cli.stdout).unwrap(), "runtime status\n");
}
