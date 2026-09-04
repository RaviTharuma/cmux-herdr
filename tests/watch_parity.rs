use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};

#[derive(Default)]
struct SocketStats {
    subscriptions: AtomicUsize,
    pane_reads: AtomicUsize,
    snapshots: AtomicUsize,
}

struct TestSocket {
    stats: Arc<SocketStats>,
    disconnect_first_subscription: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl TestSocket {
    fn start(path: &Path) -> Self {
        let listener = UnixListener::bind(path).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
        listener.set_nonblocking(true).unwrap();
        let stats = Arc::new(SocketStats::default());
        let disconnect_first_subscription = Arc::new(AtomicBool::new(false));
        let stop = Arc::new(AtomicBool::new(false));
        let thread = {
            let stats = Arc::clone(&stats);
            let disconnect = Arc::clone(&disconnect_first_subscription);
            let stop = Arc::clone(&stop);
            thread::spawn(move || {
                let mut clients = Vec::new();
                while !stop.load(Ordering::SeqCst) {
                    match listener.accept() {
                        Ok((stream, _)) => {
                            let stats = Arc::clone(&stats);
                            let disconnect = Arc::clone(&disconnect);
                            let stop = Arc::clone(&stop);
                            clients.push(thread::spawn(move || {
                                handle_client(stream, stats, disconnect, stop)
                            }));
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            thread::sleep(Duration::from_millis(5));
                        }
                        Err(error) => panic!("accept fake Herdr client: {error}"),
                    }
                }
                for client in clients {
                    let _ = client.join();
                }
            })
        };
        Self {
            stats,
            disconnect_first_subscription,
            stop,
            thread: Some(thread),
        }
    }
}

impl Drop for TestSocket {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn handle_client(
    mut stream: UnixStream,
    stats: Arc<SocketStats>,
    disconnect_first_subscription: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
) {
    stream
        .set_read_timeout(Some(Duration::from_millis(50)))
        .unwrap();
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    while !stop.load(Ordering::SeqCst) {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => return,
            Ok(_) => {}
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                continue;
            }
            Err(_) => return,
        }
        let request: Value = serde_json::from_str(&line).unwrap();
        let id = request["id"].as_str().unwrap();
        match request["method"].as_str().unwrap() {
            "events.subscribe" => {
                let subscription = stats.subscriptions.fetch_add(1, Ordering::SeqCst) + 1;
                if writeln!(
                    stream,
                    "{}",
                    json!({"id": id, "result": {"type": "subscription_started"}})
                )
                .is_err()
                    || stream.flush().is_err()
                {
                    return;
                }
                thread::sleep(Duration::from_millis(100));
                if writeln!(
                    stream,
                    "{}",
                    json!({"event": "pane.updated", "data": {"pane_id": "p1"}})
                )
                .is_err()
                    || stream.flush().is_err()
                {
                    return;
                }
                while !stop.load(Ordering::SeqCst) {
                    if subscription == 1
                        && disconnect_first_subscription.swap(false, Ordering::SeqCst)
                    {
                        return;
                    }
                    thread::sleep(Duration::from_millis(5));
                }
                return;
            }
            "session.snapshot" => {
                stats.snapshots.fetch_add(1, Ordering::SeqCst);
                write_response(&mut stream, id, snapshot());
            }
            "pane.read" => {
                stats.pane_reads.fetch_add(1, Ordering::SeqCst);
                write_response(&mut stream, id, json!({"text": "watch frame"}));
            }
            "pane.get" => write_response(
                &mut stream,
                id,
                json!({"pane_id": "p1", "agent_status": "working"}),
            ),
            method => panic!("unexpected fake Herdr RPC: {method}"),
        }
    }
}

fn snapshot() -> Value {
    json!({
        "workspaces": [{"workspace_id": "w1", "label": "work", "tab_count": 1}],
        "tabs": [{"tab_id": "t1", "workspace_id": "w1", "label": "main", "pane_count": 1}],
        "panes": [{
            "pane_id": "p1",
            "tab_id": "t1",
            "workspace_id": "w1",
            "label": "agent",
            "agent": "pi",
            "agent_status": "working",
            "focused": true
        }],
        "layouts": []
    })
}

fn write_response(stream: &mut UnixStream, id: &str, result: Value) {
    let _ = writeln!(stream, "{}", json!({"id": id, "result": result}));
    let _ = stream.flush();
}

fn write_executable(path: &Path, text: &str) {
    fs::write(path, text).unwrap();
    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).unwrap();
}

fn find_state_file(directory: &Path, prefix: &str) -> Option<PathBuf> {
    fs::read_dir(directory).ok()?.flatten().find_map(|entry| {
        entry
            .file_name()
            .to_str()
            .filter(|name| name.starts_with(prefix))
            .map(|_| entry.path())
    })
}

fn heartbeat(path: &Path) -> i64 {
    serde_json::from_str::<Value>(&fs::read_to_string(path).unwrap()).unwrap()["heartbeat_ms"]
        .as_i64()
        .unwrap()
}

fn write_native_lease(state_dir: &Path, plugin_writer: &Path) -> PathBuf {
    let mut lease: Value =
        serde_json::from_str(&fs::read_to_string(plugin_writer).unwrap()).unwrap();
    lease["owner"] = json!("native");
    lease["pid"] = json!(std::process::id());
    lease["heartbeat_ms"] = json!(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64);
    let path = state_dir.join("native-live");
    fs::write(&path, serde_json::to_vec(&lease).unwrap()).unwrap();
    path
}

fn wait_until(timeout: Duration, mut condition: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if condition() {
            return true;
        }
        thread::sleep(Duration::from_millis(10));
    }
    condition()
}

struct ChildGuard(Option<Child>);

