# Integrations

External software this project talks to. No secret values here — credentials are declared (by name only) in Varlock's `.env.schema` or the platform's credential store.

| System | Purpose | How |
| --- | --- | --- |
| Herdr | Agent/session multiplexer being projected | Herdr API/CLI |
| cmux | Host app: sidebar, right rail, status pills, panes | cmux host CLI/socket |
| macOS launchd | Optional background watch service | `scripts/com.cmux-herdr.watch.plist` |
