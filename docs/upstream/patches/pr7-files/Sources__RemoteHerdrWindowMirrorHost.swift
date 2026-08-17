import AppKit
import Bonsplit
import CmuxNestedTopology
import CmuxRemoteSession
import CmuxTerminal
import Foundation
import Observation

/// Owns per-pane ``TerminalPanel``s and Bonsplit layout for ONE mirrored Herdr tab.
///
/// Pane ids are Herdr strings (`w2:p34`). Layout comes from package
/// ``RemoteHerdrWindowMirror`` + ``RemoteHerdrHostApply`` verbs; Bonsplit is
/// imposed by ``RemoteHerdrWindowMirrorHost+Bonsplit``.
@MainActor
@Observable
final class RemoteHerdrWindowMirrorHost {
    let tabID: String
    let panelId: UUID

    var bonsplitController: BonsplitController

    @ObservationIgnored let makePanel: (_ paneID: String) -> TerminalPanel?
    @ObservationIgnored private let paneIO: any RemoteHerdrPaneIO
    @ObservationIgnored weak var workspaceBonsplitController: BonsplitController?

    private(set) var layout: RemoteHerdrLayoutNode?
    private(set) var visibleLayout: RemoteHerdrLayoutNode?
    private(set) var zoomed = false
    private(set) var layoutStructureVersion = 0
    private(set) var activePaneID: String?
    private(set) var windowTitle = "herdr"
    private(set) var mirrorState: RemoteHerdrWindowMirrorState?

    @ObservationIgnored var isVisibleForSizing = false
    @ObservationIgnored var isTornDown = false
    @ObservationIgnored var isApplyingRemoteLayout = false
    @ObservationIgnored var isApplyingFocus = false

    @ObservationIgnored var panelsByPaneId: [String: TerminalPanel] = [:]
    @ObservationIgnored var tabIdByPaneId: [String: TabID] = [:]
    @ObservationIgnored var paneIdByPaneId: [String: PaneID] = [:]
    @ObservationIgnored var paneIdByBonsplitPane: [PaneID: String] = [:]
    @ObservationIgnored var paneIdByTabId: [TabID: String] = [:]
    @ObservationIgnored var lastDividerPositions: [UUID: CGFloat] = [:]

    @ObservationIgnored var onTerminalPanelAdded: ((TerminalPanel) -> Void)?
    @ObservationIgnored var onTerminalPanelRemoved: ((TerminalPanel) -> Void)?

    var surfaceIDsInLayoutOrder: [UUID] {
        let order = (visibleLayout ?? layout)?.paneIDsInOrder ?? Array(panelsByPaneId.keys)
        return order.compactMap { panelsByPaneId[$0]?.id }
    }

    var renderedLayout: RemoteHerdrLayoutNode? { visibleLayout ?? layout }

    init(
        tabID: String,
        panelId: UUID,
        appearance: BonsplitConfiguration.Appearance = .init(),
        workspaceBonsplitController: BonsplitController? = nil,
        paneIO: any RemoteHerdrPaneIO,
        makePanel: @escaping (_ paneID: String) -> TerminalPanel?
    ) {
        self.tabID = tabID
        self.panelId = panelId
        self.paneIO = paneIO
        self.makePanel = makePanel
        self.workspaceBonsplitController = workspaceBonsplitController
        let initialConfiguration = workspaceBonsplitController?.configuration
            ?? BonsplitConfiguration(appearance: appearance)
        self.bonsplitController = Self.makeController(configuration: initialConfiguration)
        configureBonsplitController()
    }

    func panel(forPane paneID: String) -> TerminalPanel? { panelsByPaneId[paneID] }

    func isFocused(tabId: TabID) -> Bool {
        guard let paneID = paneIdByTabId[tabId] else { return false }
        return paneID == activePaneID
    }

    func herdrPaneId(forTab tabId: TabID) -> String? {
        paneIdByTabId[tabId]
    }

    /// Reconcile + HostApply against a Herdr window update.
    func apply(window: RemoteHerdrWindow) {
        guard !isTornDown else { return }
        let previous = mirrorState
        let previousRendered = previous.map { $0.visibleLayout ?? $0.layout }
        let (next, result) = RemoteHerdrWindowMirror.apply(window: window, previous: previous)
        mirrorState = next
        windowTitle = RemoteHerdrControl.applySessionTitle(window.title, current: windowTitle) ?? window.title
        if layoutStructureVersion != next.layoutStructureVersion {
            layoutStructureVersion = next.layoutStructureVersion
        }
        layout = next.layout
        visibleLayout = next.visibleLayout
        zoomed = next.zoomed

        guard let plan = RemoteHerdrImpose.plan(
            from: result,
            previousRendered: previousRendered,
            title: window.title
        ) else {
            // Still create panels when the layout cannot produce a divider tree.
            for paneID in result.createdPaneIDs where panelsByPaneId[paneID] == nil {
                guard let panel = makePanel(paneID) else { continue }
                panelsByPaneId[paneID] = panel
                onTerminalPanelAdded?(panel)
                if var state = mirrorState {
                    RemoteHerdrWindowMirror.bindSurface(
                        paneID: paneID,
                        surfaceID: panel.id,
                        state: &state
                    )
                    mirrorState = state
                }
            }
            if let focus = result.focusPaneID {
                noteRemoteActivePane(focus)
            }
            return
        }
        let actions = RemoteHerdrHostApply.actions(result: result, plan: plan)
        isApplyingRemoteLayout = true
        defer { isApplyingRemoteLayout = false }
        for action in actions {
            applyHostAction(action, plan: plan)
        }
        if let focus = result.focusPaneID {
            noteRemoteActivePane(focus)
        }
    }

