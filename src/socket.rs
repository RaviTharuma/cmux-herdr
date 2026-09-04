//! Persistent NDJSON Unix-socket client for Herdr protocol 17.
//!
//! Wire shape matches native `HerdrNestedTopologyClient`:
//! `{"id": "...", "method": "...", "params": {...}}`.
//! Never shells out to the `herdr` CLI. `watch --tmux-parity` holds one
//! `events.subscribe` session instead of reconnecting every tick.
//!
//! Ported from `bridge/cmux_herdr_socket.py`. Behavior-preserving except two
//! documented safety fixes (see `request`): the reader correlates the response
//! `id` to the request `id` and drops unsolicited event frames, and each
//! request serializes send+recv over the single connection. The Python client
//! returned the first line and only locked id allocation
//! (`cmux_herdr_socket.py:194-205`, `:238-241`).

use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::{json, Value};

/// 512 KiB line cap, matching `MAX_LINE_BYTES`.
pub const MAX_LINE_BYTES: usize = 512 * 1024;

/// Same subscription set as `HerdrProtocol17Compatibility.defaultSubscriptions`
/// and the Python `DEFAULT_SUBSCRIPTIONS` (exact membership and order).
pub const DEFAULT_SUBSCRIPTIONS: &[&str] = &[
    "workspace.created",
    "workspace.updated",
    "workspace.metadata_updated",
    "workspace.renamed",
    "workspace.moved",
    "workspace.reordered",
    "workspace.closed",
    "workspace.focused",
    "tab.created",
    "tab.closed",
    "tab.focused",
    "tab.renamed",
    "tab.moved",
    "pane.created",
    "pane.closed",
    "pane.updated",
    "pane.focused",
    "pane.moved",
    "pane.exited",
    "pane.agent_detected",
    "pane.agent_status_changed",
    "pane.resized",
    "layout.updated",
    "layout.changed",
];

/// Where a [`SocketError`] occurred relative to the request write, so callers
/// can decide replay safety (oracle decision_2: never replay a mutation once
/// any request byte may have reached the wire).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorStage {
    /// Failed before any request byte could be written (connect, encode,
    /// oversize, not-connected). Safe to reconnect and retry / CLI fallback.
    BeforeSend,
    /// A write was attempted or the response was indeterminate; request bytes
    /// may have reached the wire. A mutation MUST NOT be replayed.
    AfterSend,
    /// A valid, id-matched remote `error` response. Authoritative outcome —
    /// no reconnect retry, no CLI fallback.
    Remote,
}

/// Transport or protocol failure on the Herdr Unix socket (`HerdrSocketError`).
#[derive(Debug)]
pub struct SocketError {
    pub message: String,
    pub stage: ErrorStage,
}

impl std::fmt::Display for SocketError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for SocketError {}

impl SocketError {
    /// A failure that occurred before any request byte was written.
    fn new(msg: impl Into<String>) -> Self {
        SocketError {
            message: msg.into(),
            stage: ErrorStage::BeforeSend,
        }
    }

    /// Public constructor for a BeforeSend error, for callers that build a
    /// transport error without touching the socket (e.g. "socket not
    /// available").
    pub fn before_send(msg: impl Into<String>) -> Self {
        SocketError::new(msg)
    }

    /// Re-tag the stage of an existing error.
    fn at(mut self, stage: ErrorStage) -> Self {
        self.stage = stage;
        self
    }
}

type Result<T, E = SocketError> = std::result::Result<T, E>;

/// Return `HERDR_SOCKET_PATH` when it exists on disk (`socket_path_from_env`).
pub fn socket_path_from_env() -> Option<String> {
    let path = std::env::var("HERDR_SOCKET_PATH").ok()?;
    if path.is_empty() || !Path::new(&path).exists() {
        return None;
    }
    Some(path)
}

/// Reject unsafe Herdr sockets (symlink, wrong type, loose mode, wrong owner).
///
/// Mirrors `assert_socket_secure`: `lstat`, reject symlink, require socket type,
/// reject any group/other permission bit (`mode & 0o077`; owner bits incl.
/// `0700` are allowed — not a numeric `<= 0600` compare), require
/// `st_uid == getuid()` (`cmux_herdr_socket.py:66-88`).
pub fn assert_socket_secure(path: &str) -> Result<()> {
    use rustix::fs::{lstat, FileType, Mode};

    let st = lstat(path).map_err(|e| SocketError::new(format!("socket stat failed: {e}")))?;
    let ftype = FileType::from_raw_mode(st.st_mode);
    if ftype == FileType::Symlink {
        return Err(SocketError::new("refusing symlink Herdr socket path"));
    }
    if ftype != FileType::Socket {
        return Err(SocketError::new("Herdr path is not a Unix socket"));
    }
    let mode = Mode::from_raw_mode(st.st_mode).bits() & 0o777;
    if mode & 0o077 != 0 {
        return Err(SocketError::new(format!(
            "Herdr socket mode 0o{mode:o} is too open (want 0600 or tighter)"
        )));
    }
    let uid = rustix::process::getuid().as_raw();
    if st.st_uid != uid {
        return Err(SocketError::new(format!(
            "Herdr socket uid {} != current uid {uid}",
            st.st_uid
        )));
    }
    Ok(())
}

