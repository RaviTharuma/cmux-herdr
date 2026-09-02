// LEGACY CONTRIB FALLBACK — NOT THE PRODUCT INSTALL.
// The product path is: cmux sidebar plugin install|use|update cmux-herdr.
// This file only exists for cmux builds without `cmux sidebar plugin`; copying
// it to ~/.config/cmux/sidebars and running `cmux sidebar select|open herdr`
// replaces the left rail or opens a pane and is unsupported.
// Herdr sidebar for cmux (JS runtime — wins over herdr.swift for live drag).
// Native cmux chrome: Reorderable, context menus, accent/primary/secondary/
// tertiary tokens. Herdr is the product name. This is not an iframe, not a
// bridge panel, and not a CLI cheat-sheet. Click a row to select a workspace
// or focus a surface. Restricted JS scene (cmux docs/custom-sidebars.md).

let selectOverride = null;
const [selectTick, setSelectTick] = signal(0);

function isSelected(w) {
  selectTick();
  if (!w) return false;
  if (selectOverride) {
    if (data.selectedId() === selectOverride) selectOverride = null;
    else return w.id === selectOverride;
  }
  return !!w.selected;
}

function selectWorkspace(id) {
  if (!id) return;
  selectOverride = id;
  setSelectTick(selectTick() + 1);
  cmux("workspace.select", { workspace_id: id });
}

const closedOverride = new Set();
const [closeTick, setCloseTick] = signal(0);

function visibleWorkspaces() {
  closeTick();
  let ws = data.workspaces() ?? [];
  for (const id of Array.from(closedOverride)) {
    if (!ws.some((w) => w.id === id)) closedOverride.delete(id);
  }
  return ws.filter((w) => !closedOverride.has(w.id)).slice(0, 40);
}

function closeWorkspace(id) {
  closedOverride.add(id);
  setCloseTick(closeTick() + 1);
  cmux("workspace.close", { workspace_id: id });
}

function workspaceAction(action, id, extra) {
  const params = { action: action, workspace_id: id };
  if (extra) {
    for (const key of Object.keys(extra)) params[key] = extra[key];
  }
  cmux("workspace.action", params);
}

function tabFocusId(t) {
  if (t && t.surfaceId) return t.surfaceId;
  return t ? t.id : "";
}

function focusSurface(t) {
  const id = tabFocusId(t);
  if (!id) return;
  cmux("surface.focus", { surface_id: id });
}

function hasText(value) {
  return value != null && value !== "";
}

function statusLabel(s) {
  if (s && hasText(s.value)) return s.value;
  if (s && hasText(s.key) && String(s.key).indexOf(":") === -1) return s.key;
  return "";
}

function statusTint(s) {
  if (s && hasText(s.color)) return s.color;
  const label = statusLabel(s).toLowerCase();
  if (label === "working") return "accent";
  if (label === "blocked" || label === "needs_input") return "red";
  if (label === "done" || label === "ended") return "tertiary";
  return "secondary";
}

function statusIcon(s) {
  if (s && hasText(s.icon)) return s.icon;
  const label = statusLabel(s).toLowerCase();
  if (label === "working") return "hammer.fill";
  if (label === "blocked" || label === "needs_input") return "exclamationmark.triangle.fill";
  if (label === "done" || label === "ended") return "checkmark.circle";
  if (label === "idle") return "pause.circle";
  return "circle.fill";
}

function liveStatuses(w) {
  const items = (w && w.statuses) ? w.statuses : [];
  return items.filter((s) => statusLabel(s) !== "").slice(0, 6);
}

function handleMove(id, index) {
  cmux("workspace.reorder", { workspace_id: id, index: index });
}

function workspaceMenu(w) {
  const row = () => w();
  return [
    Button("Rename", () =>
      workspaceAction("rename", row().id)),
    Button(() => (row()?.pinned ? "Unpin" : "Pin"), () =>
      workspaceAction(row()?.pinned ? "unpin" : "pin", row().id)),
    Button(() => (row()?.unread > 0 ? "Mark as Read" : "Mark as Unread"), () =>
      workspaceAction(row()?.unread > 0 ? "mark_read" : "mark_unread", row().id)),
    Divider(),
    Menu("Move", [
      Button("Move Up", () => workspaceAction("move_up", row().id)),
      Button("Move Down", () => workspaceAction("move_down", row().id)),
      Button("Move to Top", () => workspaceAction("move_top", row().id)),
    ]),
    Divider(),
    Button("Close Others", () => workspaceAction("close_others", row().id)).destructive(),
    Button("Close", () => closeWorkspace(row().id)).destructive(),
  ];
}

