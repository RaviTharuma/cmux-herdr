//! Live event and polling pump.
//!
//! Port of `bridge/cmux_herdr_pump.py`. Event topology is normalized before
//! classification; output is always routed by explicit pane id, and timeout
//! polls never request a projection rebuild.

use std::collections::HashMap;
use std::time::Duration;

use serde_json::{json, Value};

use crate::api::{extract_agent_status, extract_read_text, HerdrApi};
use crate::engine::HerdrWindow;
use crate::live::LiveApplyHost;

pub const KIND_TOPOLOGY: &str = "topology";
pub const KIND_OUTPUT: &str = "output";
pub const KIND_FOCUS: &str = "focus";
pub const KIND_STATUS: &str = "status";
pub const KIND_METADATA: &str = "metadata";
pub const KIND_OTHER: &str = "other";

const TOPOLOGY_EVENTS: &[&str] = &[
    "workspace.created",
    "workspace.updated",
    "workspace.renamed",
    "workspace.moved",
    "workspace.reordered",
    "workspace.closed",
    "tab.created",
    "tab.closed",
    "tab.renamed",
    "tab.moved",
    "pane.created",
    "pane.closed",
    "pane.moved",
    "pane.exited",
    "pane.resized",
    "layout.updated",
    "layout.changed",
];
const OUTPUT_EVENTS: &[&str] = &["pane.updated", "pane.output_matched"];
const FOCUS_EVENTS: &[&str] = &["pane.focused", "tab.focused", "workspace.focused"];
const STATUS_EVENTS: &[&str] = &["pane.agent_status_changed", "pane.agent_detected"];
const METADATA_EVENTS: &[&str] = &["workspace.metadata_updated"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PumpResult {
    pub kind: String,
    pub resync: bool,
    pub routed_output: bool,
    pub focused: bool,
    pub status_updated: bool,
    pub pane_id: Option<String>,
    pub log: String,
}

impl Default for PumpResult {
    fn default() -> Self {
        Self {
            kind: KIND_OTHER.into(),
            resync: false,
            routed_output: false,
            focused: false,
            status_updated: false,
            pane_id: None,
            log: String::new(),
        }
    }
}

impl PumpResult {
    pub fn new(kind: &str) -> Self {
        Self {
            kind: kind.into(),
            ..Self::default()
        }
    }

    pub fn to_json(&self) -> Value {
        json!({
            "kind": self.kind,
            "resync": self.resync,
            "routed_output": self.routed_output,
            "focused": self.focused,
            "status_updated": self.status_updated,
            "pane_id": self.pane_id,
            "log": self.log,
        })
    }
}

fn truthy(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null) => false,
        Some(Value::Bool(value)) => *value,
        Some(Value::Number(value)) => value.as_f64().is_some_and(|number| number != 0.0),
        Some(Value::String(value)) => !value.is_empty(),
        Some(Value::Array(value)) => !value.is_empty(),
        Some(Value::Object(value)) => !value.is_empty(),
    }
}

pub fn unwrap_event(obj: Option<&Value>) -> Value {
    let Some(Value::Object(object)) = obj else {
        return json!({});
    };
    if let Some(Value::Object(data)) = object.get("data") {
        let mut body = data.clone();
        if !body.contains_key("type") {
            if let Some(Value::String(event)) = object.get("event") {
                body.insert("type".into(), Value::String(event.clone()));
            }
        }
        return Value::Object(body);
    }
    if let Some(Value::Object(event)) = object.get("event") {
        return Value::Object(event.clone());
    }
    if let Some(Value::Object(params)) = object.get("params") {
        if truthy(params.get("type"))
            || truthy(params.get("pane_id"))
            || truthy(params.get("event"))
        {
            return Value::Object(params.clone());
        }
    }
    Value::Object(object.clone())
}

pub fn event_type(obj: Option<&Value>) -> String {
    let body = unwrap_event(obj);
    if let Some(object) = body.as_object() {
        for key in ["type", "event", "name"] {
            if let Some(value) = object
                .get(key)
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty())
            {
                return value.to_string();
            }
        }
    }
    obj.and_then(Value::as_object)
        .and_then(|object| object.get("event"))
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .unwrap_or_default()
        .to_string()
}

pub fn event_string(body: &Value, names: &[&str]) -> String {
    let Some(object) = body.as_object() else {
        return String::new();
    };
    for name in names {
        if let Some(value) = object.get(*name).and_then(Value::as_str) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
    }
    for nested_key in ["pane", "agent", "tab", "workspace"] {
        if let Some(nested @ Value::Object(_)) = object.get(nested_key) {
            let found = event_string(nested, names);
            if !found.is_empty() {
                return found;
            }
        }
    }
    String::new()
}

