//! Cmux user mutations mapped onto published Herdr control methods.
//!
//! Behavioral port of `bridge/cmux_herdr_control.py`: key encoding, bounded
//! input and seed queues, optimistic focus rollback, navigation, and close
//! safety. This module plans mutations; it never calls a provider directly.

use std::collections::{HashMap, HashSet, VecDeque};

use serde_json::{json, Value};

use crate::layout::LayoutNode;

pub const DEFAULT_INPUT_BUDGET: i64 = 256 * 1024;
pub const DEFAULT_SEED_BUDGET: i64 = 256 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderInput {
    pub pane_id: String,
    pub kind: String,
    pub text: Option<String>,
    pub key: Option<String>,
    pub csi: Option<Vec<u8>>,
}

impl ProviderInput {
    pub fn byte_count(&self) -> usize {
        if self.kind == "key" {
            self.csi.as_ref().map(Vec::len).filter(|size| *size > 0).unwrap_or(1)
        } else {
            self.text.as_ref().map(|text| text.len()).filter(|size| *size > 0).unwrap_or(1)
        }
    }
}

fn named_csi(base: &str) -> Option<&'static [u8]> {
    Some(match base {
        "Up" => b"\x1b[A",
        "Down" => b"\x1b[B",
        "Right" => b"\x1b[C",
        "Left" => b"\x1b[D",
        "Home" => b"\x1b[H",
        "End" => b"\x1b[F",
        "PPage" => b"\x1b[5~",
        "NPage" => b"\x1b[6~",
        "DC" => b"\x1b[3~",
        "IC" => b"\x1b[2~",
        "F1" => b"\x1bOP",
        "F2" => b"\x1bOQ",
        "F3" => b"\x1bOR",
        "F4" => b"\x1bOS",
        "F5" => b"\x1b[15~",
        "F6" => b"\x1b[17~",
        "F7" => b"\x1b[18~",
        "F8" => b"\x1b[19~",
        "F9" => b"\x1b[20~",
        "F10" => b"\x1b[21~",
        "F11" => b"\x1b[23~",
        "F12" => b"\x1b[24~",
        _ => return None,
    })
}

fn tmux_to_herdr(base: &str) -> Option<&'static str> {
    Some(match base {
        "Up" => "up",
        "Down" => "down",
        "Left" => "left",
        "Right" => "right",
        "F1" => "f1",
        "F2" => "f2",
        "F3" => "f3",
        "F4" => "f4",
        "F5" => "f5",
        "F6" => "f6",
        "F7" => "f7",
        "F8" => "f8",
        "F9" => "f9",
        "F10" => "f10",
        "F11" => "f11",
        "F12" => "f12",
        _ => return None,
    })
}

fn herdr_to_tmux(base: &str) -> Option<String> {
    match base {
        "up" => Some("Up".into()),
        "down" => Some("Down".into()),
        "left" => Some("Left".into()),
        "right" => Some("Right".into()),
        _ if base.len() >= 2 && base.starts_with('f') => {
            let number = base[1..].parse::<u8>().ok()?;
            (1..=12).contains(&number).then(|| format!("F{number}"))
        }
        _ => None,
    }
}

fn herdr_special(name: &str) -> bool {
    matches!(
        name,
        "up" | "down" | "left" | "right" | "enter" | "tab" | "esc" | "escape"
            | "backspace" | "minus" | "plus" | "backtick"
    ) || name
        .strip_prefix('f')
        .and_then(|number| number.parse::<u8>().ok())
        .is_some_and(|number| (1..=12).contains(&number))
}

fn csi_with_modifiers(csi: &[u8], modifiers: &HashSet<&str>) -> Vec<u8> {
    let mut code = 1;
    if modifiers.contains("S") {
        code += 1;
    }
    if modifiers.contains("M") {
        code += 2;
    }
    if modifiers.contains("C") {
        code += 4;
    }
    if code == 1 {
        return csi.to_vec();
    }
    if csi.ends_with(b"~") {
        let mut out = b"\x1b[".to_vec();
        out.extend_from_slice(&csi[2..csi.len() - 1]);
        out.extend_from_slice(format!(";{code}~").as_bytes());
        return out;
    }
    if csi.starts_with(b"\x1bO") && csi.len() == 3 {
        let number = match csi[2] {
            b'P' => Some("11"),
            b'Q' => Some("12"),
            b'R' => Some("13"),
            b'S' => Some("14"),
            _ => None,
        };
        if let Some(number) = number {
            return format!("\x1b[{number};{code}~").into_bytes();
        }
    }
    if csi.starts_with(b"\x1b[") && csi.len() == 3 {
        let mut out = format!("\x1b[1;{code}").into_bytes();
        out.push(csi[2]);
        return out;
    }
    csi.to_vec()
}

