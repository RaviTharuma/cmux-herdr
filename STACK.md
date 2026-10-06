# Stack

| Layer | Choice |
| --- | --- |
| Language | Rust 2021 (MSRV 1.98) |
| CLI | clap 4 |
| Serialization | serde/serde_json (preserve_order), toml_edit |
| OS | rustix (fs/process/net/termios) |
| Sidebar | cmux plugin (`cmux-plugin.toml`, Swift/JS) |
| Service | macOS LaunchAgent (optional watch service) |
| CI / release | GitHub Actions, Dependabot, GitHub Releases |

Update this file in the same PR that adds, removes or replaces a runtime, framework, hosting target or paid service.
