//! Running in-memory Herdr host apply machine.
//!
//! Behavioral port of `bridge/cmux_herdr_live.py`. The actual Ghostty/AppKit
//! objects remain native; this module preserves their state transitions,
//! ordering, routing, lifecycle, restore, and writer-handoff contracts.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde_json::{json, Value};

use crate::control::{
    adjacent_pane, apply_session_title, close_intent, encode_named_key, pane_surface_entries,
    request_split, tab_activity, CloseIntent, FocusController, InputForwarder, PaneSeedQueue,
    ProviderInput, TabActivity, UserSplit,
};
use crate::engine::{
    apply_window, client_grid, impose_after_apply, output_delta, reconcile_session, HerdrWindow,
    WindowMirrorState,
};
use crate::handoff;
use crate::host::{host_actions, FakeBonsplitHost, HostAction};
use crate::impose::{begin_divider_drag, end_divider_drag, resolve_divider_hold, DividerDragHold};
use crate::io::{CwdUpdate, PaneIORouter};
use crate::lifecycle::{
    dispatch, pane_grid_payload, AttachWindowTarget, DiscoveredSession, LifecycleController,
    RestoreRecord, POST_APPLY_CLIENT_SIZE, POST_RESEED,
};
use crate::model::Snapshot;
use crate::session::{FakeSessionHost, SessionAction};
use crate::state::{collect_host_fingerprint, parent_key, SystemEnv};

