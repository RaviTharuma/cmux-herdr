//! Plugin-manager sidebar terminal UI.
//!
//! Behavioral port of `bridge/cmux_herdr_sidebar.py`. Live rows come only from
//! the documented cmux JSON-lines control socket; the renderer deliberately
//! inherits terminal colors and does not synthesize Herdr agents or teams.

use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::os::unix::net::UnixStream;
use std::time::{Duration, Instant};

use serde_json::{Map, Value};

pub const SOCKET_ENV_KEYS: [&str; 2] = ["CMUX_TUI_SOCKET", "CMUX_MUX_SOCKET"];
pub const REFRESH_SECONDS: f64 = 2.0;
pub const MAX_LINE_BYTES: usize = 512 * 1024;

/// Control-socket transport or protocol failure (`MuxSocketError`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MuxSocketError(pub String);

impl MuxSocketError {
    fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl std::fmt::Display for MuxSocketError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for MuxSocketError {}

/// One live cmux workspace row returned by `workspaces_from_tree`.
#[derive(Debug, Clone, PartialEq)]
pub struct WorkspaceRow {
    pub id: Value,
    pub name: String,
    pub active: bool,
    pub key: Option<String>,
}

impl WorkspaceRow {
    /// Serialize with the same keys and insertion order as the Python dict.
    pub fn to_value(&self) -> Value {
        let mut row = Map::new();
        row.insert("id".into(), self.id.clone());
        row.insert("name".into(), Value::String(self.name.clone()));
        row.insert("active".into(), Value::Bool(self.active));
        if let Some(key) = &self.key {
            row.insert("key".into(), Value::String(key.clone()));
        }
        Value::Object(row)
    }
}

/// Return the first non-blank mux socket path from the documented variables.
pub fn socket_path_from_env(environ: &HashMap<String, String>) -> Option<String> {
    SOCKET_ENV_KEYS.iter().find_map(|key| {
        let value = environ.get(*key).map_or("", String::as_str).trim();
        (!value.is_empty()).then(|| value.to_string())
    })
}

/// Extract `{id, name, active, key?}` rows from a `list-workspaces` result.
pub fn workspaces_from_tree(payload: &Value) -> Vec<WorkspaceRow> {
    let Some(raw) = payload.get("workspaces").and_then(Value::as_array) else {
        return Vec::new();
    };
    raw.iter()
        .filter_map(|item| {
            let item = item.as_object()?;
            let id = item.get("id")?;
            if id.is_null() {
                return None;
            }
            let name = item.get("name")?.as_str()?;
            let key = item
                .get("key")
                .and_then(Value::as_str)
                .filter(|key| !key.is_empty())
                .map(str::to_string);
            Some(WorkspaceRow {
                id: id.clone(),
                name: name.to_string(),
                active: truthy(item.get("active")),
                key,
            })
        })
        .collect()
}

/// Render one complete ANSI frame, byte-for-byte with Python `render_sidebar`.
pub fn render_sidebar(
    rows: &[WorkspaceRow],
    selected: i64,
    cols: i64,
    rows_h: i64,
    connected: bool,
    message: &str,
) -> String {
    let width = (if cols == 0 { 24 } else { cols }).max(16);
    let height = (if rows_h == 0 { 12 } else { rows_h }).max(6);
    let mut lines = Vec::new();
    lines.push(clip("\x1b[1mcmux-herdr\x1b[0m", width));
    lines.push(clip(
        if connected {
            "workspaces"
        } else {
            "waiting for mux socket"
        },
        width,
    ));
    lines.push(clip("", width));

    if !connected {
        let detail = if message.is_empty() {
            "set CMUX_TUI_SOCKET (legacy CMUX_MUX_SOCKET)"
        } else {
            message
        };
        lines.extend(wrap(detail, width).iter().map(|part| clip(part, width)));
        lines.push(clip("", width));
        lines.push(clip("retrying…", width));
    } else if rows.is_empty() {
        lines.push(clip("no workspaces in this session", width));
    } else {
        let selected = selected.clamp(0, rows.len() as i64 - 1) as usize;
        for (index, row) in rows.iter().enumerate() {
            let marker = if index == selected { '>' } else { ' ' };
            let active = if row.active { '*' } else { ' ' };
            let body = clip(&format!("{marker}{active} {}", row.name), width);
            if index == selected {
                lines.push(format!("\x1b[7m{body}\x1b[0m"));
            } else {
                lines.push(body);
            }
        }
    }

    while lines.len() < height.saturating_sub(1) as usize {
        lines.push(clip("", width));
    }
    let footer = if connected {
        format!("{} live", rows.len())
    } else {
        "offline".to_string()
    };
    lines.push(clip(&footer, width));
    lines.truncate(height as usize);
    format!("\x1b[H\x1b[J{}", lines.join("\n"))
}

/// Pad or truncate a visible line to `width` Python code points (`_clip`).
pub(crate) fn clip(text: &str, width: i64) -> String {
    let visible = strip_ansi(text);
    let chars: Vec<char> = visible.chars().collect();
    if chars.len() as i128 > width as i128 {
        let take = (width - 1).max(0) as usize;
        let mut clipped: String = chars.into_iter().take(take).collect();
        clipped.push('…');
        return clipped;
    }
    let padding = (width - chars.len() as i64).max(0) as usize;
    let mut padded = visible;
    padded.extend(std::iter::repeat_n(' ', padding));
    padded
}

/// Remove exactly the SGR sequences emitted by this renderer (`_strip_ansi`).
pub(crate) fn strip_ansi(text: &str) -> String {
    text.replace("\x1b[1m", "")
        .replace("\x1b[0m", "")
        .replace("\x1b[7m", "")
}

/// Wrap on whitespace without hyphenation (`_wrap`).
pub(crate) fn wrap(text: &str, width: i64) -> Vec<String> {
    if width <= 1 {
        return vec![text.to_string()];
    }
    let mut words = text.split_whitespace();
    let Some(first) = words.next() else {
        return vec![String::new()];
    };
    let mut lines = Vec::new();
    let mut current = first.to_string();
    for word in words {
        let trial_len = current.chars().count() + 1 + word.chars().count();
        if trial_len as i64 <= width {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(current);
            current = word.to_string();
        }
    }
    lines.push(current);
    lines
}

/// JSON-lines client for the documented cmux mux control socket.
pub struct MuxClient {
    path: String,
    timeout: Duration,
    next_id: u64,
    socket: UnixStream,
    buffer: Vec<u8>,
}

impl MuxClient {
    /// Open a mux socket with Python's default two-second timeout.
    pub fn new(path: impl Into<String>) -> Result<Self, MuxSocketError> {
        Self::connect(path, Duration::from_secs_f64(REFRESH_SECONDS))
    }

