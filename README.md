# WinMoji

WinMoji is a fast keyboard-first Unicode picker for Windows. It stays resident, opens a small native popup, searches offline, and inserts the selected character without using the clipboard.

The catalog contains the full Unicode 17 emoji set plus named Unicode symbols, arrows, math, currency, punctuation, Greek, shapes, common technical characters, and classic text emoticons. There are no GIFs, stickers, or network requests.

## Use

Run `winmoji.exe` to show the picker. The default global shortcut is `Ctrl+Shift+.`. On Dvorak, this is the physical QWERTY `E` position.

- Type to search.
- The search field shows a caret: Left, Right, Home, and End move it, Shift extends a selection, Ctrl+A selects everything, and Ctrl+V pastes text into the search.
- Misspellings, transposed letters, and stray punctuation are matched automatically.
- Empty search results are ordered by most recent use.
- Press Ctrl+Backspace to delete the previous search word, or click the clear button to empty the field.
- Press Up or Down, or Ctrl+J or Ctrl+K, to select a result.
- Press Enter to insert the selected text and keep the picker open.
- Press Ctrl+Enter to insert the selected text and close the picker.
- Clear the search or press Ctrl+G to browse the continuously scrolling catalog in the same window.
- Emoji follow the Unicode CLDR groups for smileys, people, animals, food, travel, activities, objects, and flags. One Symbols group contains emoji symbols plus the broad Unicode symbol catalog, followed by a dedicated Emoticons group.
- In the grid, use arrow keys to move, Ctrl+H or Ctrl+L to change category, Page Up or Page Down to scroll, the mouse wheel to browse smoothly, or click a category button to jump. The category rail scrolls with a vertical or horizontal wheel or trackpad gesture while hovering it, with its edge buttons, or by clicking.
- Drag the scrollbar for direct continuous positioning. Category focus follows the visible section while scrolling.
- Grid rows are rendered on demand for the visible viewport, so large categories do not create work for offscreen emoji during scrolling.
- Hover or focus a character to show its configured name, code point, and type in the footer.
- Hover category icons and buttons for their labels and shortcuts, including the Enter and Ctrl+Enter insert hints.
- Press Ctrl+, or click the settings button to configure the picker.
- Press Escape, click outside the picker, or switch to another window to close without inserting.
- PrintScreen and Win-key shortcuts pass through while the picker is open, so system screenshots keep working.

WinMoji opens without activating its window. Search and navigation keys are captured while the original control remains focused, so transient fields such as inline rename editors stay open. WinMoji releases keyboard capture before inserting UTF-16 input into that exact control. It closes if another application becomes active. Windows UIPI blocks input into an elevated application when WinMoji is not elevated. Applications with custom controls can ignore `KEYEVENTF_UNICODE`; WinMoji cancels rather than sending text to a different window.

The responsive popup uses Direct2D and DirectWrite. Settings provide continuous width and height controls, Segoe UI Emoji color glyphs, or monochrome Segoe UI Symbol glyphs. Search, browse, insert, and settings actions have matching clickable controls, including a clear button in the search field.

## Configure

The optional configuration file is `%APPDATA%\winmoji\config.toml`:

```toml
hotkey = "Ctrl+Shift+."
width = 440
height = 380
details = "both"
emoji_font = "Segoe UI Emoji"
```

Supported modifiers are `Ctrl`, `Alt`, `Shift`, and `Win`. The key can be a letter, digit, `F1` through `F24`, `Space`, `Enter`, `Tab`, `Escape`, or common punctuation. Punctuation accepts its literal form or the names `period`, `comma`, `slash`, `backslash`, `semicolon`, `apostrophe`, `minus`, `equals`, `left bracket`, `right bracket`, and `grave`. The configured shortcut always uses `MOD_NOREPEAT`.

`width` accepts 360 through 920. `height` accepts 300 through 760. Values are clamped to the active monitor work area. `details` accepts `none`, `type`, `codepoint`, or `both`. `emoji_font` accepts `Segoe UI Emoji` or `Segoe UI Symbol`.

The settings panel changes all of these values without editing the file. Escape and Enter save and return to the picker. Discard restores the values from when settings opened. Reset restores stock defaults and keeps settings open for inspection.

## Start with Windows

`winmoji.exe --install` copies the executable to `%LOCALAPPDATA%\Programs\WinMoji\winmoji.exe`, adds that stable path to the current user's `HKCU` Run entry, and starts the hotkey listener immediately. `winmoji.exe --uninstall` removes the startup entry. Both operations are idempotent and require no elevation. Add `--dry-run` to inspect the target without changing the registry.

## Build and verify

```text
cargo +stable-x86_64-pc-windows-msvc test
cargo +stable-x86_64-pc-windows-msvc clippy --all-targets -- -D warnings
cargo +stable-x86_64-pc-windows-msvc build --release --target aarch64-pc-windows-msvc --features console
target\aarch64-pc-windows-msvc\release\winmoji.exe --self-test
target\aarch64-pc-windows-msvc\release\winmoji.exe --benchmark
cargo +stable-x86_64-pc-windows-msvc build --release --target aarch64-pc-windows-msvc
```

The `console` feature keeps diagnostic output attached to the terminal. `--self-test` needs an interactive desktop session because Windows rejects foreground changes from background runners. The final command produces the default Windows-subsystem binary, so startup and ordinary launches do not create a console window. The MSVC toolchain and Windows SDK must support the selected target. `x86_64-pc-windows-msvc` is also supported.

## Command line

Run `winmoji.exe --help` for the complete command list. A second ordinary launch signals the resident instance to show its existing popup. `--startup` starts the hotkey listener without opening the popup. `--preview` keeps the window visible when it loses focus for visual inspection.

## License

MIT. Third-party attribution is in `THIRD_PARTY_NOTICES.md`.