fn csi_for_herdr_combo(combo: &str) -> Option<Vec<u8>> {
    let lowered = combo.to_ascii_lowercase();
    let parts: Vec<&str> = lowered.split('+').filter(|part| !part.is_empty()).collect();
    let base = *parts.last()?;
    let mut modifiers = HashSet::new();
    for part in &parts[..parts.len() - 1] {
        match *part {
            "ctrl" | "control" | "c" => { modifiers.insert("C"); }
            "alt" | "m" => { modifiers.insert("M"); }
            "shift" | "s" => { modifiers.insert("S"); }
            _ => {}
        }
    }
    let tmux = herdr_to_tmux(base);
    let base_csi = tmux
        .as_deref()
        .and_then(named_csi)
        .or_else(|| match base {
            "enter" => Some(b"\r" as &[u8]),
            "tab" => Some(b"\t" as &[u8]),
            "esc" | "escape" => Some(b"\x1b" as &[u8]),
            "backspace" => Some(b"\x7f" as &[u8]),
            _ => None,
        })?;
    if !modifiers.is_empty() && tmux.is_some() {
        Some(csi_with_modifiers(base_csi, &modifiers))
    } else {
        Some(base_csi.to_vec())
    }
}

pub fn encode_named_key(pane_id: &str, raw_name: &str) -> Option<ProviderInput> {
    if pane_id.is_empty() || raw_name.is_empty() {
        return None;
    }
    let name = raw_name.trim();
    let lowered = name.to_ascii_lowercase();
    if name.contains('+') || herdr_special(&lowered) {
        return Some(ProviderInput {
            pane_id: pane_id.to_string(),
            kind: "key".into(),
            text: None,
            key: Some(lowered.clone()),
            csi: csi_for_herdr_combo(&lowered),
        });
    }
    let parts: Vec<&str> = name.split('-').filter(|part| !part.is_empty()).collect();
    let base = *parts.last()?;
    let modifiers: HashSet<&str> = parts[..parts.len() - 1]
        .iter()
        .copied()
        .filter(|part| matches!(*part, "C" | "M" | "S"))
        .collect();
    let csi = named_csi(base);
    let herdr_base = tmux_to_herdr(base);
    if csi.is_none() && herdr_base.is_none() {
        return None;
    }
    let csi = csi.map(|bytes| {
        if modifiers.is_empty() {
            bytes.to_vec()
        } else {
            csi_with_modifiers(bytes, &modifiers)
        }
    });
    let key = herdr_base.map(|base| {
        let mut parts = Vec::new();
        if modifiers.contains("C") { parts.push("ctrl"); }
        if modifiers.contains("M") { parts.push("alt"); }
        if modifiers.contains("S") { parts.push("shift"); }
        parts.push(base);
        parts.join("+")
    });
    Some(ProviderInput {
        pane_id: pane_id.to_string(),
        kind: "key".into(),
        text: None,
        key,
        csi,
    })
}

pub fn encode_manual_input(
    pane_id: &str,
    text: Option<&str>,
    key: Option<&str>,
) -> Option<ProviderInput> {
    if let Some(key) = key.filter(|key| !key.is_empty()) {
        return encode_named_key(pane_id, key);
    }
    text.filter(|text| !text.is_empty()).map(|text| ProviderInput {
        pane_id: pane_id.to_string(),
        kind: "text".into(),
        text: Some(text.to_string()),
        key: None,
        csi: None,
    })
}

#[derive(Debug, Clone)]
pub struct InputForwarder {
    pub maximum_pending_bytes: i64,
    pub pending_bytes: i64,
    pub epoch: i64,
    pub active: bool,
    pub queue: VecDeque<ProviderInput>,
    pub overflowed: bool,
}

impl Default for InputForwarder {
    fn default() -> Self {
        Self {
            maximum_pending_bytes: DEFAULT_INPUT_BUDGET,
            pending_bytes: 0,
            epoch: 0,
            active: true,
            queue: VecDeque::new(),
            overflowed: false,
        }
    }
}

