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

const RECENT_LIMIT: usize = 32;

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

impl Hotkey {
    pub fn parse(value: &str) -> Result<Self, String> {
        let mut modifiers = MOD_NOREPEAT_VALUE;
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
                        return Err(format!("unsupported hotkey key: {raw_part}"));
                    }
                }
                _ => return Err(format!("unsupported hotkey component: {raw_part}")),
            }
        }

        Self::from_parts(modifiers, virtual_key.ok_or("hotkey requires one key")?)
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
        Err("hotkey contains more than one key".to_string())
    } else {
        Ok(())
    }
}

impl fmt::Display for Hotkey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = Vec::new();
        if self.modifiers & MOD_CONTROL_VALUE != 0 {
            parts.push("Ctrl".to_string());
        }
        if self.modifiers & MOD_ALT_VALUE != 0 {
            parts.push("Alt".to_string());
        }
        if self.modifiers & MOD_SHIFT_VALUE != 0 {
            parts.push("Shift".to_string());
        }
        if self.modifiers & MOD_WIN_VALUE != 0 {
            parts.push("Win".to_string());
        }
        parts.push(match self.virtual_key {
            0x20 => "Space".to_string(),
            0x0d => "Enter".to_string(),
            0x09 => "Tab".to_string(),
            0x1b => "Escape".to_string(),
            0x70..=0x87 => format!("F{}", self.virtual_key - 0x6f),
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
            value if (0x30..=0x5a).contains(&value) => {
                char::from_u32(value).unwrap_or('?').to_string()
            }
            value => format!("0x{value:02X}"),
        });
        formatter.write_str(&parts.join("+"))
    }
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
pub struct Config {
    pub hotkey: Hotkey,
    pub dimensions: PickerDimensions,
    pub details: DetailMode,
    pub emoji_font: EmojiFont,
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
    let content = format!(
        "hotkey = \"{}\"\nwidth = {}\nheight = {}\ndetails = \"{}\"\nemoji_font = \"{}\"\n",
        config.hotkey,
        dimensions.width,
        dimensions.height,
        config.details.to_string().to_ascii_lowercase(),
        config.emoji_font,
    );
    fs::write(&path, content)
        .map_err(|error| format!("failed to write {}: {error}", path.display()))
}

pub fn load_recents() -> Vec<String> {
    recent_path()
        .ok()
        .and_then(|path| fs::read_to_string(path).ok())
        .map(|content| {
            content
                .lines()
                .filter(|line| !line.is_empty())
                .take(RECENT_LIMIT)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

pub fn remember_recent(recents: &mut Vec<String>, glyph: &str) -> Result<(), String> {
    touch_recent(recents, glyph);
    let path = recent_path().map_err(|error| error.to_string())?;
    write_recents(&path, recents)
}

fn touch_recent(recents: &mut Vec<String>, glyph: &str) {
    recents.retain(|existing| existing != glyph);
    recents.insert(0, glyph.to_string());
    recents.truncate(RECENT_LIMIT);
}

fn write_recents(path: &Path, recents: &[String]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("invalid recent-items path: {}", path.display()))?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    let mut content = recents.join("\n");
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

    #[test]
    fn recent_items_are_unique_and_newest_first() {
        let mut recents = vec!["😀".to_string(), "λ".to_string()];
        touch_recent(&mut recents, "λ");
        assert_eq!(recents, ["λ", "😀"]);
        touch_recent(&mut recents, "→");
        assert_eq!(recents, ["→", "λ", "😀"]);
    }

    #[test]
    fn rejects_ambiguous_or_unmodified_hotkeys() {
        assert!(Hotkey::parse("Ctrl+A+B").is_err());
        assert!(Hotkey::parse("Space").is_err());
        assert!(Hotkey::parse("Ctrl+F25").is_err());
    }
}
