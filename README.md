# WinMoji

WinMoji is a fast keyboard-first Unicode picker for Windows. It stays resident, opens a small native popup, searches offline, and inserts the selected character without using the clipboard.

The catalog contains the full Unicode 17 emoji set plus named Unicode symbols, arrows, math, currency, punctuation, Greek, shapes, common technical characters, and classic text emoticons. There are no GIFs, stickers, or network requests.

## Use

Run `winmoji.exe` to show the picker. The default global shortcut is `Ctrl+Shift+.`. On Dvorak, this is the physical QWERTY `E` position.

- Type to search.
- The search field shows a caret: Left, Right, Home, and End move it, Shift extends a selection, Ctrl+A selects everything, and Ctrl+V pastes text into the search.
- Misspellings, transposed letters, and stray punctuation are matched automatically.
- Empty search results are ordered by most recent use.
- Results are ranked by how well they match and by what you have picked before, so choosing an entry for a query lifts it above rivals next time. The list scrolls, so matches below the fold stay reachable.
- Press Ctrl+Backspace to delete the previous search word, or click the clear button to empty the field.
- Press Up or Down, or Ctrl+J or Ctrl+K, to select a result.
- Press Enter to insert the selected text and close the picker.
- Press Shift+Enter to insert the selected text and keep the picker open.
- Press Ctrl+C to copy the selected text to the clipboard and close, or Ctrl+Shift+C to copy and keep the picker open. Copying works where inserting cannot, such as into an elevated application.
- The footer shows Copy and Insert beside a Shift cap. Holding Shift lights the cap and both buttons change to their keep-open form; clicking the cap holds that state without the key.
- Press Ctrl+= or Ctrl+- to resize the text. Everything scales together, and the size is saved.
- Clear the search or press Ctrl+G to browse the continuously scrolling catalog in the same window.
- Emoji follow the Unicode CLDR groups for smileys, people, animals, food, travel, activities, objects, and flags. Symbols holds the CLDR emoji symbols, Characters holds the broad Unicode catalog of arrows, math, currency, punctuation and Greek, and Emoticons has its own group.
- In the grid, use the arrow keys or Ctrl+H, Ctrl+J, Ctrl+K, and Ctrl+L to move, Ctrl+U and Ctrl+D to move the selection by half a page, Page Up and Page Down to scroll the view, the mouse wheel to browse smoothly, or click a category button to jump. The category rail scrolls with a vertical or horizontal wheel or trackpad gesture while hovering it, with its edge buttons, or by clicking.
- Right-click an emoji that supports skin tones to pick a variant for a single insert. The permanent default tone is a settings row and applies to display and inserts everywhere.
- Drag the scrollbar for direct continuous positioning; its grip grows under the pointer. Category focus follows the visible section while scrolling.
- Grid rows are rendered on demand for the visible viewport, so large categories do not create work for offscreen emoji during scrolling.
- Hover or focus a character to show its configured name, code point, and type in the footer.
- Hover category icons and buttons for their labels and shortcuts, including the Enter and Shift+Enter insert hints.
- Press Ctrl+, or click the settings button to configure the picker. The panel scrolls with the wheel, the scrollbar, Page Up and Page Down, or by moving the selection past an edge, so every row stays reachable at any window height.
- The Keyboard shortcuts row opens its page with Enter, an arrow key, or a click. It lists every action and the chord that runs it; Enter or a click rebinds the focused one, and a chord another action already owns is refused rather than taken. Reset restores the stock chords and writes them straight away, as rebinding does. The arrow keys always move the selection and the search field always takes typing, so neither is rebindable.
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
font_scale = 100
details = "both"
emoji_font = "Segoe UI Emoji"
skin_tone = "default"
```

Supported modifiers are `Ctrl`, `Alt`, `Shift`, and `Win`. The key can be a letter, digit, `F1` through `F24`, `Space`, `Enter`, `Tab`, `Escape`, or common punctuation. Punctuation accepts its literal form or the names `period`, `comma`, `slash`, `backslash`, `semicolon`, `apostrophe`, `minus`, `equals`, `left bracket`, `right bracket`, and `grave`. The configured shortcut always uses `MOD_NOREPEAT`.

`width` accepts 360 through 920. `height` accepts 300 through 760. `font_scale` is a percentage from 80 through 160. Each action's binding is a `key_<action>` line written in the same form as `hotkey`; unlike `hotkey` these may be bare keys, except for keys that would otherwise be typed into the search field. Values are clamped to the active monitor work area. `details` accepts `none`, `type`, `codepoint`, or `both`. `emoji_font` accepts `Segoe UI Emoji` or `Segoe UI Symbol`. `skin_tone` accepts `default`, `light`, `medium-light`, `medium`, `medium-dark`, or `dark`.

The settings panel changes all of these values without editing the file. Enter changes the focused value with wrap-around, and value changes preview immediately. Escape and the Back button save and return to the picker. Discard restores the values from when settings opened. Reset restores stock defaults and keeps settings open for inspection.

## Start with Windows

`winmoji.exe --install` copies the executable to `%LOCALAPPDATA%\Programs\WinMoji\winmoji.exe`, adds that stable path to the current user's `HKCU` Run entry, and starts the hotkey listener immediately. `winmoji.exe --uninstall` removes the startup entry. Both operations are idempotent and require no elevation. Add `--dry-run` to inspect the target without changing the registry.

## Build and verify

```text
cargo +stable-msvc test
cargo +stable-msvc clippy --all-targets -- -D warnings
cargo +stable-msvc build --release --features console
target\release\winmoji.exe --self-test
target\release\winmoji.exe --benchmark
cargo +stable-msvc build --release
```

WinMoji builds with the MSVC toolchain and a Windows SDK. `stable-msvc` resolves to the host's MSVC toolchain; `rustup default stable-msvc` removes the need for the prefix. `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc` are both supported, and adding `--target <triple>` cross-compiles to the other, placing the binary under `target\<triple>\release\`.

The `console` feature keeps diagnostic output attached to the terminal. `--self-test` needs an interactive desktop session because Windows rejects foreground changes from background runners. The final command produces the default Windows-subsystem binary, so startup and ordinary launches do not create a console window.

## Command line

Run `winmoji.exe --help` for the complete command list. A second ordinary launch signals the resident instance to show its existing popup. `--startup` starts the hotkey listener without opening the popup. `--preview` keeps the window visible when it loses focus for visual inspection.

## License

MIT. Third-party attribution is in `THIRD_PARTY_NOTICES.md`.
