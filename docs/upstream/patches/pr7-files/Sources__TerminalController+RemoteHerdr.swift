import Foundation
import CmuxControlSocket
import CmuxNestedTopology

/// Socket/CLI handlers for the native Herdr mirror (`remote.herdr.*`).
///
/// Twin of ``TerminalController+RemoteTmux``. Gates on ``RemoteHerdrController/isEnabled``
/// and never shells out to the herdr CLI.
extension TerminalController {
    /// `remote.herdr.sessions` — list Herdr workspaces on a Unix socket.
    nonisolated func v2RemoteHerdrSessions(id: Any?, params: [String: Any]) -> String {
        guard RemoteHerdrController.isEnabled else {
            return v2Error(
                id: id,
                code: "disabled",
                message: String(
                    localized: "socket.remoteHerdr.disabled",
                    defaultValue: "remote Herdr mirror beta is disabled"
                )
            )
        }
        guard let socket = RemoteHerdrLifecycle.validateSocketPath(
            (params["socket"] as? String) ?? (params["socket_path"] as? String)
        ) else {
            return v2Error(
                id: id,
                code: "invalid_params",
                message: String(
                    localized: "socket.remoteHerdr.socketRequired",
                    defaultValue: "socket is required"
                )
            )
        }
        return v2VmCall(id: id, timeoutSeconds: 30) {
            guard let controller = await MainActor.run(body: { AppDelegate.shared?.remoteHerdrController })
            else {
                throw RemoteHerdrHostError.unreachable("app not ready")
            }
            let sessions = try await controller.listSessions(socketPath: socket)
            return [
                "socket": socket,
                "sessions": sessions.map { $0.payload() },
            ]
        }
    }

    /// `remote.herdr.attach` / `remote.herdr.mirror` — mirror sessions into the resolved window.
    nonisolated func v2RemoteHerdrAttach(id: Any?, params: [String: Any]) -> String {
        remoteHerdrAttachOrWindow(id: id, params: params, dedicated: false)
    }

    /// `remote.herdr.window` — mirror into a dedicated new window.
    nonisolated func v2RemoteHerdrWindow(id: Any?, params: [String: Any]) -> String {
        remoteHerdrAttachOrWindow(id: id, params: params, dedicated: true)
    }

    nonisolated func v2RemoteHerdrMirror(id: Any?, params: [String: Any]) -> String {
        remoteHerdrAttachOrWindow(id: id, params: params, dedicated: false)
    }

    private nonisolated func remoteHerdrAttachOrWindow(
        id: Any?,
        params: [String: Any],
        dedicated: Bool
    ) -> String {
        guard RemoteHerdrController.isEnabled else {
            return v2Error(
                id: id,
                code: "disabled",
                message: String(
                    localized: "socket.remoteHerdr.disabled",
                    defaultValue: "remote Herdr mirror beta is disabled"
                )
            )
        }
        guard let socket = RemoteHerdrLifecycle.validateSocketPath(
            (params["socket"] as? String) ?? (params["socket_path"] as? String)
        ) else {
            return v2Error(
                id: id,
                code: "invalid_params",
                message: String(
                    localized: "socket.remoteHerdr.socketRequired",
                    defaultValue: "socket is required"
                )
            )
        }
        let activate = (params["activate"] as? Bool) ?? false
        let session = RemoteHerdrLifecycle.validateSessionName(params["session"] as? String)
        let target = RemoteHerdrAttachWindowTarget.fromParams(params, dedicated: dedicated)
        return v2VmCall(id: id, timeoutSeconds: 60) {
            guard let controller = await MainActor.run(body: { AppDelegate.shared?.remoteHerdrController })
            else {
                throw RemoteHerdrHostError.unreachable("app not ready")
            }
            // Resolve contextual targets with live window ids when needed.
            let resolvedTarget: RemoteHerdrAttachWindowTarget = await MainActor.run {
                if dedicated { return target }
                if target.kind == "contextual", target.windowID == nil {
                    let preferred = self.remoteHerdrPreferredWindowID(from: params)
                    return RemoteHerdrAttachWindowTarget(kind: "contextual", windowID: preferred)
                }
                return target
            }
            return try await controller.attachHost(
                socketPath: socket,
                windowTarget: resolvedTarget,
                activate: activate,
                sessionFilter: session
            )
        }
    }

    @MainActor
    private func remoteHerdrPreferredWindowID(from params: [String: Any]) -> String? {
        let routing = ControlRoutingSelectors(
            hasWindowIDParam: v2HasNonNullParam(params, "window_id"),
            windowID: v2UUID(params, "window_id"),
            groupID: v2UUID(params, "group_id"),
            workspaceID: v2UUID(params, "workspace_id"),
            surfaceID: v2UUID(params, "surface_id")
                ?? v2UUID(params, "terminal_id")
                ?? v2UUID(params, "tab_id"),
            paneID: v2UUID(params, "pane_id")
        )
        return resolveTabManager(routing: routing)
            .flatMap { AppDelegate.shared?.windowId(for: $0)?.uuidString }
    }