pub use crate::io::TitleEscapeFilter;
pub use crate::lifecycle::{
    decode_beta, endpoint_hash, grid_match, read_restore, validate_socket_path, write_restore,
    SETTING_KEY, SOCKET_METHODS,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GhosttySurface {
    pub pane_id: String,
    pub surface_id: String,
    pub buffer: Vec<u8>,
    pub cols: i64,
    pub rows: i64,
    pub first_responder: bool,
    pub live: bool,
    pub in_window: bool,
}

impl GhosttySurface {
    pub fn new(pane_id: &str, surface_id: &str) -> Self {
        Self {
            pane_id: pane_id.into(),
            surface_id: surface_id.into(),
            buffer: Vec::new(),
            cols: 80,
            rows: 24,
            first_responder: false,
            live: true,
            in_window: true,
        }
    }

    pub fn process_remote_output(&mut self, data: &[u8]) {
        if !data.is_empty() && self.live {
            self.buffer.extend_from_slice(data);
        }
    }

    pub fn resize_grid(&mut self, cols: i64, rows: i64) {
        if cols >= 1 && rows >= 1 {
            self.cols = cols;
            self.rows = rows;
        }
    }
}

#[derive(Debug, Clone)]
pub struct LiveWindowMirror {
    pub tab_id: String,
    pub title: String,
    pub bonsplit: FakeBonsplitHost,
    pub io: PaneIORouter,
    pub focus: FocusController,
    pub seed: PaneSeedQueue,
    pub input: InputForwarder,
    pub surfaces: BTreeMap<String, GhosttySurface>,
    surface_order: Vec<String>,
    pub state: Option<WindowMirrorState>,
    pub is_applying_focus: bool,
    pub is_applying_layout: bool,
    pub is_torn_down: bool,
    pub is_visible_for_sizing: bool,
    pub container_width: f64,
    pub container_height: f64,
    pub cell_width: f64,
    pub cell_height: f64,
    pub last_client_grid: Option<(i64, i64)>,
    pub drag_hold: Option<DividerDragHold>,
    pub structure_version: i64,
    pub tab_cwd: Option<String>,
}

impl LiveWindowMirror {
    pub fn new(tab_id: &str, title: &str) -> Self {
        Self {
            tab_id: tab_id.into(),
            title: title.into(),
            bonsplit: FakeBonsplitHost::default(),
            io: PaneIORouter::default(),
            focus: FocusController::default(),
            seed: PaneSeedQueue::default(),
            input: InputForwarder::default(),
            surfaces: BTreeMap::new(),
            surface_order: Vec::new(),
            state: None,
            is_applying_focus: false,
            is_applying_layout: false,
            is_torn_down: false,
            is_visible_for_sizing: true,
            container_width: 800.0,
            container_height: 400.0,
            cell_width: 8.0,
            cell_height: 16.0,
            last_client_grid: None,
            drag_hold: None,
            structure_version: 0,
            tab_cwd: None,
        }
    }

    pub fn surface(&self, pane_id: &str) -> Option<&GhosttySurface> {
        self.surfaces.get(pane_id)
    }

    pub fn surface_mut(&mut self, pane_id: &str) -> Option<&mut GhosttySurface> {
        self.surfaces.get_mut(pane_id)
    }

    pub fn make_panel(&mut self, pane_id: &str) -> &mut GhosttySurface {
        let needs_create = self
            .surfaces
            .get(pane_id)
            .is_none_or(|surface| !surface.live);
        if needs_create {
            if !self.surface_order.iter().any(|item| item == pane_id) {
                self.surface_order.push(pane_id.into());
            }
            let surface_id = format!("surf-{}-{pane_id}", self.tab_id);
            self.surfaces
                .insert(pane_id.into(), GhosttySurface::new(pane_id, &surface_id));
            self.io.bind(pane_id, &surface_id);
        }
        self.surfaces.get_mut(pane_id).unwrap()
    }

    pub fn close_panel(&mut self, pane_id: &str) {
        if let Some(surface) = self.surfaces.get_mut(pane_id) {
            surface.live = false;
        }
        self.surfaces.remove(pane_id);
        self.surface_order.retain(|item| item != pane_id);
        self.io.unbind(pane_id);
    }

    pub fn apply_window(&mut self, window: &HerdrWindow) -> Result<Vec<String>, String> {
        if self.is_torn_down {
            return Ok(Vec::new());
        }
        let previous = self.state.as_ref();
        let previous_rendered = previous.map(|state| state.layout.clone());
        let (state, result) = apply_window(window, previous);
        self.state = Some(state);
        self.title = window.title.clone();
        self.structure_version = self
            .state
            .as_ref()
            .map_or(0, |state| state.layout_structure_version);
        let plan = impose_after_apply(&result, previous_rendered.as_ref(), &window.title);
        let actions = host_actions(&result, &plan);
        let mut log = Vec::new();
        self.is_applying_layout = true;
        let applied = actions.iter().try_for_each(|action| {
            let item = self.apply_host_action(action)?;
            if !item.is_empty() {
                log.push(item);
            }
            Ok::<(), String>(())
        });
        self.is_applying_layout = false;
        applied?;

        let pane_ids = self
            .state
            .as_ref()
            .map(|state| state.pane_ids.clone())
            .unwrap_or_default();
        self.io.set_live_panes(&pane_ids);
        self.focus.live_pane_ids = pane_ids;
        if let Some(pane_id) = result.focus_pane_id.as_deref() {
            self.apply_provider_focus(pane_id);
        }
        self.title = apply_session_title(&window.title, Some(&self.title), false)
            .unwrap_or_else(|| window.title.clone());
        self.apply_cached_cwd();
        Ok(log)
    }

    fn apply_host_action(&mut self, action: &HostAction) -> Result<String, String> {
        match action.op.as_str() {
            "create_panel" => {
                if let Some(pane_id) = action.pane_id.as_deref() {
                    self.make_panel(pane_id);
                    self.bonsplit
                        .apply(std::slice::from_ref(action))
                        .map_err(|error| error.to_string())?;
                    Ok(format!("make_panel:{pane_id}"))
                } else {
                    Ok(String::new())
                }
            }
            "close_panel" => {
                if let Some(pane_id) = action.pane_id.as_deref() {
                    self.close_panel(pane_id);
                    self.bonsplit
                        .apply(std::slice::from_ref(action))
                        .map_err(|error| error.to_string())?;
                    Ok(format!("close_panel:{pane_id}"))
                } else {
                    Ok(String::new())
                }
            }
            "focus" => Ok(String::new()),
            _ => {
                self.bonsplit
                    .apply(std::slice::from_ref(action))
                    .map_err(|error| error.to_string())?;
                Ok(action.op.clone())
            }
        }
    }

    pub fn route_output(&mut self, pane_id: &str, data: &[u8]) -> bool {
        let Some(write) = self.io.route_output(pane_id, data) else {
            return false;
        };
        let Some(surface) = self
            .surfaces
            .get_mut(pane_id)
            .filter(|surface| surface.live)
        else {
            return false;
        };
        surface.process_remote_output(&write.data);
        true
    }

    pub fn route_read_snapshot(&mut self, pane_id: &str, text: &str) -> bool {
        let (chunk, _) = output_delta(self.io.last_snapshot.get(pane_id).map(String::as_str), text);
        self.io.last_snapshot.insert(pane_id.into(), text.into());
        !chunk.is_empty() && self.route_output(pane_id, chunk.as_bytes())
    }

    pub fn send_text(&mut self, pane_id: &str, text: &str) -> &'static str {
        if self.io.route_input(pane_id, text.as_bytes()).is_none() {
            return "inactive";
        }
        self.input.enqueue(ProviderInput {
            pane_id: pane_id.into(),
            kind: "text".into(),
            text: Some(text.into()),
            key: None,
            csi: None,
        })
    }

    pub fn send_named_key(&mut self, pane_id: &str, name: &str) -> &'static str {
        let Some(item) = encode_named_key(pane_id, name) else {
            return "unknown";
        };
        if !self.surfaces.contains_key(pane_id) {
            return "inactive";
        }
        self.input.enqueue(item)
    }

    pub fn apply_provider_focus(&mut self, pane_id: &str) {
        self.is_applying_focus = true;
        self.focus.provider_confirms(pane_id);
        self.io.note_remote_active(pane_id);
        self.apply_cached_cwd();
        self.is_applying_focus = false;
    }

    pub fn route_cwd(&mut self, pane_id: &str, path: &str) -> Option<CwdUpdate> {
        let update = self.io.route_cwd(pane_id, path, &self.tab_id);
        if let Some(update) = update.as_ref().filter(|update| update.apply_to_tab) {
            self.tab_cwd = Some(update.path.clone());
        }
        update
    }

    fn apply_cached_cwd(&mut self) {
        let active = self
            .io
            .active_pane_id
            .as_ref()
            .or(self.focus.active_pane_id.as_ref());
        if let Some(path) = active.and_then(|pane_id| self.io.cwd_by_pane.get(pane_id)) {
            self.tab_cwd = Some(path.clone());
        }
    }

    pub fn user_focus(&mut self, pane_id: &str) -> Option<String> {
        if !self.surfaces.contains_key(pane_id) {
            return None;
        }
        let command = self.focus.user_select(pane_id);
        if !self.is_applying_focus {
            for surface in self.surfaces.values_mut() {
                surface.first_responder = false;
            }
            self.surfaces.get_mut(pane_id).unwrap().first_responder = true;
        }
        self.io.user_focus(pane_id);
        self.apply_cached_cwd();
        command.pane_id
    }

    pub fn navigate_focus(&mut self, direction: &str) -> Option<String> {
        let state = self.state.as_ref()?;
        let active = self.focus.active_pane_id.as_deref()?;
        let neighbor = adjacent_pane(&state.layout, active, direction)?;
        self.user_focus(&neighbor)
    }

    pub fn user_split(&self, pane_id: &str, direction: &str) -> Option<UserSplit> {
        if !self.surfaces.contains_key(pane_id) {
            return None;
        }
        let vertical = matches!(direction, "down" | "vertical" | "below");
        request_split(pane_id, vertical, false, true)
    }

    pub fn update_client_size(&mut self) -> Option<(i64, i64)> {
        if !self.is_visible_for_sizing || self.is_torn_down {
            return None;
        }
        let grid = client_grid(
            self.container_width,
            self.container_height,
            self.cell_width,
            self.cell_height,
            0.0,
            0.0,
        )?;
        if self.last_client_grid == Some(grid) {
            return Some(grid);
        }
        self.last_client_grid = Some(grid);
        for surface in self.surfaces.values_mut().filter(|surface| surface.live) {
            surface.resize_grid(grid.0, grid.1);
        }
        Some(grid)
    }

    pub fn begin_drag(&mut self, split_key: &str, axis: &str, assigned_cells: i64) {
        self.drag_hold = Some(begin_divider_drag(split_key, axis, assigned_cells));
    }

    pub fn end_drag(
        &mut self,
        dragged_extent: f64,
        axis_span: f64,
        total_cells: i64,
        assigned_cells: i64,
    ) -> (i64, bool) {
        let (cells, should_send) =
            end_divider_drag(dragged_extent, axis_span, total_cells, assigned_cells);
        if should_send && self.drag_hold.is_some() {
            self.drag_hold =
                resolve_divider_hold(self.drag_hold.take(), Some(assigned_cells), true);
        }
        if should_send {
            let (split_key, axis) = self
                .drag_hold
                .as_ref()
                .map(|hold| (hold.split_key.clone(), hold.axis.clone()))
                .unwrap_or_else(|| ("s".into(), "horizontal".into()));
            self.drag_hold = Some(begin_divider_drag(&split_key, &axis, cells));
        } else {
            self.drag_hold = None;
        }
        (cells, should_send)
    }

    pub fn note_resize_reply(&mut self, assigned_cells: i64, split_exists: bool) {
        self.drag_hold =
            resolve_divider_hold(self.drag_hold.take(), Some(assigned_cells), split_exists);
    }

    pub fn seed_pane(
        &mut self,
        pane_id: &str,
        data: &[u8],
        cols: i64,
        rows: i64,
    ) -> Option<Vec<u8>> {
        self.seed.queue(pane_id, data, "full", Some((cols, rows)));
        let current = self
            .surfaces
            .get(pane_id)
            .map(|surface| (surface.cols, surface.rows))
            .unwrap_or((0, 0));
        let flushed = self.seed.note_ready(pane_id, current.0, current.1);
        if let Some(data) = flushed.as_deref() {
            self.route_output(pane_id, data);
        }
        flushed
    }

    pub fn teardown(&mut self) {
        self.is_torn_down = true;
        self.input.deactivate();
        for surface in self.surfaces.values_mut() {
            surface.live = false;
            surface.first_responder = false;
        }
    }

    pub fn pane_grids(&self) -> Value {
        let layout = self.state.as_ref().map(|state| &state.layout);
        let leaves = layout.map(walk_leaves).unwrap_or_default();
        let panes: Vec<Value> = self
            .surfaces
            .iter()
            .map(|(pane_id, surface)| {
                let assigned = leaves
                    .iter()
                    .find(|(id, _, _)| id == pane_id)
                    .map(|(_, cols, rows)| ((*cols).max(1), (*rows).max(1)))
                    .unwrap_or((surface.cols, surface.rows));
                json!({
                    "pane_id": pane_id,
                    "assigned_cols": assigned.0,
                    "assigned_rows": assigned.1,
                    "rendered_cols": surface.cols,
                    "rendered_rows": surface.rows,
                    "exact_cols": true,
                    "exact_rows": true,
                    "has_panel": surface.live,
                })
            })
            .collect();
        pane_grid_payload(
            &self.tab_id,
            &panes,
            self.structure_version,
            self.state.as_ref().is_some_and(|state| state.zoomed),
            layout.map_or(0, |layout| layout.rect.width),
            layout.map_or(0, |layout| layout.rect.height),
            self.last_client_grid,
            self.is_visible_for_sizing,
        )
    }
}

