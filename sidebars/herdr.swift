// Herdr sidebar for cmux (Swift fallback; herdr.js wins for live drag).
// Native cmux chrome: Reorderable, context menus, accent/primary/secondary/
// tertiary tokens. Herdr is the product name. This is not an iframe, not a
// bridge panel, and not a CLI cheat-sheet. Click a row to select a workspace
// or focus a surface. Restricted Swift subset (cmux docs/custom-sidebars.md).

func hasText(_ value) -> Bool {
  return value != nil && value != ""
}

func hasProgress(_ w) -> Bool {
  return w.progress != nil && w.progress.value != nil
}

func hasStatuses(_ w) -> Bool {
  return w.statuses != nil && w.statuses.count > 0
}

func hasTabs(_ w) -> Bool {
  return w.tabs != nil && w.tabs.count > 0
}

func progressLabel(_ w) -> String {
  if hasProgress(w) && hasText(w.progress.label) {
    return w.progress.label
  }
  if hasProgress(w) {
    let pct = Int(w.progress.value * 100)
    return "\(pct)%"
  }
  return ""
}

func statusLabel(_ s) -> String {
  if hasText(s.value) { return s.value }
  if hasText(s.key) && !s.key.contains(":") { return s.key }
  return ""
}

func statusTint(_ s) -> String {
  if hasText(s.color) { return s.color }
  let label = statusLabel(s).lowercased()
  if label == "working" { return "accent" }
  if label == "blocked" || label == "needs_input" { return "red" }
  if label == "done" || label == "ended" { return "tertiary" }
  return "secondary"
}

func statusIcon(_ s) -> String {
  if hasText(s.icon) { return s.icon }
  let label = statusLabel(s).lowercased()
  if label == "working" { return "hammer.fill" }
  if label == "blocked" || label == "needs_input" { return "exclamationmark.triangle.fill" }
  if label == "done" || label == "ended" { return "checkmark.circle" }
  if label == "idle" { return "pause.circle" }
  return "circle.fill"
}

func tabFocusId(_ t) -> String {
  if hasText(t.surfaceId) { return t.surfaceId }
  return t.id
}

func statusChip(_ s) -> some View {
  HStack(spacing: 4) {
    Image(systemName: statusIcon(s))
      .font(.system(size: 8))
      .foregroundColor(statusTint(s))
      .symbolRenderingMode(.hierarchical)
    if statusLabel(s) != "" {
      Text(statusLabel(s))
        .font(.system(size: 10))
        .foregroundColor(statusTint(s))
        .lineLimit(1)
    }
  }
  .padding(.horizontal, 5)
  .padding(.vertical, 2)
  .background {
    Capsule().foregroundColor(statusTint(s)).opacity(0.16)
  }
}

func workspaceMenu(_ w) -> some View {
  Group {
    Button(w.pinned ? "Unpin" : "Pin") {
      cmux("workspace.action", action: w.pinned ? "unpin" : "pin", workspace_id: w.id)
    }
    Button(w.unread > 0 ? "Mark as Read" : "Mark as Unread") {
      cmux("workspace.action", action: w.unread > 0 ? "mark_read" : "mark_unread", workspace_id: w.id)
    }
    Divider()
    Button("Move Up") { cmux("workspace.action", action: "move_up", workspace_id: w.id) }
    Button("Move Down") { cmux("workspace.action", action: "move_down", workspace_id: w.id) }
    Button("Move to Top") { cmux("workspace.action", action: "move_top", workspace_id: w.id) }
    Divider()
    Button("Close Others") { cmux("workspace.action", action: "close_others", workspace_id: w.id) }
    Button("Close") { cmux("workspace.close", workspace_id: w.id) }
  }
}

func workspaceRow(_ w) -> some View {
  VStack(alignment: .leading, spacing: 3) {
    HStack(spacing: 8) {
      Circle()
        .frame(width: 7, height: 7)
        .foregroundColor(w.selected ? "accent" : "tertiary")
      Text(w.title)
        .font(.system(size: 13))
        .fontWeight(w.selected ? .semibold : .regular)
        .foregroundColor(w.selected ? "primary" : "secondary")
        .lineLimit(1)
      Spacer()
      if w.unread > 0 {
        Text("\(w.unread)")
          .font(.system(size: 9))
          .fontWeight(.semibold)
          .foregroundColor("accent")
      }
    }
    HStack(spacing: 6) {
      Text("\(w.tabCount)")
        .font(.system(size: 10))
        .foregroundColor("tertiary")
      if hasText(w.branch) {
        Text(w.branch)
          .font(.system(size: 10))
          .foregroundColor("tertiary")
          .lineLimit(1)
      }
      if hasProgress(w) && progressLabel(w) != "" {
        Text(progressLabel(w))
          .font(.system(size: 10))
          .foregroundColor("accent")
      }
    }
    if hasStatuses(w) {
      HStack(spacing: 4) {
        ForEach(w.statuses.prefix(6)) { s in
          if statusLabel(s) != "" {
            statusChip(s)
          }
        }
      }
    }
  }
  .padding(.horizontal, 10)
  .padding(.vertical, 6)
  .background {
    RoundedRectangle(cornerRadius: 8)
      .foregroundColor(w.selected ? "accent" : "primary")
      .opacity(w.selected ? 0.14 : 0.0)
  }
  .onTapGesture { cmux("workspace.select", workspace_id: w.id) }
  .contextMenu { workspaceMenu(w) }
}

func tabRow(_ t) -> some View {
  HStack(spacing: 6) {
    Image(systemName: t.focused ? "dot.circle.fill" : "terminal")
      .font(.system(size: 10))
      .foregroundColor(t.focused ? "accent" : "secondary")
    Text(t.title)
      .font(.system(size: 12))
      .foregroundColor(t.focused ? "primary" : "secondary")
      .lineLimit(1)
    Spacer()
  }
  .padding(.horizontal, 10)
  .padding(.vertical, 5)
  .padding(.leading, 14)
  .onTapGesture { cmux("surface.focus", surface_id: tabFocusId(t)) }
}

ScrollView {
  VStack(alignment: .leading, spacing: 4) {
    HStack {
      Text("Herdr")
        .font(.system(size: 13))
        .fontWeight(.semibold)
        .foregroundColor("primary")
      Spacer()
      Text("\(workspaceCount)")
        .font(.system(size: 10))
        .foregroundColor("tertiary")
    }
    .padding(.horizontal, 10)
    .padding(.vertical, 6)

    if workspaces.count == 0 {
      Text("No workspaces")
        .font(.caption)
        .foregroundColor("tertiary")
        .padding(.horizontal, 10)
    }

    if workspaces.count > 0 {
      Reorderable(workspaces.prefix(40), move: "workspace.reorder") { w in
        workspaceRow(w)
      }
    }

    ForEach(workspaces.filter { $0.selected }.prefix(1)) { w in
      if hasTabs(w) {
        ForEach(w.tabs.prefix(12)) { t in
          tabRow(t)
        }
      }
    }
  }
  .padding(8)
}