    /// `remote.herdr.detach` — detach and remove mirror workspace; leave Herdr running.
    nonisolated func v2RemoteHerdrDetach(id: Any?, params: [String: Any]) -> String {
        guard RemoteHerdrController.isEnabled else {
            return v2Error(
                id: id,
                code: "disabled",
                message: String(
                    localized: "socket.remoteHerdr.disabled",
                    defaultValue: "remote Herdr mirror beta is disabled"
                )
            )
        }
        guard let socket = RemoteHerdrLifecycle.validateSocketPath(
            (params["socket"] as? String) ?? (params["socket_path"] as? String)
        ),
              let session = RemoteHerdrLifecycle.validateSessionName(params["session"] as? String)
        else {
            return v2Error(
                id: id,
                code: "invalid_params",
                message: String(
                    localized: "socket.remoteHerdr.socketAndSessionRequired",
                    defaultValue: "socket and session are required"
                )
            )
        }
        return v2VmCall(id: id, timeoutSeconds: 10) {
            await MainActor.run {
                AppDelegate.shared?.remoteHerdrController.detach(
                    socketPath: socket,
                    sessionID: session
                )
            }
            return [
                "socket": socket,
                "session": session,
                "detached": true,
                "server_stopped": false,
            ]
        }
    }

    /// `remote.herdr.state` — mirrored session diagnostics.
    nonisolated func v2RemoteHerdrState(id: Any?, params: [String: Any]) -> String {
        guard RemoteHerdrController.isEnabled else {
            return v2Error(
                id: id,
                code: "disabled",
                message: String(
                    localized: "socket.remoteHerdr.disabled",
                    defaultValue: "remote Herdr mirror beta is disabled"
                )
            )
        }
        guard let socket = RemoteHerdrLifecycle.validateSocketPath(
            (params["socket"] as? String) ?? (params["socket_path"] as? String)
        ),
              let session = RemoteHerdrLifecycle.validateSessionName(params["session"] as? String)
        else {
            return v2Error(
                id: id,
                code: "invalid_params",
                message: String(
                    localized: "socket.remoteHerdr.socketAndSessionRequired",
                    defaultValue: "socket and session are required"
                )
            )
        }
        return v2VmCall(id: id, timeoutSeconds: 10) {
            await MainActor.run {
                AppDelegate.shared?.remoteHerdrController.statePayload(
                    socketPath: socket,
                    sessionID: session
                ) ?? [
                    "socket": socket,
                    "session": session,
                    "attached": false,
                    "mirrored": false,
                ]
            }
        }
    }

    /// `remote.herdr.pane_surfaces` — pane id → surface id map.
    nonisolated func v2RemoteHerdrPaneSurfaces(id: Any?, params: [String: Any]) -> String {
        guard RemoteHerdrController.isEnabled else {
            return v2Error(
                id: id,
                code: "disabled",
                message: String(
                    localized: "socket.remoteHerdr.disabled",
                    defaultValue: "remote Herdr mirror beta is disabled"
                )
            )
        }
        guard let socket = RemoteHerdrLifecycle.validateSocketPath(
            (params["socket"] as? String) ?? (params["socket_path"] as? String)
        ),
              let session = RemoteHerdrLifecycle.validateSessionName(params["session"] as? String)
        else {
            return v2Error(
                id: id,
                code: "invalid_params",
                message: String(
                    localized: "socket.remoteHerdr.socketAndSessionRequired",
                    defaultValue: "socket and session are required"
                )
            )
        }
        return v2VmCall(id: id, timeoutSeconds: 10) {
            let panes = await MainActor.run {
                AppDelegate.shared?.remoteHerdrController.paneSurfaceEntries(
                    socketPath: socket,
                    sessionID: session
                ) ?? []
            }
            return [
                "socket": socket,
                "session": session,
                "mirrored": !panes.isEmpty,
                "panes": panes,
            ]
        }
    }

    /// `remote.herdr.pane_grids` — assigned vs rendered grids per pane.
    nonisolated func v2RemoteHerdrPaneGrids(id: Any?, params: [String: Any]) -> String {
        guard RemoteHerdrController.isEnabled else {
            return v2Error(
                id: id,
                code: "disabled",
                message: String(
                    localized: "socket.remoteHerdr.disabled",
                    defaultValue: "remote Herdr mirror beta is disabled"
                )
            )
        }
        guard let socket = RemoteHerdrLifecycle.validateSocketPath(
            (params["socket"] as? String) ?? (params["socket_path"] as? String)
        ),
              let session = RemoteHerdrLifecycle.validateSessionName(params["session"] as? String)
        else {
            return v2Error(
                id: id,
                code: "invalid_params",
                message: String(
                    localized: "socket.remoteHerdr.socketAndSessionRequired",
                    defaultValue: "socket and session are required"
                )
            )
        }
        return v2VmCall(id: id, timeoutSeconds: 10) {
            let windows = await MainActor.run {
                AppDelegate.shared?.remoteHerdrController.paneGrids(
                    socketPath: socket,
                    sessionID: session
                ) ?? []
            }
            return [
                "socket": socket,
                "session": session,
                "mirrored": !windows.isEmpty,
                "windows": windows,
            ]
        }
    }
}