fn walk_leaves(node: &crate::layout::LayoutNode) -> Vec<(String, i64, i64)> {
    if node.kind == "pane" {
        return node
            .pane_id
            .as_ref()
            .map(|id| vec![(id.clone(), node.rect.width, node.rect.height)])
            .unwrap_or_default();
    }
    node.children.iter().flat_map(walk_leaves).collect()
}

#[derive(Debug)]
pub struct LiveApplyHost {
    pub enabled: bool,
    pub socket_path: String,
    pub claim_native_writer: bool,
    pub windows: BTreeMap<String, LiveWindowMirror>,
    window_order: Vec<String>,
    pub session_host: FakeSessionHost,
    pub lifecycle: LifecycleController,
    pub previous_tab_ids: Vec<String>,
    pub previous_titles: BTreeMap<String, String>,
    pub defaults_open: bool,
    pub agent_statuses: Vec<(String, String)>,
    pub agent_names: Vec<(String, String)>,
    pub focused_workspace_id: Option<String>,
    pub native_live: bool,
    pub server_stopped: bool,
    pub log: Vec<String>,
}

impl Default for LiveApplyHost {
    fn default() -> Self {
        Self::new(true, "/tmp/herdr.sock", false)
    }
}

impl LiveApplyHost {
    pub fn new(enabled: bool, socket_path: &str, claim_native_writer: bool) -> Self {
        Self {
            enabled,
            socket_path: socket_path.into(),
            claim_native_writer,
            windows: BTreeMap::new(),
            window_order: Vec::new(),
            session_host: FakeSessionHost::default(),
            lifecycle: LifecycleController::new(enabled, true),
            previous_tab_ids: Vec::new(),
            previous_titles: BTreeMap::new(),
            defaults_open: true,
            agent_statuses: Vec::new(),
            agent_names: Vec::new(),
            focused_workspace_id: None,
            native_live: false,
            server_stopped: false,
            log: Vec::new(),
        }
    }