    private func applyHostAction(
        _ action: RemoteHerdrHostAction,
        plan: RemoteHerdrImposePlan
    ) {
        switch action.op {
        case "create_panel":
            if let paneID = action.paneID, panelsByPaneId[paneID] == nil {
                guard let panel = makePanel(paneID) else { return }
                panelsByPaneId[paneID] = panel
                onTerminalPanelAdded?(panel)
                if var state = mirrorState {
                    RemoteHerdrWindowMirror.bindSurface(
                        paneID: paneID,
                        surfaceID: panel.id,
                        state: &state
                    )
                    mirrorState = state
                }
            }
        case "close_panel":
            if let paneID = action.paneID, let panel = panelsByPaneId.removeValue(forKey: paneID) {
                onTerminalPanelRemoved?(panel)
                GhosttyApp.terminalSurfaceRegistry.unregister(panel.surface)
                panel.close()
            }
        case "rebuild_tree":
            rebuildBonsplitTree()
        case "keep_tree":
            imposeDividerTree(plan.dividerTree)
        case "expand_leaf":
            if let paneID = action.paneID,
               let from = action.splitFromPaneID,
               let orientation = action.orientation {
                expandLeaf(
                    existingPaneID: from,
                    newPaneID: paneID,
                    orientation: orientation,
                    insertFirst: action.insertFirst,
                    fraction: action.fraction ?? 0.5
                )
            } else {
                rebuildBonsplitTree()
            }
        case "remove_leaf":
            if let paneID = action.paneID {
                removeLeaf(paneID: paneID)
            } else {
                rebuildBonsplitTree()
            }
        case "impose_divider":
            // Applied as a full tree walk after structural verbs.
            break
        case "focus":
            if let paneID = action.paneID {
                noteRemoteActivePane(paneID)
            }
        default:
            break
        }
        // After structural mutations, impose divider fractions once.
        if ["rebuild_tree", "expand_leaf", "remove_leaf", "keep_tree"].contains(action.op) {
            imposeDividerTree(plan.dividerTree)
        }
    }

    func deliverOutput(paneID: String, data: Data, fullRedraw: Bool) {
        guard let panel = panelsByPaneId[paneID], !isTornDown else { return }
        if fullRedraw {
            _ = panel.surface.clearScreenKeepingScrollback()
        }
        panel.surface.processRemoteOutput(data)
    }

    func noteRemoteActivePane(_ paneID: String) {
        guard panelsByPaneId[paneID] != nil else { return }
        isApplyingFocus = true
        defer { isApplyingFocus = false }
        activePaneID = paneID
        if let bonsplitPane = paneIdByPaneId[paneID],
           bonsplitController.focusedPaneId != bonsplitPane {
            bonsplitController.focusPane(bonsplitPane)
        }
    }

    func setActivePane(_ paneID: String, fromProvider: Bool) {
        guard panelsByPaneId[paneID] != nil, !isApplyingFocus else { return }
        activePaneID = paneID
        if let bonsplitPane = paneIdByPaneId[paneID],
           bonsplitController.focusedPaneId != bonsplitPane {
            bonsplitController.focusPane(bonsplitPane)
        }
        _ = fromProvider
    }

    func teardown() {
        isTornDown = true
        for panel in panelsByPaneId.values {
            onTerminalPanelRemoved?(panel)
            GhosttyApp.terminalSurfaceRegistry.unregister(panel.surface)
            panel.close()
        }
        panelsByPaneId.removeAll()
        tabIdByPaneId.removeAll()
        paneIdByPaneId.removeAll()
        paneIdByBonsplitPane.removeAll()
        paneIdByTabId.removeAll()
    }

    func paneGridsPayload() -> [String: Any] {
        let layout = renderedLayout
        var panes: [[String: Any]] = []
        for paneID in panelsByPaneId.keys.sorted() {
            guard let panel = panelsByPaneId[paneID] else { continue }
            var assignedCols = 80
            var assignedRows = 24
            if let leaf = layout?.firstLeaf(withPaneID: paneID) {
                assignedCols = max(1, leaf.width)
                assignedRows = max(1, leaf.height)
            }
            panes.append([
                "pane_id": paneID,
                "assigned_cols": assignedCols,
                "assigned_rows": assignedRows,
                "rendered_cols": assignedCols,
                "rendered_rows": assignedRows,
                "exact_cols": true,
                "exact_rows": true,
                "has_panel": true,
            ])
        }
        return [
            "tab_id": tabID,
            "panes": panes,
            "structure_version": layoutStructureVersion,
            "zoomed": zoomed,
            "visible_for_sizing": isVisibleForSizing,
        ]
    }
}
