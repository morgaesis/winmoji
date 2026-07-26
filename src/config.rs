use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub const MOD_ALT_VALUE: u32 = 0x0001;
pub const MOD_CONTROL_VALUE: u32 = 0x0002;
pub const MOD_SHIFT_VALUE: u32 = 0x0004;
pub const MOD_WIN_VALUE: u32 = 0x0008;
pub const MOD_NOREPEAT_VALUE: u32 = 0x4000;

pub const MIN_PICKER_WIDTH: i32 = 360;
pub const MAX_PICKER_WIDTH: i32 = 920;
pub const MIN_PICKER_HEIGHT: i32 = 300;
pub const MAX_PICKER_HEIGHT: i32 = 760;
pub const DEFAULT_PICKER_WIDTH: i32 = 440;
pub const DEFAULT_PICKER_HEIGHT: i32 = 380;

/// Text size as a percentage of the stock sizes. Everything the picker draws
/// scales together, so rows and grid cells grow with the text they hold.
pub const MIN_FONT_SCALE: i32 = 80;
pub const MAX_FONT_SCALE: i32 = 160;
pub const DEFAULT_FONT_SCALE: i32 = 100;
pub const FONT_SCALE_STEP: i32 = 5;

/// How many glyphs keep a usage count. Longer than the grid: an emoji picked
/// often should still outrank a more literal name match once it has fallen
/// off the Recent grid.
pub const USAGE_LIMIT: usize = 200;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Hotkey {
    pub modifiers: u32,
    pub virtual_key: u32,
}

impl Default for Hotkey {
    fn default() -> Self {
        Self {
            modifiers: MOD_CONTROL_VALUE | MOD_SHIFT_VALUE | MOD_NOREPEAT_VALUE,
            virtual_key: 0xbe,
        }
    }
}

/// Split `Ctrl+Shift+X` into its modifier mask and its one key. The global
/// hotkey and the in-picker bindings are written the same way and differ only
/// in what they accept afterwards, so they read the text through here.
fn parse_chord(value: &str) -> Result<(u32, u32), String> {
    let mut modifiers = 0;
    let mut virtual_key = None;

    for raw_part in value.split('+') {
        let part = raw_part.trim().to_ascii_lowercase();
        match part.as_str() {
            "ctrl" | "control" => modifiers |= MOD_CONTROL_VALUE,
            "alt" => modifiers |= MOD_ALT_VALUE,
            "shift" => modifiers |= MOD_SHIFT_VALUE,
            "win" | "windows" | "super" => modifiers |= MOD_WIN_VALUE,
            "space" => set_key(&mut virtual_key, 0x20)?,
            "enter" | "return" => set_key(&mut virtual_key, 0x0d)?,
            "tab" => set_key(&mut virtual_key, 0x09)?,
            "escape" | "esc" => set_key(&mut virtual_key, 0x1b)?,
            "backspace" => set_key(&mut virtual_key, 0x08)?,
            "delete" | "del" => set_key(&mut virtual_key, 0x2e)?,
            "insert" | "ins" => set_key(&mut virtual_key, 0x2d)?,
            "home" => set_key(&mut virtual_key, 0x24)?,
            "end" => set_key(&mut virtual_key, 0x23)?,
            "page up" | "pageup" | "pgup" => set_key(&mut virtual_key, 0x21)?,
            "page down" | "pagedown" | "pgdn" => set_key(&mut virtual_key, 0x22)?,
            "up" => set_key(&mut virtual_key, 0x26)?,
            "down" => set_key(&mut virtual_key, 0x28)?,
            "left" => set_key(&mut virtual_key, 0x25)?,
            "right" => set_key(&mut virtual_key, 0x27)?,
            "period" | "." => set_key(&mut virtual_key, 0xbe)?,
            "comma" | "," => set_key(&mut virtual_key, 0xbc)?,
            "slash" | "/" => set_key(&mut virtual_key, 0xbf)?,
            "backslash" | "\\" => set_key(&mut virtual_key, 0xdc)?,
            "semicolon" | ";" => set_key(&mut virtual_key, 0xba)?,
            "apostrophe" | "'" => set_key(&mut virtual_key, 0xde)?,
            "minus" | "-" => set_key(&mut virtual_key, 0xbd)?,
            "equals" | "=" => set_key(&mut virtual_key, 0xbb)?,
            "left bracket" | "[" => set_key(&mut virtual_key, 0xdb)?,
            "right bracket" | "]" => set_key(&mut virtual_key, 0xdd)?,
            "grave" | "`" => set_key(&mut virtual_key, 0xc0)?,
            key if function_key(key).is_some() => {
                set_key(&mut virtual_key, function_key(key).expect("checked above"))?;
            }
            key if parse_hex_key(key).is_some() => {
                set_key(&mut virtual_key, parse_hex_key(key).expect("checked above"))?;
            }
            key if key.len() == 1 => {
                let character = key.as_bytes()[0];
                if character.is_ascii_alphanumeric() {
                    set_key(&mut virtual_key, character.to_ascii_uppercase() as u32)?;
                } else {
                    return Err(format!("unsupported key: {raw_part}"));
                }
            }
            _ => return Err(format!("unsupported shortcut component: {raw_part}")),
        }
    }

    Ok((modifiers, virtual_key.ok_or("a shortcut needs one key")?))
}

