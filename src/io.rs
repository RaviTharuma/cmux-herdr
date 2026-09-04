//! Pane-isolated output, input, focus, and cwd routing.
//!
//! Port of `bridge/cmux_herdr_io.py`. The screen-title filter is deliberately
//! stateful because `ESC k ... ESC \\` may be split at any byte boundary.

use std::collections::{HashMap, HashSet};

use crate::engine::output_delta;

const ESC: u8 = 0x1b;
const K: u8 = b'k';
const ST_SLASH: u8 = b'\\';

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum FilterState {
    #[default]
    Text,
    Esc,
    Title,
    TitleEsc,
}

#[derive(Debug, Clone, Default)]
pub struct TitleEscapeFilter {
    state: FilterState,
}

impl TitleEscapeFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.state = FilterState::Text;
    }

    pub fn filter(&mut self, data: &[u8]) -> Vec<u8> {
        if data.is_empty() {
            return Vec::new();
        }
        if self.state == FilterState::Text && !data.contains(&ESC) {
            return data.to_vec();
        }
        let mut out = Vec::with_capacity(data.len());
        let mut state = self.state;
        for byte in data.iter().copied() {
            match state {
                FilterState::Text => {
                    if byte == ESC {
                        state = FilterState::Esc;
                    } else {
                        out.push(byte);
                    }
                }
                FilterState::Esc => {
                    if byte == K {
                        state = FilterState::Title;
                    } else if byte == ESC {
                        out.push(ESC);
                    } else {
                        out.push(ESC);
                        out.push(byte);
                        state = FilterState::Text;
                    }
                }
                FilterState::Title => {
                    if byte == ESC {
                        state = FilterState::TitleEsc;
                    }
                }
                FilterState::TitleEsc => {
                    if byte == ST_SLASH {
                        state = FilterState::Text;
                    } else if byte != ESC {
                        state = FilterState::Title;
                    }
                }
            }
        }
        self.state = state;
        out
    }

    pub fn filter_text(&mut self, text: &str) -> String {
        String::from_utf8(self.filter(text.as_bytes()))
            .expect("filtering valid UTF-8 can only remove complete ASCII-delimited ranges")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceWrite {
    pub pane_id: String,
    pub surface_id: String,
    pub data: Vec<u8>,
    pub full_redraw: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputSend {
    pub pane_id: String,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FocusProjection {
    pub pane_id: Option<String>,
    pub send_to_provider: bool,
    pub changed: bool,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CwdUpdate {
    pub pane_id: String,
    pub tab_id: String,
    pub path: String,
    pub apply_to_tab: bool,
}

#[derive(Debug, Clone, Default)]
pub struct PaneIORouter {
    pub surfaces: HashMap<String, String>,
    pub buffers: HashMap<String, Vec<u8>>,
    pub last_snapshot: HashMap<String, String>,
    pub title_filters: HashMap<String, TitleEscapeFilter>,
    pub live_pane_ids: HashSet<String>,
    pub cwd_by_pane: HashMap<String, String>,
    pub active_pane_id: Option<String>,
    pub pending_user_focus: Option<String>,
    pub log: Vec<String>,
}

impl PaneIORouter {
    pub fn bind(&mut self, pane_id: &str, surface_id: &str) {
        self.surfaces.insert(pane_id.into(), surface_id.into());
        self.buffers.entry(surface_id.into()).or_default();
        self.live_pane_ids.insert(pane_id.into());
        self.title_filters.entry(pane_id.into()).or_default();
        self.log.push(format!("bind:{pane_id}:{surface_id}"));
    }

    pub fn unbind(&mut self, pane_id: &str) {
        self.surfaces.remove(pane_id);
        self.last_snapshot.remove(pane_id);
        self.title_filters.remove(pane_id);
        self.cwd_by_pane.remove(pane_id);
        self.live_pane_ids.remove(pane_id);
        if self.active_pane_id.as_deref() == Some(pane_id) {
            self.active_pane_id = None;
        }
        if self.pending_user_focus.as_deref() == Some(pane_id) {
            self.pending_user_focus = None;
        }
        self.log.push(format!("unbind:{pane_id}"));
    }

    pub fn set_live_panes(&mut self, pane_ids: &[String]) {
        self.live_pane_ids = pane_ids.iter().cloned().collect();
    }

    pub fn route_output(&mut self, pane_id: &str, data: &[u8]) -> Option<SurfaceWrite> {
        let surface_id = self.surfaces.get(pane_id)?.clone();
        if data.is_empty() {
            return None;
        }
        let cleaned = self.title_filters.entry(pane_id.into()).or_default().filter(data);
        if cleaned.is_empty() {
            return None;
        }
        self.buffers.entry(surface_id.clone()).or_default().extend_from_slice(&cleaned);
        self.log.push(format!("out:{pane_id}:{}", cleaned.len()));
        Some(SurfaceWrite {
            pane_id: pane_id.into(),
            surface_id,
            data: cleaned,
            full_redraw: false,
        })
    }

    pub fn route_output_text(&mut self, pane_id: &str, current: &str) -> Option<SurfaceWrite> {
        let surface_id = self.surfaces.get(pane_id)?.clone();
        let (chunk, full_redraw) = output_delta(
            self.last_snapshot.get(pane_id).map(String::as_str),
            current,
        );
        self.last_snapshot.insert(pane_id.into(), current.into());
        if chunk.is_empty() && !full_redraw {
            return None;
        }
        let filter = self.title_filters.entry(pane_id.into()).or_default();
        if full_redraw {
            filter.reset();
        }
        let encoded = filter.filter(chunk.as_bytes());
        if full_redraw {
            self.buffers.insert(surface_id.clone(), encoded.clone());
        } else {
            if encoded.is_empty() {
                return None;
            }
            self.buffers.entry(surface_id.clone()).or_default().extend_from_slice(&encoded);
        }
        self.log.push(format!(
            "out-text:{pane_id}:redraw={}:{}",
            i32::from(full_redraw),
            encoded.len()
        ));
        Some(SurfaceWrite {
            pane_id: pane_id.into(),
            surface_id,
            data: encoded,
            full_redraw,
        })
    }

    pub fn route_input(&mut self, pane_id: &str, data: &[u8]) -> Option<InputSend> {
        if data.is_empty() || !self.surfaces.contains_key(pane_id) {
            return None;
        }
        self.log.push(format!("in:{pane_id}:{}", data.len()));
        Some(InputSend { pane_id: pane_id.into(), data: data.to_vec() })
    }

    pub fn route_input_to_focus(&mut self, data: &[u8]) -> Option<InputSend> {
        let pane_id = self.active_pane_id.clone()?;
        self.route_input(&pane_id, data)
    }

    pub fn note_remote_active(&mut self, pane_id: &str) -> FocusProjection {
        let changed = self.active_pane_id.as_deref() != Some(pane_id);
        self.active_pane_id = Some(pane_id.into());
        if self.pending_user_focus.as_deref() == Some(pane_id) {
            self.pending_user_focus = None;
        }
        self.log.push(format!("focus-provider:{pane_id}"));
        FocusProjection {
            pane_id: Some(pane_id.into()),
            send_to_provider: false,
            changed,
            source: "provider".into(),
        }
    }

    pub fn user_focus(&mut self, pane_id: &str) -> FocusProjection {
        if !self.live_pane_ids.contains(pane_id) && !self.surfaces.contains_key(pane_id) {
            self.log.push(format!("focus-user-unknown:{pane_id}"));
            return FocusProjection {
                pane_id: None,
                send_to_provider: false,
                changed: false,
                source: "user".into(),
            };
        }
        let changed = self.active_pane_id.as_deref() != Some(pane_id);
        self.active_pane_id = Some(pane_id.into());
        if self.pending_user_focus.as_deref() == Some(pane_id) {
            self.log.push(format!("focus-user-pending:{pane_id}"));
            return FocusProjection {
                pane_id: Some(pane_id.into()),
                send_to_provider: false,
                changed,
                source: "user".into(),
            };
        }
        self.pending_user_focus = Some(pane_id.into());
        self.log.push(format!("focus-user-send:{pane_id}"));
        FocusProjection {
            pane_id: Some(pane_id.into()),
            send_to_provider: true,
            changed,
            source: "user".into(),
        }
    }

    pub fn project_focus(&mut self, pane_id: &str, from_provider: bool) -> FocusProjection {
        if from_provider {
            self.note_remote_active(pane_id)
        } else {
            self.user_focus(pane_id)
        }
    }

    pub fn route_cwd(&mut self, pane_id: &str, path: &str, tab_id: &str) -> Option<CwdUpdate> {
        let trimmed = path.trim();
        if trimmed.is_empty() {
            return None;
        }
        self.cwd_by_pane.insert(pane_id.into(), trimmed.into());
        let apply_to_tab = self.active_pane_id.as_deref() == Some(pane_id);
        self.log.push(format!("cwd:{pane_id}:tab={}", i32::from(apply_to_tab)));
        Some(CwdUpdate {
            pane_id: pane_id.into(),
            tab_id: tab_id.into(),
            path: trimmed.into(),
            apply_to_tab,
        })
    }

    pub fn buffer_for(&self, pane_id: &str) -> &[u8] {
        self.surfaces
            .get(pane_id)
            .and_then(|surface_id| self.buffers.get(surface_id))
            .map(Vec::as_slice)
            .unwrap_or_default()
    }
}

pub fn route_output(router: &mut PaneIORouter, pane_id: &str, data: &[u8]) -> Option<SurfaceWrite> {
    router.route_output(pane_id, data)
}

pub fn route_input(router: &mut PaneIORouter, pane_id: &str, data: &[u8]) -> Option<InputSend> {
    router.route_input(pane_id, data)
}

pub fn project_focus(
    router: &mut PaneIORouter,
    pane_id: &str,
    from_provider: bool,
) -> FocusProjection {
    router.project_focus(pane_id, from_provider)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_escape_can_span_chunks() {
        let mut filter = TitleEscapeFilter::new();
        assert_eq!(filter.filter(b"ab\x1bkti"), b"ab");
        assert_eq!(filter.filter(b"tle\x1b\\cd"), b"cd");
    }

    #[test]
    fn full_redraw_replaces_the_bound_surface_buffer() {
        let mut router = PaneIORouter::default();
        router.bind("p", "s");
        router.route_output_text("p", "hello");
        router.route_output_text("p", "goodbye");
        assert_eq!(router.buffer_for("p"), b"goodbye");
    }
}