    pub fn window(&self, tab_id: &str) -> Option<&LiveWindowMirror> {
        self.windows.get(tab_id)
    }

    pub fn window_mut(&mut self, tab_id: &str) -> Option<&mut LiveWindowMirror> {
        self.windows.get_mut(tab_id)
    }

    pub fn attach(&mut self, sessions: &[DiscoveredSession], activate: bool) -> Value {
        if !self.enabled {
            return json!({"ok": false, "outcome": "disabled"});
        }
        let result = self.lifecycle.attach(
            &self.socket_path,
            sessions,
            &AttachWindowTarget::new("contextual", None::<String>),
            activate,
        );
        if result["ok"].as_bool() == Some(true) {
            match result["post_attach"].as_str() {
                Some(POST_APPLY_CLIENT_SIZE) => {
                    for mirror in self.windows.values_mut() {
                        mirror.update_client_size();
                    }
                }
                Some(POST_RESEED) => {
                    for mirror in self.windows.values_mut() {
                        let seeds: Vec<_> = mirror
                            .surfaces
                            .iter()
                            .map(|(pane_id, surface)| {
                                (
                                    pane_id.clone(),
                                    surface.buffer.clone(),
                                    surface.cols,
                                    surface.rows,
                                )
                            })
                            .collect();
                        for (pane_id, data, cols, rows) in seeds {
                            mirror.seed_pane(&pane_id, &data, cols, rows);
                        }
                    }
                }
                _ => {}
            }
        }
        self.log.push(format!(
            "attach:{}",
            result["outcome"].as_str().unwrap_or("None")
        ));
        result
    }

    pub fn apply_session(&mut self, windows: &[HerdrWindow]) -> Result<Value, String> {
        if !self.enabled {
            return Ok(json!({"ok": false, "outcome": "disabled"}));
        }
        let session = reconcile_session(windows, &self.previous_tab_ids);
        let titles: std::collections::HashMap<String, String> = windows
            .iter()
            .map(|window| (window.tab_id.clone(), window.title.clone()))
            .collect();
        let previous_titles: std::collections::HashMap<String, String> =
            self.previous_titles.clone().into_iter().collect();
        let actions = crate::session::session_actions(
            &session,
            Some(&titles),
            Some(&previous_titles),
            self.defaults_open,
            None,
        );
        self.session_host.apply(&actions)?;
        if actions
            .iter()
            .any(|action| action.op == "close_default_tabs")
        {
            self.defaults_open = false;
        }
        for tab_id in &session.closed_tab_ids {
            if let Some(mut mirror) = self.windows.remove(tab_id) {
                mirror.teardown();
            }
            self.window_order.retain(|item| item != tab_id);
        }
        let mut applied = Vec::new();
        for window in windows {
            if !self.windows.contains_key(&window.tab_id) {
                self.window_order.push(window.tab_id.clone());
                self.windows.insert(
                    window.tab_id.clone(),
                    LiveWindowMirror::new(&window.tab_id, &window.title),
                );
            }
            let mirror = self.windows.get_mut(&window.tab_id).unwrap();
            applied.extend(mirror.apply_window(window)?);
            mirror.update_client_size();
        }
        self.previous_tab_ids = session.ordered_tab_ids.clone();
        self.previous_titles = titles.into_iter().collect();
        self.log
            .push(format!("session:tabs={}", self.windows.len()));
        Ok(json!({
            "ok": true,
            "tabs": self.window_order,
            "session_ops": actions.iter().map(|action| action.op.as_str()).collect::<Vec<_>>(),
            "window_ops": applied,
            "defaults_open": self.defaults_open,
        }))
    }

