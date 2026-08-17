import AppKit
import Bonsplit
import CmuxNestedTopology
import CmuxRemoteSession
import Foundation

@MainActor
extension RemoteHerdrWindowMirrorHost {
    static func makeController(configuration: BonsplitConfiguration) -> BonsplitController {
        BonsplitController(configuration: configuration.remoteTmuxEmbedded)
    }

    func configureBonsplitController() {
        bonsplitController.tabShortcutHintsEnabled = false
        bonsplitController.onExternalTabDrop = { _ in false }
    }

    func rebuildBonsplitTree() {
        isApplyingRemoteLayout = true
        defer { isApplyingRemoteLayout = false }
        resetToSingleEmptyPane()
        tabIdByPaneId.removeAll()
        paneIdByPaneId.removeAll()
        paneIdByBonsplitPane.removeAll()
        paneIdByTabId.removeAll()
        guard let rootPane = bonsplitController.allPaneIds.first,
              let rendered = renderedLayout
        else { return }
        _ = build(rendered, inPane: rootPane)
    }

    func resetToSingleEmptyPane() {
        while bonsplitController.allPaneIds.count > 1, let pane = bonsplitController.allPaneIds.last {
            _ = bonsplitController.closePane(pane)
        }
        guard let rootPane = bonsplitController.allPaneIds.first else { return }
        for tab in bonsplitController.tabs(inPane: rootPane) {
            _ = bonsplitController.closeTab(tab.id, inPane: rootPane)
        }
    }

    @discardableResult
    func build(_ node: RemoteHerdrLayoutNode, inPane pane: PaneID) -> PaneID? {
        switch node.content {
        case .pane(let paneID):
            guard panelsByPaneId[paneID] != nil else { return nil }
            guard let tabId = bonsplitController.createTab(
                title: title(forPane: paneID),
                icon: "terminal",
                kind: "terminal",
                inPane: pane
            ) else { return nil }
            tabIdByPaneId[paneID] = tabId
            paneIdByPaneId[paneID] = pane
            paneIdByBonsplitPane[pane] = paneID
            paneIdByTabId[tabId] = paneID
            return pane
        case .horizontal(let children):
            return build(children: children, orientation: .horizontal, inPane: pane)
        case .vertical(let children):
            return build(children: children, orientation: .vertical, inPane: pane)
        }
    }

    func build(
        children: [RemoteHerdrLayoutNode],
        orientation: SplitOrientation,
        inPane pane: PaneID
    ) -> PaneID? {
        guard let first = children.first else { return nil }
        guard children.count > 1 else { return build(first, inPane: pane) }
        let rest = Array(children.dropFirst())
        let fraction = dividerFraction(first: first, rest: rest, orientation: orientation)
        guard let restPane = bonsplitController.splitPane(
            pane,
            orientation: orientation,
            withTab: nil,
            initialDividerPosition: fraction
        ) else { return build(first, inPane: pane) }
        _ = build(first, inPane: pane)
        if let restNode = combined(children: rest, orientation: orientation) {
            _ = build(restNode, inPane: restPane)
        }
        return pane
    }

    func combined(children: [RemoteHerdrLayoutNode], orientation: SplitOrientation) -> RemoteHerdrLayoutNode? {
        guard let first = children.first else {
            return nil
        }
        guard children.count > 1 else { return first }
        let width = children.map(\.width).reduce(0, +)
        let height = children.map(\.height).max() ?? first.height
        switch orientation {
        case .horizontal:
            return RemoteHerdrLayoutNode(
                width: max(1, width),
                height: max(1, height),
                x: first.x,
                y: first.y,
                content: .horizontal(children)
            )
        case .vertical:
            let h = children.map(\.height).reduce(0, +)
            let w = children.map(\.width).max() ?? first.width
            return RemoteHerdrLayoutNode(
                width: max(1, w),
                height: max(1, h),
                x: first.x,
                y: first.y,
                content: .vertical(children)
            )
        }
    }