    /// Open `path` as a Unix stream socket.
    pub fn connect(path: impl Into<String>, timeout: Duration) -> Result<Self, MuxSocketError> {
        let path = path.into();
        let socket = UnixStream::connect(&path)
            .map_err(|error| MuxSocketError::new(format!("connect failed: {error}")))?;
        let _ = socket.set_read_timeout(Some(timeout));
        let _ = socket.set_write_timeout(Some(timeout));
        Ok(Self {
            path,
            timeout,
            next_id: 1,
            socket,
            buffer: Vec::new(),
        })
    }

    pub fn path(&self) -> &str {
        &self.path
    }
    /// Close the control socket. Dropping the client has the same effect.
    pub fn close(self) {
        drop(self);
    }


    /// Send one command and return its `data` value.
    pub fn call(
        &mut self,
        cmd: &str,
        fields: impl IntoIterator<Item = (String, Value)>,
    ) -> Result<Value, MuxSocketError> {
        let request_id = self.next_id;
        self.next_id += 1;
        let mut payload = Map::new();
        payload.insert("id".into(), Value::from(request_id));
        payload.insert("cmd".into(), Value::String(cmd.to_string()));
        payload.extend(fields);
        let mut raw = json_dumps_compact(&Value::Object(payload)).into_bytes();
        raw.push(b'\n');
        self.socket
            .write_all(&raw)
            .map_err(|error| MuxSocketError::new(format!("{cmd} failed: {error}")))?;
        let reply = self.read_json(cmd)?;
        let Some(reply) = reply.as_object() else {
            return Err(MuxSocketError::new(format!("{cmd} returned a non-object")));
        };
        if !json_number_equals(reply.get("id"), request_id) {
            return Err(MuxSocketError::new(format!("{cmd} reply id mismatch")));
        }
        if !truthy(reply.get("ok")) {
            let detail = reply
                .get("error")
                .filter(|value| truthy(Some(value)))
                .map(python_str)
                .unwrap_or_else(|| python_str(&Value::Object(reply.clone())));
            return Err(MuxSocketError::new(format!("{cmd} error: {detail}")));
        }
        Ok(reply.get("data").cloned().unwrap_or(Value::Null))
    }