    pub fn route_output(&mut self, pane_id: &str, data: &[u8]) -> bool {
        self.windows
            .values_mut()
            .find(|mirror| mirror.surfaces.contains_key(pane_id))
            .is_some_and(|mirror| mirror.route_output(pane_id, data))
    }

    pub fn route_cwd(&mut self, pane_id: &str, path: &str) -> Option<CwdUpdate> {
        self.windows
            .values_mut()
            .find(|mirror| mirror.surfaces.contains_key(pane_id))?
            .route_cwd(pane_id, path)
    }

    pub fn route_read_snapshot(&mut self, pane_id: &str, text: &str) -> bool {
        self.windows
            .values_mut()
            .find(|mirror| mirror.surfaces.contains_key(pane_id))
            .is_some_and(|mirror| mirror.route_read_snapshot(pane_id, text))
    }

    pub fn paint_read(&mut self, pane_id: &str, text: &str) -> bool {
        let Some(mirror) = self
            .windows
            .values_mut()
            .find(|mirror| mirror.surfaces.contains_key(pane_id))
        else {
            return false;
        };
        if mirror.io.last_snapshot.contains_key(pane_id) {
            return mirror.route_read_snapshot(pane_id, text);
        }
        let (cols, rows) = mirror
            .surfaces
            .get(pane_id)
            .map(|surface| (surface.cols, surface.rows))
            .unwrap();
        let flushed = mirror.seed_pane(pane_id, text.as_bytes(), cols, rows);
        if flushed.is_some() {
            mirror.io.last_snapshot.insert(pane_id.into(), text.into());
            true
        } else {
            false
        }
    }

    pub fn apply_provider_focus(&mut self, pane_id: &str) -> bool {
        let Some(mirror) = self
            .windows
            .values_mut()
            .find(|mirror| mirror.surfaces.contains_key(pane_id))
        else {
            return false;
        };
        mirror.apply_provider_focus(pane_id);
        true
    }

    pub fn apply_tab_focus(&mut self, tab_id: &str) -> bool {
        if !self.windows.contains_key(tab_id) {
            return false;
        }
        let mut action = SessionAction::new("focus_tab");
        action.tab_id = Some(tab_id.into());
        self.session_host.apply(&[action]).is_ok()
    }

    pub fn apply_workspace_focus(&mut self, workspace_id: &str) -> bool {
        if workspace_id.is_empty() {
            return false;
        }
        self.focused_workspace_id = Some(workspace_id.into());
        self.log.push(format!("workspace_focus:{workspace_id}"));
        true
    }

    pub fn drain_input(&mut self) -> Vec<ProviderInput> {
        let mut items = Vec::new();
        for tab_id in self.window_order.clone() {
            if let Some(mirror) = self
                .windows
                .get_mut(&tab_id)
                .filter(|mirror| !mirror.is_torn_down)
            {
                items.extend(mirror.input.drain());
            }
        }
        items
    }

    pub fn note_agent_status(&mut self, pane_id: &str, status: &str, name: Option<&str>) {
        if !pane_id.is_empty() && !status.is_empty() {
            ordered_set(&mut self.agent_statuses, pane_id, status);
        }
        if let Some(name) = name.filter(|name| !name.is_empty()) {
            if !pane_id.is_empty() {
                ordered_set(&mut self.agent_names, pane_id, name);
            }
        }
    }

    pub fn live_pane_ids(&self) -> Vec<String> {
        self.window_order
            .iter()
            .filter_map(|tab_id| self.windows.get(tab_id))
            .filter(|mirror| !mirror.is_torn_down)
            .flat_map(|mirror| {
                mirror.surface_order.iter().filter_map(|pane_id| {
                    mirror
                        .surfaces
                        .get(pane_id)
                        .filter(|surface| surface.live)
                        .map(|_| pane_id.clone())
                })
            })
            .collect()
    }

    pub fn detach(&mut self) -> Value {
        for mirror in self.windows.values_mut() {
            mirror.teardown();
        }
        self.windows.clear();
        self.window_order.clear();
        let closed = self.lifecycle.close_host("host_tab");
        self.native_live = false;
        self.log.push("detach".into());
        json!({
            "ok": true,
            "outcome": "detach",
            "server_stopped": false,
            "lifecycle": closed,
        })
    }

