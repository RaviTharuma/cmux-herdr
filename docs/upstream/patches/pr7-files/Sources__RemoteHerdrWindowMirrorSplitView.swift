import Bonsplit
import SwiftUI

@MainActor
struct RemoteHerdrWindowMirrorSplitView: View {
    let mirror: RemoteHerdrWindowMirrorHost
    let appearance: PanelAppearance
    let isOuterFocused: Bool
    let isVisibleInUI: Bool
    let portalPriority: Int
    let onOuterFocus: () -> Void
    var unreadSurfaceIDs: Set<UUID> = []
    @State private var containerSize: CGSize = .zero

    var body: some View {
        Color(nsColor: appearance.backgroundColor)
            .overlay(alignment: .topLeading) {
                splitTree
            }
            .onGeometryChange(for: CGSize.self) { proxy in
                proxy.size
            } action: { newSize in
                containerSize = newSize
                mirror.isVisibleForSizing = isVisibleInUI
            }
            .onAppear {
                mirror.isVisibleForSizing = isVisibleInUI
                mirror.bonsplitController.isInteractive = isVisibleInUI
            }
            .onChange(of: isVisibleInUI) { _, visible in
                mirror.isVisibleForSizing = visible
                mirror.bonsplitController.isInteractive = visible
            }
            .onChange(of: mirror.layoutStructureVersion) { _, _ in
                mirror.isVisibleForSizing = isVisibleInUI
            }
    }

    private var splitTree: some View {
        BonsplitView(controller: mirror.bonsplitController) { tab, paneId in
            if let herdrPaneId = mirror.herdrPaneId(forTab: tab.id),
               let panel = mirror.panel(forPane: herdrPaneId) {
                TerminalPanelView(
                    panel: panel,
                    paneId: paneId,
                    isFocused: isOuterFocused && mirror.isFocused(tabId: tab.id),
                    isVisibleInUI: isVisibleInUI,
                    portalPaneOwnershipResolver: {
                        mirror.bonsplitController.selectedTab(inPane: paneId)?.id == tab.id
                    },
                    portalPriority: portalPriority,
                    isSplit: true,
                    appearance: appearance,
                    hasUnreadNotification: unreadSurfaceIDs.contains(panel.id),
                    terminalAgentContext: "",
                    onFocus: {
                        onOuterFocus()
                        mirror.setActivePane(herdrPaneId, fromProvider: false)
                    },
                    onResumeAgentHibernation: {},
                    onAutoResumeAgentHibernation: {},
                    onTriggerFlash: {}
                )
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .onTapGesture {
                    onOuterFocus()
                    mirror.bonsplitController.focusPane(paneId)
                }
            } else {
                Color(nsColor: appearance.backgroundColor)
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        } emptyPane: { _ in
            Color(nsColor: appearance.backgroundColor)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
        .internalOnlyTabDrag()
        .frame(
            maxWidth: .infinity,
            maxHeight: .infinity,
            alignment: .topLeading
        )
    }
}
