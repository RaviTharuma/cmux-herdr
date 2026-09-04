//! Ordered session-tab host verbs.
//!
//! Port of `bridge/cmux_herdr_session.py`. Vector order is part of the public
//! contract: create, rename, close, close defaults, reorder, then focus.

use std::collections::HashMap;

use crate::engine::SessionReconcile;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionAction {
    pub op: String,
    pub tab_id: Option<String>,
    pub title: Option<String>,
    pub ordered_tab_ids: Option<Vec<String>>,
}

impl SessionAction {
    pub fn new(op: &str) -> Self {
        Self {
            op: op.to_string(),
            tab_id: None,
            title: None,
            ordered_tab_ids: None,
        }
    }
}

pub fn session_actions(
    session: &SessionReconcile,
    titles: Option<&HashMap<String, String>>,
    previous_titles: Option<&HashMap<String, String>>,
    defaults_open: bool,
    focus_tab_id: Option<&str>,
) -> Vec<SessionAction> {
    let empty = HashMap::new();
    let current_titles = titles.unwrap_or(&empty);
    let prior_titles = previous_titles.unwrap_or(&empty);
    let mut actions = Vec::new();

    for tab_id in &session.created_tab_ids {
        let mut action = SessionAction::new("create_tab");
        action.tab_id = Some(tab_id.clone());
        action.title = Some(
            current_titles
                .get(tab_id)
                .cloned()
                .unwrap_or_else(|| tab_id.clone()),
        );
        actions.push(action);
    }
    for tab_id in &session.kept_tab_ids {
        let new_title = current_titles.get(tab_id).filter(|title| !title.is_empty());
        if new_title.is_some() && new_title != prior_titles.get(tab_id) {
            let mut action = SessionAction::new("rename_tab");
            action.tab_id = Some(tab_id.clone());
            action.title = new_title.cloned();
            actions.push(action);
        }
    }
    for tab_id in &session.closed_tab_ids {
        let mut action = SessionAction::new("close_tab");
        action.tab_id = Some(tab_id.clone());
        actions.push(action);
    }
    if defaults_open && !session.ordered_tab_ids.is_empty() {
        actions.push(SessionAction::new("close_default_tabs"));
    }
    if session.order_changed && session.ordered_tab_ids.len() > 1 {
        let mut action = SessionAction::new("reorder_tabs");
        action.ordered_tab_ids = Some(session.ordered_tab_ids.clone());
        actions.push(action);
    }
    if let Some(tab_id) =
        focus_tab_id.filter(|id| session.ordered_tab_ids.iter().any(|item| item == id))
    {
        let mut action = SessionAction::new("focus_tab");
        action.tab_id = Some(tab_id.to_string());
        actions.push(action);
    }
    actions
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FakeSessionHost {
    pub tabs: Vec<String>,
    pub titles: HashMap<String, String>,
    pub mirrors: HashMap<String, bool>,
    pub default_tabs: Vec<String>,
    pub defaults_closed: bool,
    pub focus: Option<String>,
    pub log: Vec<String>,
}

impl Default for FakeSessionHost {
    fn default() -> Self {
        Self {
            tabs: Vec::new(),
            titles: HashMap::new(),
            mirrors: HashMap::new(),
            default_tabs: vec!["default".into()],
            defaults_closed: false,
            focus: None,
            log: Vec::new(),
        }
    }
}

impl FakeSessionHost {
    pub fn apply(&mut self, actions: &[SessionAction]) -> Result<(), String> {
        for action in actions {
            self.apply_one(action)?;
        }
        Ok(())
    }

    fn apply_one(&mut self, action: &SessionAction) -> Result<(), String> {
        match action.op.as_str() {
            "create_tab" => {
                let tab_id = action
                    .tab_id
                    .as_deref()
                    .filter(|id| !id.is_empty())
                    .ok_or_else(|| "create_tab requires tab_id".to_string())?;
                if !self.tabs.iter().any(|item| item == tab_id) {
                    self.tabs.push(tab_id.to_string());
                }
                self.titles.insert(
                    tab_id.to_string(),
                    action
                        .title
                        .as_ref()
                        .filter(|title| !title.is_empty())
                        .cloned()
                        .unwrap_or_else(|| tab_id.to_string()),
                );
                self.mirrors.insert(tab_id.to_string(), true);
                self.log.push(format!("create:{tab_id}"));
            }
            "rename_tab" => {
                if let (Some(tab_id), Some(title)) = (
                    action
                        .tab_id
                        .as_deref()
                        .filter(|id| self.tabs.iter().any(|item| item == id)),
                    action.title.as_deref().filter(|title| !title.is_empty()),
                ) {
                    self.titles.insert(tab_id.to_string(), title.to_string());
                    self.log.push(format!("rename:{tab_id}"));
                }
            }
            "close_tab" => {
                if let Some(tab_id) = action
                    .tab_id
                    .as_deref()
                    .filter(|id| self.tabs.iter().any(|item| item == id))
                {
                    self.tabs.retain(|item| item != tab_id);
                    self.titles.remove(tab_id);
                    self.mirrors.remove(tab_id);
                    if self.focus.as_deref() == Some(tab_id) {
                        self.focus = None;
                    }
                    self.log.push(format!("close:{tab_id}"));
                }
            }
            "close_default_tabs" => {
                if self.defaults_closed || self.tabs.is_empty() {
                    return Ok(());
                }
                for tab_id in &self.default_tabs {
                    if !self.tabs.iter().any(|item| item == tab_id) {
                        self.log.push(format!("close-default:{tab_id}"));
                    }
                }
                self.default_tabs.clear();
                self.defaults_closed = true;
            }
            "reorder_tabs" => {
                let desired: Vec<String> = action
                    .ordered_tab_ids
                    .as_deref()
                    .unwrap_or_default()
                    .iter()
                    .filter(|tab_id| self.tabs.contains(tab_id))
                    .cloned()
                    .collect();
                let extras: Vec<String> = self
                    .tabs
                    .iter()
                    .filter(|tab_id| !desired.contains(tab_id))
                    .cloned()
                    .collect();
                self.tabs = desired.into_iter().chain(extras).collect();
                self.log.push(format!("reorder:{}", self.tabs.join(",")));
            }
            "focus_tab" => {
                if let Some(tab_id) = action.tab_id.as_deref() {
                    if !self.tabs.iter().any(|item| item == tab_id) {
                        return Err(format!("focus_tab missing tab {tab_id}"));
                    }
                }
                self.focus = action.tab_id.clone();
                self.log.push(format!(
                    "focus:{}",
                    action.tab_id.as_deref().unwrap_or("None")
                ));
            }
            other => return Err(format!("unknown session op {other}")),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_action_fails_closed() {
        let mut host = FakeSessionHost::default();
        assert!(host.apply(&[SessionAction::new("explode")]).is_err());
    }
}
