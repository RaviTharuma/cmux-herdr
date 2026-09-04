//! Pure Herdr window and session reconciliation.
//!
//! Behavioral port of `bridge/cmux_herdr_engine.py`. Pane lifecycle always
//! follows the base layout; the visible layout is presentation-only zoom state.

use std::collections::{HashMap, HashSet};

use crate::impose::{self, ImposePlan, ReconcileResultLike};
use crate::layout::{split_specs, LayoutNode, SplitSpec};
use crate::session::{self, SessionAction};

#[derive(Debug, Clone, PartialEq)]
pub struct HerdrWindow {
    pub tab_id: String,
    pub title: String,
    pub order_index: i64,
    pub layout: LayoutNode,
    pub visible_layout: Option<LayoutNode>,
    pub zoomed: bool,
    pub active_pane_id: Option<String>,
}

impl HerdrWindow {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tab_id: impl Into<String>,
        title: impl Into<String>,
        order_index: i64,
        layout: LayoutNode,
        visible_layout: Option<LayoutNode>,
        zoomed: bool,
        active_pane_id: Option<String>,
    ) -> Self {
        Self {
            tab_id: tab_id.into(),
            title: title.into(),
            order_index,
            layout,
            visible_layout: if zoomed { visible_layout } else { None },
            zoomed,
            active_pane_id,
        }
    }

    pub fn rendered_layout(&self) -> &LayoutNode {
        self.visible_layout.as_ref().unwrap_or(&self.layout)
    }

    pub fn base_pane_ids(&self) -> Vec<String> {
        self.layout.pane_ids_in_order()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WindowMirrorState {
    pub tab_id: String,
    pub title: String,
    pub layout: LayoutNode,
    pub visible_layout: Option<LayoutNode>,
    pub zoomed: bool,
    pub active_pane_id: Option<String>,
    pub pane_ids: Vec<String>,
    pub layout_structure_version: i64,
    pub surface_id_by_pane_id: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReconcileResult {
    pub created_pane_ids: Vec<String>,
    pub closed_pane_ids: Vec<String>,
    pub kept_pane_ids: Vec<String>,
    pub structure_changed: bool,
    pub title_changed: bool,
    pub focus_pane_id: Option<String>,
    pub split_specs: Vec<SplitSpec>,
    pub rendered_layout: LayoutNode,
}

impl ReconcileResultLike for ReconcileResult {
    fn rendered_layout(&self) -> &LayoutNode {
        &self.rendered_layout
    }

    fn focus_pane_id(&self) -> Option<&str> {
        self.focus_pane_id.as_deref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionReconcile {
    pub created_tab_ids: Vec<String>,
    pub closed_tab_ids: Vec<String>,
    pub kept_tab_ids: Vec<String>,
    pub ordered_tab_ids: Vec<String>,
    pub order_changed: bool,
}

pub fn apply_window(
    window: &HerdrWindow,
    previous: Option<&WindowMirrorState>,
) -> (WindowMirrorState, ReconcileResult) {
    let live = window.base_pane_ids();
    let live_set: HashSet<&str> = live.iter().map(String::as_str).collect();
    let previous_ids = previous.map_or_else(Vec::new, |state| state.pane_ids.clone());
    let previous_set: HashSet<&str> = previous_ids.iter().map(String::as_str).collect();

    let created: Vec<String> = live
        .iter()
        .filter(|pane_id| !previous_set.contains(pane_id.as_str()))
        .cloned()
        .collect();
    let closed: Vec<String> = previous_ids
        .iter()
        .filter(|pane_id| !live_set.contains(pane_id.as_str()))
        .cloned()
        .collect();
    let kept: Vec<String> = live
        .iter()
        .filter(|pane_id| previous_set.contains(pane_id.as_str()))
        .cloned()
        .collect();

    let structure_changed = previous.is_none_or(|state| {
        state.layout.structure_signature() != window.layout.structure_signature()
    });
    let title_changed = previous.is_none_or(|state| state.title != window.title);
    let mut version = previous.map_or(0, |state| state.layout_structure_version);
    if previous.is_some() && structure_changed {
        version += 1;
    }
    let focus = window
        .active_pane_id
        .as_ref()
        .filter(|pane_id| live_set.contains(pane_id.as_str()))
        .cloned()
        .or_else(|| live.first().cloned());

    let mut surfaces = previous
        .map(|state| state.surface_id_by_pane_id.clone())
        .unwrap_or_default();
    for pane_id in &closed {
        surfaces.remove(pane_id);
    }

    let created_set: HashSet<&str> = created.iter().map(String::as_str).collect();
    let specs = split_specs(&window.layout)
        .into_iter()
        .filter(|spec| created_set.contains(spec.pane_id.as_str()))
        .collect();
    let state = WindowMirrorState {
        tab_id: window.tab_id.clone(),
        title: window.title.clone(),
        layout: window.layout.clone(),
        visible_layout: if window.zoomed {
            window.visible_layout.clone()
        } else {
            None
        },
        zoomed: window.zoomed,
        active_pane_id: focus.clone(),
        pane_ids: live,
        layout_structure_version: version,
        surface_id_by_pane_id: surfaces,
    };
    let result = ReconcileResult {
        created_pane_ids: created,
        closed_pane_ids: closed,
        kept_pane_ids: kept,
        structure_changed,
        title_changed,
        focus_pane_id: focus,
        split_specs: specs,
        rendered_layout: window.rendered_layout().clone(),
    };
    (state, result)
}

pub fn bind_surface(state: &mut WindowMirrorState, pane_id: &str, surface_id: &str) {
    state
        .surface_id_by_pane_id
        .insert(pane_id.to_string(), surface_id.to_string());
}

pub fn reconcile_session(
    windows: &[HerdrWindow],
    previous_tab_ids: &[String],
) -> SessionReconcile {
    let mut sorted: Vec<&HerdrWindow> = windows.iter().collect();
    sorted.sort_by(|left, right| {
        left.order_index
            .cmp(&right.order_index)
            .then_with(|| left.tab_id.cmp(&right.tab_id))
    });
    let ordered: Vec<String> = sorted.iter().map(|window| window.tab_id.clone()).collect();
    let desired: HashSet<&str> = ordered.iter().map(String::as_str).collect();
    let previous_set: HashSet<&str> = previous_tab_ids.iter().map(String::as_str).collect();

    let created = ordered
        .iter()
        .filter(|tab_id| !previous_set.contains(tab_id.as_str()))
        .cloned()
        .collect();
    let closed = previous_tab_ids
        .iter()
        .filter(|tab_id| !desired.contains(tab_id.as_str()))
        .cloned()
        .collect();
    let kept: Vec<String> = ordered
        .iter()
        .filter(|tab_id| previous_set.contains(tab_id.as_str()))
        .cloned()
        .collect();
    let previous_live: Vec<&str> = previous_tab_ids
        .iter()
        .map(String::as_str)
        .filter(|tab_id| desired.contains(tab_id))
        .collect();
    let kept_desired: Vec<&str> = kept.iter().map(String::as_str).collect();
    let order_changed = previous_live != kept_desired;

    SessionReconcile {
        created_tab_ids: created,
        closed_tab_ids: closed,
        kept_tab_ids: kept,
        ordered_tab_ids: ordered,
        order_changed,
    }
}

pub fn client_grid(
    content_width: f64,
    content_height: f64,
    cell_width: f64,
    cell_height: f64,
    chrome_width: f64,
    chrome_height: f64,
) -> Option<(i64, i64)> {
    if cell_width <= 0.0 || cell_height <= 0.0 {
        return None;
    }
    let available_width = content_width - chrome_width;
    let available_height = content_height - chrome_height;
    if available_width <= 0.0 || available_height <= 0.0 {
        return None;
    }
    let cols = (available_width / cell_width) as i64;
    let rows = (available_height / cell_height) as i64;
    (cols >= 1 && rows >= 1).then_some((cols, rows))
}

fn py_round(value: f64) -> i64 {
    let floor = value.floor();
    let fraction = value - floor;
    if fraction < 0.5 {
        floor as i64
    } else if fraction > 0.5 {
        floor as i64 + 1
    } else {
        let lower = floor as i64;
        if lower % 2 == 0 { lower } else { lower + 1 }
    }
}

pub fn resize_cells(dragged_extent: f64, axis_span: f64, total_cells: i64) -> i64 {
    if axis_span <= 0.0 || total_cells < 1 {
        return 1;
    }
    let fraction = (dragged_extent / axis_span).clamp(0.05, 0.95);
    let cells = py_round(fraction * total_cells as f64);
    cells.max(1).min((total_cells - 1).max(1))
}

pub fn output_delta(previous: Option<&str>, current: &str) -> (String, bool) {
    let Some(previous) = previous else {
        return (current.to_string(), true);
    };
    if current == previous {
        return (String::new(), false);
    }
    if let Some(suffix) = current.strip_prefix(previous) {
        return (suffix.to_string(), false);
    }
    (current.to_string(), true)
}

pub fn impose_after_apply(
    result: &ReconcileResult,
    previous_rendered: Option<&LayoutNode>,
    title: &str,
) -> ImposePlan {
    impose::plan_from_reconcile(result, previous_rendered, title, None, None, None, None)
}

pub fn session_host_actions(
    reconcile: &SessionReconcile,
    titles: Option<&HashMap<String, String>>,
    previous_titles: Option<&HashMap<String, String>>,
    defaults_open: bool,
    focus_tab_id: Option<&str>,
) -> Vec<SessionAction> {
    session::session_actions(
        reconcile,
        titles,
        previous_titles,
        defaults_open,
        focus_tab_id,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{LayoutNode, LayoutRect};

    fn leaf(id: &str) -> LayoutNode {
        LayoutNode {
            kind: "pane".into(),
            pane_id: Some(id.into()),
            children: Vec::new(),
            rect: LayoutRect::default(),
        }
    }

    #[test]
    fn non_zoom_window_drops_visible_layout() {
        let window = HerdrWindow::new("t", "t", 0, leaf("base"), Some(leaf("visible")), false, None);
        assert!(window.visible_layout.is_none());
        assert_eq!(window.rendered_layout().pane_id.as_deref(), Some("base"));
    }
}
