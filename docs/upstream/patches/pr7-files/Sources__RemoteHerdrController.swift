import Foundation
import CmuxNestedTopology
import CmuxSettings
import OSLog

/// Coordinates cmux's native Herdr Unix-socket mirroring (ssh-tmux parity host).
///
/// Attaches via ``HerdrNestedTopologyClient`` — never SSH / ``tmux -CC`` / herdr CLI.
/// Owns one ``RemoteHerdrSessionHost`` per endpoint+session. Host close always
/// detaches without ``server.stop``.
@MainActor
final class RemoteHerdrController {
    nonisolated static let logger = Logger(subsystem: "com.cmuxterm.app", category: "RemoteHerdr")

    /// Live session hosts keyed `endpointHash\u{1}session`.
    private(set) var sessionHosts: [String: RemoteHerdrSessionHost] = [:]

    /// In-flight attach guards (tmux ``RemoteTmuxWindowRegistry`` twin).
    private var attachRegistry = RemoteHerdrAttachRegistry()

    /// Shared plugin-writer handoff directory (same tree as nested topology).
    private let handoff: NestedPluginWriterHandoff

    init(handoffDirectory: URL? = nil) {
        let directory = handoffDirectory ?? Self.defaultHandoffDirectory()
        self.handoff = NestedPluginWriterHandoff(directoryURL: directory)
    }

    /// Beta gate: ``remoteHerdrMirror`` **or** nested topology.
    nonisolated static var isEnabled: Bool {
        let catalog = SettingCatalog().betaFeatures
        let herdr = Bool.decodeFromUserDefaults(
            UserDefaults.standard.object(forKey: catalog.remoteHerdrMirror.userDefaultsKey)
        ) ?? catalog.remoteHerdrMirror.defaultValue
        if herdr { return true }
        let nested = Bool.decodeFromUserDefaults(
            UserDefaults.standard.object(forKey: catalog.nestedTopology.userDefaultsKey)
        ) ?? catalog.nestedTopology.defaultValue
        return nested
    }

    static func connectionKey(endpointHash: String, sessionID: String) -> String {
        "\(endpointHash)\u{1}\(sessionID)"
    }

    /// Discovers Herdr workspaces (sessions) on a Unix socket.
    func listSessions(socketPath: String) async throws -> [RemoteHerdrDiscoveredSession] {
        guard Self.isEnabled else {
            throw RemoteHerdrHostError.disabled
        }
        guard let path = RemoteHerdrLifecycle.validateSocketPath(socketPath) else {
            throw RemoteHerdrHostError.invalidParams
        }
        let attachmentID = UUID()
        let hostSurfaceID = UUID()
        let client = HerdrNestedTopologyClient(
            configuration: HerdrNestedTopologyClientConfiguration(
                socketPath: path,
                attachmentID: attachmentID,
                hostStableSurfaceID: hostSurfaceID
            )
        )
        _ = try await client.handshake()
        let snap = try await client.snapshot()
        return snap.workspaces.map { workspace in
            let windowCount = snap.tabs.filter { $0.workspaceID == workspace.id }.count
            return RemoteHerdrDiscoveredSession(
                sessionID: workspace.id.rawID,
                name: workspace.displayTitle.isEmpty ? workspace.id.rawID : workspace.displayTitle,
                windowCount: windowCount,
                attached: sessionHosts[Self.connectionKey(
                    endpointHash: RemoteHerdrLifecycle.endpointHash(path),
                    sessionID: workspace.id.rawID
                )] != nil
            )
        }
    }

    func sessionHost(socketPath: String, sessionID: String) -> RemoteHerdrSessionHost? {
        guard let path = RemoteHerdrLifecycle.validateSocketPath(socketPath),
              let session = RemoteHerdrLifecycle.validateSessionName(sessionID)
        else { return nil }
        let key = Self.connectionKey(
            endpointHash: RemoteHerdrLifecycle.endpointHash(path),
            sessionID: session
        )
        return sessionHosts[key]
    }

    func sessionHost(workspaceId: UUID) -> RemoteHerdrSessionHost? {
        sessionHosts.values.first { $0.mirroredWorkspaceId == workspaceId }
    }

    /// Detach one mirrored session. Never calls ``server.stop``.
    func detach(socketPath: String, sessionID: String) {
        guard let path = RemoteHerdrLifecycle.validateSocketPath(socketPath),
              let session = RemoteHerdrLifecycle.validateSessionName(sessionID)
        else { return }
        let key = Self.connectionKey(
            endpointHash: RemoteHerdrLifecycle.endpointHash(path),
            sessionID: session
        )
        guard let host = sessionHosts.removeValue(forKey: key) else { return }
        host.detach(reason: "explicit_detach")
        releaseHandoffIfNeeded(host: host)
        if let workspaceId = host.mirroredWorkspaceId,
           let manager = AppDelegate.shared?.tabManagerFor(tabId: workspaceId) {
            if let workspace = manager.tabs.first(where: { $0.id == workspaceId }) {
                manager.closeWorkspace(workspace, recordHistory: false)
            }
        }
    }

    /// Detach every Herdr mirror on app quit / host close. Never ``server.stop``.
    func detachAll() {
        let hosts = Array(sessionHosts.values)
        sessionHosts.removeAll()
        for host in hosts {
            host.detach(reason: "app_terminate")
            releaseHandoffIfNeeded(host: host)
        }
    }

