# Passkey auth in the cmux browser

[← Back to blog](https://cmux.com/blog)


[![](https://cmux.com/_next/image?url=%2Favatars%2Flawrencecchen.jpg&w=64&q=75&dpl=dpl_AT5HfVg9fuRYjj7rCgqhY9VxGYH8)Lawrence Chen@lawrencecchen](https://x.com/lawrencecchen)May 22, 2026

cmux's browser now supports passkey authentication. Sign-in flows that depend on passkeys, WebAuthn, Touch ID, or hardware security keys can complete inside a cmux browser pane.

If you already use another browser for the same site, import compatible cookies once:

```
cmux browser import
```

Requires cmux v0.64+. This helps when Claude Code, Codex, OpenCode, Gemini CLI, or another agent needs to test an authenticated local app without leaving cmux.

[Download for Mac](https://cmux.com/download/confirmation?dl=1)

[View on GitHub](https://github.com/manaflow-ai/cmux)

[cmux Vault](https://cmux.com/blog/cmux-vault) [Task Manager in cmux](https://cmux.com/blog/task-manager)

Canonical: https://cmux.com/blog/passkey-auth