/// Render a modifier mask and key back into `Ctrl+Shift+X`.
fn chord_label(modifiers: u32, virtual_key: u32) -> String {
    let mut parts = Vec::new();
    if modifiers & MOD_CONTROL_VALUE != 0 {
        parts.push("Ctrl".to_string());
    }
    if modifiers & MOD_ALT_VALUE != 0 {
        parts.push("Alt".to_string());
    }
    if modifiers & MOD_SHIFT_VALUE != 0 {
        parts.push("Shift".to_string());
    }
    if modifiers & MOD_WIN_VALUE != 0 {
        parts.push("Win".to_string());
    }
    parts.push(match virtual_key {
        0x08 => "Backspace".to_string(),
        0x09 => "Tab".to_string(),
        0x0d => "Enter".to_string(),
        0x1b => "Escape".to_string(),
        0x20 => "Space".to_string(),
        0x21 => "Page Up".to_string(),
        0x22 => "Page Down".to_string(),
        0x23 => "End".to_string(),
        0x24 => "Home".to_string(),
        0x25 => "Left".to_string(),
        0x26 => "Up".to_string(),
        0x27 => "Right".to_string(),
        0x28 => "Down".to_string(),
        0x2d => "Insert".to_string(),
        0x2e => "Delete".to_string(),
        0x70..=0x87 => format!("F{}", virtual_key - 0x6f),
        0xba => ";".to_string(),
        0xbb => "=".to_string(),
        0xbc => ",".to_string(),
        0xbd => "-".to_string(),
        0xbe => ".".to_string(),
        0xbf => "/".to_string(),
        0xc0 => "`".to_string(),
        0xdb => "[".to_string(),
        0xdc => "\\".to_string(),
        0xdd => "]".to_string(),
        0xde => "'".to_string(),
        value if (0x30..=0x5a).contains(&value) => char::from_u32(value).unwrap_or('?').to_string(),
        value => format!("0x{value:02X}"),
    });
    parts.join("+")
}

impl Hotkey {
    pub fn parse(value: &str) -> Result<Self, String> {
        let (modifiers, virtual_key) = parse_chord(value)?;
        Self::from_parts(modifiers, virtual_key)
    }

    pub fn from_parts(modifiers: u32, virtual_key: u32) -> Result<Self, String> {
        let modifiers = modifiers | MOD_NOREPEAT_VALUE;
        let active_modifiers =
            modifiers & (MOD_CONTROL_VALUE | MOD_ALT_VALUE | MOD_SHIFT_VALUE | MOD_WIN_VALUE);
        if active_modifiers == 0 {
            return Err("hotkey requires at least one modifier".to_string());
        }
        Ok(Self {
            modifiers,
            virtual_key,
        })
    }
}

fn set_key(slot: &mut Option<u32>, key: u32) -> Result<(), String> {
    if slot.replace(key).is_some() {
        Err("a shortcut holds more than one key".to_string())
    } else {
        Ok(())
    }
}

/// The action a `key_*` configuration line names.
fn action_for_key(name: &str) -> Option<Action> {
    let id = name.strip_prefix("key_")?;
    Action::ALL.into_iter().find(|action| action.id() == id)
}