impl InputForwarder {
    pub fn with_budget(maximum_pending_bytes: i64) -> Self {
        Self { maximum_pending_bytes, ..Self::default() }
    }

    pub fn enqueue(&mut self, item: ProviderInput) -> &'static str {
        if !self.active {
            return "inactive";
        }
        let size = item.byte_count() as i64;
        if self.pending_bytes + size > self.maximum_pending_bytes {
            self.overflowed = true;
            return "overflow";
        }
        self.pending_bytes += size;
        self.queue.push_back(item);
        "enqueued"
    }

    pub fn drain(&mut self) -> Vec<ProviderInput> {
        let items = self.queue.drain(..).collect();
        self.pending_bytes = 0;
        items
    }

    pub fn deactivate(&mut self) {
        self.active = false;
        self.epoch += 1;
        self.queue.clear();
        self.pending_bytes = 0;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingFocus {
    pub request_id: String,
    pub pane_id: String,
    pub previous_pane_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FocusCommand {
    pub pane_id: Option<String>,
    pub send_to_provider: bool,
    pub rolled_back: bool,
    pub request_id: Option<String>,
}

impl FocusCommand {
    fn quiet(pane_id: Option<String>) -> Self {
        Self { pane_id, send_to_provider: false, rolled_back: false, request_id: None }
    }
}

#[derive(Debug, Clone, Default)]
pub struct FocusController {
    pub live_pane_ids: Vec<String>,
    pub active_pane_id: Option<String>,
    pub pending: Option<PendingFocus>,
    next_id: i64,
}

impl FocusController {
    pub fn user_select(&mut self, pane_id: &str) -> FocusCommand {
        if !self.live_pane_ids.iter().any(|item| item == pane_id) {
            return FocusCommand::quiet(None);
        }
        if let Some(pending) = self.pending.as_ref().filter(|pending| pending.pane_id == pane_id) {
            return FocusCommand {
                pane_id: Some(pane_id.into()),
                send_to_provider: false,
                rolled_back: false,
                request_id: Some(pending.request_id.clone()),
            };
        }
        self.next_id += 1;
        let request_id = format!("f{}", self.next_id);
        self.pending = Some(PendingFocus {
            request_id: request_id.clone(),
            pane_id: pane_id.into(),
            previous_pane_id: self.active_pane_id.clone(),
        });
        self.active_pane_id = Some(pane_id.into());
        FocusCommand {
            pane_id: Some(pane_id.into()),
            send_to_provider: true,
            rolled_back: false,
            request_id: Some(request_id),
        }
    }

    pub fn command_rejected(&mut self, request_id: &str) -> FocusCommand {
        let Some(pending) = self.pending.as_ref().filter(|pending| pending.request_id == request_id).cloned() else {
            return FocusCommand::quiet(self.active_pane_id.clone());
        };
        self.pending = None;
        self.active_pane_id = pending.previous_pane_id.clone();
        FocusCommand {
            pane_id: pending.previous_pane_id,
            send_to_provider: false,
            rolled_back: true,
            request_id: Some(request_id.into()),
        }
    }

    pub fn provider_confirms(&mut self, pane_id: &str) -> FocusCommand {
        if self.pending.as_ref().is_some_and(|pending| pending.pane_id == pane_id) {
            self.pending = None;
        }
        self.active_pane_id = Some(pane_id.into());
        FocusCommand::quiet(Some(pane_id.into()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserSplit {
    pub from_pane_id: String,
    pub orientation: String,
    pub insert_first: bool,
    pub focus_created: bool,
}

pub fn request_split(
    from_pane_id: &str,
    vertical: bool,
    insert_first: bool,
    focus_created: bool,
) -> Option<UserSplit> {
    (!from_pane_id.is_empty()).then(|| UserSplit {
        from_pane_id: from_pane_id.into(),
        orientation: if vertical { "vertical" } else { "horizontal" }.into(),
        insert_first,
        focus_created,
    })
}

fn path_to_pane(node: &LayoutNode, pane_id: &str, path: &mut Vec<usize>) -> bool {
    if node.kind == "pane" {
        return node.pane_id.as_deref() == Some(pane_id);
    }
    for (index, child) in node.children.iter().enumerate() {
        path.push(index);
        if path_to_pane(child, pane_id, path) {
            return true;
        }
        path.pop();
    }
    false
}

fn edge_pane(node: &LayoutNode, approaching: &str) -> Option<String> {
    if node.kind == "pane" {
        return node.pane_id.clone();
    }
    let child = if (approaching == "left" && node.kind == "horizontal")
        || (approaching == "up" && node.kind == "vertical")
    {
        node.children.last()
    } else {
        node.children.first()
    }?;
    edge_pane(child, approaching)
}

pub fn adjacent_pane(node: &LayoutNode, pane_id: &str, direction: &str) -> Option<String> {
    let mut indexes = Vec::new();
    if !path_to_pane(node, pane_id, &mut indexes) || indexes.is_empty() {
        return None;
    }
    let want_horizontal = matches!(direction, "left" | "right");
    for depth in (0..indexes.len()).rev() {
        let mut parent = node;
        for index in &indexes[..depth] {
            parent = parent.children.get(*index)?;
        }
        if parent.kind == "pane" || (parent.kind == "horizontal") != want_horizontal {
            continue;
        }
        let child_index = indexes[depth];
        let neighbor = if matches!(direction, "left" | "up") && child_index > 0 {
            parent.children.get(child_index - 1)
        } else if matches!(direction, "right" | "down") && child_index + 1 < parent.children.len() {
            parent.children.get(child_index + 1)
        } else {
            None
        };
        if let Some(neighbor) = neighbor {
            return edge_pane(neighbor, direction);
        }
    }
    None
}

#[derive(Debug, Clone)]
pub struct PaneSeedQueue {
    pub maximum_bytes: i64,
    pub pending: HashMap<String, Vec<u8>>,
    pub kinds: HashMap<String, String>,
    pub targets: HashMap<String, (i64, i64)>,
    pub deferred_full: HashSet<String>,
}

impl Default for PaneSeedQueue {
    fn default() -> Self {
        Self {
            maximum_bytes: DEFAULT_SEED_BUDGET,
            pending: HashMap::new(),
            kinds: HashMap::new(),
            targets: HashMap::new(),
            deferred_full: HashSet::new(),
        }
    }
}

impl PaneSeedQueue {
    pub fn with_budget(maximum_bytes: i64) -> Self {
        Self { maximum_bytes, ..Self::default() }
    }

    pub fn queue(
        &mut self,
        pane_id: &str,
        data: &[u8],
        kind: &str,
        target_grid: Option<(i64, i64)>,
    ) -> &'static str {
        if data.is_empty() {
            return "empty";
        }
        if data.len() as i64 > self.maximum_bytes {
            self.deferred_full.insert(pane_id.into());
            self.pending.remove(pane_id);
            return "overflow";
        }
        self.pending.insert(pane_id.into(), data.to_vec());
        self.kinds.insert(pane_id.into(), kind.into());
        if let Some(target) = target_grid {
            self.targets.insert(pane_id.into(), target);
        }
        "queued"
    }

    pub fn note_ready(&mut self, pane_id: &str, cols: i64, rows: i64) -> Option<Vec<u8>> {
        if self.targets.get(pane_id).is_some_and(|target| *target != (cols, rows)) {
            return None;
        }
        let data = self.pending.remove(pane_id);
        self.kinds.remove(pane_id);
        self.targets.remove(pane_id);
        if data.is_some() {
            self.deferred_full.remove(pane_id);
        }
        data
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabActivity {
    pub has_active_command: bool,
    pub active_command_name: Option<String>,
    pub needs_close_confirmation: bool,
}

fn busy_status(status: &str) -> bool {
    matches!(status.to_ascii_lowercase().as_str(), "working" | "blocked" | "running" | "command")
}

pub fn tab_activity(
    statuses: &[(String, String)],
    agents: Option<&[(String, String)]>,
) -> TabActivity {
    let busy: Vec<&str> = statuses
        .iter()
        .filter(|(_, status)| busy_status(status))
        .map(|(pane_id, _)| pane_id.as_str())
        .collect();
    let command = agents.and_then(|names| {
        busy.iter().find_map(|pane_id| {
            names
                .iter()
                .find(|(id, _)| id == pane_id)
                .map(|(_, name)| name)
                .filter(|name| !name.is_empty())
                .cloned()
        })
    });
    TabActivity {
        has_active_command: !busy.is_empty(),
        active_command_name: command,
        needs_close_confirmation: !busy.is_empty(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloseIntent {
    pub action: String,
    pub pane_id: Option<String>,
}

pub fn close_intent(source: &str, pane_id: Option<&str>, agent_status: Option<&str>) -> CloseIntent {
    if matches!(source, "host_tab" | "host_panel" | "detach") {
        return CloseIntent { action: "detach".into(), pane_id: None };
    }
    let Some(pane_id) = pane_id.filter(|pane_id| !pane_id.is_empty()) else {
        return CloseIntent { action: "noop".into(), pane_id: None };
    };
    if source != "user_pane" {
        return CloseIntent { action: "noop".into(), pane_id: None };
    }
    CloseIntent {
        action: if agent_status.is_some_and(busy_status) {
            "confirm_then_close_pane"
        } else {
            "close_pane"
        }
        .into(),
        pane_id: Some(pane_id.into()),
    }
}

fn strip_title_escapes(name: &str) -> String {
    let chars: Vec<char> = name.chars().collect();
    let mut out = String::new();
    let mut index = 0;
    while index < chars.len() {
        if chars[index] != '\u{1b}' {
            out.push(chars[index]);
            index += 1;
            continue;
        }
        if index + 1 >= chars.len() {
            index += 1;
            continue;
        }
        if chars[index + 1] != '[' {
            index += 2;
            continue;
        }
        let mut cursor = index + 2;
        while cursor < chars.len() && matches!(chars[cursor], '0'..='9' | ';' | '?') {
            cursor += 1;
        }
        while cursor < chars.len() && (' '..='/').contains(&chars[cursor]) {
            cursor += 1;
        }
        if cursor < chars.len() && ('@'..='~').contains(&chars[cursor]) {
            index = cursor + 1;
        } else {
            out.push('\u{1b}');
            index += 1;
        }
    }
    out
}

fn python_printable(ch: char) -> bool {
    if ch == ' ' {
        return true;
    }
    if ch.is_control() || ch.is_whitespace() {
        return false;
    }
    !matches!(ch as u32,
        0x00AD | 0x0600..=0x0605 | 0x061C | 0x06DD | 0x070F | 0x0890..=0x0891 |
        0x08E2 | 0x180E | 0x200B..=0x200F | 0x202A..=0x202E | 0x2060..=0x2064 |
        0x2066..=0x206F | 0xFEFF | 0xFFF9..=0xFFFB | 0x110BD | 0x110CD |
        0x13430..=0x1343F | 0x1BCA0..=0x1BCA3 | 0x1D173..=0x1D17A |
        0xE0001 | 0xE0020..=0xE007F
    )
}

pub fn apply_session_title(
    name: &str,
    current: Option<&str>,
    propagate_to_provider: bool,
) -> Option<String> {
    if propagate_to_provider {
        return None;
    }
    let stripped = strip_title_escapes(name);
    let cleaned: String = stripped.chars().filter(|ch| python_printable(*ch)).collect();
    let cleaned = cleaned.trim();
    if cleaned.is_empty() || current == Some(cleaned) {
        return None;
    }
    Some(cleaned.chars().take(200).collect())
}

pub fn pane_surface_entries(bindings: &[(String, String, String, bool)]) -> Vec<Value> {
    let mut rows: Vec<Value> = bindings
        .iter()
        .map(|(tab_id, pane_id, surface_id, on_screen)| {
            json!({
                "tab_id": tab_id,
                "pane_id": pane_id,
                "surface_id": surface_id,
                "on_screen": on_screen,
            })
        })
        .collect();
    rows.sort_by(|left, right| {
        left["tab_id"]
            .as_str()
            .cmp(&right["tab_id"].as_str())
            .then_with(|| left["pane_id"].as_str().cmp(&right["pane_id"].as_str()))
    });
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focus_rejection_restores_previous_pane() {
        let mut focus = FocusController {
            live_pane_ids: vec!["p1".into(), "p2".into()],
            active_pane_id: Some("p1".into()),
            ..FocusController::default()
        };
        let sent = focus.user_select("p2");
        let rolled = focus.command_rejected(sent.request_id.as_deref().unwrap());
        assert!(rolled.rolled_back);
        assert_eq!(focus.active_pane_id.as_deref(), Some("p1"));
    }

    #[test]
    fn seed_overflow_never_queues_a_truncated_snapshot() {
        let mut seed = PaneSeedQueue::with_budget(4);
        assert_eq!(seed.queue("p", b"12345", "full", None), "overflow");
        assert!(!seed.pending.contains_key("p"));
        assert!(seed.deferred_full.contains("p"));
    }
}