function statusChip(s) {
  return HStack({ spacing: 4 }, [
    Image(() => statusIcon(s())).font(8).color(() => statusTint(s())),
    Text(() => statusLabel(s())).font(10).color(() => statusTint(s())).lineLimit(1),
  ])
    .paddingHorizontal(5)
    .paddingVertical(2)
    .cornerRadius(7);
}

function workspaceRow(w) {
  const selected = () => isSelected(w());
  return VStack({ spacing: 3 }, [
    HStack({ spacing: 8 }, [
      Circle({ size: 7 }).fill(() => (selected() ? "accent" : "tertiary")),
      Text(() => w()?.title ?? "")
        .font(13)
        .lineLimit(1)
        .truncation("tail")
        .marquee()
        .color(() => (selected() ? "primary" : "secondary")),
      Spacer({ minLength: 0 }),
      Text(() => (w()?.unread > 0 ? String(w().unread) : ""))
        .font(9)
        .weight("semibold")
        .color("accent"),
    ]),
    HStack({ spacing: 6 }, [
      Text(() => String(w()?.tabCount ?? 0)).font(10).color("tertiary"),
      Text(() => (hasText(w()?.branch) ? w().branch : ""))
        .font(10)
        .color("tertiary")
        .lineLimit(1),
      Text(() => (w()?.progress && hasText(w().progress.label) ? w().progress.label : ""))
        .font(10)
        .color("accent")
        .lineLimit(1),
    ]),
    ForEach(
      { items: () => liveStatuses(w()), key: (s) => (s.key || s.value || s.icon || "s") },
      (s) => statusChip(s)
    ),
  ])
    .paddingHorizontal(10)
    .paddingVertical(6)
    .cornerRadius(8)
    .background(() => (selected() ? "#7f7f7f3d" : null))
    .hoverBackground(() => (selected() ? "#7f7f7f3d" : "#7f7f7f24"))
    .frame({ maxWidth: "infinity" })
    .onTap(() => selectWorkspace(w().id))
    .contextMenu(workspaceMenu(w));
}

function tabRow(t) {
  return HStack({ spacing: 6 }, [
    Image(() => (t()?.focused ? "dot.circle.fill" : "terminal"))
      .font(10)
      .color(() => (t()?.focused ? "accent" : "secondary")),
    Text(() => t()?.title ?? "")
      .font(12)
      .lineLimit(1)
      .truncation("tail")
      .color(() => (t()?.focused ? "primary" : "secondary")),
    Spacer({ minLength: 0 }),
  ])
    .paddingHorizontal(10)
    .paddingVertical(5)
    .marginLeading(14)
    .cornerRadius(8)
    .hoverBackground("#7f7f7f24")
    .frame({ maxWidth: "infinity" })
    .onTap(() => focusSurface(t()));
}

function selectedTabs() {
  const ws = visibleWorkspaces();
  const current = ws.find((w) => isSelected(w));
  const tabs = current && current.tabs ? current.tabs : [];
  return tabs.slice(0, 12);
}

sidebar(() =>
  VStack({ spacing: 4 }, [
    HStack({ spacing: 8 }, [
      Text("Herdr").font(13).weight("semibold").color("primary"),
      Spacer(),
      Text(() => String((data.workspaces() ?? []).length)).font(10).color("tertiary"),
    ])
      .paddingHorizontal(10)
      .paddingVertical(6),
    Reorderable(
      {
        items: visibleWorkspaces,
        key: (w) => w.id,
        spacing: 2,
        onMove: handleMove,
      },
      (w) => workspaceRow(w)
    ),
    ForEach({ items: selectedTabs, key: (t) => t.id }, (t) => tabRow(t)),
  ]),
  { surface: "glass" }
);