impl fmt::Display for Hotkey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&chord_label(self.modifiers, self.virtual_key))
    }
}

/// A key and its modifiers, as pressed inside the picker. Unlike the global
/// hotkey these may be bare keys: Windows only insists on a modifier for
/// shortcuts it has to register system-wide.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Binding {
    pub modifiers: u32,
    pub virtual_key: u32,
}

impl Binding {
    pub const fn new(modifiers: u32, virtual_key: u32) -> Self {
        Self {
            modifiers,
            virtual_key,
        }
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        let (modifiers, virtual_key) = parse_chord(value)?;
        Self::from_parts(modifiers, virtual_key)
    }

    /// A binding must be reachable without swallowing typing. A bare letter,
    /// digit or punctuation key would shadow the search field, so those need
    /// a modifier; keys that never produce text are fine on their own.
    pub fn from_parts(modifiers: u32, virtual_key: u32) -> Result<Self, String> {
        let binding = Self::new(modifiers, virtual_key);
        if binding.modifiers == 0 && produces_text(virtual_key) {
            return Err(format!(
                "{binding} would be typed into the search field; add a modifier"
            ));
        }
        Ok(binding)
    }

    pub fn matches(self, virtual_key: u32, control: bool, shift: bool) -> bool {
        self.virtual_key == virtual_key
            && (self.modifiers & MOD_CONTROL_VALUE != 0) == control
            && (self.modifiers & MOD_SHIFT_VALUE != 0) == shift
    }
}

impl fmt::Display for Binding {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&chord_label(self.modifiers, self.virtual_key))
    }
}

/// Whether pressing this key on its own puts a character in the search field.
fn produces_text(virtual_key: u32) -> bool {
    matches!(virtual_key, 0x20 | 0x30..=0x5a | 0xba..=0xc0 | 0xdb..=0xde)
}

fn function_key(value: &str) -> Option<u32> {
    let number = value.strip_prefix('f')?.parse::<u32>().ok()?;
    (1..=24).contains(&number).then_some(0x6f + number)
}

fn parse_hex_key(value: &str) -> Option<u32> {
    let value = value.strip_prefix("0x")?;
    let key = u32::from_str_radix(value, 16).ok()?;
    (key <= 0xff).then_some(key)
}

/// Everything the picker does that answers to a key. The arrow keys always
/// move the selection and the search field always takes typing; those are the
/// primitives the rest is built on, so they are not rebindable.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Action {
    Insert,
    InsertKeep,
    Copy,
    CopyKeep,
    Dismiss,
    Settings,
    Browse,
    SelectUp,
    SelectDown,
    SelectLeft,
    SelectRight,
    HalfPageUp,
    HalfPageDown,
    PageUp,
    PageDown,
    TextBigger,
    TextSmaller,
}

impl Action {
    pub const ALL: [Self; 17] = [
        Self::Insert,
        Self::InsertKeep,
        Self::Copy,
        Self::CopyKeep,
        Self::Dismiss,
        Self::Settings,
        Self::Browse,
        Self::SelectUp,
        Self::SelectDown,
        Self::SelectLeft,
        Self::SelectRight,
        Self::HalfPageUp,
        Self::HalfPageDown,
        Self::PageUp,
        Self::PageDown,
        Self::TextBigger,
        Self::TextSmaller,
    ];