pub fn classify_event(obj: Option<&Value>) -> &'static str {
    let name = event_type(obj);
    if TOPOLOGY_EVENTS.contains(&name.as_str()) {
        KIND_TOPOLOGY
    } else if OUTPUT_EVENTS.contains(&name.as_str()) {
        KIND_OUTPUT
    } else if FOCUS_EVENTS.contains(&name.as_str()) {
        KIND_FOCUS
    } else if STATUS_EVENTS.contains(&name.as_str()) {
        KIND_STATUS
    } else if METADATA_EVENTS.contains(&name.as_str()) {
        KIND_METADATA
    } else {
        KIND_OTHER
    }
}

pub trait PumpTransport {
    fn read_pane(&mut self, pane_id: &str) -> String;
    fn pane_info(&mut self, pane_id: &str) -> Value;
    fn send_text(&mut self, _pane_id: &str, _text: &str) {}
    fn send_keys(&mut self, _pane_id: &str, _keys: &str) {}
    fn close(&mut self) {}
}

#[derive(Debug, Clone, Default)]
pub struct MemoryTransport {
    pub reads: HashMap<String, String>,
    pub panes: HashMap<String, Value>,
    pub read_calls: Vec<String>,
    pub sent: Vec<(String, String, String)>,
}

impl MemoryTransport {
    pub fn new(reads: HashMap<String, String>, panes: HashMap<String, Value>) -> Self {
        Self {
            reads,
            panes,
            read_calls: Vec::new(),
            sent: Vec::new(),
        }
    }
}

impl PumpTransport for MemoryTransport {
    fn read_pane(&mut self, pane_id: &str) -> String {
        self.read_calls.push(pane_id.into());
        self.reads.get(pane_id).cloned().unwrap_or_default()
    }

    fn pane_info(&mut self, pane_id: &str) -> Value {
        self.panes
            .get(pane_id)
            .cloned()
            .filter(Value::is_object)
            .unwrap_or_else(|| json!({}))
    }

    fn send_text(&mut self, pane_id: &str, text: &str) {
        self.sent.push(("text".into(), pane_id.into(), text.into()));
    }

    fn send_keys(&mut self, pane_id: &str, keys: &str) {
        self.sent.push(("key".into(), pane_id.into(), keys.into()));
    }
}

pub struct ApiTransport<'a> {
    pub api: HerdrApi<'a>,
}

impl<'a> ApiTransport<'a> {
    pub fn new(api: Option<HerdrApi<'a>>) -> Self {
        Self {
            api: api.unwrap_or_else(|| HerdrApi::new(None, Duration::from_secs(8))),
        }
    }
}

impl PumpTransport for ApiTransport<'_> {
    fn read_pane(&mut self, pane_id: &str) -> String {
        let attempts = [
            (
                json!({"pane_id": pane_id, "source": "recent", "lines": 200, "ansi": true}),
                true,
            ),
            (
                json!({"pane_id": pane_id, "source": "recent", "lines": 200}),
                false,
            ),
        ];
        for (params, socket_only) in attempts {
            if let Ok(result) = self.api.call("pane.read", params, socket_only) {
                let text = extract_read_text(&result.result);
                if !text.is_empty() {
                    return text;
                }
            }
        }
        String::new()
    }

    fn pane_info(&mut self, pane_id: &str) -> Value {
        self.api
            .call("pane.get", json!({"pane_id": pane_id}), false)
            .ok()
            .map(|result| result.result)
            .filter(Value::is_object)
            .unwrap_or_else(|| json!({}))
    }

    fn send_text(&mut self, pane_id: &str, text: &str) {
        let _ = self.api.call(
            "pane.send_text",
            json!({"pane_id": pane_id, "text": text}),
            false,
        );
    }

    fn send_keys(&mut self, pane_id: &str, keys: &str) {
        let _ = self.api.call(
            "pane.send_keys",
            json!({"pane_id": pane_id, "keys": keys}),
            false,
        );
    }

    fn close(&mut self) {
        self.api.close();
    }
}

pub struct LivePump<T: PumpTransport> {
    pub transport: T,
    pub windows_builder: Option<Box<dyn FnMut() -> Vec<HerdrWindow>>>,
    pub log: Vec<String>,
}