    pub fn restore(&mut self, sessions: &[DiscoveredSession], windows: &[HerdrWindow]) -> Value {
        let restored = self.lifecycle.restore(sessions);
        let applied = self
            .apply_session(windows)
            .unwrap_or_else(|error| json!({"ok": false, "error": error}));
        for mirror in self.windows.values_mut() {
            let seeds: Vec<_> = mirror
                .surfaces
                .iter()
                .map(|(pane_id, surface)| {
                    (
                        pane_id.clone(),
                        surface.buffer.clone(),
                        surface.cols,
                        surface.rows,
                    )
                })
                .collect();
            for (pane_id, data, cols, rows) in seeds {
                mirror.seed_pane(&pane_id, &data, cols, rows);
            }
        }
        json!({
            "ok": restored["ok"].as_bool().unwrap_or(false) && applied["ok"].as_bool().unwrap_or(false),
            "mode": "reattach",
            "post_attach": POST_RESEED,
            "restore": restored,
            "apply": applied,
        })
    }

    pub fn set_native_live(&mut self, marker_writer: Option<&mut dyn FnMut()>) {
        if !self.claim_native_writer {
            return;
        }
        if let Some(marker_writer) = marker_writer {
            marker_writer();
        } else {
            let _ = handoff::claim_native_writer(
                &fingerprint_key(),
                &self.socket_path,
                &endpoint_hash(&self.socket_path),
                None,
            );
        }
        self.native_live = true;
        self.log.push("native_live".into());
    }

    pub fn close_user_pane(&self, pane_id: &str) -> CloseIntent {
        let status = ordered_get(&self.agent_statuses, pane_id);
        close_intent("user_pane", Some(pane_id), status)
    }

    pub fn activity(&self) -> TabActivity {
        tab_activity(&self.agent_statuses, Some(&self.agent_names))
    }

    pub fn state(&self, session_id: &str) -> Value {
        let mut payload = self.lifecycle.state(session_id);
        if let Some(object) = payload.as_object_mut() {
            object.insert("window_count".into(), json!(self.windows.len()));
            object.insert("window_ids".into(), json!(self.window_order));
            object.insert(
                "total_output_bytes".into(),
                json!(self
                    .windows
                    .values()
                    .flat_map(|mirror| mirror.surfaces.values())
                    .map(|surface| surface.buffer.len())
                    .sum::<usize>()),
            );
        }
        payload
    }

    pub fn pane_surfaces(&self) -> Vec<Value> {
        let bindings: Vec<_> = self
            .windows
            .iter()
            .flat_map(|(tab_id, mirror)| {
                mirror.surfaces.iter().map(move |(pane_id, surface)| {
                    (
                        tab_id.clone(),
                        pane_id.clone(),
                        surface.surface_id.clone(),
                        surface.live && !mirror.is_torn_down,
                    )
                })
            })
            .collect();
        pane_surface_entries(&bindings)
    }

    pub fn pane_grids(&self) -> Vec<Value> {
        self.window_order
            .iter()
            .filter_map(|tab_id| self.windows.get(tab_id))
            .map(LiveWindowMirror::pane_grids)
            .collect()
    }

    pub fn observe(&mut self, method: &str, params: Option<&Value>) -> Value {
        let fallback = json!({"socket": self.socket_path, "session": "main"});
        let mut gate = dispatch(method, params.unwrap_or(&fallback), self.enabled).to_value();
        if gate["ok"].as_bool() != Some(true) {
            return gate;
        }
        match method {
            "remote.herdr.pane_surfaces" => {
                let panes = self.pane_surfaces();
                let mirrored = !self.windows.is_empty();
                let object = gate.as_object_mut().unwrap();
                object.insert("panes".into(), Value::Array(panes));
                object.insert("mirrored".into(), json!(mirrored));
            }
            "remote.herdr.pane_grids" => {
                let windows = self.pane_grids();
                let mirrored = !self.windows.is_empty();
                let object = gate.as_object_mut().unwrap();
                object.insert("windows".into(), Value::Array(windows));
                object.insert("mirrored".into(), json!(mirrored));
            }
            "remote.herdr.state" => {
                let session = gate["session"].as_str().unwrap_or("main").to_string();
                if let Some(state) = self.state(&session).as_object() {
                    gate.as_object_mut().unwrap().extend(state.clone());
                }
            }
            "remote.herdr.detach" => {
                if let Some(detached) = self.detach().as_object() {
                    gate.as_object_mut().unwrap().extend(detached.clone());
                }
            }
            _ => {}
        }
        gate
    }
}

fn ordered_set(entries: &mut Vec<(String, String)>, key: &str, value: &str) {
    if let Some((_, old)) = entries.iter_mut().find(|(item, _)| item == key) {
        *old = value.into();
    } else {
        entries.push((key.into(), value.into()));
    }
}

fn ordered_get<'a>(entries: &'a [(String, String)], key: &str) -> Option<&'a str> {
    entries
        .iter()
        .find(|(item, _)| item == key)
        .map(|(_, value)| value.as_str())
}

