// Herdr bridge sidebar — best-effort custom sidebar for cmux.
// Live herdr pane data is mirrored via `cmux-herdr watch` into workspace
// status pills; this sidebar explains dual hierarchy and navigates cmux
// workspaces. Restricted Swift subset (see cmux docs/custom-sidebars.md).
// Root must be a top-level View expression (not a struct), matching Examples/.

func shortId(_ id) -> String {
  if id.count <= 8 { return id }
  return String(id.prefix(8))
}

func wsTint(_ w) -> String {
  if w.selected { return "#0A84FF" }
  if w.unread > 0 { return "#FF9F0A" }
  return "#8E8E93"
}

func wsIcon(_ w) -> String {
  let t = w.title.lowercased()
  if t.contains("herdr") { return "rectangle.split.3x3" }
  if w.pinned { return "pin.fill" }
  if w.remote != nil && w.remote.connected == true { return "network" }
  return "folder.fill"
}

func progressLabel(_ w) -> String {
  if w.progress != nil && w.progress.label != nil && w.progress.label != "" {
    return w.progress.label
  }
  if w.progress != nil && w.progress.value != nil {
    let pct = Int(w.progress.value * 100)
    return "\(pct)%"
  }
  return ""
}

func hasProgress(_ w) -> Bool {
  return w.progress != nil && w.progress.value != nil
}

func herdrWorkspaceRow(_ w) -> some View {
  Button(action: { cmux("workspace.select", workspace_id: w.id) }) {
    HStack(spacing: 8) {
      Image(systemName: wsIcon(w))
        .font(.system(size: 12))
        .foregroundColor(wsTint(w))
        .frame(width: 16)
      VStack(alignment: .leading, spacing: 2) {
        Text(w.title)
          .font(.system(size: 12))
          .fontWeight(w.selected ? .semibold : .regular)
          .lineLimit(1)
        HStack(spacing: 6) {
          Text("\(w.tabCount) tabs")
            .font(.system(size: 10))
            .foregroundColor(.secondary)
          if hasProgress(w) {
            Text(progressLabel(w))
              .font(.system(size: 10))
              .foregroundColor("#FF9F0A")
          }
          if w.unread > 0 {
            Text("\(w.unread) unread")
              .font(.system(size: 10))
              .foregroundColor("#FF9F0A")
          }
        }
      }
      Spacer()
      if w.selected {
        Image(systemName: "checkmark.circle.fill")
          .font(.system(size: 11))
          .foregroundColor("#0A84FF")
      }
    }
    .padding(.vertical, 4)
    .padding(.horizontal, 6)
    .background(w.selected ? "#0A84FF22" : "#00000000")
    .cornerRadius(6)
  }
}

ScrollView {
  VStack(alignment: .leading, spacing: 12) {
    VStack(alignment: .leading, spacing: 4) {
      HStack(spacing: 8) {
        Image(systemName: "rectangle.split.3x3")
          .foregroundColor("#FF9F0A")
        Text("Herdr")
          .font(.headline)
          .fontWeight(.semibold)
        Spacer()
        Text("bridge")
          .font(.caption)
          .foregroundColor(.secondary)
      }
      Text("Inner agent mux nested inside cmux")
        .font(.caption)
        .foregroundColor(.secondary)
    }

    Divider()

    VStack(alignment: .leading, spacing: 6) {
      Text("Dual hierarchy")
        .font(.caption)
        .fontWeight(.semibold)
        .foregroundColor(.secondary)
      Text("cmux: window → workspace → pane → surface")
        .font(.system(size: 11, design: .monospaced))
        .foregroundColor(.secondary)
      Text("herdr: workspace → tab → pane → agent")
        .font(.system(size: 11, design: .monospaced))
        .foregroundColor(.secondary)
      Text("Status pills on the outer workspace are written by cmux-herdr sync|watch (keys herdr:<pane_id>).")
        .font(.caption)
        .foregroundColor(.secondary)
        .lineLimit(4)
    }

    Divider()

    VStack(alignment: .leading, spacing: 6) {
      Text("cmux workspaces")
        .font(.caption)
        .fontWeight(.semibold)
        .foregroundColor(.secondary)
      ForEach(cmux.workspaces) { w in
        herdrWorkspaceRow(w)
      }
    }

    Divider()

    VStack(alignment: .leading, spacing: 4) {
      Text("Commands")
        .font(.caption)
        .fontWeight(.semibold)
        .foregroundColor(.secondary)
      Text("cmux-herdr status")
        .font(.system(size: 11, design: .monospaced))
      Text("cmux-herdr tree")
        .font(.system(size: 11, design: .monospaced))
      Text("cmux-herdr sync")
        .font(.system(size: 11, design: .monospaced))
      Text("cmux-herdr watch")
        .font(.system(size: 11, design: .monospaced))
      Text("cmux-herdr agents")
        .font(.system(size: 11, design: .monospaced))
      Text("Run these inside the herdr-hosted surface. Do not assume tmux.")
        .font(.caption)
        .foregroundColor(.secondary)
        .padding(.top, 4)
    }
  }
  .padding(12)
}