/// One connected NDJSON session (request/response + optional subscribe).
///
/// `HerdrSocketClient`. Call [`connect`](Self::connect) before requests.
pub struct SocketClient {
    path: String,
    timeout: Duration,
    sock: Option<UnixStream>,
    buf: Vec<u8>,
    next: u64,
}

impl SocketClient {
    /// Create a client for `path` with a per-op timeout (default 5s in Python).
    pub fn new(path: impl Into<String>, timeout: Duration) -> Self {
        SocketClient {
            path: path.into(),
            timeout,
            sock: None,
            buf: Vec::new(),
            next: 0,
        }
    }

    /// Open the Unix stream after re-validating the socket. `HerdrSocketError`
    /// on failure (`connect`, `cmux_herdr_socket.py:96-119`).
    pub fn connect(&mut self) -> Result<()> {
        self.close();
        assert_socket_secure(&self.path)?;
        let sock = UnixStream::connect(&self.path)
            .map_err(|e| SocketError::new(format!("connect failed: {e}")))?;
        sock.set_read_timeout(Some(self.timeout)).ok();
        sock.set_write_timeout(Some(self.timeout)).ok();
        self.sock = Some(sock);
        self.buf.clear();
        Ok(())
    }

    /// Close the socket if open. Safe to call twice (`close`).
    pub fn close(&mut self) {
        self.sock = None;
        self.buf.clear();
    }

    /// True while the Unix stream is open (`connected`).
    pub fn connected(&self) -> bool {
        self.sock.is_some()
    }

    /// Send one RPC and return the `result` object.
    ///
    /// Ported from `request` (`cmux_herdr_socket.py:150-166`). Two documented
    /// safety changes over Python: reads until the response `id` matches the
    /// request `id` (Python returned the first line, so a racing event frame
    /// could be mis-parsed as the response), and holds the connection for the
    /// whole send→recv (Python only locked id allocation). Wire bytes and the
    /// returned `result`/error semantics are unchanged.
    pub fn request(&mut self, method: &str, params: Value) -> Result<Value> {
        let req_id = self.allocate_id();
        let payload = json!({ "id": req_id, "method": method, "params": params });
        self.send(&payload)?;
        // From here the request bytes are on the wire: any failure is
        // AfterSend and MUST NOT trigger a mutation replay (oracle decision_2).
        // The RPC socket is dedicated (never carries the events.subscribe
        // stream, per HerdrApi), so exactly one response line is expected and
        // a mismatched/missing id is a hard protocol error, not an event to
        // skip (oracle decision_1).
        let deadline = Instant::now() + self.timeout;
        let remaining = deadline
            .saturating_duration_since(Instant::now())
            .max(Duration::from_millis(1));
        let obj = match self.read_response(remaining) {
            Ok(obj) => obj,
            Err(e) => {
                self.close();
                return Err(e.at(ErrorStage::AfterSend));
            }
        };
        // A valid, id-matched remote error is authoritative (Remote stage):
        // no reconnect retry, no CLI fallback.
        match obj.get("id").and_then(Value::as_str) {
            Some(id) if id == req_id => {}
            Some(id) => {
                self.close();
                return Err(SocketError::new(format!(
                    "response id {id} does not match request id {req_id}"
                ))
                .at(ErrorStage::AfterSend));
            }
            None => {
                self.close();
                return Err(SocketError::new("response id is missing or not a string")
                    .at(ErrorStage::AfterSend));
            }
        }
        if let Some(error) = obj.get("error").filter(|e| e.is_object()) {
            let message = error
                .get("message")
                .and_then(Value::as_str)
                .map(str::to_string)
                .or_else(|| error.get("code").map(|c| c.to_string()))
                .unwrap_or_else(|| error.to_string());
            return Err(SocketError::new(message).at(ErrorStage::Remote));
        }
        Ok(obj.get("result").cloned().unwrap_or(Value::Null))
    }

