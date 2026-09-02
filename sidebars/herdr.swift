// cmux-herdr plugin sidebar (`herdr`).
// Restricted Swift subset (cmux docs/custom-sidebars.md). Root is a view
// expression, not a struct. Bind only live cmux context: workspaces, tabs,
// statuses, agents, progress, git, color. No invented team or fake rows.
// Chrome uses Ghostty/cmux theme tokens so dark/light follow the host.

func hasText(_ value) -> Bool {
  return value != nil && value != ""
}

func hasColor(_ w) -> Bool {
  return hasText(w.color)
}

func hasBranch(_ w) -> Bool {
  return hasText(w.branch)
}

func hasProgress(_ w) -> Bool {
  return w.progress != nil && w.progress.value != nil
}

func hasStatuses(_ w) -> Bool {
  return w.statuses != nil && w.statuses.count > 0
}

func hasAgents(_ w) -> Bool {
  return w.agents != nil && w.agents.count > 0
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

func rowTint(_ w) -> String {
  if hasColor(w) { return w.color }
  if w.selected { return "accent" }
  if w.unread > 0 { return "accent" }
  return "secondary"
}

func wsIcon(_ w) -> String {
  let t = w.title.lowercased()
  if t.contains("herdr") { return "rectangle.split.3x3" }
  if w.pinned { return "pin.fill" }
  if w.remote != nil && w.remote.connected == true { return "network" }
  return "folder.fill"
}

func statusTint(_ s) -> String {
  if hasText(s.color) { return s.color }
  return "accent"
}

func statusIcon(_ s) -> String {
  if hasText(s.icon) { return s.icon }
  if hasText(s.key) && s.key.hasPrefix("herdr:") { return "rectangle.split.3x3" }
  return "circle.fill"
}

func statusLabel(_ s) -> String {
  if hasText(s.value) { return s.value }
  if hasText(s.key) { return s.key }
  return ""
}

func agentTint(_ a) -> String {
  if a.status == "working" { return "accent" }
  if a.status == "needs_input" { return "red" }
  if a.status == "ended" { return "tertiary" }
  return "secondary"
}

func agentIcon(_ a) -> String {
  if a.status == "working" { return "hammer.fill" }
  if a.status == "needs_input" { return "exclamationmark.triangle.fill" }
  if a.status == "ended" { return "checkmark.circle" }
  return "pause.circle"
}

func tabFocusId(_ t) -> String {
  if hasText(t.surfaceId) { return t.surfaceId }
  return t.id
}

func herdrStatusChip(_ s) -> some View {
  HStack(spacing: 4) {
    Image(systemName: statusIcon(s))
      .font(.system(size: 8))
      .foregroundColor(statusTint(s))
      .symbolRenderingMode(.hierarchical)
    Text(statusLabel(s))
      .font(.system(size: 10, design: .monospaced))
      .foregroundColor(hasText(s.color) ? s.color : "secondary")
      .lineLimit(1)
  }
  .padding(.horizontal, 5)
  .padding(.vertical, 2)
  .background {
    Capsule().foregroundColor(statusTint(s)).opacity(0.16)
  }
}

func herdrAgentChip(_ a) -> some View {
  HStack(spacing: 4) {
    Image(systemName: agentIcon(a))
      .font(.system(size: 8))
      .foregroundColor(agentTint(a))
      .symbolRenderingMode(.hierarchical)
    Text(a.name)
      .font(.system(size: 10))
      .foregroundColor(agentTint(a))
      .lineLimit(1)
  }
}

func herdrWorkspaceRow(_ w) -> some View {
  Button(action: { cmux("workspace.select", workspace_id: w.id) }) {
    HStack(alignment: .top, spacing: 8) {
      Circle()
        .frame(width: 7, height: 7)
        .foregroundColor(w.selected ? rowTint(w) : "tertiary")
        .padding(.top, 5)
      VStack(alignment: .leading, spacing: 3) {
        HStack(spacing: 6) {
          Image(systemName: wsIcon(w))
            .font(.system(size: 11))
            .foregroundColor(rowTint(w))
            .symbolRenderingMode(.hierarchical)
            .frame(width: 14)
          Text(w.title)
            .font(.system(size: 12))
            .fontWeight(w.selected ? .semibold : .regular)
            .foregroundColor("primary")
            .lineLimit(1)
          Spacer()
          if w.unread > 0 {
            Text("\(w.unread)")
              .font(.system(size: 9, design: .monospaced))
              .fontWeight(.semibold)
              .foregroundColor("accent")
          }
        }
        HStack(spacing: 6) {
          Text("\(w.tabCount) tabs")
            .font(.system(size: 10, design: .monospaced))
            .foregroundColor("secondary")
          if hasBranch(w) {
            Text(w.branch)
              .font(.system(size: 10, design: .monospaced))
              .foregroundColor("tertiary")
              .lineLimit(1)
          }
          if w.dirty == true {
            Text("*")
              .font(.system(size: 10, design: .monospaced))
              .foregroundColor("accent")
          }
          if hasProgress(w) {
            Text(progressLabel(w))
              .font(.system(size: 10, design: .monospaced))
              .foregroundColor("accent")
          }
        }
        if hasProgress(w) && w.progress.value < 1.0 {
          ProgressView(value: w.progress.value, total: 1.0).tint(rowTint(w))
        }
        if hasStatuses(w) {
          HStack(spacing: 4) {
            ForEach(w.statuses.prefix(6)) { s in
              herdrStatusChip(s)
            }
          }
        }
      }
    }
    .padding(.vertical, 5)
    .padding(.horizontal, 6)
    .background {
      RoundedRectangle(cornerRadius: 6)
        .foregroundColor(w.selected ? "accent" : "primary")
        .opacity(w.selected ? 0.14 : 0.04)
    }
  }
}

func herdrTabRow(_ t) -> some View {
  Button(action: { cmux("surface.focus", surface_id: tabFocusId(t)) }) {
    HStack(spacing: 6) {
      Image(systemName: t.focused ? "dot.circle.fill" : "terminal")
        .font(.system(size: 10))
        .foregroundColor(t.focused ? "accent" : "secondary")
        .frame(width: 14)
      Text(t.title)
        .font(.system(size: 11))
        .foregroundColor(t.focused ? "primary" : "secondary")
        .lineLimit(1)
      Spacer()
    }
    .padding(.vertical, 3)
    .padding(.horizontal, 6)
    .background {
      RoundedRectangle(cornerRadius: 4)
        .foregroundColor("primary")
        .opacity(0.04)
    }
  }
}

func selectedWorkspaceDetail(_ w) -> some View {
  VStack(alignment: .leading, spacing: 6) {
    Text("Focused workspace")
      .font(.caption)
      .fontWeight(.semibold)
      .foregroundColor("secondary")
    Text(w.title)
      .font(.system(size: 12))
      .fontWeight(.semibold)
      .foregroundColor("primary")
      .lineLimit(1)
    if hasStatuses(w) {
      Text("Live statuses")
        .font(.system(size: 10))
        .fontWeight(.semibold)
        .foregroundColor("tertiary")
      ForEach(w.statuses.prefix(12)) { s in
        herdrStatusChip(s)
      }
    }
    if hasAgents(w) {
      Text("Live agents")
        .font(.system(size: 10))
        .fontWeight(.semibold)
        .foregroundColor("tertiary")
      ForEach(w.agents.prefix(8)) { a in
        herdrAgentChip(a)
      }
    }
    if hasTabs(w) {
      Text("Surfaces")
        .font(.system(size: 10))
        .fontWeight(.semibold)
        .foregroundColor("tertiary")
      ForEach(w.tabs.prefix(12)) { t in
        herdrTabRow(t)
      }
    }
  }
}

ScrollView {
  VStack(alignment: .leading, spacing: 10) {
    HStack(spacing: 8) {
      Image(systemName: "rectangle.split.3x3")
        .foregroundColor("accent")
        .symbolRenderingMode(.hierarchical)
      Text("Herdr")
        .font(.headline)
        .fontWeight(.semibold)
        .foregroundColor("primary")
      Spacer()
      Text(clock.time)
        .font(.system(size: 10, design: .monospaced))
        .foregroundColor("tertiary")
    }
    if hasText(selectedTitle) {
      Text(selectedTitle)
        .font(.system(size: 11, design: .monospaced))
        .foregroundColor("secondary")
        .lineLimit(1)
    }

    Divider()

    VStack(alignment: .leading, spacing: 6) {
      HStack {
        Text("cmux workspaces")
          .font(.caption)
          .fontWeight(.semibold)
          .foregroundColor("secondary")
        Spacer()
        Text("\(workspaceCount)")
          .font(.system(size: 10, design: .monospaced))
          .foregroundColor("tertiary")
      }
      if workspaces.count == 0 {
        Text("No live cmux workspaces")
          .font(.caption)
          .foregroundColor("tertiary")
      }
      if workspaces.count > 0 {
        Reorderable(workspaces.prefix(40), move: "workspace.reorder") { w in
          herdrWorkspaceRow(w)
        }
      }
    }

    Divider()

    ForEach(workspaces.filter { $0.selected }.prefix(1)) { w in
      selectedWorkspaceDetail(w)
    }

    Divider()

    Text("Status pills update after cmux-herdr sync or watch.")
      .font(.caption)
      .foregroundColor("tertiary")
      .lineLimit(3)
  }
  .padding(12)
}
