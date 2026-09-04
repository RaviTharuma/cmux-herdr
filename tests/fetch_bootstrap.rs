//! Behavioral tests for the plugin-manager release-binary bootstrap.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use sha2::{Digest, Sha256};

const VERSION: &str = "0.6.1";
const TARGET: &str = "x86_64-unknown-linux-gnu";
const PAYLOAD: &[u8] = b"verified cmux-herdr binary\n";

fn write_executable(path: &Path, body: &str) {
    fs::write(path, body).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

fn fixture() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    fs::create_dir_all(root.join("bin")).unwrap();
    fs::write(root.join("VERSION"), format!("{VERSION}\n")).unwrap();
    let source = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/bin/cmux-herdr-fetch"
    ))
    .unwrap();
    let fetch = root.join("bin/cmux-herdr-fetch");
    write_executable(&fetch, &source);
    let fake_bin = root.join("fake-bin");
    fs::create_dir(&fake_bin).unwrap();
    write_executable(
        &fake_bin.join("uname"),
        "#!/bin/sh\ncase \"${1-}\" in -s) echo Linux;; -m) echo x86_64;; *) echo Linux;; esac\n",
    );
    (tmp, fetch, fake_bin)
}

fn curl_script(hash: &str) -> String {
    let asset = format!("cmux-herdr-{VERSION}-{TARGET}");
    format!(
        r#"#!/bin/sh
out=
url=
secure=
secure_redirect=
while [ "$#" -gt 0 ]; do
  case "$1" in
    --output) shift; out=$1 ;;
    --proto) shift; [ "$1" = '=https' ] && secure=1 ;;
    --proto-redir) shift; [ "$1" = '=https' ] && secure_redirect=1 ;;
    https://*) url=$1 ;;
  esac
  shift
done
[ "$secure" = 1 ] && [ "$secure_redirect" = 1 ] || exit 90
case "$url" in
  https://example.invalid/releases/v{VERSION}/SHA256SUMS)
    printf '%s  %s\n' '{hash}' '{asset}' > "$out" ;;
  https://example.invalid/releases/v{VERSION}/{asset})
    printf 'verified cmux-herdr binary\n' > "$out" ;;
  *) exit 91 ;;
esac
"#
    )
}

fn run_fetch(fetch: &Path, fake_bin: &Path) -> std::process::Output {
    let system_path = std::env::var("PATH").unwrap_or_else(|_| "/usr/bin:/bin".into());
    Command::new("sh")
        .arg(fetch)
        .env("PATH", format!("{}:{system_path}", fake_bin.display()))
        .env("CMUX_HERDR_RELEASE_BASE_URL", "https://example.invalid/releases")
        .output()
        .unwrap()
}

#[test]
fn installs_verified_asset_atomically_and_executable() {
    let (tmp, fetch, fake_bin) = fixture();
    let hash = format!("{:x}", Sha256::digest(PAYLOAD));
    write_executable(&fake_bin.join("curl"), &curl_script(&hash));

    let output = run_fetch(&fetch, &fake_bin);
    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let installed = tmp.path().join(".cmux-herdr/bin/cmux-herdr");
    assert_eq!(fs::read(&installed).unwrap(), PAYLOAD);
    assert_ne!(fs::metadata(installed).unwrap().permissions().mode() & 0o111, 0);
}

#[test]
fn checksum_mismatch_leaves_existing_install_untouched() {
    let (tmp, fetch, fake_bin) = fixture();
    let installed = tmp.path().join(".cmux-herdr/bin/cmux-herdr");
    fs::create_dir_all(installed.parent().unwrap()).unwrap();
    fs::write(&installed, b"existing-good-binary").unwrap();
    write_executable(&fake_bin.join("curl"), &curl_script(&"0".repeat(64)));

    let output = run_fetch(&fetch, &fake_bin);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("checksum mismatch"));
    assert_eq!(fs::read(installed).unwrap(), b"existing-good-binary");
}

#[test]
fn rejects_non_https_release_base_before_touching_install() {
    let (tmp, fetch, fake_bin) = fixture();
    let output = Command::new("sh")
        .arg(fetch)
        .env("PATH", format!("{}:/usr/bin:/bin", fake_bin.display()))
        .env("CMUX_HERDR_RELEASE_BASE_URL", "http://example.invalid/releases")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("must use https://"));
    assert!(!tmp.path().join(".cmux-herdr/bin/cmux-herdr").exists());
}

#[test]
fn unsupported_platform_uses_source_build_when_cargo_exists() {
    let (tmp, fetch, fake_bin) = fixture();
    write_executable(
        &fake_bin.join("uname"),
        "#!/bin/sh\ncase \"${1-}\" in -s) echo Plan9;; -m) echo mips;; esac\n",
    );
    let root = tmp.path().to_string_lossy();
    write_executable(
        &fake_bin.join("cargo"),
        &format!(
            "#!/bin/sh\nmkdir -p '{root}/target/release'\nprintf 'source-built binary\\n' > '{root}/target/release/cmux-herdr'\n"
        ),
    );

    let output = run_fetch(&fetch, &fake_bin);
    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read(tmp.path().join(".cmux-herdr/bin/cmux-herdr")).unwrap(),
        b"source-built binary\n"
    );
}

#[test]
fn unsupported_platform_without_cargo_keeps_existing_install() {
    let (tmp, fetch, fake_bin) = fixture();
    write_executable(
        &fake_bin.join("uname"),
        "#!/bin/sh\ncase \"${1-}\" in -s) echo Plan9;; -m) echo mips;; esac\n",
    );
    let installed = tmp.path().join(".cmux-herdr/bin/cmux-herdr");
    fs::create_dir_all(installed.parent().unwrap()).unwrap();
    fs::write(&installed, b"existing-good-binary").unwrap();

    let output = Command::new("sh")
        .arg(fetch)
        .env("PATH", format!("{}:/usr/bin:/bin", fake_bin.display()))
        .env("CMUX_HERDR_RELEASE_BASE_URL", "https://example.invalid/releases")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("unsupported platform"));
    assert_eq!(fs::read(installed).unwrap(), b"existing-good-binary");
}


#[test]
fn refuses_symlinked_install_directory() {
    let (tmp, fetch, fake_bin) = fixture();
    let redirected = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(redirected.path(), tmp.path().join(".cmux-herdr")).unwrap();
    let hash = format!("{:x}", Sha256::digest(PAYLOAD));
    write_executable(&fake_bin.join("curl"), &curl_script(&hash));

    let output = run_fetch(&fetch, &fake_bin);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("refusing symlinked install directory"));
    assert!(fs::read_dir(redirected.path()).unwrap().next().is_none());
}