    /// Suffix of this action's `key_*` line in the configuration file.
    pub fn id(self) -> &'static str {
        match self {
            Self::Insert => "insert",
            Self::InsertKeep => "insert_keep",
            Self::Copy => "copy",
            Self::CopyKeep => "copy_keep",
            Self::Dismiss => "dismiss",
            Self::Settings => "settings",
            Self::Browse => "browse",
            Self::SelectUp => "select_up",
            Self::SelectDown => "select_down",
            Self::SelectLeft => "select_left",
            Self::SelectRight => "select_right",
            Self::HalfPageUp => "half_page_up",
            Self::HalfPageDown => "half_page_down",
            Self::PageUp => "page_up",
            Self::PageDown => "page_down",
            Self::TextBigger => "text_bigger",
            Self::TextSmaller => "text_smaller",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Insert => "Insert",
            Self::InsertKeep => "Insert, keep open",
            Self::Copy => "Copy",
            Self::CopyKeep => "Copy, keep open",
            Self::Dismiss => "Close",
            Self::Settings => "Settings",
            Self::Browse => "Browse catalog",
            Self::SelectUp => "Select up",
            Self::SelectDown => "Select down",
            Self::SelectLeft => "Select left",
            Self::SelectRight => "Select right",
            Self::HalfPageUp => "Half page up",
            Self::HalfPageDown => "Half page down",
            Self::PageUp => "Scroll page up",
            Self::PageDown => "Scroll page down",
            Self::TextBigger => "Larger text",
            Self::TextSmaller => "Smaller text",
        }
    }

    pub const fn default_binding(self) -> Binding {
        const CTRL: u32 = MOD_CONTROL_VALUE;
        const SHIFT: u32 = MOD_SHIFT_VALUE;
        match self {
            Self::Insert => Binding::new(0, 0x0d),
            Self::InsertKeep => Binding::new(SHIFT, 0x0d),
            Self::Copy => Binding::new(CTRL, 0x43),
            Self::CopyKeep => Binding::new(CTRL | SHIFT, 0x43),
            Self::Dismiss => Binding::new(0, 0x1b),
            Self::Settings => Binding::new(CTRL, 0xbc),
            Self::Browse => Binding::new(CTRL, 0x47),
            Self::SelectUp => Binding::new(CTRL, 0x4b),
            Self::SelectDown => Binding::new(CTRL, 0x4a),
            Self::SelectLeft => Binding::new(CTRL, 0x48),
            Self::SelectRight => Binding::new(CTRL, 0x4c),
            Self::HalfPageUp => Binding::new(CTRL, 0x55),
            Self::HalfPageDown => Binding::new(CTRL, 0x44),
            Self::PageUp => Binding::new(0, 0x21),
            Self::PageDown => Binding::new(0, 0x22),
            Self::TextBigger => Binding::new(CTRL, 0xbb),
            Self::TextSmaller => Binding::new(CTRL, 0xbd),
        }
    }

    fn index(self) -> usize {
        Self::ALL
            .iter()
            .position(|action| *action == self)
            .expect("every action is in ALL")
    }
}

/// The binding for every action, in `Action::ALL` order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Keybinds([Binding; Action::ALL.len()]);

impl Default for Keybinds {
    fn default() -> Self {
        let mut bindings = [Action::Insert.default_binding(); Action::ALL.len()];
        let mut index = 0;
        while index < Action::ALL.len() {
            bindings[index] = Action::ALL[index].default_binding();
            index += 1;
        }
        Self(bindings)
    }
}

impl Keybinds {
    pub fn get(&self, action: Action) -> Binding {
        self.0[action.index()]
    }

    /// Point `action` at `binding`, refusing a chord another action already
    /// owns. Two actions sharing a chord would make one of them unreachable
    /// and which one won would depend on lookup order.
    pub fn set(&mut self, action: Action, binding: Binding) -> Result<(), String> {
        if let Some(existing) = self.conflict(action, binding) {
            return Err(format!("{binding} is already {}", existing.label()));
        }
        self.0[action.index()] = binding;
        Ok(())
    }

    pub fn conflict(&self, action: Action, binding: Binding) -> Option<Action> {
        Action::ALL
            .iter()
            .find(|other| **other != action && self.get(**other) == binding)
            .copied()
    }