pub fn apply_live_windows(
    windows: &[HerdrWindow],
    host: Option<LiveApplyHost>,
    enabled: bool,
) -> Result<LiveApplyHost, String> {
    let mut machine = host.unwrap_or_else(|| LiveApplyHost::new(enabled, "/tmp/herdr.sock", false));
    machine.apply_session(windows)?;
    Ok(machine)
}

fn fingerprint_key() -> String {
    parent_key(&collect_host_fingerprint(&SystemEnv))
}

pub fn restore_record_path(socket_path: &str) -> PathBuf {
    handoff::restore_paths(&endpoint_hash(socket_path))
        .into_iter()
        .next()
        .unwrap_or_default()
}

pub fn resolve_socket_path(explicit: Option<&str>) -> Option<String> {
    let home_default = std::env::var("HOME")
        .ok()
        .map(|home| format!("{home}/.config/herdr/herdr.sock"));
    [
        explicit.map(str::to_string),
        std::env::var("HERDR_SOCKET_PATH").ok(),
        home_default,
    ]
    .into_iter()
    .flatten()
    .find_map(|path| validate_socket_path(Some(&path)))
}

pub fn sessions_from_snapshot(snapshot: &Snapshot) -> Vec<DiscoveredSession> {
    if !snapshot.workspaces.is_empty() {
        return snapshot
            .workspaces
            .iter()
            .map(|workspace| {
                let session_id = if workspace.workspace_id.is_empty() {
                    "main"
                } else {
                    &workspace.workspace_id
                };
                DiscoveredSession::new(
                    session_id,
                    workspace
                        .label
                        .as_deref()
                        .filter(|label| !label.is_empty())
                        .unwrap_or(session_id),
                    workspace.tab_count,
                    false,
                )
            })
            .collect();
    }
    vec![DiscoveredSession::new(
        "main",
        "main",
        snapshot.tabs.len() as i64,
        false,
    )]
}

pub fn persist_host_restore(host: &LiveApplyHost) -> Option<String> {
    let record = host.lifecycle.persist.as_ref()?;
    handoff::write_shared_restore(&record.endpoint_hash, &record.to_value()).ok()
}

pub fn clear_host_restore(socket_path: &str) -> bool {
    handoff::clear_shared_restore(&endpoint_hash(socket_path))
}

fn foreign_payload(action: &str, method: Option<&str>) -> Option<Value> {
    let decision = handoff::resolve_writer(&fingerprint_key(), None, None);
    let observe_foreign_plugin = action == "observe"
        && decision.plugin_live
        && decision
            .lease
            .as_ref()
            .is_some_and(|lease| lease.pid != 0 && lease.pid != std::process::id() as i64);
    if action == "observe" && (decision.yields() || observe_foreign_plugin) {
        return Some(handoff::observe_foreign(
            &decision,
            method.unwrap_or("remote.herdr.state"),
        ));
    }
    if decision.yields() {
        let mut body = decision.payload(action, method);
        if action == "restore" {
            body.as_object_mut()
                .unwrap()
                .insert("mode".into(), json!("reattach"));
        }
        return Some(body);
    }
    None
}

pub fn attach_live(
    windows: &[HerdrWindow],
    sessions: &[DiscoveredSession],
    socket_path: &str,
    activate: bool,
    persist: bool,
) -> (Option<LiveApplyHost>, Value) {
    if let Some(mut yielded) = foreign_payload("attach", None) {
        let outcome = yielded["outcome"].clone();
        let object = yielded.as_object_mut().unwrap();
        object.insert("restore_path".into(), Value::Null);
        object.insert("apply".into(), Value::Null);
        object.insert("attach".into(), json!({"ok": true, "outcome": outcome}));
        return (None, yielded);
    }
    let mut host = LiveApplyHost::new(true, socket_path, false);
    let applied = host
        .apply_session(windows)
        .unwrap_or_else(|error| json!({"ok": false, "error": error}));
    let attached = host.attach(sessions, activate);
    let path = if persist && attached["ok"].as_bool() == Some(true) {
        persist_host_restore(&host)
    } else {
        None
    };
    if attached["ok"].as_bool() == Some(true) {
        let _ = handoff::claim_plugin_writer(
            &fingerprint_key(),
            socket_path,
            &endpoint_hash(socket_path),
        );
    }
    let result = json!({
        "ok": applied["ok"].as_bool().unwrap_or(false) && attached["ok"].as_bool().unwrap_or(false),
        "apply": applied,
        "attach": attached,
        "restore_path": path,
        "server_stopped": false,
        "writer": "plugin",
        "outcome": attached["outcome"],
    });
    (Some(host), result)
}