    /// Workspace closed from chrome — detach without killing Herdr.
    func handleWorkspaceClosed(workspaceId: UUID) {
        guard let entry = sessionHosts.first(where: { $0.value.mirroredWorkspaceId == workspaceId })
        else { return }
        sessionHosts.removeValue(forKey: entry.key)
        entry.value.detach(reason: "host_tab")
        releaseHandoffIfNeeded(host: entry.value)
    }

    /// Detach when the workspace stays open as a local tab.
    func detachMirrorWorkspaceKeptOpenLocally(workspaceId: UUID) {
        guard let entry = sessionHosts.first(where: { $0.value.mirroredWorkspaceId == workspaceId })
        else { return }
        sessionHosts.removeValue(forKey: entry.key)
        entry.value.detach(reason: "host_tab")
        releaseHandoffIfNeeded(host: entry.value)
    }

    /// Whether `surfaceId` is a pane of a mirrored Herdr window.
    func isMirrorPaneSurface(_ surfaceId: UUID) -> Bool {
        sessionHosts.values.contains { $0.containsSurface(surfaceId) }
    }

    /// Paste single-line text into the Herdr pane behind `surfaceId`.
    @discardableResult
    func pasteIntoMirror(surfaceId: UUID, text: String) async -> Bool {
        for host in sessionHosts.values {
            if await host.pasteIntoMirror(surfaceId: surfaceId, text: text) {
                return true
            }
        }
        return false
    }

    /// Route a split request from a mirrored Ghostty surface to `pane.split`.
    @discardableResult
    func handleMirrorSplitRequested(
        surfaceId: UUID,
        vertical: Bool
    ) async -> Bool {
        for host in sessionHosts.values where host.containsSurface(surfaceId) {
            return await host.handleMirrorSplitRequested(
                surfaceId: surfaceId,
                vertical: vertical
            )
        }
        return false
    }

    func beginAttach(endpointHash: String) -> Bool {
        attachRegistry.beginAttach(endpointHash)
    }

    func endAttach(endpointHash: String) {
        attachRegistry.endAttach(endpointHash)
    }

    func isAttaching(endpointHash: String) -> Bool {
        attachRegistry.isAttaching(endpointHash)
    }

    func register(_ host: RemoteHerdrSessionHost, key: String) {
        sessionHosts[key] = host
        acquireHandoff(host: host)
    }

    func purgeDeadHosts(endpointHash: String) {
        for (key, host) in sessionHosts
        where key.hasPrefix(endpointHash) && host.mirroredWorkspaceId == nil {
            sessionHosts.removeValue(forKey: key)
            host.detach(reason: "host_tab")
            releaseHandoffIfNeeded(host: host)
        }
    }

    func existingMirrorManager(endpointHash: String) -> TabManager? {
        for host in sessionHosts.values
        where RemoteHerdrLifecycle.endpointHash(host.socketPath) == endpointHash {
            guard let workspaceId = host.mirroredWorkspaceId,
                  let manager = AppDelegate.shared?.tabManagerFor(tabId: workspaceId)
            else { continue }
            return manager
        }
        return nil
    }

    func paneSurfaceEntries(socketPath: String, sessionID: String) -> [[String: Any]] {
        sessionHost(socketPath: socketPath, sessionID: sessionID)?.paneSurfaceEntries() ?? []
    }

    func paneGrids(socketPath: String, sessionID: String) -> [[String: Any]] {
        sessionHost(socketPath: socketPath, sessionID: sessionID)?.paneGrids() ?? []
    }

    func statePayload(socketPath: String, sessionID: String) -> [String: Any] {
        guard let host = sessionHost(socketPath: socketPath, sessionID: sessionID) else {
            return [
                "socket": socketPath,
                "session": sessionID,
                "attached": false,
                "mirrored": false,
            ]
        }
        return host.statePayload()
    }

    // MARK: - Plugin writer handoff

    private func acquireHandoff(host: RemoteHerdrSessionHost) {
        do {
            try handoff.acquire(
                hostStableSurfaceID: host.hostStableSurfaceID,
                attachmentID: host.attachmentID
            )
            host.nativeLive = true
        } catch {
            Self.logger.warning("remote-herdr: failed to acquire plugin writer handoff")
        }
    }

    private func releaseHandoffIfNeeded(host: RemoteHerdrSessionHost) {
        do {
            try handoff.release(hostStableSurfaceID: host.hostStableSurfaceID)
        } catch {
            Self.logger.warning("remote-herdr: failed to release plugin writer handoff")
        }
        host.nativeLive = false
    }

    private static func defaultHandoffDirectory() -> URL {
        let base = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
            ?? URL(fileURLWithPath: NSTemporaryDirectory(), isDirectory: true)
        return base
            .appendingPathComponent("cmux", isDirectory: true)
            .appendingPathComponent("nested-topology", isDirectory: true)
    }
}

/// Host-side errors for the native Herdr mirror path.
enum RemoteHerdrHostError: Error, LocalizedError {
    case disabled
    case invalidParams
    case unreachable(String)
    case noSessions
    case mirrorFailed
    case alreadyAttaching
    case windowCreationFailed

    var errorDescription: String? {
        switch self {
        case .disabled:
            return "remote Herdr mirror beta is disabled"
        case .invalidParams:
            return "invalid remote Herdr params"
        case .unreachable(let detail):
            return "remote Herdr unreachable: \(detail)"
        case .noSessions:
            return "no Herdr sessions on socket"
        case .mirrorFailed:
            return "could not mirror Herdr session"
        case .alreadyAttaching:
            return "already attaching to Herdr socket"
        case .windowCreationFailed:
            return "could not create window for Herdr mirror"
        }
    }
}