impl ChildGuard {
    fn terminate(mut self) -> Output {
        let child = self.0.take().unwrap();
        let pid = rustix::process::Pid::from_raw(child.id() as i32).unwrap();
        rustix::process::kill_process(pid, rustix::process::Signal::Term).unwrap();
        child.wait_with_output().unwrap()
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

#[test]
fn continuous_watch_holds_writer_pumps_events_resyncs_gaps_and_cleans_up() {
    let temp = tempfile::tempdir().unwrap();
    let socket_path = temp.path().join("herdr.sock");
    let socket = TestSocket::start(&socket_path);
    let herdr_log = temp.path().join("herdr.log");
    let cmux_log = temp.path().join("cmux.log");
    write_executable(
        &temp.path().join("herdr"),
        r#"#!/bin/sh
printf '%s\n' "$*" >> "$FAKE_HERDR_LOG"
case "$*" in
  "status") printf '%s\n' '{"status":"ok"}' ;;
  *) printf 'unexpected command: %s\n' "$*" >&2; exit 9 ;;
esac
"#,
    );
    write_executable(
        &temp.path().join("cmux"),
        r#"#!/bin/sh
printf '%s\n' "$*" >> "$FAKE_CMUX_LOG"
case "$1" in
  create-terminal|run) printf '%s\n' '{"surface_id":"surface-p1","pane_id":"pane-p1"}' ;;
  tree|list-terminals|ids) printf '%s\n' '{"items":[]}' ;;
  *) printf '%s\n' '{"ok":true}' ;;
esac
"#,
    );
    let inherited_path = std::env::var("PATH").unwrap_or_default();
    let mut command = Command::new(env!("CARGO_BIN_EXE_cmux-herdr"));
    command
        .args(["watch", "--interval", "0.5", "--workspace", "ws1"])
        .env(
            "PATH",
            format!("{}:{inherited_path}", temp.path().display()),
        )
        .env("HOME", temp.path())
        .env("XDG_STATE_HOME", temp.path().join("state"))
        .env("HERDR_ENV", "1")
        .env("HERDR_SOCKET_PATH", &socket_path)
        .env("HERDR_WORKSPACE_ID", "w1")
        .env("CMUX_SURFACE_ID", "surface-cli")
        .env("FAKE_HERDR_LOG", &herdr_log)
        .env("FAKE_CMUX_LOG", &cmux_log)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let child = ChildGuard(Some(command.spawn().unwrap()));
    let state_dir = temp.path().join("state/cmux-herdr");

    assert!(
        wait_until(Duration::from_secs(3), || {
            find_state_file(&state_dir, "writer-").is_some()
                && find_state_file(&state_dir, "restore-").is_some()
                && socket.stats.pane_reads.load(Ordering::SeqCst) >= 2
        }),
        "watch never acquired the writer, persisted restore state, and pumped pane output"
    );
    assert_eq!(
        socket.stats.subscriptions.load(Ordering::SeqCst),
        1,
        "watch should retain its initial events.subscribe stream"
    );

    let writer = find_state_file(&state_dir, "writer-").unwrap();
    let first_heartbeat = heartbeat(&writer);
    assert!(
        wait_until(Duration::from_secs(2), || heartbeat(&writer)
            > first_heartbeat),
        "writer heartbeat did not advance"
    );

    let snapshots_before_gap = socket.stats.snapshots.load(Ordering::SeqCst);
    socket
        .disconnect_first_subscription
        .store(true, Ordering::SeqCst);
    assert!(
        wait_until(Duration::from_secs(3), || {
            socket.stats.subscriptions.load(Ordering::SeqCst) >= 2
                && socket.stats.snapshots.load(Ordering::SeqCst) >= snapshots_before_gap + 2
        }),
        "a lost event stream did not reconnect and resync live + projected topology"
    );

    let native_lease = write_native_lease(&state_dir, &writer);
    assert!(
        wait_until(Duration::from_secs(2), || {
            find_state_file(&state_dir, "writer-").is_none()
                && find_state_file(&state_dir, "plugin-live").is_none()
        }),
        "watch did not release plugin ownership after native takeover"
    );
    let reads_after_yield = socket.stats.pane_reads.load(Ordering::SeqCst);
    thread::sleep(Duration::from_millis(650));
    assert_eq!(
        socket.stats.pane_reads.load(Ordering::SeqCst),
        reads_after_yield,
        "yielded plugin kept pumping the detached host"
    );
    fs::remove_file(native_lease).unwrap();
    assert!(
        wait_until(Duration::from_secs(3), || {
            find_state_file(&state_dir, "writer-").is_some()
                && socket.stats.pane_reads.load(Ordering::SeqCst) > reads_after_yield
        }),
        "watch did not resume plugin ownership after native lease removal"
    );

    let resumed_writer = find_state_file(&state_dir, "writer-").unwrap();
    let native_lease = write_native_lease(&state_dir, &resumed_writer);
    assert!(
        wait_until(Duration::from_secs(2), || {
            find_state_file(&state_dir, "writer-").is_none()
                && find_state_file(&state_dir, "plugin-live").is_none()
        }),
        "watch did not finish yielding before shutdown"
    );

    let output = child.terminate();
    assert!(
        output.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    fs::remove_file(native_lease).unwrap();
    assert!(find_state_file(&state_dir, "writer-").is_none());
    assert!(find_state_file(&state_dir, "plugin-live").is_none());
    assert!(find_state_file(&state_dir, "restore-").is_none());
    let herdr_calls = fs::read_to_string(herdr_log).unwrap_or_default();
    assert!(
        !herdr_calls.contains("server stop") && !herdr_calls.contains("server.stop"),
        "watch shutdown must leave Herdr running: {herdr_calls}"
    );
}