impl<T: PumpTransport> LivePump<T> {
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            windows_builder: None,
            log: Vec::new(),
        }
    }

    pub fn with_windows_builder(
        mut self,
        builder: impl FnMut() -> Vec<HerdrWindow> + 'static,
    ) -> Self {
        self.windows_builder = Some(Box::new(builder));
        self
    }

    pub fn handle_event(
        &mut self,
        event: Option<&Value>,
        host: Option<&mut LiveApplyHost>,
    ) -> PumpResult {
        let Some(host) = host else {
            return PumpResult {
                kind: KIND_OTHER.into(),
                log: "no_host".into(),
                ..PumpResult::default()
            };
        };
        let kind = classify_event(event);
        let body = unwrap_event(event);
        let pane_id = event_string(&body, &["pane_id"]);
        match kind {
            KIND_TOPOLOGY => {
                if self.windows_builder.is_some() {
                    let log = format!("event:{}", event_type(event));
                    return self.resync(Some(host), &log);
                }
                self.log.push(format!("topology:{}", event_type(event)));
                PumpResult {
                    kind: kind.into(),
                    resync: true,
                    pane_id: (!pane_id.is_empty()).then_some(pane_id),
                    log: "topology".into(),
                    ..PumpResult::default()
                }
            }
            KIND_OUTPUT => self.route_output(host, &pane_id),
            KIND_FOCUS => self.route_focus(host, &pane_id, &body),
            KIND_STATUS => self.route_status(host, &pane_id, &body),
            KIND_METADATA => {
                self.log.push("metadata".into());
                PumpResult {
                    kind: kind.into(),
                    log: "metadata".into(),
                    ..PumpResult::default()
                }
            }
            _ => PumpResult {
                kind: kind.into(),
                pane_id: (!pane_id.is_empty()).then_some(pane_id),
                log: "ignored".into(),
                ..PumpResult::default()
            },
        }
    }

    pub fn poll(&mut self, host: Option<&mut LiveApplyHost>) -> PumpResult {
        let Some(host) = host else {
            return PumpResult {
                kind: KIND_OUTPUT.into(),
                log: "no_host".into(),
                ..PumpResult::default()
            };
        };
        let pane_ids = host.live_pane_ids();
        let mut routed = 0;
        for pane_id in &pane_ids {
            if self.paint(host, pane_id) {
                routed += 1;
            }
        }
        let flushed = self.flush_input(host);
        self.log
            .push(format!("poll:{routed}/{} in:{flushed}", pane_ids.len()));
        PumpResult {
            kind: KIND_OUTPUT.into(),
            routed_output: routed > 0,
            log: format!("poll:{routed}"),
            ..PumpResult::default()
        }
    }

    pub fn close(&mut self) {
        self.transport.close();
    }

    pub fn flush_input(&mut self, host: &mut LiveApplyHost) -> usize {
        let mut count = 0;
        for item in host.drain_input() {
            if item.kind == "key" {
                if let Some(keys) = item.key.as_deref().filter(|keys| !keys.is_empty()) {
                    if !item.pane_id.is_empty() {
                        self.transport.send_keys(&item.pane_id, keys);
                        count += 1;
                    }
                }
            } else if let Some(text) = item.text.as_deref().filter(|text| !text.is_empty()) {
                if !item.pane_id.is_empty() {
                    self.transport.send_text(&item.pane_id, text);
                    count += 1;
                }
            }
        }
        if count > 0 {
            self.log.push(format!("flush:{count}"));
        }
        count
    }

    pub fn resync(&mut self, host: Option<&mut LiveApplyHost>, log: &str) -> PumpResult {
        let Some(host) = host else {
            return PumpResult {
                kind: KIND_TOPOLOGY.into(),
                resync: true,
                log: "no_host".into(),
                ..PumpResult::default()
            };
        };
        let Some(builder) = self.windows_builder.as_mut() else {
            self.log.push("resync:no_builder".into());
            return PumpResult {
                kind: KIND_TOPOLOGY.into(),
                resync: true,
                log: "no_builder".into(),
                ..PumpResult::default()
            };
        };
        let windows = builder();
        let _ = host.apply_session(&windows);
        let painted = self.poll(Some(host));
        self.log.push(log.into());
        PumpResult {
            kind: KIND_TOPOLOGY.into(),
            resync: true,
            routed_output: painted.routed_output,
            log: log.into(),
            ..PumpResult::default()
        }
    }

    fn route_output(&mut self, host: &mut LiveApplyHost, pane_id: &str) -> PumpResult {
        if pane_id.is_empty() {
            return PumpResult {
                kind: KIND_OUTPUT.into(),
                log: "missing_pane".into(),
                ..PumpResult::default()
            };
        }
        let routed = self.paint(host, pane_id);
        PumpResult {
            kind: KIND_OUTPUT.into(),
            routed_output: routed,
            pane_id: Some(pane_id.into()),
            log: if routed { "output" } else { "output_noop" }.into(),
            ..PumpResult::default()
        }
    }

    fn route_focus(&mut self, host: &mut LiveApplyHost, pane_id: &str, body: &Value) -> PumpResult {
        let target = if pane_id.is_empty() {
            event_string(body, &["focused_pane_id", "active_pane_id"])
        } else {
            pane_id.to_string()
        };
        if target.is_empty() {
            let tab_id = event_string(body, &["tab_id"]);
            if !tab_id.is_empty() && host.apply_tab_focus(&tab_id) {
                self.log.push(format!("tab_focus:{tab_id}"));
                return PumpResult {
                    kind: KIND_FOCUS.into(),
                    focused: true,
                    log: "tab_focus".into(),
                    ..PumpResult::default()
                };
            }
            let workspace_id = event_string(body, &["workspace_id"]);
            if !workspace_id.is_empty() && host.apply_workspace_focus(&workspace_id) {
                self.log.push(format!("workspace_focus:{workspace_id}"));
                return PumpResult {
                    kind: KIND_FOCUS.into(),
                    focused: true,
                    log: "workspace_focus".into(),
                    ..PumpResult::default()
                };
            }
            return PumpResult {
                kind: KIND_FOCUS.into(),
                resync: true,
                log: "focus_resync".into(),
                ..PumpResult::default()
            };
        }
        let applied = host.apply_provider_focus(&target);
        let cwd = event_string(body, &["foreground_cwd", "cwd"]);
        if !cwd.is_empty() {
            let _ = host.route_cwd(&target, &cwd);
        }
        self.log.push(format!("focus:{target}"));
        PumpResult {
            kind: KIND_FOCUS.into(),
            focused: applied,
            pane_id: Some(target),
            log: "focus".into(),
            ..PumpResult::default()
        }
    }

    fn route_status(
        &mut self,
        host: &mut LiveApplyHost,
        pane_id: &str,
        body: &Value,
    ) -> PumpResult {
        if pane_id.is_empty() {
            return PumpResult {
                kind: KIND_STATUS.into(),
                log: "missing_pane".into(),
                ..PumpResult::default()
            };
        }
        let mut status = event_string(body, &["agent_status", "status", "state"]);
        if status.is_empty() {
            status = "unknown".into();
        }
        let name = event_string(body, &["agent", "display_agent", "label"]);
        host.note_agent_status(
            pane_id,
            &status,
            (!name.is_empty()).then_some(name.as_str()),
        );
        let info = self.transport.pane_info(pane_id);
        if status == "unknown" {
            if let Some(extracted) = extract_agent_status(&info) {
                status = extracted;
                host.note_agent_status(
                    pane_id,
                    &status,
                    (!name.is_empty()).then_some(name.as_str()),
                );
            }
        }
        let mut cwd = event_string(&info, &["foreground_cwd", "cwd"]);
        if cwd.is_empty() {
            cwd = event_string(body, &["foreground_cwd", "cwd"]);
        }
        if !cwd.is_empty() {
            let _ = host.route_cwd(pane_id, &cwd);
        }
        self.log.push(format!("status:{pane_id}:{status}"));
        PumpResult {
            kind: KIND_STATUS.into(),
            status_updated: true,
            pane_id: Some(pane_id.into()),
            log: format!("status:{status}"),
            ..PumpResult::default()
        }
    }

    fn paint(&mut self, host: &mut LiveApplyHost, pane_id: &str) -> bool {
        let text = self.transport.read_pane(pane_id);
        !text.is_empty() && host.paint_read(pane_id, &text)
    }
}

pub fn watch_followup(
    result: Option<&PumpResult>,
    had_event: bool,
    event_gap: bool,
) -> &'static str {
    if event_gap {
        return "project";
    }
    let Some(result) = result else {
        return if had_event { "none" } else { "project" };
    };
    if result.resync {
        "project"
    } else if result.status_updated {
        "pills"
    } else {
        "none"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_envelopes_are_classified_from_the_body() {
        let event = json!({"event": "pane.focused", "data": {"pane_id": "p"}});
        assert_eq!(classify_event(Some(&event)), KIND_FOCUS);
        assert_eq!(event_string(&unwrap_event(Some(&event)), &["pane_id"]), "p");
    }
}