    /// The action this key press runs, if any.
    pub fn action_for(&self, virtual_key: u32, control: bool, shift: bool) -> Option<Action> {
        Action::ALL
            .iter()
            .find(|action| self.get(**action).matches(virtual_key, control, shift))
            .copied()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PickerDimensions {
    pub width: i32,
    pub height: i32,
}

impl Default for PickerDimensions {
    fn default() -> Self {
        Self {
            width: DEFAULT_PICKER_WIDTH,
            height: DEFAULT_PICKER_HEIGHT,
        }
    }
}

impl PickerDimensions {
    pub fn clamped(self) -> Self {
        Self {
            width: self.width.clamp(MIN_PICKER_WIDTH, MAX_PICKER_WIDTH),
            height: self.height.clamp(MIN_PICKER_HEIGHT, MAX_PICKER_HEIGHT),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum DetailMode {
    None,
    Type,
    Codepoint,
    #[default]
    Both,
}

impl DetailMode {
    pub fn next(self, delta: isize) -> Self {
        let values = [Self::None, Self::Type, Self::Codepoint, Self::Both];
        let current: usize = match self {
            Self::None => 0,
            Self::Type => 1,
            Self::Codepoint => 2,
            Self::Both => 3,
        };
        values[current.saturating_add_signed(delta).min(values.len() - 1)]
    }

    fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "none" => Ok(Self::None),
            "type" => Ok(Self::Type),
            "codepoint" | "codepoints" => Ok(Self::Codepoint),
            "both" => Ok(Self::Both),
            other => Err(format!("unsupported detail mode: {other}")),
        }
    }
}

impl fmt::Display for DetailMode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::None => "None",
            Self::Type => "Type",
            Self::Codepoint => "Codepoint",
            Self::Both => "Both",
        })
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum EmojiFont {
    #[default]
    SegoeEmoji,
    SegoeSymbol,
}

impl EmojiFont {
    pub fn next(self, delta: isize) -> Self {
        let values = [Self::SegoeEmoji, Self::SegoeSymbol];
        let current: usize = match self {
            Self::SegoeEmoji => 0,
            Self::SegoeSymbol => 1,
        };
        values[current.saturating_add_signed(delta).min(values.len() - 1)]
    }

    fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "segoe ui emoji" | "emoji" | "color" => Ok(Self::SegoeEmoji),
            "segoe ui symbol" | "symbol" | "monochrome" => Ok(Self::SegoeSymbol),
            other => Err(format!("unsupported emoji font: {other}")),
        }
    }
}

impl fmt::Display for EmojiFont {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::SegoeEmoji => "Segoe UI Emoji",
            Self::SegoeSymbol => "Segoe UI Symbol",
        })
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SkinTone {
    #[default]
    Default,
    Light,
    MediumLight,
    Medium,
    MediumDark,
    Dark,
}

impl SkinTone {
    pub const ALL: [Self; 6] = [
        Self::Default,
        Self::Light,
        Self::MediumLight,
        Self::Medium,
        Self::MediumDark,
        Self::Dark,
    ];

    pub fn index(self) -> usize {
        Self::ALL.iter().position(|tone| *tone == self).unwrap_or(0)
    }

    pub fn next(self, delta: isize) -> Self {
        let next = self.index().saturating_add_signed(delta);
        Self::ALL[next.min(Self::ALL.len() - 1)]
    }

    pub fn cycled(self) -> Self {
        Self::ALL[(self.index() + 1) % Self::ALL.len()]
    }

    fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "default" | "none" | "yellow" => Ok(Self::Default),
            "light" => Ok(Self::Light),
            "medium-light" | "medium light" => Ok(Self::MediumLight),
            "medium" => Ok(Self::Medium),
            "medium-dark" | "medium dark" => Ok(Self::MediumDark),
            "dark" => Ok(Self::Dark),
            other => Err(format!("unsupported skin tone: {other}")),
        }
    }
}

impl fmt::Display for SkinTone {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Default => "Default",
            Self::Light => "Light",
            Self::MediumLight => "Medium-light",
            Self::Medium => "Medium",
            Self::MediumDark => "Medium-dark",
            Self::Dark => "Dark",
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Config {
    pub hotkey: Hotkey,
    pub keys: Keybinds,
    pub dimensions: PickerDimensions,
    pub font_scale: i32,
    pub details: DetailMode,
    pub emoji_font: EmojiFont,
    pub skin_tone: SkinTone,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            hotkey: Hotkey::default(),
            keys: Keybinds::default(),
            dimensions: PickerDimensions::default(),
            font_scale: DEFAULT_FONT_SCALE,
            details: DetailMode::default(),
            emoji_font: EmojiFont::default(),
            skin_tone: SkinTone::default(),
        }
    }
}

impl Config {
    /// The text scale as a multiplier, clamped to the supported range.
    pub fn scale(self) -> f32 {
        self.font_scale.clamp(MIN_FONT_SCALE, MAX_FONT_SCALE) as f32 / 100.0
    }
}

pub fn config_path() -> io::Result<PathBuf> {
    app_data_directory().map(|path| path.join("config.toml"))
}

pub fn recent_path() -> io::Result<PathBuf> {
    app_data_directory().map(|path| path.join("recent.txt"))
}