pub fn restore_live(
    windows: &[HerdrWindow],
    sessions: &[DiscoveredSession],
    socket_path: &str,
) -> (Option<LiveApplyHost>, Value) {
    if let Some(yielded) = foreign_payload("restore", None) {
        return (None, yielded);
    }
    let mut host = LiveApplyHost::new(true, socket_path, false);
    let hashed = endpoint_hash(socket_path);
    let record = handoff::read_shared_restore(&hashed)
        .as_ref()
        .and_then(RestoreRecord::from_value)
        .or_else(|| {
            read_restore(&restore_record_path(socket_path))
                .ok()
                .flatten()
        });
    let Some(record) = record else {
        return (
            Some(host),
            json!({"ok": false, "outcome": "no_persist", "server_stopped": false}),
        );
    };
    host.lifecycle.persist = Some(record);
    let mut restored = host.restore(sessions, windows);
    let path = if restored["ok"].as_bool() == Some(true) {
        persist_host_restore(&host)
    } else {
        None
    };
    if restored["ok"].as_bool() == Some(true) {
        let _ = handoff::claim_plugin_writer(&fingerprint_key(), socket_path, &hashed);
    }
    let object = restored.as_object_mut().unwrap();
    object.insert("restore_path".into(), json!(path));
    object.insert("server_stopped".into(), json!(false));
    object.insert("writer".into(), json!("plugin"));
    (Some(host), restored)
}

pub fn observe_live(
    windows: &[HerdrWindow],
    socket_path: &str,
    method: &str,
    session: &str,
) -> (Option<LiveApplyHost>, Value) {
    if let Some(yielded) = foreign_payload("observe", Some(method)) {
        return (None, yielded);
    }
    let Ok(mut host) = apply_live_windows(windows, None, true) else {
        return (None, json!({"ok": false, "code": "apply_failed"}));
    };
    host.socket_path = socket_path.into();
    let observed = host.observe(
        method,
        Some(&json!({"socket": socket_path, "session": session})),
    );
    (Some(host), observed)
}

pub fn detach_live(windows: &[HerdrWindow], socket_path: &str) -> Value {
    if let Some(mut yielded) = foreign_payload("detach", None) {
        let object = yielded.as_object_mut().unwrap();
        object.insert("detached".into(), json!(false));
        object.insert("restore_cleared".into(), json!(false));
        return yielded;
    }
    let Ok(mut host) = apply_live_windows(windows, None, true) else {
        return json!({"ok": false, "outcome": "apply_failed", "server_stopped": false});
    };
    host.socket_path = socket_path.into();
    let mut closed = host.detach();
    let object = closed.as_object_mut().unwrap();
    object.insert(
        "restore_cleared".into(),
        json!(clear_host_restore(socket_path)),
    );
    object.insert("detached".into(), json!(true));
    handoff::release_plugin_writer(&fingerprint_key());
    closed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{LayoutNode, LayoutRect};

    fn window(tab_id: &str, pane_id: &str) -> HerdrWindow {
        HerdrWindow::new(
            tab_id,
            tab_id,
            0,
            LayoutNode {
                kind: "pane".into(),
                pane_id: Some(pane_id.into()),
                children: Vec::new(),
                rect: LayoutRect::default(),
            },
            None,
            false,
            Some(pane_id.into()),
        )
    }

    #[test]
    fn teardown_never_marks_provider_stopped() {
        let mut host = LiveApplyHost::default();
        let window = HerdrWindow::new(
            "t",
            "t",
            0,
            LayoutNode {
                kind: "pane".into(),
                pane_id: Some("p".into()),
                children: Vec::new(),
                rect: LayoutRect::default(),
            },
            None,
            false,
            Some("p".into()),
        );
        host.apply_session(&[window]).unwrap();
        let result = host.detach();
        assert_eq!(result["server_stopped"], false);
        assert!(host.windows.is_empty());
    }

    #[test]
    fn title_filter_is_the_io_implementation() {
        let mut filter = TitleEscapeFilter::new();
        assert_eq!(filter.filter(b"a\x1bkname\x1b\\b"), b"ab");
    }

    #[test]
    fn input_and_live_pane_order_follow_window_arrival() {
        let mut host = LiveApplyHost::default();
        host.apply_session(&[window("t2", "z"), window("t1", "a")])
            .unwrap();
        host.window_mut("t2").unwrap().send_text("z", "first");
        host.window_mut("t1").unwrap().send_text("a", "second");
        assert_eq!(host.live_pane_ids(), vec!["z", "a"]);
        assert_eq!(
            host.drain_input()
                .into_iter()
                .map(|item| item.pane_id)
                .collect::<Vec<_>>(),
            vec!["z", "a"]
        );
    }

    #[test]
    fn resolved_drag_hold_uses_python_default_split_identity() {
        let mut mirror = LiveWindowMirror::new("t", "t");
        mirror.begin_drag("custom", "vertical", 100);
        let (_, send) = mirror.end_drag(50.0, 200.0, 200, 100);
        assert!(send);
        let hold = mirror.drag_hold.unwrap();
        assert_eq!(
            (hold.split_key.as_str(), hold.axis.as_str()),
            ("s", "horizontal")
        );
    }

    #[test]
    fn native_writer_callback_runs_before_marking_live() {
        let mut host = LiveApplyHost::new(true, "/tmp/herdr.sock", true);
        let mut called = false;
        let mut marker = || called = true;
        host.set_native_live(Some(&mut marker));
        assert!(called);
        assert!(host.native_live);
        assert_eq!(host.log.last().map(String::as_str), Some("native_live"));
    }
}