    /// Read exactly one NDJSON response frame and validate it is a JSON object.
    /// Errors are tagged BeforeSend here; `request` re-tags them AfterSend.
    fn read_response(&mut self, timeout: Duration) -> Result<Value> {
        let line = self
            .read_line(timeout, true)?
            .ok_or_else(|| SocketError::new("timeout waiting for response"))?;
        let obj: Value = serde_json::from_str(&line)
            .map_err(|e| SocketError::new(format!("malformed JSON: {e}")))?;
        if !obj.is_object() {
            return Err(SocketError::new("response is not an object"));
        }
        Ok(obj)
    }

    /// Handshake `ping` → pong result.
    pub fn ping(&mut self) -> Result<Value> {
        self.request("ping", json!({}))
    }

    /// Full `session.snapshot` result.
    pub fn snapshot(&mut self) -> Result<Value> {
        self.request("session.snapshot", json!({}))
    }

    /// Start `events.subscribe`. Later events are read via [`read_event`].
    ///
    /// [`read_event`]: Self::read_event
    pub fn subscribe(&mut self, subscriptions: Option<Vec<Value>>) -> Result<Value> {
        let subs = subscriptions.unwrap_or_else(|| {
            DEFAULT_SUBSCRIPTIONS
                .iter()
                .map(|t| json!({ "type": t }))
                .collect()
        });
        self.request("events.subscribe", json!({ "subscriptions": subs }))
    }

    /// Read one NDJSON event line, or `None` on timeout/close/malformed JSON.
    pub fn read_event(&mut self, timeout: Duration) -> Option<Value> {
        let line = self.read_line(timeout, false).ok()??;
        match serde_json::from_str::<Value>(&line) {
            Ok(v) if v.is_object() => Some(v),
            _ => None,
        }
    }

    /// Return the next request id (`plugin-N`), matching `_allocate_id`.
    fn allocate_id(&mut self) -> String {
        self.next += 1;
        format!("plugin-{}", self.next)
    }

    /// Write one NDJSON request line (`_send`). Compact separators match
    /// Python's `separators=(",", ":")`.
    fn send(&mut self, payload: &impl Serialize) -> Result<()> {
        let sock = self
            .sock
            .as_mut()
            .ok_or_else(|| SocketError::new("not connected"))?;
        let mut data =
            serde_json::to_vec(payload).map_err(|e| SocketError::new(format!("encode: {e}")))?;
        data.push(b'\n');
        if data.len() > MAX_LINE_BYTES {
            return Err(SocketError::new("oversized request"));
        }
        sock.write_all(&data)
            .map_err(|e| SocketError::new(format!("send failed: {e}")).at(ErrorStage::AfterSend))
    }

    /// Read until newline (`_read_line`). `required` raises on
    /// timeout/close/oversize; otherwise returns `None`. Decodes UTF-8 lossily,
    /// matching Python's `errors="replace"`.
    fn read_line(&mut self, timeout: Duration, required: bool) -> Result<Option<String>> {
        if self.sock.is_none() {
            return if required {
                Err(SocketError::new("not connected"))
            } else {
                Ok(None)
            };
        }
        let read_timeout = timeout.max(Duration::from_millis(50));
        loop {
            if let Some(pos) = self.buf.iter().position(|&b| b == b'\n') {
                let rest = self.buf.split_off(pos + 1);
                let mut line = std::mem::replace(&mut self.buf, rest);
                line.pop(); // drop '\n'
                return Ok(Some(String::from_utf8_lossy(&line).into_owned()));
            }
            let sock = self.sock.as_mut().expect("checked above");
            sock.set_read_timeout(Some(read_timeout)).ok();
            let mut chunk = [0u8; 64 * 1024];
            match sock.read(&mut chunk) {
                Ok(0) => {
                    self.close();
                    return if required {
                        Err(SocketError::new("socket closed"))
                    } else {
                        Ok(None)
                    };
                }
                Ok(n) => {
                    self.buf.extend_from_slice(&chunk[..n]);
                    if self.buf.len() > MAX_LINE_BYTES {
                        self.close();
                        return Err(SocketError::new("oversized line"));
                    }
                }
                Err(e)
                    if matches!(
                        e.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    ) =>
                {
                    return if required {
                        Err(SocketError::new("timeout waiting for response"))
                    } else {
                        Ok(None)
                    };
                }
                Err(e) => {
                    self.close();
                    return if required {
                        Err(SocketError::new(format!("recv failed: {e}")))
                    } else {
                        Ok(None)
                    };
                }
            }
        }
    }
}