    /// Identify, then fetch the live workspace list.
    pub fn list_workspaces(&mut self) -> Result<Vec<WorkspaceRow>, MuxSocketError> {
        self.call("identify", std::iter::empty())?;
        let tree = self.call("list-workspaces", std::iter::empty())?;
        Ok(workspaces_from_tree(&tree))
    }

    /// Select a live workspace by zero-based index.
    pub fn select_workspace(&mut self, index: i64) -> Result<(), MuxSocketError> {
        self.call(
            "select-workspace",
            [("index".to_string(), Value::from(index))],
        )?;
        Ok(())
    }

    fn read_json(&mut self, cmd: &str) -> Result<Value, MuxSocketError> {
        let deadline = Instant::now() + self.timeout;
        loop {
            if let Some(newline) = self.buffer.iter().position(|byte| *byte == b'\n') {
                let line: Vec<u8> = self.buffer.drain(..=newline).collect();
                let line = &line[..line.len() - 1];
                if line.is_empty() {
                    continue;
                }
                if line.len() > MAX_LINE_BYTES {
                    return Err(MuxSocketError::new("reply exceeds size limit"));
                }
                let text = std::str::from_utf8(line)
                    .map_err(|error| MuxSocketError::new(format!("invalid UTF-8: {error}")))?;
                return serde_json::from_str(text)
                    .map_err(|error| MuxSocketError::new(format!("invalid JSON: {error}")));
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(MuxSocketError::new("timed out waiting for reply"));
            }
            let _ = self.socket.set_read_timeout(Some(remaining));
            let mut chunk = [0_u8; 4096];
            match self.socket.read(&mut chunk) {
                Ok(0) => return Err(MuxSocketError::new("socket closed")),
                Ok(size) => self.buffer.extend_from_slice(&chunk[..size]),
                Err(error)
                    if matches!(
                        error.kind(),
                        io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
                    ) =>
                {
                    return Err(MuxSocketError::new(format!("{cmd} failed: timed out")));
                }
                Err(error) => {
                    return Err(MuxSocketError::new(format!("{cmd} failed: {error}")));
                }
            }
            if self.buffer.len() > MAX_LINE_BYTES {
                return Err(MuxSocketError::new("reply exceeds size limit"));
            }
        }
    }
}

fn json_dumps_compact(value: &Value) -> String {
    let encoded = serde_json::to_string(value).expect("JSON values always serialize successfully");
    let mut out = String::with_capacity(encoded.len());
    let mut in_string = false;
    let mut escaped = false;
    for ch in encoded.chars() {
        if !in_string {
            out.push(ch);
            if ch == '"' {
                in_string = true;
            }
            continue;
        }
        if escaped {
            out.push(ch);
            escaped = false;
            continue;
        }
        match ch {
            '\\' => {
                out.push(ch);
                escaped = true;
            }
            '"' => {
                out.push(ch);
                in_string = false;
            }
            ch if ch.is_ascii() && !ch.is_ascii_control() => out.push(ch),
            ch => {
                let code = ch as u32;
                if code <= 0xffff {
                    use std::fmt::Write as _;
                    write!(out, "\\u{code:04x}").expect("writing to String cannot fail");
                } else {
                    let code = code - 0x1_0000;
                    let high = 0xd800 + (code >> 10);
                    let low = 0xdc00 + (code & 0x3ff);
                    use std::fmt::Write as _;
                    write!(out, "\\u{high:04x}\\u{low:04x}")
                        .expect("writing to String cannot fail");
                }
            }
        }
    }
    out
}

fn json_number_equals(value: Option<&Value>, expected: u64) -> bool {
    match value {
        Some(Value::Bool(value)) => u64::from(*value) == expected,
        Some(Value::Number(value)) => value
            .as_u64()
            .map(|value| value == expected)
            .or_else(|| value.as_f64().map(|value| value == expected as f64))
            .unwrap_or(false),
        _ => false,
    }
}

fn python_str(value: &Value) -> String {
    match value {
        Value::Null => "None".into(),
        Value::Bool(true) => "True".into(),
        Value::Bool(false) => "False".into(),
        Value::Number(number) => number.to_string(),
        Value::String(text) => text.clone(),
        Value::Array(items) => format!(
            "[{}]",
            items.iter().map(python_repr).collect::<Vec<_>>().join(", ")
        ),
        Value::Object(items) => format!(
            "{{{}}}",
            items
                .iter()
                .map(|(key, value)| format!("{}: {}", python_repr_string(key), python_repr(value)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

fn python_repr(value: &Value) -> String {
    match value {
        Value::String(text) => python_repr_string(text),
        _ => python_str(value),
    }
}

fn python_repr_string(text: &str) -> String {
    let quote = if text.contains('\'') && !text.contains('"') {
        '"'
    } else {
        '\''
    };
    let mut out = String::with_capacity(text.len() + 2);
    out.push(quote);
    for ch in text.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch == quote => {
                out.push('\\');
                out.push(ch);
            }
            ch => out.push(ch),
        }
    }
    out.push(quote);
    out
}

fn truthy(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null) | Some(Value::Bool(false)) => false,
        Some(Value::Bool(true)) => true,
        Some(Value::Number(number)) => number.as_f64().map(|n| n != 0.0).unwrap_or(true),
        Some(Value::String(text)) => !text.is_empty(),
        Some(Value::Array(items)) => !items.is_empty(),
        Some(Value::Object(items)) => !items.is_empty(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InputAction {
    Exit,
    Ignore,
    Move(i64),
    Select,
}

fn input_action(data: &[u8]) -> InputAction {
    match data {
        b"\x03" => InputAction::Exit,
        b"\x1b" | b"\x1b\x1b" => InputAction::Ignore,
        b"\x1b[A" | b"k" => InputAction::Move(-1),
        b"\x1b[B" | b"j" => InputAction::Move(1),
        b"\r" | b"\n" => InputAction::Select,
        _ => InputAction::Ignore,
    }
}

fn move_selection(selected: usize, delta: i64, row_count: usize) -> usize {
    if row_count == 0 {
        return selected;
    }
    (selected as i64 + delta).rem_euclid(row_count as i64) as usize
}

fn refresh(environ: &HashMap<String, String>, state: &mut SidebarState) {
    state.last_refresh = Instant::now();
    let Some(path) = socket_path_from_env(environ) else {
        state.connected = false;
        state.rows.clear();
        state.message = "CMUX_TUI_SOCKET is unset".into();
        return;
    };
    match MuxClient::connect(path, Duration::from_secs_f64(REFRESH_SECONDS))
        .and_then(|mut client| client.list_workspaces())
    {
        Ok(rows) => {
            state.rows = rows;
            state.connected = true;
            state.message.clear();
            if !state.rows.is_empty() {
                state.selected = state.selected.min(state.rows.len() - 1);
            }
        }
        Err(error) => {
            state.connected = false;
            state.rows.clear();
            state.message = error.to_string();
        }
    }
}

struct SidebarState {
    selected: usize,
    rows: Vec<WorkspaceRow>,
    connected: bool,
    message: String,
    last_refresh: Instant,
}

impl SidebarState {
    fn new() -> Self {
        Self {
            selected: 0,
            rows: Vec::new(),
            connected: false,
            message: String::new(),
            last_refresh: Instant::now(),
        }
    }
}

fn terminal_size() -> (i64, i64) {
    match rustix::termios::tcgetwinsize(io::stdout()) {
        Ok(size) => (i64::from(size.ws_col), i64::from(size.ws_row)),
        Err(_) => (28, 20),
    }
}

fn draw(stdout: &mut impl Write, state: &SidebarState) -> io::Result<()> {
    let (cols, rows_h) = terminal_size();
    stdout.write_all(
        render_sidebar(
            &state.rows,
            state.selected as i64,
            cols,
            rows_h,
            state.connected,
            &state.message,
        )
        .as_bytes(),
    )?;
    stdout.flush()
}

/// Refresh and draw one frame to an injected writer, then close the client.
pub fn run_sidebar_once_to(
    environ: &HashMap<String, String>,
    stdout: &mut impl Write,
) -> i32 {
    let mut state = SidebarState::new();
    refresh(environ, &mut state);
    let _ = draw(stdout, &state);
    0
}

/// Run the sidebar event loop. `once` draws one frame and returns.
pub fn run_sidebar(environ: Option<&HashMap<String, String>>, once: bool) -> i32 {
    let inherited: HashMap<String, String> = std::env::vars().collect();
    let environ = environ.unwrap_or(&inherited);
    let mut stdout = io::stdout().lock();
    if once {
        return run_sidebar_once_to(environ, &mut stdout);
    }
    let mut state = SidebarState::new();
    refresh(environ, &mut state);
    if draw(&mut stdout, &state).is_err() {
        return 0;
    }

    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut stdin = io::stdin().lock();
        loop {
            let mut input = [0_u8; 32];
            match stdin.read(&mut input) {
                Ok(0) | Err(_) => break,
                Ok(size) if sender.send(input[..size].to_vec()).is_err() => break,
                Ok(_) => {}
            }
        }
    });

    loop {
        let refresh_after = Duration::from_secs_f64(REFRESH_SECONDS);
        if state.last_refresh.elapsed() >= refresh_after {
            refresh(environ, &mut state);
            if draw(&mut stdout, &state).is_err() {
                return 0;
            }
        }
        let elapsed = state.last_refresh.elapsed();
        let timeout = refresh_after
            .saturating_sub(elapsed)
            .max(Duration::from_millis(50));
        match receiver.recv_timeout(timeout) {
            Ok(data) => match input_action(&data) {
                InputAction::Exit => return 0,
                InputAction::Ignore => {}
                InputAction::Move(delta) if !state.rows.is_empty() => {
                    state.selected = move_selection(state.selected, delta, state.rows.len());
                    if draw(&mut stdout, &state).is_err() {
                        return 0;
                    }
                }
                InputAction::Move(_) => {}
                InputAction::Select if !state.rows.is_empty() => {
                    if let Some(path) = socket_path_from_env(environ) {
                        if let Err(error) = MuxClient::connect(
                            path,
                            Duration::from_secs_f64(REFRESH_SECONDS),
                        )
                        .and_then(|mut client| client.select_workspace(state.selected as i64))
                        {
                            state.message = error.to_string();
                            state.connected = false;
                        }
                        refresh(environ, &mut state);
                        if draw(&mut stdout, &state).is_err() {
                            return 0;
                        }
                    }
                }
                InputAction::Select => {}
            },
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                refresh(environ, &mut state);
                if draw(&mut stdout, &state).is_err() {
                    return 0;
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return 0,
        }
    }
}

/// Sidebar executable argument handling (`main`).
pub fn main(args: &[String]) -> i32 {
    if matches!(args, [arg] if arg == "--help" || arg == "-h") {
        print!(
            "cmux-herdr-sidebar — cmux sidebar plugin TUI\n\
             Hosted by: cmux sidebar plugin use cmux-herdr\n\
             Socket: CMUX_TUI_SOCKET (legacy CMUX_MUX_SOCKET)\n\
             Keys: j/k or arrows move, enter selects, Ctrl-C exits.\n\
             Esc is ignored (cmux owns the sidebar escape chord).\n"
        );
        return 0;
    }
    run_sidebar(None, matches!(args, [arg] if arg == "--once"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn rows() -> Vec<WorkspaceRow> {
        workspaces_from_tree(&json!({
            "workspaces": [
                {"id": 1, "name": "one", "active": false},
                {"id": 2, "name": "two", "active": true}
            ]
        }))
    }

    #[test]
    fn selected_row_is_the_only_reverse_video_row() {
        let frame = render_sidebar(&rows(), 1, 20, 8, true, "");
        assert_eq!(frame.matches("\x1b[7m").count(), 1);
        assert!(frame.contains("\x1b[7m>* two"));
        assert!(!frame.contains("\x1b[32m"));
    }

    #[test]
    fn movement_wraps_and_empty_input_preserves_selection() {
        assert_eq!(move_selection(0, -1, 2), 1);
        assert_eq!(move_selection(1, 1, 2), 0);
        assert_eq!(move_selection(7, 1, 0), 7);
    }

    #[test]
    fn focus_keys_and_escape_have_exact_actions() {
        assert_eq!(input_action(b"k"), InputAction::Move(-1));
        assert_eq!(input_action(b"\x1b[B"), InputAction::Move(1));
        assert_eq!(input_action(b"\n"), InputAction::Select);
        assert_eq!(input_action(b"\x1b"), InputAction::Ignore);
        assert_eq!(input_action(b"\x03"), InputAction::Exit);
    }

    #[test]
    fn disconnected_renderer_never_invents_rows_or_color() {
        let frame = render_sidebar(&[], 0, 32, 10, false, "socket failed");
        assert!(frame.contains("waiting for mux socket"));
        assert!(frame.contains("retrying…"));
        assert!(!frame.to_lowercase().contains("team"));
        assert!(!frame.contains("\x1b[32m"));
    }

    #[test]
    fn mux_client_identifies_then_lists_live_workspaces() {
        use std::os::unix::net::UnixListener;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mux.sock");
        let listener = UnixListener::bind(&path).unwrap();
        let server = std::thread::spawn(move || {
            let (mut connection, _) = listener.accept().unwrap();
            let mut reader = std::io::BufReader::new(connection.try_clone().unwrap());
            for (expected, reply) in [
                (
                    "{\"id\":1,\"cmd\":\"identify\"}\n",
                    json!({"id": 1, "ok": true, "data": {"protocol": 12}}),
                ),
                (
                    "{\"id\":2,\"cmd\":\"list-workspaces\"}\n",
                    json!({
                        "id": 2,
                        "ok": true,
                        "data": {"workspaces": [{"id": 4, "name": "lab", "active": true}]}
                    }),
                ),
                (
                    "{\"id\":3,\"cmd\":\"select-workspace\",\"index\":1}\n",
                    json!({"id": 3, "ok": true, "data": null}),
                ),
            ] {
                let mut request = String::new();
                std::io::BufRead::read_line(&mut reader, &mut request).unwrap();
                assert_eq!(request, expected);
                writeln!(connection, "{}", serde_json::to_string(&reply).unwrap()).unwrap();
            }
        });

        let mut client = MuxClient::connect(path.to_string_lossy(), Duration::from_secs(2)).unwrap();
        let live = client.list_workspaces().unwrap();
        assert_eq!(live.len(), 1);
        assert_eq!(live[0].name, "lab");
        assert!(live[0].active);
        client.select_workspace(1).unwrap();
        client.close();
        server.join().unwrap();
    }

    #[test]
    fn once_without_socket_draws_only_offline_state() {
        let mut output = Vec::new();
        assert_eq!(run_sidebar_once_to(&HashMap::new(), &mut output), 0);
        let frame = String::from_utf8(output).unwrap();
        assert!(frame.contains("waiting for mux socket"));
        assert!(!frame.contains("lab"));
    }

    #[test]
    fn once_draws_socket_rows_and_closes_client() {
        use std::os::unix::net::UnixListener;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("once.sock");
        let listener = UnixListener::bind(&path).unwrap();
        let server = std::thread::spawn(move || {
            let (mut connection, _) = listener.accept().unwrap();
            let mut reader = std::io::BufReader::new(connection.try_clone().unwrap());
            for reply in [
                json!({"id": 1, "ok": true, "data": {"protocol": 12}}),
                json!({
                    "id": 2,
                    "ok": true,
                    "data": {"workspaces": [{"id": 4, "name": "lab-west", "active": true}]}
                }),
            ] {
                let mut request = String::new();
                std::io::BufRead::read_line(&mut reader, &mut request).unwrap();
                writeln!(connection, "{}", serde_json::to_string(&reply).unwrap()).unwrap();
            }
            let mut eof = [0_u8; 1];
            assert_eq!(reader.read(&mut eof).unwrap(), 0);
        });
        let env = HashMap::from([(
            "CMUX_TUI_SOCKET".to_string(),
            path.to_string_lossy().into_owned(),
        )]);
        let mut output = Vec::new();
        assert_eq!(run_sidebar_once_to(&env, &mut output), 0);
        assert!(String::from_utf8(output).unwrap().contains("lab-west"));
        server.join().unwrap();
    }

    #[test]
    fn compact_json_uses_python_ascii_escapes() {
        assert_eq!(
            json_dumps_compact(&json!({"cmd": "café 🧪", "quote": "\\\""})),
            "{\"cmd\":\"caf\\u00e9 \\ud83e\\uddea\",\"quote\":\"\\\\\\\"\"}"
        );
        assert_eq!(json_dumps_compact(&json!("\u{7f}")), "\"\\u007f\"");
    }
}