fn app_data_directory() -> io::Result<PathBuf> {
    let base = std::env::var_os("APPDATA").ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "APPDATA is unavailable; cannot locate WinMoji settings",
        )
    })?;
    Ok(PathBuf::from(base).join("winmoji"))
}

pub fn load_config() -> Result<Config, String> {
    let path = config_path().map_err(|error| error.to_string())?;
    let content = match fs::read_to_string(&path) {
        Ok(content) => content,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Config::default()),
        Err(error) => return Err(format!("failed to read {}: {error}", path.display())),
    };

    parse_config(&content)
}

pub fn load_hotkey() -> Result<Hotkey, String> {
    load_config().map(|config| config.hotkey)
}

fn parse_config(content: &str) -> Result<Config, String> {
    let mut config = Config::default();
    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(format!("invalid config line: {line}"));
        };
        let value = value.trim().trim_matches('"');
        match key.trim() {
            "hotkey" => config.hotkey = Hotkey::parse(value)?,
            name if action_for_key(name).is_some() => {
                let action = action_for_key(name).expect("checked above");
                // A conflicting file is reported rather than silently
                // dropped, so an unreachable action is visible at startup.
                config.keys.set(action, Binding::parse(value)?)?;
            }
            "font_scale" => {
                config.font_scale = value
                    .parse::<i32>()
                    .map_err(|_| format!("invalid font scale: {value}"))?
                    .clamp(MIN_FONT_SCALE, MAX_FONT_SCALE)
            }
            "width" => {
                config.dimensions.width = value
                    .parse()
                    .map_err(|_| format!("invalid picker width: {value}"))?
            }
            "height" => {
                config.dimensions.height = value
                    .parse()
                    .map_err(|_| format!("invalid picker height: {value}"))?
            }
            "size" => {
                config.dimensions = match value.to_ascii_lowercase().as_str() {
                    "compact" => PickerDimensions {
                        width: 520,
                        height: 408,
                    },
                    "medium" => PickerDimensions {
                        width: 620,
                        height: 512,
                    },
                    "large" => PickerDimensions {
                        width: 740,
                        height: 620,
                    },
                    other => return Err(format!("unsupported picker size: {other}")),
                }
            }
            "details" => config.details = DetailMode::parse(value)?,
            "emoji_font" => config.emoji_font = EmojiFont::parse(value)?,
            "skin_tone" => config.skin_tone = SkinTone::parse(value)?,
            _ => {}
        }
    }
    config.dimensions = config.dimensions.clamped();
    Ok(config)
}

pub fn save_config(config: Config) -> Result<(), String> {
    let path = config_path().map_err(|error| error.to_string())?;
    let parent = path
        .parent()
        .ok_or_else(|| format!("invalid config path: {}", path.display()))?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    let dimensions = config.dimensions.clamped();
    let keys = Action::ALL
        .iter()
        .map(|action| format!("key_{} = \"{}\"\n", action.id(), config.keys.get(*action)))
        .collect::<String>();
    let content = format!(
        "hotkey = \"{}\"\nwidth = {}\nheight = {}\nfont_scale = {}\ndetails = \"{}\"\nemoji_font = \"{}\"\nskin_tone = \"{}\"\n",
        config.hotkey,
        dimensions.width,
        dimensions.height,
        config.font_scale.clamp(MIN_FONT_SCALE, MAX_FONT_SCALE),
        config.details.to_string().to_ascii_lowercase(),
        config.emoji_font,
        config.skin_tone.to_string().to_ascii_lowercase(),
    );
    let content = content + &keys;
    fs::write(&path, content)
        .map_err(|error| format!("failed to write {}: {error}", path.display()))
}

/// One glyph the user has picked, with how often. The list is ordered most
/// recent first: the Recent grid reads the order, search ranking reads the
/// counts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecentGlyph {
    pub glyph: String,
    pub uses: u32,
}

pub fn load_recents() -> Vec<RecentGlyph> {
    recent_path()
        .ok()
        .and_then(|path| fs::read_to_string(path).ok())
        .map(|content| {
            content
                .lines()
                .filter(|line| !line.is_empty())
                .take(USAGE_LIMIT)
                .map(parse_recent)
                .collect()
        })
        .unwrap_or_default()
}