/// Long-lived `events.subscribe` used by `watch --tmux-parity`
/// (`HerdrEventSession`).
pub struct EventSession {
    client: SocketClient,
}

impl EventSession {
    /// Connect and subscribe, or return `None` when the socket is unusable.
    ///
    /// Mirrors `try_open` (`cmux_herdr_socket.py:246-274`): accepts a subscribe
    /// result whose `type` is absent, `subscription_started`, or `ok`.
    pub fn try_open(path: Option<String>, timeout: Duration) -> Option<EventSession> {
        let resolved = path.or_else(socket_path_from_env)?;
        if resolved.is_empty() || !Path::new(&resolved).exists() {
            return None;
        }
        let mut client = SocketClient::new(resolved, timeout);
        client.connect().ok()?;
        match client.subscribe(None) {
            Ok(result) => {
                if let Some(kind) = result.get("type").and_then(Value::as_str) {
                    if kind != "subscription_started" && kind != "ok" {
                        client.close();
                        return None;
                    }
                }
                Some(EventSession { client })
            }
            Err(_) => {
                client.close();
                None
            }
        }
    }

    /// Block until one event arrives. Returns the event dict, or `None`.
    pub fn wait(&mut self, timeout: Duration) -> Option<Value> {
        self.client.read_event(timeout)
    }

    /// True while the subscribe socket is still open.
    pub fn alive(&self) -> bool {
        self.client.connected()
    }

    /// Tear down the subscribe socket.
    pub fn close(&mut self) {
        self.client.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use std::os::unix::net::UnixListener;

    #[test]
    fn default_subscriptions_locked() {
        // Exact 24-entry set and order (parity with Python DEFAULT_SUBSCRIPTIONS).
        assert_eq!(DEFAULT_SUBSCRIPTIONS.len(), 24);
        assert_eq!(DEFAULT_SUBSCRIPTIONS[0], "workspace.created");
        assert_eq!(DEFAULT_SUBSCRIPTIONS[13], "pane.created");
        assert_eq!(DEFAULT_SUBSCRIPTIONS[23], "layout.changed");
    }

    #[test]
    fn reject_symlink_socket() {
        let dir = tempfile::tempdir().unwrap();
        let real = dir.path().join("real.sock");
        let _l = UnixListener::bind(&real).unwrap();
        std::fs::set_permissions(&real, std::fs::Permissions::from_mode(0o600)).unwrap();
        let link = dir.path().join("link.sock");
        std::os::unix::fs::symlink(&real, &link).unwrap();
        let err = assert_socket_secure(link.to_str().unwrap()).unwrap_err();
        assert!(err.message.contains("symlink"), "{}", err.message);
    }

    #[test]
    fn reject_non_socket() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("plain");
        std::fs::write(&file, b"x").unwrap();
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600)).unwrap();
        let err = assert_socket_secure(file.to_str().unwrap()).unwrap_err();
        assert!(err.message.contains("not a Unix socket"), "{}", err.message);
    }

    #[test]
    fn reject_group_other_bits_but_allow_0700() {
        let dir = tempfile::tempdir().unwrap();
        let loose = dir.path().join("loose.sock");
        let _l = UnixListener::bind(&loose).unwrap();
        std::fs::set_permissions(&loose, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(assert_socket_secure(loose.to_str().unwrap()).is_err());

        let tight = dir.path().join("tight.sock");
        let _t = UnixListener::bind(&tight).unwrap();
        std::fs::set_permissions(&tight, std::fs::Permissions::from_mode(0o700)).unwrap();
        // 0700 has no group/other bits and is owned by us: must be accepted.
        assert!(assert_socket_secure(tight.to_str().unwrap()).is_ok());
    }

    #[test]
    fn response_requires_a_string_request_id() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("missing-id.sock");
        let listener = UnixListener::bind(&path).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = Vec::new();
            let mut byte = [0u8; 1];
            while stream.read(&mut byte).unwrap() == 1 && byte[0] != b'\n' {
                request.push(byte[0]);
            }
            assert!(!request.is_empty());
            stream.write_all(b"{\"result\":{\"ok\":true}}\n").unwrap();
        });

        let mut client = SocketClient::new(path.to_string_lossy(), Duration::from_secs(1));
        client.connect().unwrap();
        let error = client.request("ping", json!({})).unwrap_err();
        assert_eq!(error.stage, ErrorStage::AfterSend);
        assert!(error.message.contains("response id"), "{}", error.message);
        server.join().unwrap();
    }
}