    func dividerFraction(
        first: RemoteHerdrLayoutNode,
        rest: [RemoteHerdrLayoutNode],
        orientation: SplitOrientation
    ) -> CGFloat {
        let firstSpan = orientation == .horizontal ? first.width : first.height
        let restSpans = rest.map { orientation == .horizontal ? $0.width : $0.height }
        return CGFloat(RemoteHerdrImpose.dividerFraction(firstSpan: firstSpan, restSpans: restSpans))
    }

    func title(forPane paneID: String) -> String {
        "\(windowTitle) · \(paneID)"
    }

    func expandLeaf(
        existingPaneID: String,
        newPaneID: String,
        orientation: RemoteHerdrSplitOrientation,
        insertFirst: Bool,
        fraction: Double
    ) {
        guard let existingPane = paneIdByPaneId[existingPaneID] else {
            rebuildBonsplitTree()
            return
        }
        let bonsplitOrientation: SplitOrientation =
            orientation == .horizontal ? .horizontal : .vertical
        let clamped = RemoteHerdrImpose.clampRatio(fraction)
        guard let newPane = bonsplitController.splitPane(
            existingPane,
            orientation: bonsplitOrientation,
            withTab: nil,
            initialDividerPosition: CGFloat(insertFirst ? (1.0 - clamped) : clamped)
        ) else {
            rebuildBonsplitTree()
            return
        }
        let targetPane = insertFirst ? existingPane : newPane
        // Ensure panel exists before building the leaf tab.
        if panelsByPaneId[newPaneID] == nil, let panel = makePanel(newPaneID) {
            panelsByPaneId[newPaneID] = panel
            onTerminalPanelAdded?(panel)
        }
        guard panelsByPaneId[newPaneID] != nil else { return }
        guard let tabId = bonsplitController.createTab(
            title: title(forPane: newPaneID),
            icon: "terminal",
            kind: "terminal",
            inPane: targetPane
        ) else { return }
        tabIdByPaneId[newPaneID] = tabId
        paneIdByPaneId[newPaneID] = targetPane
        paneIdByBonsplitPane[targetPane] = newPaneID
        paneIdByTabId[tabId] = newPaneID
    }

    func removeLeaf(paneID: String) {
        guard let pane = paneIdByPaneId[paneID] else {
            rebuildBonsplitTree()
            return
        }
        if let tabId = tabIdByPaneId[paneID] {
            _ = bonsplitController.closeTab(tabId, inPane: pane)
        }
        if bonsplitController.allPaneIds.count > 1 {
            _ = bonsplitController.closePane(pane)
        }
        tabIdByPaneId.removeValue(forKey: paneID)
        paneIdByPaneId.removeValue(forKey: paneID)
        paneIdByBonsplitPane.removeValue(forKey: pane)
        if let tabId = tabIdByPaneId[paneID] {
            paneIdByTabId.removeValue(forKey: tabId)
        }
    }

    /// Walk the binary divider tree in lockstep with Bonsplit and impose fractions.
    func imposeDividerTree(_ node: RemoteHerdrDividerNode) {
        let treeNode = bonsplitController.treeSnapshot()
        impose(node, onto: treeNode)
    }

    private func impose(_ node: RemoteHerdrDividerNode, onto treeNode: ExternalTreeNode) {
        switch (node, treeNode) {
        case (.leaf, .pane):
            return
        case let (
            .split(orientation, fraction, firstExtent, first, second),
            .split(let split)
        ):
            let expected: String = orientation == .horizontal ? "horizontal" : "vertical"
            guard split.orientation == expected,
                  let splitId = UUID(uuidString: split.id)
            else { return }
            if let firstExtent {
                _ = bonsplitController.setImposedFirstExtent(
                    CGFloat(firstExtent), forSplit: splitId, fromExternal: true
                )
            } else {
                let clamped = CGFloat(RemoteHerdrImpose.clampRatio(fraction))
                _ = bonsplitController.setDividerPosition(
                    clamped, forSplit: splitId, fromExternal: true
                )
                lastDividerPositions[splitId] = clamped
            }
            impose(first, onto: split.first)
            impose(second, onto: split.second)
        default:
            return
        }
    }
}
