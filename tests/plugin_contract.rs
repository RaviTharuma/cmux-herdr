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
    let doc = text.parse::<toml_edit::DocumentMut>().unwrap();
    assert_eq!(doc["plugin"]["name"].as_str(), Some("cmux-herdr"));
    assert_eq!(doc["plugin"]["kind"].as_str(), Some("sidebar"));
    assert_eq!(
        doc["plugin"]["version"].as_str(),
        Some(fs::read_to_string(root.join("VERSION")).unwrap().trim())
    );
    assert_eq!(
        doc["run"]["command"].as_array().unwrap().get(0).unwrap().as_str(),
        Some("bin/cmux-herdr-sidebar")
    );
    assert_eq!(
        doc["build"]["command"].as_array().unwrap().get(0).unwrap().as_str(),
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
fn launchers_exec_one_cached_binary_with_sidebar_subcommand() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR"));
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
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
    assert_eq!(String::from_utf8(sidebar.stdout).unwrap(), "sidebar --once\n");
}

#[test]
fn cli_launcher_resolves_symlink_to_checkout() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR"));
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("checkout");
    fs::create_dir_all(root.join("bin")).unwrap();
    fs::create_dir_all(root.join(".cmux-herdr/bin")).unwrap();
    copy_executable(
        &source.join("bin/cmux-herdr"),
        &root.join("bin/cmux-herdr"),
    );
    // Fetch is not called because the cached runtime exists.
    let runtime = root.join(".cmux-herdr/bin/cmux-herdr");
    fs::write(&runtime, "#!/bin/sh\nprintf 'resolved:%s\\n' \"$*\"\n").unwrap();
    fs::set_permissions(&runtime, fs::Permissions::from_mode(0o755)).unwrap();
    let link_dir = tmp.path().join("home/.local/bin");
    fs::create_dir_all(&link_dir).unwrap();
    std::os::unix::fs::symlink(root.join("bin/cmux-herdr"), link_dir.join("cmux-herdr")).unwrap();

    let output = Command::new(link_dir.join("cmux-herdr"))
        .arg("tree")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "resolved:tree\n");
}
