# TextBox (Beta)

TextBox is cmux's rich terminal input surface for composing prompts before sending them to a terminal.

TextBox is a beta feature. Its defaults and behavior may change while it is being tested.

## Defaults

The TextBox settings live in Settings > TextBox:

-   Show TextBox on New Terminals opens new workspaces, terminal tabs, and terminal splits with the TextBox visible while keeping focus in the terminal.
-   Focus TextBox on New Terminals puts keyboard focus in the TextBox. Focusing also shows the TextBox.
-   TextBox Max Lines caps the TextBox height before its content scrolls.

## cmux.json

Set the same defaults in ~/.config/cmux/cmux.json under terminal:

~/.config/cmux/cmux.json

```
{
  "terminal": {
    "showTextBoxOnNewTerminals": true,
    "focusTextBoxOnNewTerminals": false,
    "textBoxMaxLines": 10
  }
}
```

Restored sessions keep their saved TextBox visibility and focus state. These defaults apply only to newly created terminal surfaces.

[Configuration](https://cmux-docs-release.vercel.app/docs/configuration) [Session Restore](https://cmux-docs-release.vercel.app/docs/session-restore)

Canonical: https://cmux-docs-release.vercel.app/docs/textbox