/// `uses`, a tab, then the glyph. A line with no tab was written before
/// counts existed and stands for a single use.
fn parse_recent(line: &str) -> RecentGlyph {
    match line.split_once('\t') {
        Some((uses, glyph)) => RecentGlyph {
            glyph: glyph.to_string(),
            uses: uses.parse().unwrap_or(1).max(1),
        },
        None => RecentGlyph {
            glyph: line.to_string(),
            uses: 1,
        },
    }
}

pub fn remember_recent(recents: &mut Vec<RecentGlyph>, glyph: &str) -> Result<(), String> {
    touch_recent(recents, glyph);
    let path = recent_path().map_err(|error| error.to_string())?;
    write_recents(&path, recents)
}

fn touch_recent(recents: &mut Vec<RecentGlyph>, glyph: &str) {
    let uses = recents
        .iter()
        .find(|existing| existing.glyph == glyph)
        .map_or(0, |existing| existing.uses)
        .saturating_add(1);
    recents.retain(|existing| existing.glyph != glyph);
    recents.insert(
        0,
        RecentGlyph {
            glyph: glyph.to_string(),
            uses,
        },
    );
    recents.truncate(USAGE_LIMIT);
}

fn write_recents(path: &Path, recents: &[RecentGlyph]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("invalid recent-items path: {}", path.display()))?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    let mut content = recents
        .iter()
        .map(|recent| format!("{}\t{}", recent.uses, recent.glyph))
        .collect::<Vec<_>>()
        .join("\n");
    if !content.is_empty() {
        content.push('\n');
    }
    fs::write(path, content).map_err(|error| format!("failed to write {}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_default_hotkey() {
        let hotkey = Hotkey::parse("Ctrl+Shift+.").expect("hotkey should parse");
        assert_eq!(hotkey, Hotkey::default());
        assert_eq!(hotkey.to_string(), "Ctrl+Shift+.");
    }

    #[test]
    fn parses_letter_and_hex_hotkeys() {
        let hotkey = Hotkey::parse("win+shift+u").expect("hotkey should parse");
        assert_eq!(hotkey.virtual_key, b'U' as u32);
        assert_eq!(
            hotkey.modifiers,
            MOD_WIN_VALUE | MOD_SHIFT_VALUE | MOD_NOREPEAT_VALUE
        );
        assert_eq!(
            Hotkey::parse("Ctrl+0xE2")
                .expect("hex hotkey should parse")
                .virtual_key,
            0xe2
        );
    }

    #[test]
    fn parses_function_and_punctuation_hotkeys() {
        assert_eq!(
            Hotkey::parse("Ctrl+Alt+F24")
                .expect("function key should parse")
                .to_string(),
            "Ctrl+Alt+F24"
        );
        assert_eq!(
            Hotkey::parse("Win+left bracket")
                .expect("named punctuation should parse")
                .to_string(),
            "Win+["
        );
    }

    #[test]
    fn parses_continuous_display_settings() {
        let config = parse_config(
            "hotkey = \"Ctrl+Shift+.\"\nwidth = 487\nheight = 533\ndetails = \"codepoint\"\nemoji_font = \"Segoe UI Symbol\"\n",
        )
        .expect("config should parse");
        assert_eq!(
            config.dimensions,
            PickerDimensions {
                width: 487,
                height: 533
            }
        );
        assert_eq!(config.details, DetailMode::Codepoint);
        assert_eq!(config.emoji_font, EmojiFont::SegoeSymbol);
    }

    #[test]
    fn migrates_old_size_presets_and_clamps_dimensions() {
        let old = parse_config("size = \"compact\"\n").expect("old config should parse");
        assert_eq!(
            old.dimensions,
            PickerDimensions {
                width: 520,
                height: 408
            }
        );
        let clamped = parse_config("width = 2000\nheight = 20\n").expect("config should clamp");
        assert_eq!(
            clamped.dimensions,
            PickerDimensions {
                width: MAX_PICKER_WIDTH,
                height: MIN_PICKER_HEIGHT
            }
        );
    }

    #[test]
    fn cycles_display_options_without_wrapping() {
        assert_eq!(DetailMode::None.next(-1), DetailMode::None);
        assert_eq!(DetailMode::Both.next(1), DetailMode::Both);
        assert_eq!(EmojiFont::SegoeEmoji.next(1), EmojiFont::SegoeSymbol);
        assert_eq!(EmojiFont::SegoeSymbol.next(1), EmojiFont::SegoeSymbol);
    }

    fn recent(glyph: &str, uses: u32) -> RecentGlyph {
        RecentGlyph {
            glyph: glyph.to_string(),
            uses,
        }
    }

    #[test]
    fn recent_items_are_unique_and_newest_first() {
        let mut recents = vec![recent("😀", 3), recent("λ", 1)];
        touch_recent(&mut recents, "λ");
        assert_eq!(recents, [recent("λ", 2), recent("😀", 3)]);
        touch_recent(&mut recents, "→");
        assert_eq!(recents, [recent("→", 1), recent("λ", 2), recent("😀", 3)]);
    }

    /// A store written before counts existed still loads, with every glyph
    /// standing for a single use.
    #[test]
    fn recent_items_parse_with_and_without_counts() {
        assert_eq!(parse_recent("7\t😀"), recent("😀", 7));
        assert_eq!(parse_recent("😀"), recent("😀", 1));
        assert_eq!(parse_recent("0\t😀"), recent("😀", 1));
    }

    #[test]
    fn config_file_carries_rebound_actions() {
        let config = parse_config(
            "hotkey = \"Ctrl+Shift+.\"\nkey_insert = \"Ctrl+Alt+I\"\nkey_page_down = \"F8\"\n",
        )
        .expect("config should parse");
        assert_eq!(
            config.keys.get(Action::Insert),
            Binding::parse("Ctrl+Alt+I").expect("binding should parse")
        );
        assert_eq!(config.keys.get(Action::PageDown), Binding::new(0, 0x77));
        // Actions the file says nothing about keep their defaults.
        assert_eq!(
            config.keys.get(Action::Copy),
            Action::Copy.default_binding()
        );
        // A file that would make two actions share a chord is rejected, not
        // silently applied with one of them unreachable.
        assert!(parse_config("key_browse = \"Ctrl+C\"\n").is_err());
    }

    #[test]
    fn default_bindings_are_unique_and_round_trip() {
        let keys = Keybinds::default();
        for action in Action::ALL {
            let binding = keys.get(action);
            assert_eq!(
                keys.conflict(action, binding),
                None,
                "{} shares its chord",
                action.label()
            );
            assert_eq!(
                Binding::parse(&binding.to_string()),
                Ok(binding),
                "{binding} does not survive a round trip"
            );
        }
    }

    #[test]
    fn bindings_resolve_the_action_they_name() {
        let keys = Keybinds::default();
        // Enter alone inserts; adding Shift is a different action entirely.
        assert_eq!(keys.action_for(0x0d, false, false), Some(Action::Insert));
        assert_eq!(keys.action_for(0x0d, false, true), Some(Action::InsertKeep));
        assert_eq!(keys.action_for(0x43, true, false), Some(Action::Copy));
        assert_eq!(keys.action_for(0x43, true, true), Some(Action::CopyKeep));
        assert_eq!(keys.action_for(0x41, false, false), None);
    }

    #[test]
    fn rebinding_refuses_a_chord_another_action_owns() {
        let mut keys = Keybinds::default();
        let copy = keys.get(Action::Copy);
        assert!(keys.set(Action::Browse, copy).is_err());
        assert_eq!(keys.get(Action::Browse), Action::Browse.default_binding());
        // Rebinding an action to the chord it already has is not a conflict.
        assert!(keys.set(Action::Copy, copy).is_ok());
    }

    /// A bare key that types would be swallowed by the search field, so it is
    /// refused; keys that never produce text are fine unmodified.
    #[test]
    fn bindings_reject_bare_typing_keys() {
        assert!(Binding::parse("A").is_err());
        assert!(Binding::parse("Space").is_err());
        assert!(Binding::parse("Ctrl+A").is_ok());
        assert!(Binding::parse("Enter").is_ok());
        assert!(Binding::parse("Page Down").is_ok());
        assert!(Binding::parse("F5").is_ok());
    }

    #[test]
    fn rejects_ambiguous_or_unmodified_hotkeys() {
        assert!(Hotkey::parse("Ctrl+A+B").is_err());
        assert!(Hotkey::parse("Space").is_err());
        assert!(Hotkey::parse("Ctrl+F25").is_err());
    }
}
