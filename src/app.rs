use std::ffi::c_void;
use std::fs;
use std::mem::size_of;
use std::os::windows::ffi::OsStrExt;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{
    COLORREF, CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HGLOBAL, HINSTANCE, HWND, LPARAM,
    LRESULT, POINT, RECT, WAIT_OBJECT_0, WPARAM,
};
use windows::Win32::Graphics::Direct2D::Common::{
    D2D_RECT_F, D2D_SIZE_U, D2D1_ALPHA_MODE_UNKNOWN, D2D1_COLOR_F, D2D1_PIXEL_FORMAT,
};
use windows::Win32::Graphics::Direct2D::{
    D2D1_ANTIALIAS_MODE_ALIASED, D2D1_DRAW_TEXT_OPTIONS_CLIP,
    D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT, D2D1_DRAW_TEXT_OPTIONS_NONE, D2D1_ELLIPSE,
    D2D1_FACTORY_TYPE_SINGLE_THREADED, D2D1_FEATURE_LEVEL_DEFAULT,
    D2D1_HWND_RENDER_TARGET_PROPERTIES, D2D1_PRESENT_OPTIONS_NONE, D2D1_RENDER_TARGET_PROPERTIES,
    D2D1_RENDER_TARGET_TYPE_DEFAULT, D2D1_RENDER_TARGET_USAGE_NONE, D2D1_ROUNDED_RECT,
    D2D1CreateFactory, ID2D1Factory, ID2D1HwndRenderTarget, ID2D1SolidColorBrush,
};
use windows::Win32::Graphics::DirectWrite::{
    DWRITE_FACTORY_TYPE_SHARED, DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL,
    DWRITE_FONT_WEIGHT_NORMAL, DWRITE_FONT_WEIGHT_SEMI_BOLD, DWRITE_HIT_TEST_METRICS,
    DWRITE_MEASURING_MODE_NATURAL, DWRITE_PARAGRAPH_ALIGNMENT_CENTER, DWRITE_TEXT_ALIGNMENT_CENTER,
    DWRITE_TEXT_ALIGNMENT_LEADING, DWRITE_TEXT_ALIGNMENT_TRAILING, DWRITE_WORD_WRAPPING_NO_WRAP,
    DWriteCreateFactory, IDWriteFactory, IDWriteFontCollection, IDWriteFontFace, IDWriteTextFormat,
    IDWriteTextLayout,
};
use windows::Win32::Graphics::Dwm::{
    DWM_WINDOW_CORNER_PREFERENCE, DWMWA_BORDER_COLOR, DWMWA_USE_IMMERSIVE_DARK_MODE,
    DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND, DwmSetWindowAttribute,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_UNKNOWN;
use windows::Win32::Graphics::Gdi::{
    BeginPaint, EndPaint, GetMonitorInfoW, InvalidateRect, MONITOR_DEFAULTTONEAREST, MONITORINFO,
    MonitorFromPoint, PAINTSTRUCT, ScreenToClient,
};
use windows::Win32::Storage::FileSystem::{
    MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
};
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
    CoUninitialize,
};
use windows::Win32::System::DataExchange::{
    CloseClipboard, GetClipboardData, GetClipboardSequenceNumber, OpenClipboard,
};
use windows::Win32::System::Memory::{GlobalLock, GlobalUnlock};
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_SET_VALUE, REG_OPTION_NON_VOLATILE, REG_SZ, RegCloseKey,
    RegCreateKeyExW, RegDeleteValueW, RegSetValueExW,
};
use windows::Win32::System::Threading::{
    AttachThreadInput, CreateMutexW, GetCurrentThreadId, OpenProcess, PROCESS_SYNCHRONIZE, Sleep,
    WaitForSingleObject,
};
use windows::Win32::UI::Accessibility::{CLSID_AccPropServices, IAccPropServices, PROPID_ACC_NAME};
use windows::Win32::UI::Controls::{
    ICC_STANDARD_CLASSES, INITCOMMONCONTROLSEX, InitCommonControlsEx,
};
use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, GetDpiForWindow, SetProcessDpiAwarenessContext,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, GetKeyState, GetKeyboardLayout, GetKeyboardState, HOT_KEY_MODIFIERS, INPUT,
    INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE, MOD_NOREPEAT,
    RegisterHotKey, ReleaseCapture, SendInput, SetCapture, SetFocus, ToUnicodeEx, UnregisterHotKey,
    VIRTUAL_KEY, VK_BACK, VK_CONTROL, VK_DELETE, VK_DOWN, VK_END, VK_ESCAPE, VK_HOME, VK_LCONTROL,
    VK_LEFT, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_MENU, VK_NEXT, VK_PRIOR, VK_RCONTROL, VK_RETURN,
    VK_RIGHT, VK_RMENU, VK_RSHIFT, VK_RWIN, VK_SHIFT, VK_SNAPSHOT, VK_TAB, VK_UP,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CREATESTRUCTW, CS_DROPSHADOW, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, CallNextHookEx,
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, ES_AUTOHSCROLL, FindWindowW,
    GUITHREADINFO, GWLP_USERDATA, GetClientRect, GetCursorPos, GetForegroundWindow,
    GetGUIThreadInfo, GetMessageW, GetWindowLongPtrW, GetWindowRect, GetWindowTextLengthW,
    GetWindowTextW, GetWindowThreadProcessId, HHOOK, HMENU, IDC_ARROW, IsChild, IsWindow,
    KBDLLHOOKSTRUCT, KillTimer, LB_ADDSTRING, LB_GETCURSEL, LB_RESETCONTENT, LB_SETCURSEL,
    LBN_DBLCLK, LBN_SELCHANGE, LBS_HASSTRINGS, LBS_NOINTEGRALHEIGHT, LBS_NOTIFY, LLKHF_INJECTED,
    LWA_ALPHA, LoadCursorW, MSG, MSLLHOOKSTRUCT, OBJID_CLIENT, PostMessageW, PostQuitMessage,
    RegisterClassW, SW_HIDE, SW_SHOW, SW_SHOWNOACTIVATE, SWP_NOACTIVATE, SWP_NOZORDER,
    SetForegroundWindow, SetLayeredWindowAttributes, SetTimer, SetWindowLongPtrW, SetWindowPos,
    SetWindowsHookExW, ShowWindow, TranslateMessage, UnhookWindowsHookEx, WH_KEYBOARD_LL,
    WH_MOUSE_LL, WINDOW_STYLE, WM_APP, WM_CLOSE, WM_COMMAND, WM_DESTROY, WM_DPICHANGED,
    WM_ERASEBKGND, WM_HOTKEY, WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDOWN,
    WM_MOUSEHWHEEL, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_NCCREATE, WM_NCDESTROY, WM_PAINT,
    WM_RBUTTONDOWN, WM_SIZE, WM_SYSKEYDOWN, WM_SYSKEYUP, WM_TIMER, WM_XBUTTONDOWN, WNDCLASSW,
    WS_CHILD, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT,
    WS_OVERLAPPEDWINDOW, WS_POPUP, WS_TABSTOP, WS_VISIBLE,
};
#[cfg(not(feature = "console"))]
use windows::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MESSAGEBOX_STYLE, MessageBoxW};
use windows::core::{Error, HRESULT, PCWSTR, Result, w};
use windows_numerics::Vector2;

use crate::catalog::{self, Match};
use crate::config::{
    Config, DetailMode, EmojiFont, Hotkey, MAX_PICKER_HEIGHT, MAX_PICKER_WIDTH, MIN_PICKER_HEIGHT,
    MIN_PICKER_WIDTH, MOD_ALT_VALUE, MOD_CONTROL_VALUE, MOD_NOREPEAT_VALUE, MOD_SHIFT_VALUE,
    MOD_WIN_VALUE, PickerDimensions, load_config, load_recents, remember_recent, save_config,
};

const CLASS_NAME: PCWSTR = w!("WinMojiPickerWindow");
const WINDOW_TITLE: PCWSTR = w!("WinMoji");
const MUTEX_NAME: PCWSTR = w!("Local\\WinMoji.SingleInstance");
const RUN_KEY: PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
const RUN_VALUE: PCWSTR = w!("WinMoji");
const HOTKEY_ID: i32 = 0x574d;
const WM_SHOW_PICKER: u32 = WM_APP + 0x17;
const WM_CAPTURED_KEY: u32 = WM_APP + 0x18;
const WM_CAPTURE_TARGET_LOST: u32 = WM_APP + 0x19;
const SEARCH_TOP: i32 = 32;
const SEARCH_HEIGHT: i32 = 42;
const SEARCH_RESULTS_TOP: i32 = 80;
const CATEGORY_TOP: i32 = 80;
const CATEGORY_HEIGHT: i32 = 34;
const CATEGORY_BUTTON_WIDTH: f32 = 40.0;
const CATEGORY_EDGE_WIDTH: f32 = 24.0;
const BROWSE_CONTENT_TOP: i32 = 118;
const FOOTER_HEIGHT: i32 = 42;
const RESULT_ROW_HEIGHT: i32 = 40;
const GRID_CELL: i32 = 48;
const SECTION_HEADING_HEIGHT: i32 = 26;
const SECTION_GAP: i32 = 10;
const RESULTS_ID: usize = 2;
const VK_A_VALUE: u16 = 0x41;
const VK_D_VALUE: u16 = 0x44;
const VK_G_VALUE: u16 = 0x47;
const VK_U_VALUE: u16 = 0x55;
const VK_V_VALUE: u16 = 0x56;
const VK_H_VALUE: u16 = 0x48;
const VK_J_VALUE: u16 = 0x4a;
const VK_K_VALUE: u16 = 0x4b;
const VK_L_VALUE: u16 = 0x4c;
const VK_OEM_COMMA_VALUE: u16 = 0xbc;
const SCROLL_TIMER_ID: usize = 0x0057_4d01;
const FOCUS_TIMER_ID: usize = 0x0057_4d02;
const SCROLL_FRAME_MS: u32 = 16;
const FOCUS_FRAME_MS: u32 = 100;

static ACTIVE_PICKER: AtomicPtr<AppState> = AtomicPtr::new(std::ptr::null_mut());

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Mode {
    Run { startup: bool },
    Preview,
    Install { uninstall: bool, dry_run: bool },
    SelfTest,
    Benchmark,
    Help,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum View {
    Search,
    Settings,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BrowseCategory {
    Recent,
    Smileys,
    People,
    Animals,
    Food,
    Travel,
    Activities,
    Objects,
    Flags,
    Symbols,
    Emoticons,
}

impl BrowseCategory {
    const ALL: [Self; 11] = [
        Self::Recent,
        Self::Smileys,
        Self::People,
        Self::Animals,
        Self::Food,
        Self::Travel,
        Self::Activities,
        Self::Objects,
        Self::Flags,
        Self::Symbols,
        Self::Emoticons,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Recent => "Home and recent",
            Self::Smileys => "Smileys and emotion",
            Self::People => "People and body",
            Self::Animals => "Animals and nature",
            Self::Food => "Food and drink",
            Self::Travel => "Travel and places",
            Self::Activities => "Activities",
            Self::Objects => "Objects",
            Self::Flags => "Flags",
            Self::Symbols => "Symbols",
            Self::Emoticons => "Emoticons",
        }
    }

    fn heading(self) -> &'static str {
        match self {
            Self::Recent => "Recent",
            Self::Smileys => "Smileys and emotion",
            Self::People => "People and body",
            Self::Animals => "Animals and nature",
            Self::Food => "Food and drink",
            Self::Travel => "Travel and places",
            Self::Activities => "Activities",
            Self::Objects => "Objects",
            Self::Flags => "Flags",
            Self::Symbols => "Symbols",
            Self::Emoticons => "Emoticons",
        }
    }

    fn contains(self, entry: &catalog::Entry) -> bool {
        use emojis::Group;

        match self {
            Self::Recent => false,
            Self::Smileys => entry.emoji_group == Some(Group::SmileysAndEmotion),
            Self::People => entry.emoji_group == Some(Group::PeopleAndBody),
            Self::Animals => entry.emoji_group == Some(Group::AnimalsAndNature),
            Self::Food => entry.emoji_group == Some(Group::FoodAndDrink),
            Self::Travel => entry.emoji_group == Some(Group::TravelAndPlaces),
            Self::Activities => entry.emoji_group == Some(Group::Activities),
            Self::Objects => entry.emoji_group == Some(Group::Objects),
            Self::Flags => entry.emoji_group == Some(Group::Flags),
            Self::Symbols => {
                entry.emoji_group == Some(Group::Symbols)
                    || (entry.emoji_group.is_none() && entry.kind != "Emoticon")
            }
            Self::Emoticons => entry.kind == "Emoticon",
        }
    }

    fn icon(self) -> &'static str {
        match self {
            Self::Recent => "🏠",
            Self::Smileys => "😀",
            Self::People => "👋",
            Self::Animals => "🐻",
            Self::Food => "🍕",
            Self::Travel => "🚗",
            Self::Activities => "⚽",
            Self::Objects => "💡",
            Self::Flags => "⚑",
            Self::Symbols => "Ω",
            Self::Emoticons => "¯\\_(ツ)_/¯",
        }
    }

    fn uses_color_icon(self) -> bool {
        matches!(
            self,
            Self::Recent
                | Self::Smileys
                | Self::People
                | Self::Animals
                | Self::Food
                | Self::Travel
                | Self::Activities
                | Self::Objects
        )
    }
}

#[derive(Clone, Debug)]
struct BrowseSection {
    category: BrowseCategory,
    indices: Vec<usize>,
}

#[derive(Clone, Copy, Debug)]
struct SectionLayout {
    top: i32,
    grid_top: i32,
    bottom: i32,
    columns: usize,
    cell_width: f32,
    cell_height: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Slider {
    Width,
    Height,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum HitTarget {
    Close,
    Settings,
    Browse,
    SearchClear,
    Category(usize),
    CategoryScrollLeft,
    CategoryScrollRight,
    SearchResult(usize),
    BrowseItem { section: usize, item: usize },
    BrowseScrollbar,
    Insert,
    InsertClose,
    SettingRow(usize),
    WidthSlider,
    HeightSlider,
    SettingsDiscard,
    SettingsReset,
    SettingsBack,
}

struct TextFormats {
    label: IDWriteTextFormat,
    brand: IDWriteTextFormat,
    title: IDWriteTextFormat,
    metadata: IDWriteTextFormat,
    search: IDWriteTextFormat,
    glyph: IDWriteTextFormat,
    symbol: IDWriteTextFormat,
    math: IDWriteTextFormat,
    emoticon: IDWriteTextFormat,
    emoticon_small: IDWriteTextFormat,
    emoticon_icon: IDWriteTextFormat,
    icon: IDWriteTextFormat,
    center: IDWriteTextFormat,
    center_title: IDWriteTextFormat,
}

impl TextFormats {
    unsafe fn new(factory: &IDWriteFactory, emoji_font: EmojiFont) -> Result<Self> {
        let emoji_family = match emoji_font {
            EmojiFont::SegoeEmoji => w!("Segoe UI Emoji"),
            EmojiFont::SegoeSymbol => w!("Segoe UI Symbol"),
        };
        let glyph = unsafe { create_text_format(factory, emoji_family, 26.0, false)? };
        unsafe {
            glyph.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER)?;
        }
        let symbol = unsafe { create_text_format(factory, w!("Segoe UI Symbol"), 23.0, false)? };
        unsafe {
            symbol.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER)?;
        }
        let math = unsafe { create_text_format(factory, w!("Cambria Math"), 22.0, false)? };
        unsafe {
            math.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER)?;
        }
        let icon = unsafe { create_text_format(factory, w!("Segoe UI Symbol"), 14.0, false)? };
        unsafe {
            icon.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER)?;
        }
        let center =
            unsafe { create_text_format(factory, w!("Segoe UI Variable Text"), 12.0, false)? };
        unsafe {
            center.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER)?;
        }
        let center_title =
            unsafe { create_text_format(factory, w!("Segoe UI Variable Text"), 16.0, true)? };
        unsafe {
            center_title.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER)?;
        }
        let brand =
            unsafe { create_text_format(factory, w!("Segoe UI Variable Text"), 10.0, false)? };
        unsafe {
            brand.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_TRAILING)?;
        }
        let centered = |format: IDWriteTextFormat| -> Result<IDWriteTextFormat> {
            unsafe {
                format.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER)?;
            }
            Ok(format)
        };
        Ok(Self {
            label: unsafe {
                create_text_format(factory, w!("Segoe UI Variable Text"), 12.0, true)?
            },
            brand,
            title: unsafe {
                create_text_format(factory, w!("Segoe UI Variable Text"), 14.0, true)?
            },
            metadata: unsafe {
                create_text_format(factory, w!("Segoe UI Variable Text"), 11.0, false)?
            },
            search: unsafe {
                create_text_format(factory, w!("Segoe UI Variable Text"), 14.0, false)?
            },
            glyph,
            symbol,
            math,
            emoticon: centered(unsafe {
                create_text_format(factory, w!("Segoe UI Variable Text"), 14.0, false)?
            })?,
            emoticon_small: centered(unsafe {
                create_text_format(factory, w!("Segoe UI Variable Text"), 10.0, false)?
            })?,
            emoticon_icon: centered(unsafe {
                create_text_format(factory, w!("Segoe UI Variable Text"), 8.0, false)?
            })?,
            icon,
            center,
            center_title,
        })
    }
}

unsafe fn create_text_format(
    factory: &IDWriteFactory,
    family: PCWSTR,
    size: f32,
    semibold: bool,
) -> Result<IDWriteTextFormat> {
    let format = unsafe {
        factory.CreateTextFormat(
            family,
            None,
            if semibold {
                DWRITE_FONT_WEIGHT_SEMI_BOLD
            } else {
                DWRITE_FONT_WEIGHT_NORMAL
            },
            DWRITE_FONT_STYLE_NORMAL,
            DWRITE_FONT_STRETCH_NORMAL,
            size,
            w!("en-us"),
        )?
    };
    unsafe {
        format.SetWordWrapping(DWRITE_WORD_WRAPPING_NO_WRAP)?;
        format.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_LEADING)?;
        format.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER)?;
    }
    Ok(format)
}

unsafe fn build_displayable_entry_index(factory: &IDWriteFactory) -> Result<Vec<bool>> {
    let mut collection: Option<IDWriteFontCollection> = None;
    unsafe {
        factory.GetSystemFontCollection(&mut collection, false)?;
    }
    let collection = collection.ok_or_else(|| {
        Error::new(
            HRESULT(0x80004005u32 as i32),
            "DirectWrite did not return the system font collection",
        )
    })?;
    let faces = [
        w!("Segoe UI Emoji"),
        w!("Segoe UI Symbol"),
        w!("Segoe UI"),
        w!("Cambria Math"),
    ]
    .into_iter()
    .filter_map(|family| unsafe { system_font_face(&collection, family).ok() })
    .collect::<Vec<_>>();
    if faces.is_empty() {
        return Err(Error::new(
            HRESULT(0x80004005u32 as i32),
            "no picker font faces are available",
        ));
    }
    Ok(catalog::entries()
        .iter()
        .map(|entry| {
            if entry.kind == "Emoji" || entry.kind == "Emoticon" {
                return true;
            }
            entry.glyph.chars().all(|character| {
                faces
                    .iter()
                    .any(|face| unsafe { font_face_has_character(face, character) })
            })
        })
        .collect())
}

unsafe fn system_font_face(
    collection: &IDWriteFontCollection,
    family_name: PCWSTR,
) -> Result<IDWriteFontFace> {
    let mut index = 0;
    let mut exists = Default::default();
    unsafe {
        collection.FindFamilyName(family_name, &mut index, &mut exists)?;
    }
    if !exists.as_bool() {
        return Err(Error::new(
            HRESULT(0x80070002u32 as i32),
            "font family is unavailable",
        ));
    }
    let family = unsafe { collection.GetFontFamily(index)? };
    let font = unsafe {
        family.GetFirstMatchingFont(
            DWRITE_FONT_WEIGHT_NORMAL,
            DWRITE_FONT_STRETCH_NORMAL,
            DWRITE_FONT_STYLE_NORMAL,
        )?
    };
    unsafe { font.CreateFontFace() }
}

unsafe fn font_face_has_character(face: &IDWriteFontFace, character: char) -> bool {
    let codepoint = character as u32;
    let mut glyph = 0u16;
    unsafe { face.GetGlyphIndices(&codepoint, 1, &mut glyph).is_ok() && glyph != 0 }
}

/// Query text plus caret and selection, rendered by the picker itself.
///
/// The picker window never activates, so a native EDIT control could never
/// show a caret or receive focus-dependent shortcuts; the field state lives
/// here instead. Offsets are byte indices at char boundaries; `anchor` equals
/// `caret` when nothing is selected.
#[derive(Clone, Debug, Default)]
struct SearchField {
    text: String,
    caret: usize,
    anchor: usize,
    scroll: f32,
}

impl SearchField {
    fn selection(&self) -> (usize, usize) {
        (self.caret.min(self.anchor), self.caret.max(self.anchor))
    }

    fn has_selection(&self) -> bool {
        self.caret != self.anchor
    }

    fn clear(&mut self) {
        self.text.clear();
        self.caret = 0;
        self.anchor = 0;
        self.scroll = 0.0;
    }

    fn insert(&mut self, value: &str) {
        let (start, end) = self.selection();
        self.text.replace_range(start..end, value);
        self.caret = start + value.len();
        self.anchor = self.caret;
    }

    fn backspace(&mut self, word: bool) {
        if self.has_selection() {
            self.insert("");
            return;
        }
        if self.caret == 0 {
            return;
        }
        let start = if word {
            previous_word_boundary(&self.text, self.caret)
        } else {
            previous_char_boundary(&self.text, self.caret)
        };
        self.text.replace_range(start..self.caret, "");
        self.caret = start;
        self.anchor = start;
    }

    fn delete_forward(&mut self) {
        if self.has_selection() {
            self.insert("");
            return;
        }
        if self.caret < self.text.len() {
            let end = next_char_boundary(&self.text, self.caret);
            self.text.replace_range(self.caret..end, "");
        }
    }

    fn move_caret(&mut self, delta: isize, extend: bool) {
        if !extend && self.has_selection() {
            let (start, end) = self.selection();
            self.caret = if delta < 0 { start } else { end };
            self.anchor = self.caret;
            return;
        }
        self.caret = if delta < 0 {
            previous_char_boundary(&self.text, self.caret)
        } else {
            next_char_boundary(&self.text, self.caret)
        };
        if !extend {
            self.anchor = self.caret;
        }
    }

    fn move_home(&mut self, extend: bool) {
        self.caret = 0;
        if !extend {
            self.anchor = 0;
        }
    }

    fn move_end(&mut self, extend: bool) {
        self.caret = self.text.len();
        if !extend {
            self.anchor = self.caret;
        }
    }

    fn select_all(&mut self) {
        self.anchor = 0;
        self.caret = self.text.len();
    }
}

fn previous_char_boundary(text: &str, index: usize) -> usize {
    text[..index]
        .char_indices()
        .next_back()
        .map_or(0, |(offset, _)| offset)
}

fn next_char_boundary(text: &str, index: usize) -> usize {
    text[index..]
        .chars()
        .next()
        .map_or(text.len(), |character| index + character.len_utf8())
}

fn previous_word_boundary(text: &str, index: usize) -> usize {
    let mut end = index;
    while end > 0 {
        let previous = previous_char_boundary(text, end);
        if text[previous..end].chars().all(char::is_whitespace) {
            end = previous;
        } else {
            break;
        }
    }
    let mut start = end;
    while start > 0 {
        let previous = previous_char_boundary(text, start);
        if text[previous..start].chars().any(char::is_whitespace) {
            break;
        }
        start = previous;
    }
    start
}

struct AppState {
    hwnd: HWND,
    accessible_results: HWND,
    target: HWND,
    target_focus: HWND,
    search: SearchField,
    matches: Vec<Match>,
    selected: usize,
    recents: Vec<String>,
    config: Config,
    display_dimensions: PickerDimensions,
    view: View,
    browse_sections: Vec<BrowseSection>,
    displayable_entries: Vec<bool>,
    browse_focus: (usize, usize),
    browse_scroll: f32,
    browse_scroll_target: f32,
    category_scroll: f32,
    active_category: usize,
    hovered_entry: Option<usize>,
    hovered_target: Option<HitTarget>,
    settings_selected: usize,
    settings_original: Config,
    dragging_slider: Option<Slider>,
    dragging_scrollbar: Option<f32>,
    capturing_shortcut: bool,
    keyboard_hook: Option<HHOOK>,
    mouse_hook: Option<HHOOK>,
    keyboard_state: [u8; 256],
    pending_commit: Option<bool>,
    capture_active: bool,
    registered_hotkey: Hotkey,
    dpi: u32,
    status: Option<String>,
    d2d_factory: ID2D1Factory,
    dwrite_factory: IDWriteFactory,
    render: Option<(ID2D1HwndRenderTarget, Brushes)>,
    formats: TextFormats,
    keep_visible: bool,
}

#[derive(Clone)]
struct Brushes {
    surface: ID2D1SolidColorBrush,
    surface_border: ID2D1SolidColorBrush,
    selection: ID2D1SolidColorBrush,
    selection_border: ID2D1SolidColorBrush,
    glyph_surface: ID2D1SolidColorBrush,
    primary: ID2D1SolidColorBrush,
    secondary: ID2D1SolidColorBrush,
    accent: ID2D1SolidColorBrush,
    danger: ID2D1SolidColorBrush,
}

unsafe fn create_brushes(target: &ID2D1HwndRenderTarget) -> Result<Brushes> {
    unsafe {
        Ok(Brushes {
            surface: solid_brush(target, 0x1b1e25)?,
            surface_border: solid_brush(target, 0x30343e)?,
            selection: solid_brush(target, 0x2b3140)?,
            selection_border: solid_brush(target, 0x59647c)?,
            glyph_surface: solid_brush(target, 0x181b21)?,
            primary: solid_brush(target, 0xf4f6fb)?,
            secondary: solid_brush(target, 0x9ba3b4)?,
            accent: solid_brush(target, 0x9b8cff)?,
            danger: solid_brush(target, 0xff716c)?,
        })
    }
}

impl AppState {
    unsafe fn new(keep_visible: bool, config: Config) -> Result<Self> {
        let d2d_factory: ID2D1Factory =
            unsafe { D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)? };
        let dwrite_factory: IDWriteFactory =
            unsafe { DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)? };
        let formats = unsafe { TextFormats::new(&dwrite_factory, config.emoji_font)? };
        let displayable_entries = unsafe { build_displayable_entry_index(&dwrite_factory)? };
        let mut state = Self {
            hwnd: HWND::default(),
            accessible_results: HWND::default(),
            target: HWND::default(),
            target_focus: HWND::default(),
            search: SearchField::default(),
            matches: Vec::new(),
            selected: 0,
            recents: load_recents(),
            config,
            display_dimensions: config.dimensions,
            view: View::Search,
            browse_sections: Vec::new(),
            displayable_entries,
            browse_focus: (0, 0),
            browse_scroll: 0.0,
            browse_scroll_target: 0.0,
            category_scroll: 0.0,
            active_category: 0,
            hovered_entry: None,
            hovered_target: None,
            settings_selected: 0,
            settings_original: config,
            dragging_slider: None,
            dragging_scrollbar: None,
            capturing_shortcut: false,
            keyboard_hook: None,
            mouse_hook: None,
            keyboard_state: [0; 256],
            pending_commit: None,
            capture_active: false,
            registered_hotkey: config.hotkey,
            dpi: 96,
            status: None,
            d2d_factory,
            dwrite_factory,
            render: None,
            formats,
            keep_visible,
        };
        state.rebuild_browse_sections();
        state.matches = Vec::new();
        Ok(state)
    }

    fn dimensions(&self) -> (i32, i32) {
        (
            self.display_dimensions.width,
            self.display_dimensions.height,
        )
    }

    fn row_height(&self) -> i32 {
        RESULT_ROW_HEIGHT
    }

    fn footer_top(&self) -> i32 {
        self.dimensions().1 - FOOTER_HEIGHT
    }

    fn result_limit(&self) -> usize {
        ((self.footer_top() - SEARCH_RESULTS_TOP - 4) / self.row_height()).max(1) as usize
    }

    fn grid_columns(&self) -> usize {
        ((self.dimensions().0 - 24) / GRID_CELL).max(1) as usize
    }

    fn query(&self) -> &str {
        &self.search.text
    }

    fn browsing(&self) -> bool {
        self.query().trim().is_empty()
    }

    unsafe fn update_results(&mut self) {
        self.matches = if self.browsing() {
            Vec::new()
        } else {
            catalog::search(&self.search.text, self.result_limit())
        };
        self.selected = 0;
        self.hovered_entry = None;
        self.status = None;
        unsafe {
            self.sync_accessible_results();
        }
        unsafe {
            let _ = InvalidateRect(Some(self.hwnd), None, false);
        }
    }

    unsafe fn move_selection(&mut self, delta: isize) {
        if self.view == View::Search && self.query().trim().is_empty() {
            unsafe {
                self.move_browse_selection(delta);
            }
            return;
        }
        if self.matches.is_empty() {
            return;
        }
        self.selected = self
            .selected
            .saturating_add_signed(delta)
            .min(self.matches.len() - 1);
        unsafe {
            windows::Win32::UI::WindowsAndMessaging::SendMessageW(
                self.accessible_results,
                LB_SETCURSEL,
                Some(WPARAM(self.selected)),
                None,
            );
            let _ = InvalidateRect(Some(self.hwnd), None, false);
        }
    }

    unsafe fn move_browse_selection(&mut self, delta: isize) {
        let total = self
            .browse_sections
            .iter()
            .map(|section| section.indices.len())
            .sum::<usize>();
        if total == 0 {
            return;
        }
        let current = self.browse_flat_position();
        let next = current.saturating_add_signed(delta).min(total - 1);
        self.set_browse_flat_position(next);
        self.ensure_browse_selection_visible();
        unsafe {
            self.sync_accessible_results();
            let _ = InvalidateRect(Some(self.hwnd), None, false);
        }
    }

    fn browse_flat_position(&self) -> usize {
        self.browse_sections
            .iter()
            .take(self.browse_focus.0)
            .map(|section| section.indices.len())
            .sum::<usize>()
            + self.browse_focus.1
    }

    fn set_browse_flat_position(&mut self, mut position: usize) {
        for (section_index, section) in self.browse_sections.iter().enumerate() {
            if position < section.indices.len() {
                self.browse_focus = (section_index, position);
                return;
            }
            position = position.saturating_sub(section.indices.len());
        }
    }

    fn ensure_browse_selection_visible(&mut self) {
        let Some(layout) = self.section_layouts().get(self.browse_focus.0).copied() else {
            return;
        };
        let row = self.browse_focus.1 / layout.columns;
        let item_top = (layout.grid_top + row as i32 * layout.cell_height) as f32;
        let item_bottom = item_top + layout.cell_height as f32;
        let viewport_height =
            (self.footer_top() - BROWSE_CONTENT_TOP).max(layout.cell_height) as f32;
        if item_top < self.browse_scroll_target {
            self.browse_scroll_target = item_top;
        } else if item_bottom > self.browse_scroll_target + viewport_height {
            self.browse_scroll_target = item_bottom - viewport_height;
        }
        self.clamp_browse_scroll();
        unsafe {
            let _ = SetTimer(Some(self.hwnd), SCROLL_TIMER_ID, SCROLL_FRAME_MS, None);
        }
    }

    fn rebuild_browse_sections(&mut self) {
        let entries = catalog::entries();
        let mut recent = self
            .recents
            .iter()
            .filter_map(|glyph| entries.iter().position(|entry| entry.glyph == *glyph))
            .collect::<Vec<_>>();
        if recent.is_empty() {
            recent.extend(catalog::search("", 24).into_iter().map(|found| found.index));
        }
        let mut sections = vec![BrowseSection {
            category: BrowseCategory::Recent,
            indices: recent,
        }];
        sections.extend(
            BrowseCategory::ALL
                .iter()
                .copied()
                .filter(|category| *category != BrowseCategory::Recent)
                .map(|category| BrowseSection {
                    category,
                    indices: entries
                        .iter()
                        .enumerate()
                        .filter(|(index, entry)| {
                            self.displayable_entries[*index] && category.contains(entry)
                        })
                        .map(|(index, _)| index)
                        .collect(),
                }),
        );
        self.browse_sections = sections;
        self.browse_focus = (0, 0);
        self.browse_scroll = 0.0;
        self.browse_scroll_target = 0.0;
        self.category_scroll = 0.0;
        self.active_category = 0;
    }

    /// Rebuild sections (the Recent row changes after an insert) without
    /// losing the user's place: focus, scroll, and category stay put.
    fn rebuild_browse_sections_preserving_view(&mut self) {
        let focus = self.browse_focus;
        let scroll = self.browse_scroll;
        let scroll_target = self.browse_scroll_target;
        let category_scroll = self.category_scroll;
        let active_category = self.active_category;
        self.rebuild_browse_sections();
        let section = focus.0.min(self.browse_sections.len().saturating_sub(1));
        let item_count = self
            .browse_sections
            .get(section)
            .map_or(0, |entries| entries.indices.len());
        self.browse_focus = (section, focus.1.min(item_count.saturating_sub(1)));
        self.browse_scroll = scroll;
        self.browse_scroll_target = scroll_target;
        self.category_scroll = category_scroll;
        self.active_category = active_category;
        self.clamp_browse_scroll();
        self.clamp_category_scroll();
    }

    fn section_layouts(&self) -> Vec<SectionLayout> {
        let mut top = 0;
        self.browse_sections
            .iter()
            .map(|section| {
                let (columns, cell_width, cell_height) =
                    if section.category == BrowseCategory::Emoticons {
                        let columns = ((self.dimensions().0 - 24) / 132).max(2) as usize;
                        (
                            columns,
                            (self.dimensions().0 - 24) as f32 / columns as f32,
                            42,
                        )
                    } else {
                        (self.grid_columns(), GRID_CELL as f32, GRID_CELL)
                    };
                let rows = section.indices.len().div_ceil(columns);
                let grid_top = top + SECTION_HEADING_HEIGHT;
                let bottom = grid_top + rows as i32 * cell_height + SECTION_GAP;
                let layout = SectionLayout {
                    top,
                    grid_top,
                    bottom,
                    columns,
                    cell_width,
                    cell_height,
                };
                top = bottom;
                layout
            })
            .collect()
    }

    fn total_browse_height(&self) -> i32 {
        self.section_layouts()
            .last()
            .map_or(0, |layout| layout.bottom)
    }

    fn maximum_browse_scroll(&self) -> f32 {
        let viewport = (self.footer_top() - BROWSE_CONTENT_TOP).max(1);
        (self.total_browse_height() - viewport).max(0) as f32
    }

    fn clamp_browse_scroll(&mut self) {
        let maximum = self.maximum_browse_scroll();
        self.browse_scroll = self.browse_scroll.clamp(0.0, maximum);
        self.browse_scroll_target = self.browse_scroll_target.clamp(0.0, maximum);
    }

    fn clamp_category_scroll(&mut self) {
        self.category_scroll = self
            .category_scroll
            .clamp(0.0, maximum_category_scroll(self.dimensions().0));
    }

    fn ensure_active_category_visible(&mut self) {
        let viewport = category_viewport(self.dimensions().0);
        let item_left = self.active_category as f32 * CATEGORY_BUTTON_WIDTH;
        let item_right = item_left + CATEGORY_BUTTON_WIDTH;
        if item_left < self.category_scroll {
            self.category_scroll = item_left;
        } else if item_right > self.category_scroll + (viewport.right - viewport.left) {
            self.category_scroll = item_right - (viewport.right - viewport.left);
        }
        self.clamp_category_scroll();
    }

    fn update_active_category(&mut self) {
        let marker = self.browse_scroll + 8.0;
        let layouts = self.section_layouts();
        if let Some((section_index, _)) = layouts
            .iter()
            .enumerate()
            .rev()
            .find(|(_, layout)| layout.top as f32 <= marker)
        {
            self.active_category = BrowseCategory::ALL
                .iter()
                .position(|category| *category == self.browse_sections[section_index].category)
                .unwrap_or(0);
            self.ensure_active_category_visible();
        }
    }

    unsafe fn scroll_categories(&mut self, delta: f32) {
        self.category_scroll += delta;
        self.clamp_category_scroll();
        unsafe {
            let _ = InvalidateRect(Some(self.hwnd), None, false);
        }
    }

    unsafe fn scroll_browse(&mut self, delta: f32) {
        self.browse_scroll_target += delta;
        self.clamp_browse_scroll();
        unsafe {
            let _ = SetTimer(Some(self.hwnd), SCROLL_TIMER_ID, SCROLL_FRAME_MS, None);
            let _ = InvalidateRect(Some(self.hwnd), None, false);
        }
    }

    unsafe fn tick_browse_scroll(&mut self) {
        let distance = self.browse_scroll_target - self.browse_scroll;
        if distance.abs() < 0.35 {
            self.browse_scroll = self.browse_scroll_target;
            unsafe {
                KillTimer(Some(self.hwnd), SCROLL_TIMER_ID).ok();
                self.sync_accessible_results();
            }
        } else {
            self.browse_scroll = smooth_scroll_step(self.browse_scroll, self.browse_scroll_target);
        }
        self.update_active_category();
        unsafe {
            let _ = InvalidateRect(Some(self.hwnd), None, false);
        }
    }

    unsafe fn set_browse_scroll_immediate(&mut self, position: f32) {
        let position = position.clamp(0.0, self.maximum_browse_scroll());
        self.browse_scroll = position;
        self.browse_scroll_target = position;
        self.update_active_category();
        unsafe {
            let _ = InvalidateRect(Some(self.hwnd), None, false);
        }
    }

    unsafe fn jump_to_category(&mut self, category_index: usize) {
        let category = BrowseCategory::ALL[category_index.min(BrowseCategory::ALL.len() - 1)];
        if let Some(section_index) = self
            .browse_sections
            .iter()
            .position(|section| section.category == category)
        {
            let layouts = self.section_layouts();
            self.browse_scroll_target = layouts[section_index].top as f32;
            self.browse_focus = (section_index, 0);
            self.active_category = category_index;
            self.ensure_active_category_visible();
            self.clamp_browse_scroll();
            unsafe {
                let _ = SetTimer(Some(self.hwnd), SCROLL_TIMER_ID, SCROLL_FRAME_MS, None);
                self.sync_accessible_results();
                let _ = InvalidateRect(Some(self.hwnd), None, false);
            }
        }
    }

    fn visible_browse_items(&self) -> Vec<(usize, usize, usize)> {
        let viewport_top = self.browse_scroll;
        let viewport_bottom = self.browse_scroll + (self.footer_top() - BROWSE_CONTENT_TOP) as f32;
        let layouts = self.section_layouts();
        let mut visible = Vec::new();
        for (section_index, (section, layout)) in
            self.browse_sections.iter().zip(layouts.iter()).enumerate()
        {
            if (layout.bottom as f32) < viewport_top || (layout.top as f32) > viewport_bottom {
                continue;
            }
            let range = visible_item_range(
                *layout,
                section.indices.len(),
                viewport_top,
                viewport_bottom,
            );
            for item_index in range {
                visible.push((section_index, item_index, section.indices[item_index]));
            }
        }
        visible
    }

    fn selected_entry_index(&self) -> Option<usize> {
        if self.view != View::Search {
            return None;
        }
        if self.query().trim().is_empty() {
            self.browse_sections
                .get(self.browse_focus.0)?
                .indices
                .get(self.browse_focus.1)
                .copied()
        } else {
            self.matches.get(self.selected).map(|found| found.index)
        }
    }

    fn hover_or_selected_entry(&self) -> Option<usize> {
        self.hovered_entry.or_else(|| self.selected_entry_index())
    }

    unsafe fn rebuild_formats(&mut self) -> Result<()> {
        self.formats = unsafe { TextFormats::new(&self.dwrite_factory, self.config.emoji_font)? };
        self.render = None;
        unsafe {
            let _ = InvalidateRect(Some(self.hwnd), None, false);
        }
        Ok(())
    }

    unsafe fn sync_accessible_results(&self) {
        if self.accessible_results.is_invalid() {
            return;
        }
        unsafe {
            windows::Win32::UI::WindowsAndMessaging::SendMessageW(
                self.accessible_results,
                LB_RESETCONTENT,
                None,
                None,
            );
        }
        let browsing = self.view == View::Search && self.query().trim().is_empty();
        let visible_browse = browsing.then(|| self.visible_browse_items());
        let labels = match self.view {
            View::Search if browsing => visible_browse
                .as_ref()
                .expect("visible items exist")
                .iter()
                .map(|(_, _, index)| {
                    let entry = &catalog::entries()[*index];
                    format!("{} {}, {}", entry.glyph, entry.name, entry.kind)
                })
                .collect::<Vec<_>>(),
            View::Search => self
                .matches
                .iter()
                .map(|item| {
                    let entry = &catalog::entries()[item.index];
                    format!("{} {}, {}", entry.glyph, entry.name, entry.kind)
                })
                .collect::<Vec<_>>(),
            View::Settings => vec![
                format!("Width, {}", self.config.dimensions.width),
                format!("Height, {}", self.config.dimensions.height),
                format!("Hover details, {}", self.config.details),
                format!("Emoji font, {}", self.config.emoji_font),
                format!("Open shortcut, {}", self.config.hotkey),
            ],
        };
        for value in labels {
            let label = to_wide(&value);
            unsafe {
                windows::Win32::UI::WindowsAndMessaging::SendMessageW(
                    self.accessible_results,
                    LB_ADDSTRING,
                    None,
                    Some(LPARAM(label.as_ptr() as isize)),
                );
            }
        }
        let selection = if browsing {
            visible_browse
                .as_ref()
                .and_then(|items| {
                    items
                        .iter()
                        .position(|(section, item, _)| (*section, *item) == self.browse_focus)
                })
                .unwrap_or(0)
        } else {
            self.selected
        };
        unsafe {
            windows::Win32::UI::WindowsAndMessaging::SendMessageW(
                self.accessible_results,
                LB_SETCURSEL,
                Some(WPARAM(selection)),
                None,
            );
        }
    }

    fn selected_text(&self) -> Option<&str> {
        self.selected_entry_index()
            .map(|index| catalog::entries()[index].glyph.as_str())
    }
}

pub fn run() -> Result<()> {
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    let mode = parse_mode(arguments.into_iter()).map_err(argument_error)?;
    match mode {
        Mode::Help => {
            print_help();
            Ok(())
        }
        Mode::Install { uninstall, dry_run } => manage_startup(uninstall, dry_run),
        Mode::Preview => run_picker(false, true),
        Mode::SelfTest => self_test(),
        Mode::Benchmark => {
            report_success(&benchmark());
            Ok(())
        }
        Mode::Run { startup } => run_picker(startup, false),
    }
}

pub fn report_fatal(message: &str) {
    #[cfg(feature = "console")]
    eprintln!("winmoji: {message}");
    #[cfg(not(feature = "console"))]
    show_message(message, MB_OK | MB_ICONERROR);
}

fn report_success(message: &str) {
    #[cfg(feature = "console")]
    println!("{message}");
    #[cfg(not(feature = "console"))]
    show_message(message, MB_OK);
}

#[cfg(not(feature = "console"))]
fn show_message(message: &str, style: MESSAGEBOX_STYLE) {
    let title = to_wide("WinMoji");
    let message = to_wide(message);
    unsafe {
        let _ = MessageBoxW(
            None,
            PCWSTR(message.as_ptr()),
            PCWSTR(title.as_ptr()),
            style,
        );
    }
}

fn parse_mode(arguments: impl Iterator<Item = String>) -> std::result::Result<Mode, String> {
    let arguments: Vec<_> = arguments.collect();
    if arguments.is_empty() {
        return Ok(Mode::Run { startup: false });
    }
    let dry_run = arguments.iter().any(|argument| argument == "--dry-run");
    let actions: Vec<_> = arguments
        .iter()
        .filter(|argument| argument.as_str() != "--dry-run")
        .collect();
    if actions.len() != 1 {
        return Err("choose exactly one action; run winmoji --help for usage".to_string());
    }
    match actions[0].as_str() {
        "--startup" if !dry_run => Ok(Mode::Run { startup: true }),
        "--preview" if !dry_run => Ok(Mode::Preview),
        "--install" => Ok(Mode::Install {
            uninstall: false,
            dry_run,
        }),
        "--uninstall" => Ok(Mode::Install {
            uninstall: true,
            dry_run,
        }),
        "--self-test" if !dry_run => Ok(Mode::SelfTest),
        "--benchmark" if !dry_run => Ok(Mode::Benchmark),
        "--help" | "-h" if !dry_run => Ok(Mode::Help),
        other => Err(format!("unknown or incompatible option: {other}")),
    }
}

fn print_help() {
    report_success(
        "WinMoji, a keyboard-first Unicode picker for Windows\n\
\n\
Usage:\n\
  winmoji                 Show the picker or show the running instance\n\
  winmoji --startup       Start the hotkey listener without showing the picker\n\
  winmoji --preview       Keep the picker visible for visual inspection\n\
  winmoji --install       Add WinMoji to the current user's startup apps\n\
  winmoji --uninstall     Remove WinMoji from the current user's startup apps\n\
  winmoji --self-test     Test search, hotkey registration, and Unicode input\n\
  winmoji --benchmark     Measure representative full-catalog search latency\n\
  winmoji --help          Show this help\n\
\n\
Options:\n\
  --dry-run               Show the registry change without applying it",
    );
}

fn argument_error(message: String) -> Error {
    eprintln!("winmoji: {message}");
    Error::new(HRESULT(0x80070057u32 as i32), message)
}

fn run_picker(startup: bool, keep_visible: bool) -> Result<()> {
    unsafe {
        let mutex = CreateMutexW(None, false, MUTEX_NAME)?;
        if GetLastError() == ERROR_ALREADY_EXISTS {
            for _ in 0..200 {
                if let Ok(existing) = FindWindowW(CLASS_NAME, PCWSTR::null()) {
                    let target = GetForegroundWindow();
                    let target_focus = focused_child_for(target);
                    PostMessageW(
                        Some(existing),
                        WM_SHOW_PICKER,
                        WPARAM(target.0 as usize),
                        LPARAM(target_focus.0 as isize),
                    )?;
                    CloseHandle(mutex)?;
                    return Ok(());
                }
                Sleep(25);
            }
            CloseHandle(mutex)?;
            return Err(Error::new(
                HRESULT(0x80004005u32 as i32),
                "the existing WinMoji instance did not expose its window",
            ));
        }

        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        // The common-controls v6 manifest redirects the standard control
        // classes to comctl32, which only registers them on this call.
        let controls = INITCOMMONCONTROLSEX {
            dwSize: size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_STANDARD_CLASSES,
        };
        let _ = InitCommonControlsEx(&controls);
        let config = load_config().map_err(argument_error)?;
        let hotkey = config.hotkey;
        let instance = HINSTANCE(windows::Win32::System::LibraryLoader::GetModuleHandleW(None)?.0);
        register_picker_class(instance)?;

        let state = Box::new(AppState::new(keep_visible, config)?);
        let state_pointer = Box::into_raw(state);
        let extended_style = if keep_visible {
            WS_EX_TOPMOST | WS_EX_NOACTIVATE
        } else {
            WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE
        };
        let window_width = config.dimensions.width;
        let window_height = config.dimensions.height;
        let hwnd = match CreateWindowExW(
            extended_style,
            CLASS_NAME,
            WINDOW_TITLE,
            WS_POPUP,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            window_width,
            window_height,
            None,
            None,
            Some(instance),
            Some(state_pointer.cast::<c_void>()),
        ) {
            Ok(hwnd) => hwnd,
            Err(error) => {
                drop(Box::from_raw(state_pointer));
                CloseHandle(mutex)?;
                return Err(error);
            }
        };
        configure_window_frame(hwnd);

        if let Err(error) = RegisterHotKey(
            Some(hwnd),
            HOTKEY_ID,
            HOT_KEY_MODIFIERS(hotkey.modifiers | MOD_NOREPEAT.0),
            hotkey.virtual_key,
        ) {
            DestroyWindow(hwnd)?;
            drop(Box::from_raw(state_pointer));
            CloseHandle(mutex)?;
            return Err(Error::new(
                error.code(),
                format!("cannot register global hotkey {hotkey}: {error}"),
            ));
        }

        if !startup {
            show_picker(state_pointer, None, None);
        }

        let mut message = MSG::default();
        loop {
            let message_result = GetMessageW(&mut message, None, 0, 0).0;
            if message_result == -1 {
                let _ = UnregisterHotKey(Some(hwnd), HOTKEY_ID);
                let _ = DestroyWindow(hwnd);
                drop(Box::from_raw(state_pointer));
                CloseHandle(mutex)?;
                return Err(Error::from_win32());
            }
            if message_result == 0 {
                break;
            }
            let state = &mut *state_pointer;
            // Wheel messages can land on the accessibility listbox child;
            // route them here before dispatch reaches the system control.
            if (message.message == WM_MOUSEWHEEL || message.message == WM_MOUSEHWHEEL)
                && (message.hwnd == state.hwnd || message.hwnd == state.accessible_results)
            {
                route_wheel(
                    state,
                    message.message == WM_MOUSEHWHEEL,
                    message.wParam,
                    message.lParam,
                );
                continue;
            }
            if (message.message == WM_KEYDOWN || message.message == WM_SYSKEYDOWN)
                && (message.hwnd == state.accessible_results || message.hwnd == state.hwnd)
            {
                let key = VIRTUAL_KEY(message.wParam.0 as u16);
                let control = GetKeyState(VK_CONTROL.0 as i32) < 0;
                let handled = if state.view == View::Settings {
                    handle_settings_key(state, key, control)
                } else {
                    handle_picker_key(state, key, control)
                };
                if handled {
                    continue;
                }
            }
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }

        let _ = UnregisterHotKey(Some(hwnd), HOTKEY_ID);
        drop(Box::from_raw(state_pointer));
        CloseHandle(mutex)?;
        Ok(())
    }
}

unsafe fn configure_window_frame(hwnd: HWND) {
    let dark_mode = 1i32;
    let corner_preference: DWM_WINDOW_CORNER_PREFERENCE = DWMWCP_ROUND;
    let border_color = COLORREF(0x003e3430);
    unsafe {
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            (&dark_mode as *const i32).cast(),
            size_of::<i32>() as u32,
        );
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            (&corner_preference as *const DWM_WINDOW_CORNER_PREFERENCE).cast(),
            size_of_val(&corner_preference) as u32,
        );
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_BORDER_COLOR,
            (&border_color as *const COLORREF).cast(),
            size_of::<COLORREF>() as u32,
        );
    }
}

unsafe fn set_accessible_name(hwnd: HWND, name: PCWSTR) {
    let initialized = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_ok() };
    let service: Result<IAccPropServices> = unsafe {
        CoCreateInstance(
            &CLSID_AccPropServices,
            None::<&windows::core::IUnknown>,
            CLSCTX_INPROC_SERVER,
        )
    };
    if let Ok(service) = service {
        let _ = unsafe {
            service.SetHwndPropStr(hwnd, OBJID_CLIENT.0 as u32, 0, PROPID_ACC_NAME, name)
        };
    }
    if initialized {
        unsafe {
            CoUninitialize();
        }
    }
}

unsafe fn register_picker_class(instance: HINSTANCE) -> Result<()> {
    let cursor = unsafe { LoadCursorW(None, IDC_ARROW)? };
    let class = WNDCLASSW {
        style: CS_HREDRAW | CS_VREDRAW | CS_DROPSHADOW,
        lpfnWndProc: Some(window_proc),
        hInstance: instance,
        hCursor: cursor,
        lpszClassName: CLASS_NAME,
        ..Default::default()
    };
    if unsafe { RegisterClassW(&class) } == 0 {
        return Err(Error::from_win32());
    }
    Ok(())
}

unsafe extern "system" fn keyboard_hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let state_pointer = ACTIVE_PICKER.load(Ordering::Acquire);
        if !state_pointer.is_null() {
            let state = unsafe { &*state_pointer };
            if state.capture_active {
                let foreground = unsafe { GetForegroundWindow() };
                if !state.keep_visible && foreground != state.target {
                    unsafe {
                        let _ = PostMessageW(
                            Some(state.hwnd),
                            WM_CAPTURE_TARGET_LOST,
                            WPARAM(0),
                            LPARAM(0),
                        );
                    }
                    return unsafe { CallNextHookEx(None, code, wparam, lparam) };
                }
                // Preview mode has no target window to scope the capture, so it
                // captures only while the cursor is over the picker; everything
                // else keeps typing into the rest of the desktop normally.
                if state.keep_visible && !cursor_over_window(state.hwnd) {
                    return unsafe { CallNextHookEx(None, code, wparam, lparam) };
                }
                let message = wparam.0 as u32;
                if matches!(message, WM_KEYDOWN | WM_SYSKEYDOWN | WM_KEYUP | WM_SYSKEYUP) {
                    let event = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
                    // Synthetic events pass untouched. Our own SendInput batches
                    // arrive here while the hook is live; eating one of their
                    // UTF-16 halves would corrupt the inserted character.
                    if event.flags.contains(LLKHF_INJECTED) {
                        return unsafe { CallNextHookEx(None, code, wparam, lparam) };
                    }
                    let key_up = matches!(message, WM_KEYUP | WM_SYSKEYUP);
                    let packed = event.scanCode as u64 | ((key_up as u64) << 32);
                    unsafe {
                        let _ = PostMessageW(
                            Some(state.hwnd),
                            WM_CAPTURED_KEY,
                            WPARAM(event.vkCode as usize),
                            LPARAM(packed as isize),
                        );
                    }
                    // Modifier events must reach the system: discarding them here
                    // freezes the system key-state tables, which leaves Ctrl/Shift
                    // stuck down after the picker closes and blocks SendInput,
                    // whose preflight waits for all modifiers to be released.
                    // PrintScreen and Win-modified shortcuts (screenshots, OS
                    // shortcuts) also stay with the system.
                    let virtual_key = VIRTUAL_KEY(event.vkCode as u16);
                    if is_modifier_key(virtual_key)
                        || virtual_key == VK_SNAPSHOT
                        || unsafe { GetAsyncKeyState(VK_LWIN.0 as i32) } < 0
                        || unsafe { GetAsyncKeyState(VK_RWIN.0 as i32) } < 0
                    {
                        return unsafe { CallNextHookEx(None, code, wparam, lparam) };
                    }
                    return LRESULT(1);
                }
            }
        }
    }
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

fn cursor_over_window(hwnd: HWND) -> bool {
    let mut point = POINT::default();
    let mut window = RECT::default();
    unsafe { GetCursorPos(&mut point) }.is_ok()
        && unsafe { GetWindowRect(hwnd, &mut window) }.is_ok()
        && point.x >= window.left
        && point.x < window.right
        && point.y >= window.top
        && point.y < window.bottom
}

unsafe extern "system" fn mouse_hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let state_pointer = ACTIVE_PICKER.load(Ordering::Acquire);
        if !state_pointer.is_null() {
            let state = unsafe { &*state_pointer };
            let message = wparam.0 as u32;
            if state.capture_active
                && !state.keep_visible
                && matches!(
                    message,
                    WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN | WM_XBUTTONDOWN
                )
            {
                let event = unsafe { &*(lparam.0 as *const MSLLHOOKSTRUCT) };
                let mut window = RECT::default();
                let inside = unsafe { GetWindowRect(state.hwnd, &mut window) }.is_ok()
                    && event.pt.x >= window.left
                    && event.pt.x < window.right
                    && event.pt.y >= window.top
                    && event.pt.y < window.bottom;
                if !inside {
                    unsafe {
                        let _ = PostMessageW(
                            Some(state.hwnd),
                            WM_CAPTURE_TARGET_LOST,
                            WPARAM(0),
                            LPARAM(0),
                        );
                    }
                }
            }
        }
    }
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

unsafe fn start_keyboard_capture(state: &mut AppState) -> Result<()> {
    if state.keyboard_hook.is_some() {
        return Ok(());
    }
    unsafe {
        GetKeyboardState(&mut state.keyboard_state)?;
    }
    state.pending_commit = None;
    state.capture_active = true;
    ACTIVE_PICKER.store(state as *mut AppState, Ordering::Release);
    let instance = HINSTANCE(
        unsafe { windows::Win32::System::LibraryLoader::GetModuleHandleW(None) }
            .map(|module| module.0)?,
    );
    match unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook_proc), Some(instance), 0) }
    {
        Ok(hook) => {
            state.keyboard_hook = Some(hook);
            state.mouse_hook =
                unsafe { SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook_proc), Some(instance), 0) }
                    .ok();
            Ok(())
        }
        Err(error) => {
            state.capture_active = false;
            ACTIVE_PICKER.store(std::ptr::null_mut(), Ordering::Release);
            Err(error)
        }
    }
}

unsafe fn stop_keyboard_capture(state: &mut AppState) {
    state.capture_active = false;
    state.pending_commit = None;
    ACTIVE_PICKER.store(std::ptr::null_mut(), Ordering::Release);
    if let Some(hook) = state.keyboard_hook.take() {
        unsafe {
            UnhookWindowsHookEx(hook).ok();
        }
    }
    if let Some(hook) = state.mouse_hook.take() {
        unsafe {
            UnhookWindowsHookEx(hook).ok();
        }
    }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if message == WM_NCCREATE {
        let create = unsafe { &*(lparam.0 as *const CREATESTRUCTW) };
        let state = create.lpCreateParams.cast::<AppState>();
        unsafe {
            (*state).hwnd = hwnd;
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, state as isize);
        }
    }

    let state_pointer = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut AppState };
    if state_pointer.is_null() {
        return unsafe { DefWindowProcW(hwnd, message, wparam, lparam) };
    }
    let state = unsafe { &mut *state_pointer };

    match message {
        WM_NCCREATE => LRESULT(1),
        windows::Win32::UI::WindowsAndMessaging::WM_CREATE => {
            let instance = HINSTANCE(
                unsafe { windows::Win32::System::LibraryLoader::GetModuleHandleW(None) }
                    .map(|module| module.0)
                    .unwrap_or_default(),
            );
            state.accessible_results = match unsafe {
                CreateWindowExW(
                    WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_NOACTIVATE,
                    w!("LISTBOX"),
                    PCWSTR::null(),
                    WS_CHILD
                        | WS_VISIBLE
                        | WS_TABSTOP
                        | WINDOW_STYLE(
                            LBS_NOTIFY as u32 | LBS_HASSTRINGS as u32 | LBS_NOINTEGRALHEIGHT as u32,
                        ),
                    scale(22, state.dpi),
                    scale(SEARCH_RESULTS_TOP, state.dpi),
                    scale(state.dimensions().0 - 44, state.dpi),
                    scale(state.footer_top() - SEARCH_RESULTS_TOP, state.dpi),
                    Some(hwnd),
                    Some(HMENU(RESULTS_ID as *mut c_void)),
                    Some(instance),
                    None,
                )
            } {
                Ok(results) => results,
                Err(_) => return LRESULT(-1),
            };
            unsafe {
                let _ =
                    SetLayeredWindowAttributes(state.accessible_results, COLORREF(0), 0, LWA_ALPHA);
                set_accessible_name(state.accessible_results, w!("Search results"));
                state.sync_accessible_results();
            }
            LRESULT(0)
        }
        WM_SHOW_PICKER => {
            unsafe {
                let requested_target = HWND(wparam.0 as *mut c_void);
                let requested_focus = HWND(lparam.0 as *mut c_void);
                show_picker(state_pointer, Some(requested_target), Some(requested_focus));
            }
            LRESULT(0)
        }
        WM_HOTKEY => {
            unsafe {
                show_picker(state_pointer, None, None);
            }
            LRESULT(0)
        }
        WM_COMMAND => {
            let control_id = wparam.0 & 0xffff;
            let notification = ((wparam.0 >> 16) & 0xffff) as u16;
            if control_id == RESULTS_ID
                && (notification as u32 == LBN_SELCHANGE || notification as u32 == LBN_DBLCLK)
            {
                let selected = unsafe {
                    windows::Win32::UI::WindowsAndMessaging::SendMessageW(
                        state.accessible_results,
                        LB_GETCURSEL,
                        None,
                        None,
                    )
                    .0
                };
                let browsing = state.view == View::Search && state.query().trim().is_empty();
                let visible_browse = browsing.then(|| state.visible_browse_items());
                let item_count = match state.view {
                    View::Search if browsing => visible_browse.as_ref().map_or(0, Vec::len),
                    View::Search => state.matches.len(),
                    View::Settings => 5,
                };
                if selected >= 0 && (selected as usize) < item_count {
                    if browsing {
                        let (section, item, _) =
                            visible_browse.as_ref().expect("browse items")[selected as usize];
                        state.browse_focus = (section, item);
                    } else {
                        state.selected = selected as usize;
                    }
                    if state.view == View::Settings {
                        state.settings_selected = state.selected.min(4);
                    }
                    unsafe {
                        let _ = InvalidateRect(Some(state.hwnd), None, false);
                        if notification as u32 == LBN_DBLCLK && state.view != View::Settings {
                            commit_selection(state, true);
                        }
                    }
                }
            }
            LRESULT(0)
        }
        WM_MOUSEMOVE => {
            let (x, y) = mouse_point_dip(lparam, state.dpi);
            unsafe {
                if state.dragging_slider.is_some() {
                    update_dragged_slider(state, x);
                } else if state.dragging_scrollbar.is_some() {
                    update_dragged_scrollbar(state, y);
                } else {
                    update_hover(state, x, y);
                }
            }
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            let (x, y) = mouse_point_dip(lparam, state.dpi);
            unsafe {
                handle_click(state, x, y);
            }
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            let was_dragging_scrollbar = state.dragging_scrollbar.is_some();
            let was_dragging = state.dragging_slider.is_some() || was_dragging_scrollbar;
            if state.dragging_slider.take().is_some() {
                unsafe {
                    resize_window_in_place(state);
                }
            }
            state.dragging_scrollbar = None;
            if was_dragging {
                unsafe {
                    ReleaseCapture().ok();
                    if was_dragging_scrollbar {
                        state.sync_accessible_results();
                    }
                }
            }
            LRESULT(0)
        }
        WM_MOUSEWHEEL | WM_MOUSEHWHEEL => {
            unsafe {
                route_wheel(state, message == WM_MOUSEHWHEEL, wparam, lparam);
            }
            LRESULT(0)
        }
        WM_TIMER if wparam.0 == SCROLL_TIMER_ID => {
            if state.view == View::Search && state.browsing() {
                unsafe {
                    state.tick_browse_scroll();
                }
            } else {
                unsafe {
                    KillTimer(Some(state.hwnd), SCROLL_TIMER_ID).ok();
                }
            }
            LRESULT(0)
        }
        WM_TIMER if wparam.0 == FOCUS_TIMER_ID => {
            if !state.keep_visible && unsafe { GetForegroundWindow() } != state.target {
                unsafe {
                    hide_picker(state);
                }
            }
            LRESULT(0)
        }
        WM_CAPTURED_KEY => {
            let scan_code = lparam.0 as u64 as u32;
            let key_up = ((lparam.0 as u64 >> 32) & 1) != 0;
            unsafe {
                handle_captured_key(state, VIRTUAL_KEY(wparam.0 as u16), scan_code, key_up);
            }
            LRESULT(0)
        }
        WM_CAPTURE_TARGET_LOST => {
            unsafe {
                hide_picker(state);
            }
            LRESULT(0)
        }
        WM_DPICHANGED => {
            let dpi = (wparam.0 & 0xffff) as u32;
            let suggested = unsafe { &*(lparam.0 as *const RECT) };
            state.render = None;
            state.dpi = dpi.max(96);
            unsafe {
                SetWindowPos(
                    hwnd,
                    None,
                    suggested.left,
                    suggested.top,
                    suggested.right - suggested.left,
                    suggested.bottom - suggested.top,
                    SWP_NOZORDER | SWP_NOACTIVATE,
                )
                .ok();
                layout(state);
            }
            LRESULT(0)
        }
        WM_SIZE => {
            state.render = None;
            unsafe {
                let _ = InvalidateRect(Some(hwnd), None, false);
            }
            LRESULT(0)
        }
        WM_ERASEBKGND => LRESULT(1),
        WM_PAINT => {
            unsafe {
                paint(state);
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            unsafe {
                stop_keyboard_capture(state);
                PostQuitMessage(0);
            }
            LRESULT(0)
        }
        WM_NCDESTROY => unsafe {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            DefWindowProcW(hwnd, message, wparam, lparam)
        },
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

unsafe fn show_picker(
    state_pointer: *mut AppState,
    requested_target: Option<HWND>,
    requested_focus: Option<HWND>,
) {
    let state = unsafe { &mut *state_pointer };
    let foreground = requested_target
        .filter(|target| !target.is_invalid() && unsafe { IsWindow(Some(*target)).as_bool() })
        .unwrap_or_else(|| unsafe { GetForegroundWindow() });
    if foreground != state.hwnd && !foreground.is_invalid() {
        state.target = foreground;
        state.target_focus = requested_focus
            .filter(|focus| valid_target_focus(foreground, *focus))
            .unwrap_or_else(|| unsafe { focused_child_for(foreground) });
    }
    state.search.clear();
    state.view = View::Search;
    state.status = None;
    unsafe {
        state.update_results();
        position_near_cursor(state);
        let _ = ShowWindow(state.hwnd, SW_SHOWNOACTIVATE);
        arm_focus_watch(state);
        if let Err(error) = start_keyboard_capture(state) {
            state.status = Some(format!("Keyboard capture unavailable: {error}"));
            let _ = InvalidateRect(Some(state.hwnd), None, false);
            return;
        }
        let _ = InvalidateRect(Some(state.hwnd), None, false);
    }
}

unsafe fn arm_focus_watch(state: &AppState) {
    if !state.keep_visible {
        unsafe {
            let _ = SetTimer(Some(state.hwnd), FOCUS_TIMER_ID, FOCUS_FRAME_MS, None);
        }
    }
}

unsafe fn hide_picker(state: &mut AppState) {
    unsafe {
        stop_keyboard_capture(state);
        KillTimer(Some(state.hwnd), FOCUS_TIMER_ID).ok();
        let _ = ShowWindow(state.hwnd, SW_HIDE);
    }
}

unsafe fn enter_search(state: &mut AppState) {
    state.view = View::Search;
    state.capturing_shortcut = false;
    unsafe {
        state.update_results();
        let _ = InvalidateRect(Some(state.hwnd), None, false);
    }
}

unsafe fn focus_browser(state: &mut AppState) {
    state.view = View::Search;
    state.status = None;
    state.browse_scroll = 0.0;
    state.browse_scroll_target = 0.0;
    state.browse_focus = (0, 0);
    state.search.clear();
    unsafe {
        state.rebuild_browse_sections();
        state.update_results();
    }
}

unsafe fn enter_settings(state: &mut AppState) {
    if state.view != View::Settings {
        state.settings_original = state.config;
    }
    state.view = View::Settings;
    state.status = None;
    state.settings_selected = 0;
    state.selected = 0;
    state.capturing_shortcut = false;
    unsafe {
        state.sync_accessible_results();
        let _ = InvalidateRect(Some(state.hwnd), None, false);
    }
}

unsafe fn adjust_setting(state: &mut AppState, delta: isize) {
    match state.settings_selected {
        0 => {
            state.config.dimensions.width = state
                .config
                .dimensions
                .width
                .saturating_add((delta * 4) as i32)
                .clamp(MIN_PICKER_WIDTH, MAX_PICKER_WIDTH);
            state.display_dimensions = state.config.dimensions;
            unsafe {
                resize_window_in_place(state);
            }
        }
        1 => {
            state.config.dimensions.height = state
                .config
                .dimensions
                .height
                .saturating_add((delta * 4) as i32)
                .clamp(MIN_PICKER_HEIGHT, MAX_PICKER_HEIGHT);
            state.display_dimensions = state.config.dimensions;
            unsafe {
                resize_window_in_place(state);
            }
        }
        2 => state.config.details = state.config.details.next(delta),
        3 => {
            state.config.emoji_font = state.config.emoji_font.next(delta);
            if let Err(error) = unsafe { state.rebuild_formats() } {
                state.status = Some(format!("Could not change emoji font: {error}"));
            }
        }
        _ => {}
    }
    state.selected = state.settings_selected;
    unsafe {
        state.sync_accessible_results();
        let _ = InvalidateRect(Some(state.hwnd), None, false);
    }
}

/// Enter on a settings row changes that row's value in place (cycling with
/// wrap-around) or records a new shortcut; the change previews immediately.
unsafe fn activate_setting(state: &mut AppState) {
    match state.settings_selected {
        2 => {
            state.config.details = if state.config.details == DetailMode::Both {
                DetailMode::None
            } else {
                state.config.details.next(1)
            };
        }
        3 => {
            state.config.emoji_font = match state.config.emoji_font {
                EmojiFont::SegoeEmoji => EmojiFont::SegoeSymbol,
                EmojiFont::SegoeSymbol => EmojiFont::SegoeEmoji,
            };
            if let Err(error) = unsafe { state.rebuild_formats() } {
                state.status = Some(format!("Could not change emoji font: {error}"));
            }
        }
        4 => {
            state.capturing_shortcut = true;
            state.status = Some("Press the new shortcut".to_string());
        }
        _ => {}
    }
    unsafe {
        state.sync_accessible_results();
        let _ = InvalidateRect(Some(state.hwnd), None, false);
    }
}

unsafe fn save_settings(state: &mut AppState) {
    let previous_hotkey = state.registered_hotkey;
    if let Err(error) = unsafe { apply_registered_hotkey(state) } {
        state.status = Some(error);
        unsafe {
            let _ = InvalidateRect(Some(state.hwnd), None, false);
        }
        return;
    }
    if let Err(error) = save_config(state.config) {
        if state.registered_hotkey != previous_hotkey {
            let _ = unsafe { UnregisterHotKey(Some(state.hwnd), HOTKEY_ID) };
            let _ = unsafe {
                RegisterHotKey(
                    Some(state.hwnd),
                    HOTKEY_ID,
                    HOT_KEY_MODIFIERS(previous_hotkey.modifiers),
                    previous_hotkey.virtual_key,
                )
            };
            state.registered_hotkey = previous_hotkey;
        }
        state.status = Some(format!("Could not save settings: {error}"));
        unsafe {
            let _ = InvalidateRect(Some(state.hwnd), None, false);
        }
        return;
    }
    state.settings_original = state.config;
    unsafe {
        enter_search(state);
    }
}

unsafe fn discard_settings(state: &mut AppState) {
    state.config = state.settings_original;
    state.display_dimensions = state.config.dimensions;
    unsafe {
        let _ = state.rebuild_formats();
        resize_window_in_place(state);
        enter_search(state);
    }
}

unsafe fn reset_settings(state: &mut AppState) {
    state.config = Config::default();
    state.display_dimensions = state.config.dimensions;
    state.status = None;
    unsafe {
        let _ = state.rebuild_formats();
        resize_window_in_place(state);
        state.sync_accessible_results();
        let _ = InvalidateRect(Some(state.hwnd), None, false);
    }
}

unsafe fn apply_registered_hotkey(state: &mut AppState) -> std::result::Result<(), String> {
    if state.config.hotkey == state.registered_hotkey {
        return Ok(());
    }
    let previous = state.registered_hotkey;
    let _ = unsafe { UnregisterHotKey(Some(state.hwnd), HOTKEY_ID) };
    if let Err(error) = unsafe {
        RegisterHotKey(
            Some(state.hwnd),
            HOTKEY_ID,
            HOT_KEY_MODIFIERS(state.config.hotkey.modifiers),
            state.config.hotkey.virtual_key,
        )
    } {
        let _ = unsafe {
            RegisterHotKey(
                Some(state.hwnd),
                HOTKEY_ID,
                HOT_KEY_MODIFIERS(previous.modifiers),
                previous.virtual_key,
            )
        };
        return Err(format!(
            "Shortcut {} is unavailable: {error}",
            state.config.hotkey
        ));
    }
    state.registered_hotkey = state.config.hotkey;
    Ok(())
}

unsafe fn handle_captured_key(
    state: &mut AppState,
    key: VIRTUAL_KEY,
    scan_code: u32,
    key_up: bool,
) {
    update_captured_keyboard_state(&mut state.keyboard_state, key, key_up);
    if key_up {
        if state.pending_commit.is_some() && captured_commit_keys_released(&state.keyboard_state) {
            let close_after = state.pending_commit.take().unwrap_or(false);
            unsafe {
                commit_selection(state, close_after);
            }
        }
        return;
    }
    if is_modifier_key(key) {
        return;
    }
    let control = captured_key_down(&state.keyboard_state, VK_CONTROL);
    if state.view == View::Settings {
        unsafe {
            handle_settings_key(state, key, control);
        }
        return;
    }
    if key == VK_RETURN {
        state.pending_commit = Some(control);
        return;
    }
    if unsafe { handle_picker_key(state, key, control) } {
        return;
    }
    let shift = captured_key_down(&state.keyboard_state, VK_SHIFT);
    match key {
        VK_BACK => {
            state.search.backspace(control);
            unsafe { state.update_results() };
            return;
        }
        VK_DELETE => {
            state.search.delete_forward();
            unsafe { state.update_results() };
            return;
        }
        VK_LEFT | VK_RIGHT => {
            state
                .search
                .move_caret(if key == VK_LEFT { -1 } else { 1 }, shift);
            unsafe {
                let _ = InvalidateRect(Some(state.hwnd), None, false);
            }
            return;
        }
        VK_HOME | VK_END => {
            if key == VK_HOME {
                state.search.move_home(shift);
            } else {
                state.search.move_end(shift);
            }
            unsafe {
                let _ = InvalidateRect(Some(state.hwnd), None, false);
            }
            return;
        }
        _ => {}
    }
    if control && key.0 == VK_A_VALUE {
        state.search.select_all();
        unsafe {
            let _ = InvalidateRect(Some(state.hwnd), None, false);
        }
        return;
    }
    if control && key.0 == VK_V_VALUE {
        if let Some(text) = clipboard_text(state.hwnd) {
            state.search.insert(&sanitize_query(&text));
            unsafe { state.update_results() };
        }
        return;
    }
    let alt = captured_key_down(&state.keyboard_state, VK_MENU);
    let win = captured_key_down(&state.keyboard_state, VK_LWIN)
        || captured_key_down(&state.keyboard_state, VK_RWIN);
    let alt_graph = captured_key_down(&state.keyboard_state, VK_RMENU);
    if win || ((control || alt) && !alt_graph) {
        return;
    }
    let target_thread = unsafe { GetWindowThreadProcessId(state.target, None) };
    let keyboard_layout = unsafe { GetKeyboardLayout(target_thread) };
    let mut translated = [0u16; 8];
    let count = unsafe {
        ToUnicodeEx(
            key.0 as u32,
            scan_code,
            &state.keyboard_state,
            &mut translated,
            0,
            Some(keyboard_layout),
        )
    };
    if count > 0 {
        let typed = String::from_utf16_lossy(&translated[..count as usize]);
        let clean: String = typed.chars().filter(|c| !c.is_control()).collect();
        if !clean.is_empty() {
            state.search.insert(&clean);
            unsafe { state.update_results() };
        }
    }
}

fn sanitize_query(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .take(128)
        .collect()
}

fn clipboard_text(hwnd: HWND) -> Option<String> {
    const CF_UNICODETEXT: u32 = 13;
    unsafe {
        OpenClipboard(Some(hwnd)).ok()?;
        let text = GetClipboardData(CF_UNICODETEXT).ok().and_then(|handle| {
            let global = HGLOBAL(handle.0);
            let pointer = GlobalLock(global) as *const u16;
            if pointer.is_null() {
                return None;
            }
            let mut length = 0;
            while length < 4096 && *pointer.add(length) != 0 {
                length += 1;
            }
            let value = String::from_utf16_lossy(std::slice::from_raw_parts(pointer, length));
            let _ = GlobalUnlock(global);
            Some(value)
        });
        let _ = CloseClipboard();
        text
    }
}

fn is_modifier_key(key: VIRTUAL_KEY) -> bool {
    matches!(
        key,
        VK_CONTROL
            | VK_LCONTROL
            | VK_RCONTROL
            | VK_SHIFT
            | VK_LSHIFT
            | VK_RSHIFT
            | VK_MENU
            | VK_LMENU
            | VK_RMENU
            | VK_LWIN
            | VK_RWIN
    )
}

fn captured_key_down(keyboard_state: &[u8; 256], key: VIRTUAL_KEY) -> bool {
    keyboard_state[key.0 as usize] & 0x80 != 0
}

fn update_captured_keyboard_state(keyboard_state: &mut [u8; 256], key: VIRTUAL_KEY, key_up: bool) {
    let index = key.0 as usize;
    if index >= keyboard_state.len() {
        return;
    }
    if key_up {
        keyboard_state[index] &= 0x7f;
    } else {
        keyboard_state[index] |= 0x80;
    }
    let aggregate = match key {
        VK_LCONTROL | VK_RCONTROL => Some((VK_CONTROL, VK_LCONTROL, VK_RCONTROL)),
        VK_LSHIFT | VK_RSHIFT => Some((VK_SHIFT, VK_LSHIFT, VK_RSHIFT)),
        VK_LMENU | VK_RMENU => Some((VK_MENU, VK_LMENU, VK_RMENU)),
        _ => None,
    };
    if let Some((generic, left, right)) = aggregate {
        let down =
            captured_key_down(keyboard_state, left) || captured_key_down(keyboard_state, right);
        let generic_index = generic.0 as usize;
        keyboard_state[generic_index] =
            (keyboard_state[generic_index] & 0x7f) | if down { 0x80 } else { 0 };
    }
}

fn captured_commit_keys_released(keyboard_state: &[u8; 256]) -> bool {
    [VK_CONTROL, VK_MENU, VK_SHIFT, VK_LWIN, VK_RWIN, VK_RETURN]
        .iter()
        .all(|key| !captured_key_down(keyboard_state, *key))
}

fn captured_hotkey_modifiers(keyboard_state: &[u8; 256]) -> u32 {
    let mut modifiers = MOD_NOREPEAT_VALUE;
    if captured_key_down(keyboard_state, VK_CONTROL) {
        modifiers |= MOD_CONTROL_VALUE;
    }
    if captured_key_down(keyboard_state, VK_MENU) {
        modifiers |= MOD_ALT_VALUE;
    }
    if captured_key_down(keyboard_state, VK_SHIFT) {
        modifiers |= MOD_SHIFT_VALUE;
    }
    if captured_key_down(keyboard_state, VK_LWIN) || captured_key_down(keyboard_state, VK_RWIN) {
        modifiers |= MOD_WIN_VALUE;
    }
    modifiers
}

unsafe fn handle_picker_key(state: &mut AppState, key: VIRTUAL_KEY, control: bool) -> bool {
    if control && key.0 == VK_OEM_COMMA_VALUE {
        unsafe {
            enter_settings(state);
        }
        return true;
    }
    if control && key.0 == VK_G_VALUE {
        unsafe {
            focus_browser(state);
        }
        return true;
    }
    if key == VK_ESCAPE {
        unsafe {
            hide_picker(state);
        }
        return true;
    }
    if key == VK_TAB {
        return true;
    }

    let browsing = state.browsing();
    if browsing {
        let columns = state
            .section_layouts()
            .get(state.browse_focus.0)
            .map_or(1, |layout| layout.columns) as isize;
        if key == VK_LEFT || (control && key.0 == VK_H_VALUE) {
            unsafe {
                state.move_browse_selection(-1);
            }
            return true;
        }
        if key == VK_RIGHT || (control && key.0 == VK_L_VALUE) {
            unsafe {
                state.move_browse_selection(1);
            }
            return true;
        }
        if key == VK_UP || (control && key.0 == VK_K_VALUE) {
            unsafe {
                state.move_browse_selection(-columns);
            }
            return true;
        }
        if key == VK_DOWN || (control && key.0 == VK_J_VALUE) {
            unsafe {
                state.move_browse_selection(columns);
            }
            return true;
        }
        let viewport = (state.footer_top() - BROWSE_CONTENT_TOP) as f32;
        if key == VK_PRIOR || (control && key.0 == VK_U_VALUE) {
            let amount = if key == VK_PRIOR { 0.88 } else { 0.5 };
            unsafe {
                state.scroll_browse(-viewport * amount);
            }
            return true;
        }
        if key == VK_NEXT || (control && key.0 == VK_D_VALUE) {
            let amount = if key == VK_NEXT { 0.88 } else { 0.5 };
            unsafe {
                state.scroll_browse(viewport * amount);
            }
            return true;
        }
    } else {
        if key == VK_UP || (control && key.0 == VK_K_VALUE) {
            unsafe {
                state.move_selection(-1);
            }
            return true;
        }
        if key == VK_DOWN || (control && key.0 == VK_J_VALUE) {
            unsafe {
                state.move_selection(1);
            }
            return true;
        }
    }

    if key == VK_RETURN {
        unsafe {
            commit_selection(state, control);
        }
        return true;
    }
    false
}

unsafe fn handle_settings_key(state: &mut AppState, key: VIRTUAL_KEY, control: bool) -> bool {
    if state.capturing_shortcut {
        if key == VK_ESCAPE {
            state.capturing_shortcut = false;
            state.status = None;
            unsafe {
                let _ = InvalidateRect(Some(state.hwnd), None, false);
            }
            return true;
        }
        if matches!(key, VK_CONTROL | VK_SHIFT | VK_MENU | VK_LWIN | VK_RWIN) {
            return true;
        }
        let modifiers = if state.capture_active {
            captured_hotkey_modifiers(&state.keyboard_state)
        } else {
            current_hotkey_modifiers()
        };
        match Hotkey::from_parts(modifiers, key.0 as u32) {
            Ok(hotkey) => {
                state.config.hotkey = hotkey;
                state.capturing_shortcut = false;
                state.status = None;
                unsafe {
                    state.sync_accessible_results();
                    let _ = InvalidateRect(Some(state.hwnd), None, false);
                }
            }
            Err(error) => state.status = Some(error),
        }
        return true;
    }

    if key == VK_ESCAPE {
        unsafe {
            save_settings(state);
        }
        return true;
    }
    if key == VK_RETURN {
        unsafe {
            activate_setting(state);
        }
        return true;
    }
    if key == VK_UP || (control && key.0 == VK_K_VALUE) {
        state.settings_selected = state.settings_selected.saturating_sub(1);
        state.selected = state.settings_selected;
        unsafe {
            state.sync_accessible_results();
            let _ = InvalidateRect(Some(state.hwnd), None, false);
        }
        return true;
    }
    if key == VK_DOWN || (control && key.0 == VK_J_VALUE) || key == VK_TAB {
        state.settings_selected = (state.settings_selected + 1).min(4);
        state.selected = state.settings_selected;
        unsafe {
            state.sync_accessible_results();
            let _ = InvalidateRect(Some(state.hwnd), None, false);
        }
        return true;
    }
    if key == VK_LEFT || key == VK_RIGHT {
        unsafe {
            adjust_setting(state, if key == VK_LEFT { -1 } else { 1 });
        }
        return true;
    }
    if key.0 == 0x20 && state.settings_selected == 4 {
        state.capturing_shortcut = true;
        state.status = Some("Press the new shortcut".to_string());
        unsafe {
            let _ = InvalidateRect(Some(state.hwnd), None, false);
        }
        return true;
    }
    false
}

fn current_hotkey_modifiers() -> u32 {
    let mut modifiers = MOD_NOREPEAT_VALUE;
    if unsafe { GetKeyState(VK_CONTROL.0 as i32) } < 0 {
        modifiers |= MOD_CONTROL_VALUE;
    }
    if unsafe { GetKeyState(VK_MENU.0 as i32) } < 0 {
        modifiers |= MOD_ALT_VALUE;
    }
    if unsafe { GetKeyState(VK_SHIFT.0 as i32) } < 0 {
        modifiers |= MOD_SHIFT_VALUE;
    }
    if unsafe { GetKeyState(VK_LWIN.0 as i32) } < 0 || unsafe { GetKeyState(VK_RWIN.0 as i32) } < 0
    {
        modifiers |= MOD_WIN_VALUE;
    }
    modifiers
}

unsafe fn commit_selection(state: &mut AppState, close_after: bool) {
    let Some(text) = state.selected_text().map(str::to_owned) else {
        return;
    };
    let target = state.target;
    let target_focus = state.target_focus;
    // Capture stays active: the hook passes injected events through, so the
    // send is not re-captured. An insert that keeps the picker open leaves
    // the window exactly where it is.
    if close_after {
        unsafe {
            hide_picker(state);
        }
    }
    match unsafe { inject_unicode(target, target_focus, &text) } {
        Ok(()) => {
            if let Err(error) = remember_recent(&mut state.recents, &text) {
                eprintln!("winmoji: could not save recent item: {error}");
            }
            if !close_after {
                state.rebuild_browse_sections_preserving_view();
                unsafe {
                    state.sync_accessible_results();
                    let _ = InvalidateRect(Some(state.hwnd), None, false);
                }
            } else {
                state.rebuild_browse_sections();
            }
        }
        Err(error) => {
            eprintln!("winmoji: input failed: {error}");
            state.status =
                Some("Could not return to the previous app. Nothing was inserted.".to_string());
            if close_after {
                unsafe { restore_picker(state) };
            } else {
                unsafe {
                    let _ = InvalidateRect(Some(state.hwnd), None, false);
                }
            }
        }
    }
}

unsafe fn restore_picker(state: &mut AppState) {
    unsafe {
        if state.keep_visible {
            let _ = start_keyboard_capture(state);
            state.sync_accessible_results();
            let _ = InvalidateRect(Some(state.hwnd), None, false);
            return;
        }
        let _ = ShowWindow(state.hwnd, SW_SHOWNOACTIVATE);
        if GetForegroundWindow() == state.target && start_keyboard_capture(state).is_ok() {
            arm_focus_watch(state);
            state.sync_accessible_results();
            let _ = InvalidateRect(Some(state.hwnd), None, false);
        } else {
            hide_picker(state);
        }
    }
}

unsafe fn position_near_cursor(state: &mut AppState) {
    let mut cursor = POINT::default();
    unsafe {
        GetCursorPos(&mut cursor).ok();
    }
    let monitor = unsafe { MonitorFromPoint(cursor, MONITOR_DEFAULTTONEAREST) };
    let mut monitor_info = MONITORINFO {
        cbSize: size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    unsafe {
        let _ = GetMonitorInfoW(monitor, &mut monitor_info);
    }
    constrain_dimensions_to_work_area(state, &monitor_info.rcWork);
    let (base_width, base_height) = state.dimensions();
    unsafe {
        SetWindowPos(
            state.hwnd,
            None,
            cursor.x,
            cursor.y,
            scale(base_width, state.dpi),
            scale(base_height, state.dpi),
            SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .ok();
    }
    let dpi_x = unsafe { GetDpiForWindow(state.hwnd) }.max(96);
    let dpi_y = dpi_x;
    state.dpi = dpi_x;
    let width = scale(base_width, dpi_x);
    let height = scale(base_height, dpi_y);
    let mut x = cursor.x + scale(14, dpi_x);
    let mut y = cursor.y + scale(14, dpi_y);
    if x + width > monitor_info.rcWork.right {
        x = cursor.x - width - scale(14, dpi_x);
    }
    if y + height > monitor_info.rcWork.bottom {
        y = cursor.y - height - scale(14, dpi_y);
    }
    x = x.max(monitor_info.rcWork.left);
    y = y.max(monitor_info.rcWork.top);
    unsafe {
        SetWindowPos(
            state.hwnd,
            None,
            x,
            y,
            width,
            height,
            SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .ok();
        layout(state);
    }
}

fn constrain_dimensions_to_work_area(state: &mut AppState, work_area: &RECT) {
    let maximum_width = ((work_area.right - work_area.left) * 96 / state.dpi as i32)
        .clamp(MIN_PICKER_WIDTH, MAX_PICKER_WIDTH);
    let maximum_height = ((work_area.bottom - work_area.top) * 96 / state.dpi as i32)
        .clamp(MIN_PICKER_HEIGHT, MAX_PICKER_HEIGHT);
    state.config.dimensions.width = state
        .config
        .dimensions
        .width
        .clamp(MIN_PICKER_WIDTH, maximum_width);
    state.config.dimensions.height = state
        .config
        .dimensions
        .height
        .clamp(MIN_PICKER_HEIGHT, maximum_height);
    state.display_dimensions = state.config.dimensions;
    state.clamp_browse_scroll();
    state.clamp_category_scroll();
}

unsafe fn resize_window_in_place(state: &mut AppState) {
    let mut window = RECT::default();
    if unsafe { GetWindowRect(state.hwnd, &mut window) }.is_err() {
        return;
    }
    let monitor = unsafe {
        MonitorFromPoint(
            POINT {
                x: window.left,
                y: window.top,
            },
            MONITOR_DEFAULTTONEAREST,
        )
    };
    let mut monitor_info = MONITORINFO {
        cbSize: size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    if !unsafe { GetMonitorInfoW(monitor, &mut monitor_info) }.as_bool() {
        return;
    }
    constrain_dimensions_to_work_area(state, &monitor_info.rcWork);
    let width = scale(state.config.dimensions.width, state.dpi);
    let height = scale(state.config.dimensions.height, state.dpi);
    let x = window
        .left
        .clamp(monitor_info.rcWork.left, monitor_info.rcWork.right - width);
    let y = window
        .top
        .clamp(monitor_info.rcWork.top, monitor_info.rcWork.bottom - height);
    unsafe {
        state.render = None;
        SetWindowPos(
            state.hwnd,
            None,
            x,
            y,
            width,
            height,
            SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .ok();
        layout(state);
        state.sync_accessible_results();
        let _ = InvalidateRect(Some(state.hwnd), None, false);
    }
}

unsafe fn layout(state: &AppState) {
    let (width, _) = state.dimensions();
    unsafe {
        SetWindowPos(
            state.accessible_results,
            None,
            scale(12, state.dpi),
            scale(SEARCH_RESULTS_TOP, state.dpi),
            scale(width - 24, state.dpi),
            scale(state.footer_top() - SEARCH_RESULTS_TOP, state.dpi),
            SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .ok();
    }
}

fn mouse_point_dip(lparam: LPARAM, dpi: u32) -> (f32, f32) {
    let x = lparam.0 as u16 as i16 as i32;
    let y = (lparam.0 >> 16) as u16 as i16 as i32;
    (x as f32 * 96.0 / dpi as f32, y as f32 * 96.0 / dpi as f32)
}

unsafe fn route_wheel(state: &mut AppState, horizontal: bool, wparam: WPARAM, lparam: LPARAM) {
    if state.view != View::Search || !state.browsing() {
        return;
    }
    let notches = ((wparam.0 >> 16) as u16 as i16 as f32) / 120.0;
    // Wheel messages carry the cursor position in screen coordinates.
    let mut point = POINT {
        x: lparam.0 as u16 as i16 as i32,
        y: (lparam.0 >> 16) as u16 as i16 as i32,
    };
    if !unsafe { ScreenToClient(state.hwnd, &mut point) }.as_bool() {
        return;
    }
    let y = point.y as f32 * 96.0 / state.dpi as f32;
    let over_categories = (CATEGORY_TOP..CATEGORY_TOP + CATEGORY_HEIGHT).contains(&(y as i32));
    if horizontal {
        if over_categories {
            unsafe {
                state.scroll_categories(notches * CATEGORY_BUTTON_WIDTH * 2.0);
            }
        }
    } else if over_categories {
        unsafe {
            state.scroll_categories(-notches * CATEGORY_BUTTON_WIDTH * 2.0);
        }
    } else {
        unsafe {
            state.scroll_browse(-notches * 76.0);
        }
    }
}

fn contains(bounds: D2D_RECT_F, x: f32, y: f32) -> bool {
    x >= bounds.left && x <= bounds.right && y >= bounds.top && y <= bounds.bottom
}

fn header_button_rect(width: i32, position: usize) -> D2D_RECT_F {
    let right = width as f32 - 8.0 - position as f32 * 30.0;
    rect(right - 26.0, 4.0, right, 30.0)
}

fn category_viewport(width: i32) -> D2D_RECT_F {
    let overflow = maximum_category_scroll(width) > 0.0;
    let inset = if overflow {
        CATEGORY_EDGE_WIDTH + 4.0
    } else {
        12.0
    };
    rect(
        inset,
        CATEGORY_TOP as f32,
        width as f32 - inset,
        (CATEGORY_TOP + CATEGORY_HEIGHT) as f32,
    )
}

fn maximum_category_scroll(width: i32) -> f32 {
    let available = (width as f32 - (CATEGORY_EDGE_WIDTH + 4.0) * 2.0).max(1.0);
    (BrowseCategory::ALL.len() as f32 * CATEGORY_BUTTON_WIDTH - available).max(0.0)
}

fn category_rect(width: i32, scroll: f32, index: usize) -> D2D_RECT_F {
    let viewport = category_viewport(width);
    let left = viewport.left + index as f32 * CATEGORY_BUTTON_WIDTH - scroll;
    rect(
        left,
        CATEGORY_TOP as f32,
        left + CATEGORY_BUTTON_WIDTH,
        (CATEGORY_TOP + CATEGORY_HEIGHT) as f32,
    )
}

fn category_edge_rects(width: i32) -> Option<(D2D_RECT_F, D2D_RECT_F)> {
    (maximum_category_scroll(width) > 0.0).then(|| {
        (
            rect(
                2.0,
                CATEGORY_TOP as f32,
                CATEGORY_EDGE_WIDTH + 2.0,
                (CATEGORY_TOP + CATEGORY_HEIGHT) as f32,
            ),
            rect(
                width as f32 - CATEGORY_EDGE_WIDTH - 2.0,
                CATEGORY_TOP as f32,
                width as f32 - 2.0,
                (CATEGORY_TOP + CATEGORY_HEIGHT) as f32,
            ),
        )
    })
}

fn visible_item_range(
    layout: SectionLayout,
    item_count: usize,
    viewport_top: f32,
    viewport_bottom: f32,
) -> std::ops::Range<usize> {
    if item_count == 0
        || layout.bottom as f32 <= viewport_top
        || layout.top as f32 >= viewport_bottom
    {
        return 0..0;
    }
    let rows = item_count.div_ceil(layout.columns);
    let first_row = ((viewport_top - layout.grid_top as f32).max(0.0).floor() as usize)
        / layout.cell_height as usize;
    let last_row = (((viewport_bottom - layout.grid_top as f32).max(0.0)
        / layout.cell_height as f32)
        .ceil() as usize)
        .min(rows);
    let start = (first_row * layout.columns).min(item_count);
    let end = (last_row * layout.columns).min(item_count);
    start..end.max(start)
}

fn search_clear_rect(width: i32) -> D2D_RECT_F {
    rect(
        width as f32 - 44.0,
        SEARCH_TOP as f32 + 9.0,
        width as f32 - 20.0,
        (SEARCH_TOP + SEARCH_HEIGHT) as f32 - 9.0,
    )
}

fn footer_button_rects(width: i32, footer_top: i32) -> (D2D_RECT_F, D2D_RECT_F) {
    let top = footer_top as f32 + 8.0;
    let bottom = footer_top as f32 + 34.0;
    let close_right = width as f32 - 12.0;
    let close_left = close_right - 104.0;
    let insert_right = close_left - 8.0;
    let insert_left = insert_right - 60.0;
    (
        rect(insert_left, top, insert_right, bottom),
        rect(close_left, top, close_right, bottom),
    )
}

fn browse_scrollbar_rects(state: &AppState) -> Option<(D2D_RECT_F, D2D_RECT_F)> {
    let (width, _) = state.dimensions();
    let viewport = (state.footer_top() - BROWSE_CONTENT_TOP).max(1) as f32;
    let total = state.total_browse_height().max(viewport as i32) as f32;
    if total <= viewport {
        return None;
    }
    let track = rect(
        width as f32 - 13.0,
        BROWSE_CONTENT_TOP as f32 + 4.0,
        width as f32 - 2.0,
        state.footer_top() as f32 - 4.0,
    );
    let track_height = track.bottom - track.top;
    let thumb_height = (track_height * viewport / total).max(24.0);
    let maximum = state.maximum_browse_scroll().max(1.0);
    let thumb_top = track.top + (track_height - thumb_height) * state.browse_scroll / maximum;
    Some((
        track,
        rect(
            width as f32 - 8.0,
            thumb_top,
            width as f32 - 3.0,
            thumb_top + thumb_height,
        ),
    ))
}

fn settings_row_rect(width: i32, index: usize) -> D2D_RECT_F {
    let top = 42.0 + index as f32 * 38.0;
    rect(12.0, top, width as f32 - 12.0, top + 34.0)
}

fn slider_rect(width: i32, index: usize) -> D2D_RECT_F {
    let row = settings_row_rect(width, index);
    rect(
        (width as f32 * 0.45).max(164.0),
        row.top,
        width as f32 - 24.0,
        row.bottom,
    )
}

fn settings_footer_rects(width: i32, footer_top: i32) -> (D2D_RECT_F, D2D_RECT_F, D2D_RECT_F) {
    (
        rect(
            12.0,
            footer_top as f32 + 7.0,
            74.0,
            footer_top as f32 + 35.0,
        ),
        rect(
            80.0,
            footer_top as f32 + 7.0,
            134.0,
            footer_top as f32 + 35.0,
        ),
        rect(
            width as f32 - 68.0,
            footer_top as f32 + 7.0,
            width as f32 - 12.0,
            footer_top as f32 + 35.0,
        ),
    )
}

fn hit_test(state: &AppState, x: f32, y: f32) -> Option<HitTarget> {
    let (width, _) = state.dimensions();
    if contains(header_button_rect(width, 0), x, y) {
        return Some(HitTarget::Close);
    }
    if contains(header_button_rect(width, 1), x, y) {
        return Some(HitTarget::Settings);
    }
    if state.view == View::Search && contains(header_button_rect(width, 2), x, y) {
        return Some(HitTarget::Browse);
    }
    if state.view == View::Search
        && !state.query().trim().is_empty()
        && contains(search_clear_rect(width), x, y)
    {
        return Some(HitTarget::SearchClear);
    }
    if state.view == View::Settings {
        for index in 0..5 {
            if contains(settings_row_rect(width, index), x, y) {
                if index == 0 && contains(slider_rect(width, index), x, y) {
                    return Some(HitTarget::WidthSlider);
                }
                if index == 1 && contains(slider_rect(width, index), x, y) {
                    return Some(HitTarget::HeightSlider);
                }
                return Some(HitTarget::SettingRow(index));
            }
        }
        let (discard, reset, back) = settings_footer_rects(width, state.footer_top());
        return if contains(discard, x, y) {
            Some(HitTarget::SettingsDiscard)
        } else if contains(reset, x, y) {
            Some(HitTarget::SettingsReset)
        } else if contains(back, x, y) {
            Some(HitTarget::SettingsBack)
        } else {
            None
        };
    }
    let (insert, insert_close) = footer_button_rects(width, state.footer_top());
    if contains(insert, x, y) {
        return Some(HitTarget::Insert);
    }
    if contains(insert_close, x, y) {
        return Some(HitTarget::InsertClose);
    }
    if state.query().trim().is_empty() {
        if let Some((left, right)) = category_edge_rects(width) {
            if contains(left, x, y) {
                return Some(HitTarget::CategoryScrollLeft);
            }
            if contains(right, x, y) {
                return Some(HitTarget::CategoryScrollRight);
            }
        }
        let category_viewport = category_viewport(width);
        for index in 0..BrowseCategory::ALL.len() {
            if contains(category_viewport, x, y)
                && contains(category_rect(width, state.category_scroll, index), x, y)
            {
                return Some(HitTarget::Category(index));
            }
        }
        if browse_scrollbar_rects(state).is_some_and(|(track, _)| contains(track, x, y)) {
            return Some(HitTarget::BrowseScrollbar);
        }
        if y < BROWSE_CONTENT_TOP as f32 || y >= state.footer_top() as f32 {
            return None;
        }
        let content_y = y - BROWSE_CONTENT_TOP as f32 + state.browse_scroll;
        for (section, layout) in state.section_layouts().iter().enumerate() {
            if content_y < layout.grid_top as f32 || content_y >= layout.bottom as f32 {
                continue;
            }
            let grid_width = layout.columns as f32 * layout.cell_width;
            let grid_left = (width as f32 - grid_width) / 2.0;
            let row = ((content_y - layout.grid_top as f32) / layout.cell_height as f32) as usize;
            let column = ((x - grid_left) / layout.cell_width).floor() as isize;
            if column < 0 || column >= layout.columns as isize {
                return None;
            }
            let item = row * layout.columns + column as usize;
            if item < state.browse_sections[section].indices.len() {
                return Some(HitTarget::BrowseItem { section, item });
            }
        }
        None
    } else if y >= SEARCH_RESULTS_TOP as f32 && y < state.footer_top() as f32 {
        let row = ((y as i32 - SEARCH_RESULTS_TOP) / RESULT_ROW_HEIGHT) as usize;
        (row < state.matches.len()).then_some(HitTarget::SearchResult(row))
    } else {
        None
    }
}

unsafe fn update_hover(state: &mut AppState, x: f32, y: f32) {
    let target = hit_test(state, x, y);
    let entry = match target {
        Some(HitTarget::SearchResult(row)) => state.matches.get(row).map(|found| found.index),
        Some(HitTarget::BrowseItem { section, item }) => state
            .browse_sections
            .get(section)
            .and_then(|section| section.indices.get(item))
            .copied(),
        _ => None,
    };
    if target != state.hovered_target || entry != state.hovered_entry {
        state.hovered_target = target;
        state.hovered_entry = entry;
        unsafe {
            let _ = InvalidateRect(Some(state.hwnd), None, false);
        }
    }
}

unsafe fn update_dragged_slider(state: &mut AppState, x: f32) {
    let Some(slider) = state.dragging_slider else {
        return;
    };
    let index = if slider == Slider::Width { 0 } else { 1 };
    let track = slider_rect(state.dimensions().0, index);
    let ratio = ((x - track.left) / (track.right - track.left)).clamp(0.0, 1.0);
    match slider {
        Slider::Width => {
            state.config.dimensions.width =
                MIN_PICKER_WIDTH + ((MAX_PICKER_WIDTH - MIN_PICKER_WIDTH) as f32 * ratio) as i32;
        }
        Slider::Height => {
            state.config.dimensions.height =
                MIN_PICKER_HEIGHT + ((MAX_PICKER_HEIGHT - MIN_PICKER_HEIGHT) as f32 * ratio) as i32;
        }
    }
    unsafe {
        state.sync_accessible_results();
        let _ = InvalidateRect(Some(state.hwnd), None, false);
    }
}

unsafe fn update_dragged_scrollbar(state: &mut AppState, y: f32) {
    let Some(offset) = state.dragging_scrollbar else {
        return;
    };
    let Some((track, thumb)) = browse_scrollbar_rects(state) else {
        return;
    };
    let thumb_height = thumb.bottom - thumb.top;
    let available = (track.bottom - track.top - thumb_height).max(1.0);
    let thumb_top = (y - offset).clamp(track.top, track.bottom - thumb_height);
    let position = (thumb_top - track.top) / available * state.maximum_browse_scroll();
    unsafe {
        state.set_browse_scroll_immediate(position);
    }
}

unsafe fn handle_click(state: &mut AppState, x: f32, y: f32) {
    let Some(target) = hit_test(state, x, y) else {
        return;
    };
    match target {
        HitTarget::Close if state.view == View::Settings => unsafe { save_settings(state) },
        HitTarget::Close => unsafe { hide_picker(state) },
        HitTarget::Settings => unsafe { enter_settings(state) },
        HitTarget::Browse => unsafe { focus_browser(state) },
        HitTarget::SearchClear => {
            state.search.clear();
            unsafe {
                state.update_results();
            }
        }
        HitTarget::Category(index) => unsafe { state.jump_to_category(index) },
        HitTarget::CategoryScrollLeft => unsafe {
            state.scroll_categories(-CATEGORY_BUTTON_WIDTH * 2.0)
        },
        HitTarget::CategoryScrollRight => unsafe {
            state.scroll_categories(CATEGORY_BUTTON_WIDTH * 2.0)
        },
        HitTarget::SearchResult(row) => {
            state.selected = row;
            unsafe {
                state.sync_accessible_results();
                commit_selection(state, false);
            }
        }
        HitTarget::BrowseItem { section, item } => {
            state.browse_focus = (section, item);
            unsafe {
                state.sync_accessible_results();
                commit_selection(state, false);
            }
        }
        HitTarget::BrowseScrollbar => {
            if let Some((_, thumb)) = browse_scrollbar_rects(state) {
                state.dragging_scrollbar = Some(if contains(thumb, x, y) {
                    y - thumb.top
                } else {
                    (thumb.bottom - thumb.top) / 2.0
                });
                unsafe {
                    let _ = SetCapture(state.hwnd);
                    update_dragged_scrollbar(state, y);
                }
            }
        }
        HitTarget::Insert => unsafe { commit_selection(state, false) },
        HitTarget::InsertClose => unsafe { commit_selection(state, true) },
        HitTarget::SettingRow(index) => {
            state.settings_selected = index;
            state.selected = index;
            if index == 4 {
                state.capturing_shortcut = true;
                state.status = Some("Press the new shortcut.".to_string());
            } else if index >= 2 && x >= state.dimensions().0 as f32 * 0.42 {
                let midpoint = state.dimensions().0 as f32 * 0.7;
                unsafe {
                    adjust_setting(state, if x < midpoint { -1 } else { 1 });
                }
            }
            unsafe {
                state.sync_accessible_results();
                let _ = InvalidateRect(Some(state.hwnd), None, false);
            }
        }
        HitTarget::WidthSlider | HitTarget::HeightSlider => {
            state.dragging_slider = Some(if target == HitTarget::WidthSlider {
                Slider::Width
            } else {
                Slider::Height
            });
            unsafe {
                let _ = SetCapture(state.hwnd);
                update_dragged_slider(state, x);
            }
        }
        HitTarget::SettingsDiscard => unsafe { discard_settings(state) },
        HitTarget::SettingsReset => unsafe { reset_settings(state) },
        HitTarget::SettingsBack => unsafe { save_settings(state) },
    }
}

unsafe fn paint(state: &mut AppState) {
    let mut paint = PAINTSTRUCT::default();
    unsafe {
        BeginPaint(state.hwnd, &mut paint);
    }
    if let Err(error) = unsafe { draw_picker(state) } {
        state.render = None;
        eprintln!("winmoji: rendering failed: {error}");
    }
    unsafe {
        let _ = EndPaint(state.hwnd, &paint);
    }
}

unsafe fn ensure_render_target(state: &mut AppState) -> Result<(ID2D1HwndRenderTarget, Brushes)> {
    if let Some((target, brushes)) = &state.render {
        return Ok((target.clone(), brushes.clone()));
    }
    let mut client = RECT::default();
    unsafe {
        GetClientRect(state.hwnd, &mut client)?;
    }
    let properties = D2D1_RENDER_TARGET_PROPERTIES {
        r#type: D2D1_RENDER_TARGET_TYPE_DEFAULT,
        pixelFormat: D2D1_PIXEL_FORMAT {
            format: DXGI_FORMAT_UNKNOWN,
            alphaMode: D2D1_ALPHA_MODE_UNKNOWN,
        },
        dpiX: state.dpi as f32,
        dpiY: state.dpi as f32,
        usage: D2D1_RENDER_TARGET_USAGE_NONE,
        minLevel: D2D1_FEATURE_LEVEL_DEFAULT,
    };
    let window_properties = D2D1_HWND_RENDER_TARGET_PROPERTIES {
        hwnd: state.hwnd,
        pixelSize: D2D_SIZE_U {
            width: client.right.max(1) as u32,
            height: client.bottom.max(1) as u32,
        },
        presentOptions: D2D1_PRESENT_OPTIONS_NONE,
    };
    let target = unsafe {
        state
            .d2d_factory
            .CreateHwndRenderTarget(&properties, &window_properties)?
    };
    let brushes = unsafe { create_brushes(&target)? };
    state.render = Some((target.clone(), brushes.clone()));
    Ok((target, brushes))
}

unsafe fn draw_picker(state: &mut AppState) -> Result<()> {
    match state.view {
        View::Search => unsafe { draw_search_picker(state) },
        View::Settings => unsafe { draw_settings_picker(state) },
    }
}

unsafe fn draw_search_picker(state: &mut AppState) -> Result<()> {
    let (target, brushes) = unsafe { ensure_render_target(state)? };
    let (width, height) = state.dimensions();
    let footer_top = state.footer_top() as f32;

    unsafe {
        target.BeginDraw();
        target.Clear(Some(&color(0x101217)));
        target.DrawRoundedRectangle(
            &rounded_rect(0.5, 0.5, width as f32 - 0.5, height as f32 - 0.5, 11.0),
            &brushes.surface_border,
            1.0,
            None,
        );
        draw_text(
            &target,
            "WinMoji",
            &state.formats.label,
            rect(18.0, 7.0, 180.0, 34.0),
            &brushes.primary,
            D2D1_DRAW_TEXT_OPTIONS_NONE,
        );
        draw_header_button(
            &target,
            state,
            width,
            2,
            "⌂",
            &brushes.surface,
            &brushes.secondary,
        );
        draw_header_button(
            &target,
            state,
            width,
            1,
            "⚙",
            &brushes.surface,
            &brushes.secondary,
        );
        draw_header_button(
            &target,
            state,
            width,
            0,
            "×",
            &brushes.surface,
            &brushes.secondary,
        );
    }

    let search = rounded_rect(
        14.0,
        SEARCH_TOP as f32,
        width as f32 - 14.0,
        (SEARCH_TOP + SEARCH_HEIGHT) as f32,
        12.0,
    );
    unsafe {
        target.FillRoundedRectangle(&search, &brushes.surface);
        target.DrawRoundedRectangle(&search, &brushes.surface_border, 1.0, None);
        target.DrawEllipse(
            &D2D1_ELLIPSE {
                point: Vector2 {
                    X: 31.0,
                    Y: (SEARCH_TOP + SEARCH_HEIGHT / 2) as f32,
                },
                radiusX: 6.0,
                radiusY: 6.0,
            },
            &brushes.secondary,
            1.7,
            None,
        );
        target.DrawLine(
            Vector2 {
                X: 35.5,
                Y: (SEARCH_TOP + SEARCH_HEIGHT / 2 + 4) as f32,
            },
            Vector2 {
                X: 40.0,
                Y: (SEARCH_TOP + SEARCH_HEIGHT / 2 + 8) as f32,
            },
            &brushes.secondary,
            1.7,
            None,
        );
        draw_search_text(state, &target, &brushes)?;
        if !state.query().trim().is_empty() {
            draw_button(
                &target,
                search_clear_rect(width),
                "×",
                matches!(state.hovered_target, Some(HitTarget::SearchClear)),
                &brushes.surface,
                &brushes.surface_border,
                &brushes.secondary,
                &state.formats.icon,
            );
        }
    }

    if state.browsing() {
        unsafe {
            draw_browser(
                state,
                &target,
                &brushes.surface,
                &brushes.surface_border,
                &brushes.selection,
                &brushes.selection_border,
                &brushes.glyph_surface,
                &brushes.primary,
                &brushes.secondary,
                &brushes.accent,
            );
        }
    } else {
        unsafe {
            draw_search_results(
                state,
                &target,
                &brushes.selection,
                &brushes.selection_border,
                &brushes.glyph_surface,
                &brushes.primary,
                &brushes.secondary,
                &brushes.accent,
            );
        }
    }

    unsafe {
        target.DrawLine(
            Vector2 {
                X: 16.0,
                Y: footer_top,
            },
            Vector2 {
                X: width as f32 - 16.0,
                Y: footer_top,
            },
            &brushes.surface_border,
            1.0,
            None,
        );
        let (insert, insert_close) = footer_button_rects(width, state.footer_top());
        let information = rect(14.0, footer_top, insert.left - 10.0, height as f32 - 2.0);
        if let Some(status) = &state.status {
            draw_text(
                &target,
                status,
                &state.formats.metadata,
                information,
                &brushes.danger,
                D2D1_DRAW_TEXT_OPTIONS_CLIP,
            );
        } else {
            draw_entry_information(state, &target, information, &brushes.secondary);
        }
        draw_button(
            &target,
            insert,
            "Insert",
            matches!(state.hovered_target, Some(HitTarget::Insert)),
            &brushes.surface,
            &brushes.selection_border,
            &brushes.primary,
            &state.formats.center,
        );
        draw_button(
            &target,
            insert_close,
            "Insert + close",
            matches!(state.hovered_target, Some(HitTarget::InsertClose)),
            &brushes.surface,
            &brushes.selection_border,
            &brushes.primary,
            &state.formats.center,
        );
        draw_hover_help(
            state,
            &target,
            &brushes.surface,
            &brushes.surface_border,
            &brushes.primary,
        );
        target.EndDraw(None, None)?;
    }
    Ok(())
}

/// Draw the query text, selection highlight, and caret. The picker never
/// activates, so no native control could ever show a caret here; the field
/// is rendered directly instead.
unsafe fn draw_search_text(
    state: &mut AppState,
    target: &ID2D1HwndRenderTarget,
    brushes: &Brushes,
) -> Result<()> {
    let (width, _) = state.dimensions();
    let text_left = 48.0f32;
    let text_right = width as f32 - if state.query().is_empty() { 24.0 } else { 52.0 };
    let top = SEARCH_TOP as f32 + 4.0;
    let bottom = (SEARCH_TOP + SEARCH_HEIGHT) as f32 - 4.0;
    if state.search.text.is_empty() {
        unsafe {
            draw_text(
                target,
                "Search names, symbols, or code points",
                &state.formats.search,
                rect(text_left, top, text_right, bottom),
                &brushes.secondary,
                D2D1_DRAW_TEXT_OPTIONS_CLIP,
            );
        }
        return Ok(());
    }

    let wide: Vec<u16> = state.search.text.encode_utf16().collect();
    let layout = unsafe {
        state
            .dwrite_factory
            .CreateTextLayout(&wide, &state.formats.search, 4096.0, bottom - top)?
    };
    let utf16_offset = |byte_offset: usize| -> u32 {
        state.search.text[..byte_offset].encode_utf16().count() as u32
    };
    let caret_x = unsafe { layout_caret_x(&layout, utf16_offset(state.search.caret))? };
    let visible = (text_right - text_left).max(8.0);
    if caret_x - state.search.scroll > visible - 2.0 {
        state.search.scroll = caret_x - visible + 2.0;
    }
    if caret_x < state.search.scroll {
        state.search.scroll = caret_x;
    }
    state.search.scroll = state.search.scroll.max(0.0);
    let origin_x = text_left - state.search.scroll;

    unsafe {
        target.PushAxisAlignedClip(
            &rect(text_left, top, text_right, bottom),
            D2D1_ANTIALIAS_MODE_ALIASED,
        );
        if state.search.has_selection() {
            let (start, end) = state.search.selection();
            let start_x = layout_caret_x(&layout, utf16_offset(start))?;
            let end_x = layout_caret_x(&layout, utf16_offset(end))?;
            target.FillRectangle(
                &rect(
                    origin_x + start_x,
                    top + 3.0,
                    origin_x + end_x,
                    bottom - 3.0,
                ),
                &brushes.selection_border,
            );
        }
        target.DrawTextLayout(
            Vector2 {
                X: origin_x,
                Y: top,
            },
            &layout,
            &brushes.primary,
            D2D1_DRAW_TEXT_OPTIONS_NONE,
        );
        let caret_line_x = (origin_x + caret_x).clamp(text_left, text_right - 1.0);
        target.DrawLine(
            Vector2 {
                X: caret_line_x,
                Y: top + 5.0,
            },
            Vector2 {
                X: caret_line_x,
                Y: bottom - 5.0,
            },
            &brushes.accent,
            1.6,
            None,
        );
        target.PopAxisAlignedClip();
    }
    Ok(())
}

unsafe fn layout_caret_x(layout: &IDWriteTextLayout, position: u32) -> Result<f32> {
    let mut x = 0.0f32;
    let mut y = 0.0f32;
    let mut metrics = DWRITE_HIT_TEST_METRICS::default();
    unsafe {
        layout.HitTestTextPosition(position, false, &mut x, &mut y, &mut metrics)?;
    }
    Ok(x)
}

#[allow(clippy::too_many_arguments)]
unsafe fn draw_browser(
    state: &AppState,
    target: &ID2D1HwndRenderTarget,
    surface: &ID2D1SolidColorBrush,
    border: &ID2D1SolidColorBrush,
    selection: &ID2D1SolidColorBrush,
    selection_border: &ID2D1SolidColorBrush,
    glyph_surface: &ID2D1SolidColorBrush,
    primary: &ID2D1SolidColorBrush,
    secondary: &ID2D1SolidColorBrush,
    accent: &ID2D1SolidColorBrush,
) {
    let (width, _) = state.dimensions();
    let category_viewport = category_viewport(width);
    unsafe {
        target.PushAxisAlignedClip(&category_viewport, D2D1_ANTIALIAS_MODE_ALIASED);
    }
    for (index, category) in BrowseCategory::ALL.iter().enumerate() {
        let bounds = category_rect(width, state.category_scroll, index);
        if bounds.right <= category_viewport.left || bounds.left >= category_viewport.right {
            continue;
        }
        let active = index == state.active_category;
        unsafe {
            if active {
                target.FillRoundedRectangle(
                    &rounded_rect(
                        bounds.left + 2.0,
                        bounds.top + 2.0,
                        bounds.right - 2.0,
                        bounds.bottom - 3.0,
                        7.0,
                    ),
                    selection,
                );
                target.FillRoundedRectangle(
                    &rounded_rect(
                        bounds.left + 12.0,
                        bounds.bottom - 3.0,
                        bounds.right - 12.0,
                        bounds.bottom - 1.0,
                        1.0,
                    ),
                    accent,
                );
            }
            let format = if *category == BrowseCategory::Emoticons {
                &state.formats.emoticon_icon
            } else if category.uses_color_icon() {
                &state.formats.glyph
            } else {
                &state.formats.symbol
            };
            draw_text(
                target,
                category.icon(),
                format,
                bounds,
                if active { primary } else { secondary },
                if category.uses_color_icon() {
                    D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT
                } else {
                    D2D1_DRAW_TEXT_OPTIONS_NONE
                },
            );
        }
    }
    unsafe {
        target.PopAxisAlignedClip();
    }
    if let Some((left, right)) = category_edge_rects(width) {
        unsafe {
            draw_text(
                target,
                "<",
                &state.formats.center_title,
                left,
                if state.category_scroll > 0.0 {
                    secondary
                } else {
                    border
                },
                D2D1_DRAW_TEXT_OPTIONS_NONE,
            );
            draw_text(
                target,
                ">",
                &state.formats.center_title,
                right,
                if state.category_scroll < maximum_category_scroll(width) {
                    secondary
                } else {
                    border
                },
                D2D1_DRAW_TEXT_OPTIONS_NONE,
            );
        }
    }
    unsafe {
        target.PushAxisAlignedClip(
            &rect(
                0.0,
                BROWSE_CONTENT_TOP as f32,
                width as f32,
                state.footer_top() as f32,
            ),
            D2D1_ANTIALIAS_MODE_ALIASED,
        );
    }
    let layouts = state.section_layouts();
    let viewport_top = state.browse_scroll;
    let viewport_bottom = state.browse_scroll + (state.footer_top() - BROWSE_CONTENT_TOP) as f32;
    for (section_index, (section, layout)) in
        state.browse_sections.iter().zip(layouts.iter()).enumerate()
    {
        let heading_y = BROWSE_CONTENT_TOP as f32 + layout.top as f32 - state.browse_scroll;
        if heading_y + SECTION_HEADING_HEIGHT as f32 >= BROWSE_CONTENT_TOP as f32
            && heading_y < state.footer_top() as f32
        {
            unsafe {
                draw_text(
                    target,
                    section.category.heading(),
                    &state.formats.label,
                    rect(
                        18.0,
                        heading_y,
                        width as f32 - 18.0,
                        heading_y + SECTION_HEADING_HEIGHT as f32,
                    ),
                    secondary,
                    D2D1_DRAW_TEXT_OPTIONS_NONE,
                );
            }
        }
        let grid_width = layout.columns as f32 * layout.cell_width;
        let grid_left = (width as f32 - grid_width) / 2.0;
        let visible = visible_item_range(
            *layout,
            section.indices.len(),
            viewport_top,
            viewport_bottom,
        );
        for item_index in visible {
            let entry_index = section.indices[item_index];
            let column = item_index % layout.columns;
            let row = item_index / layout.columns;
            let left = grid_left + column as f32 * layout.cell_width;
            let top = BROWSE_CONTENT_TOP as f32
                + layout.grid_top as f32
                + row as f32 * layout.cell_height as f32
                - state.browse_scroll;
            let selected_item = (section_index, item_index) == state.browse_focus;
            let hovered = matches!(
                state.hovered_target,
                Some(HitTarget::BrowseItem { section, item })
                    if section == section_index && item == item_index
            );
            let entry = &catalog::entries()[entry_index];
            unsafe {
                let tile = rounded_rect(
                    left + 3.0,
                    top + 3.0,
                    left + layout.cell_width - 3.0,
                    top + layout.cell_height as f32 - 3.0,
                    9.0,
                );
                target.FillRoundedRectangle(
                    &tile,
                    if selected_item || hovered {
                        selection
                    } else {
                        glyph_surface
                    },
                );
                if selected_item {
                    target.DrawRoundedRectangle(&tile, selection_border, 1.0, None);
                }
                let glyph_bounds = rect(
                    left + 5.0,
                    top + 3.0,
                    left + layout.cell_width - 5.0,
                    top + layout.cell_height as f32 - 3.0,
                );
                draw_glyph(state, target, entry, glyph_bounds, primary);
            }
        }
    }
    unsafe {
        target.PopAxisAlignedClip();
    }
    if let Some((track, thumb)) = browse_scrollbar_rects(state) {
        unsafe {
            target.FillRoundedRectangle(
                &rounded_rect(
                    width as f32 - 7.0,
                    track.top,
                    width as f32 - 4.0,
                    track.bottom,
                    1.5,
                ),
                surface,
            );
            target.FillRoundedRectangle(
                &rounded_rect(thumb.left, thumb.top, thumb.right, thumb.bottom, 2.5),
                if matches!(state.hovered_target, Some(HitTarget::BrowseScrollbar))
                    || state.dragging_scrollbar.is_some()
                {
                    selection_border
                } else {
                    border
                },
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
unsafe fn draw_search_results(
    state: &AppState,
    target: &ID2D1HwndRenderTarget,
    selection: &ID2D1SolidColorBrush,
    selection_border: &ID2D1SolidColorBrush,
    glyph_surface: &ID2D1SolidColorBrush,
    primary: &ID2D1SolidColorBrush,
    secondary: &ID2D1SolidColorBrush,
    accent: &ID2D1SolidColorBrush,
) {
    let (width, _) = state.dimensions();
    for (row, found) in state.matches.iter().enumerate() {
        let top = (SEARCH_RESULTS_TOP + row as i32 * RESULT_ROW_HEIGHT) as f32;
        if top + RESULT_ROW_HEIGHT as f32 > state.footer_top() as f32 {
            break;
        }
        let entry = &catalog::entries()[found.index];
        let hovered =
            matches!(state.hovered_target, Some(HitTarget::SearchResult(index)) if index == row);
        if row == state.selected || hovered {
            unsafe {
                let bounds = rounded_rect(
                    12.0,
                    top + 1.0,
                    width as f32 - 12.0,
                    top + RESULT_ROW_HEIGHT as f32 - 2.0,
                    8.0,
                );
                target.FillRoundedRectangle(&bounds, selection);
                if row == state.selected {
                    target.DrawRoundedRectangle(&bounds, selection_border, 1.0, None);
                    target.FillRoundedRectangle(
                        &rounded_rect(
                            12.0,
                            top + 10.0,
                            15.0,
                            top + RESULT_ROW_HEIGHT as f32 - 10.0,
                            1.5,
                        ),
                        accent,
                    );
                }
            }
        }
        unsafe {
            target.FillRoundedRectangle(
                &rounded_rect(
                    20.0,
                    top + 4.0,
                    54.0,
                    top + RESULT_ROW_HEIGHT as f32 - 4.0,
                    7.0,
                ),
                glyph_surface,
            );
            draw_glyph(
                state,
                target,
                entry,
                rect(22.0, top + 2.0, 52.0, top + RESULT_ROW_HEIGHT as f32 - 2.0),
                primary,
            );
            draw_text(
                target,
                &entry.name,
                &state.formats.title,
                rect(
                    64.0,
                    top,
                    width as f32 - 20.0,
                    top + RESULT_ROW_HEIGHT as f32,
                ),
                primary,
                D2D1_DRAW_TEXT_OPTIONS_NONE,
            );
        }
    }
    if state.matches.is_empty() {
        let query = state.query();
        let headline = if query.chars().count() <= 24 {
            format!("No match for \"{}\"", query.trim())
        } else {
            "No matching character".to_string()
        };
        let center_y = (SEARCH_RESULTS_TOP as f32 + state.footer_top() as f32) / 2.0;
        unsafe {
            draw_text(
                target,
                &headline,
                &state.formats.center_title,
                rect(24.0, center_y - 28.0, width as f32 - 24.0, center_y),
                primary,
                D2D1_DRAW_TEXT_OPTIONS_NONE,
            );
            draw_text(
                target,
                "Try fewer letters, or click the grid to browse",
                &state.formats.center,
                rect(24.0, center_y + 2.0, width as f32 - 24.0, center_y + 28.0),
                secondary,
                D2D1_DRAW_TEXT_OPTIONS_NONE,
            );
        }
    }
}

unsafe fn draw_settings_picker(state: &mut AppState) -> Result<()> {
    let (target, brushes) = unsafe { ensure_render_target(state)? };
    let (width, height) = state.dimensions();
    let footer_top = state.footer_top() as f32;
    let border = brushes.surface_border.clone();
    let selection = brushes.selection.clone();
    let selection_border = brushes.selection_border.clone();
    let accent = brushes.accent.clone();
    let primary = brushes.primary.clone();
    let secondary = brushes.secondary.clone();
    let danger = brushes.danger.clone();

    unsafe {
        target.BeginDraw();
        target.Clear(Some(&color(0x101217)));
        target.DrawRoundedRectangle(
            &rounded_rect(0.5, 0.5, width as f32 - 0.5, height as f32 - 0.5, 11.0),
            &border,
            1.0,
            None,
        );
        draw_text(
            &target,
            "Settings",
            &state.formats.label,
            rect(18.0, 7.0, 160.0, 34.0),
            &primary,
            D2D1_DRAW_TEXT_OPTIONS_NONE,
        );
        draw_header_button(&target, state, width, 0, "×", &selection, &secondary);
    }

    let settings = [
        ("Width", format!("{} px", state.config.dimensions.width)),
        ("Height", format!("{} px", state.config.dimensions.height)),
        ("Hover details", state.config.details.to_string()),
        (
            "Emoji font",
            match state.config.emoji_font {
                EmojiFont::SegoeEmoji => "Color emoji".to_string(),
                EmojiFont::SegoeSymbol => "Monochrome".to_string(),
            },
        ),
        (
            "Open shortcut",
            if state.capturing_shortcut {
                "Press shortcut…".to_string()
            } else {
                state.config.hotkey.to_string()
            },
        ),
    ];
    for (index, (label, value)) in settings.iter().enumerate() {
        let bounds = settings_row_rect(width, index);
        if index == state.settings_selected {
            unsafe {
                target.FillRoundedRectangle(
                    &rounded_rect(bounds.left, bounds.top, bounds.right, bounds.bottom, 8.0),
                    &selection,
                );
                target.DrawRoundedRectangle(
                    &rounded_rect(bounds.left, bounds.top, bounds.right, bounds.bottom, 8.0),
                    &selection_border,
                    1.0,
                    None,
                );
            }
        }
        unsafe {
            draw_text(
                &target,
                label,
                &state.formats.label,
                rect(24.0, bounds.top, width as f32 * 0.44, bounds.bottom),
                &primary,
                D2D1_DRAW_TEXT_OPTIONS_NONE,
            );
            if index < 2 {
                draw_slider(
                    &target,
                    slider_rect(width, index),
                    if index == 0 {
                        state.config.dimensions.width
                    } else {
                        state.config.dimensions.height
                    },
                    if index == 0 {
                        MIN_PICKER_WIDTH
                    } else {
                        MIN_PICKER_HEIGHT
                    },
                    if index == 0 {
                        MAX_PICKER_WIDTH
                    } else {
                        MAX_PICKER_HEIGHT
                    },
                    value,
                    &selection_border,
                    &accent,
                    &primary,
                    &state.formats.metadata,
                );
            } else {
                draw_text(
                    &target,
                    &format!("‹  {value}  ›"),
                    &state.formats.brand,
                    rect(
                        width as f32 * 0.42,
                        bounds.top,
                        width as f32 - 24.0,
                        bounds.bottom,
                    ),
                    &secondary,
                    D2D1_DRAW_TEXT_OPTIONS_NONE,
                );
            }
        }
    }

    let hint_top = settings_row_rect(width, 4).bottom + 8.0;
    unsafe {
        draw_text(
            &target,
            "Arrow keys adjust. Enter changes the focused value.",
            &state.formats.center,
            rect(24.0, hint_top, width as f32 - 24.0, hint_top + 22.0),
            &secondary,
            D2D1_DRAW_TEXT_OPTIONS_NONE,
        );
        target.DrawLine(
            Vector2 {
                X: 16.0,
                Y: footer_top,
            },
            Vector2 {
                X: width as f32 - 16.0,
                Y: footer_top,
            },
            &border,
            1.0,
            None,
        );
        let (discard, reset, back) = settings_footer_rects(width, state.footer_top());
        if let Some(status) = &state.status {
            draw_text(
                &target,
                status,
                &state.formats.metadata,
                rect(140.0, footer_top, width as f32 - 74.0, height as f32 - 2.0),
                &danger,
                D2D1_DRAW_TEXT_OPTIONS_CLIP,
            );
        } else {
            draw_text(
                &target,
                "Esc goes back",
                &state.formats.center,
                rect(140.0, footer_top, width as f32 - 74.0, height as f32 - 2.0),
                &secondary,
                D2D1_DRAW_TEXT_OPTIONS_NONE,
            );
        }
        draw_button(
            &target,
            discard,
            "Discard",
            matches!(state.hovered_target, Some(HitTarget::SettingsDiscard)),
            &selection,
            &selection_border,
            &primary,
            &state.formats.center,
        );
        draw_button(
            &target,
            reset,
            "Reset",
            matches!(state.hovered_target, Some(HitTarget::SettingsReset)),
            &selection,
            &selection_border,
            &primary,
            &state.formats.center,
        );
        draw_button(
            &target,
            back,
            "Back",
            matches!(state.hovered_target, Some(HitTarget::SettingsBack)),
            &selection,
            &selection_border,
            &primary,
            &state.formats.center,
        );
        target.EndDraw(None, None)?;
    }
    Ok(())
}

unsafe fn draw_header_button(
    target: &ID2D1HwndRenderTarget,
    state: &AppState,
    width: i32,
    position: usize,
    label: &str,
    surface: &ID2D1SolidColorBrush,
    text: &ID2D1SolidColorBrush,
) {
    let bounds = header_button_rect(width, position);
    unsafe {
        if state.hovered_target.is_some_and(|hovered| {
            matches!(
                (position, hovered),
                (0, HitTarget::Close) | (1, HitTarget::Settings) | (2, HitTarget::Browse)
            )
        }) {
            target.FillRoundedRectangle(
                &rounded_rect(bounds.left, bounds.top, bounds.right, bounds.bottom, 6.0),
                surface,
            );
        }
        draw_text(
            target,
            label,
            &state.formats.icon,
            bounds,
            text,
            D2D1_DRAW_TEXT_OPTIONS_NONE,
        );
    }
}

#[allow(clippy::too_many_arguments)]
unsafe fn draw_button(
    target: &ID2D1HwndRenderTarget,
    bounds: D2D_RECT_F,
    label: &str,
    hovered: bool,
    surface: &ID2D1SolidColorBrush,
    border: &ID2D1SolidColorBrush,
    text: &ID2D1SolidColorBrush,
    format: &IDWriteTextFormat,
) {
    unsafe {
        target.FillRoundedRectangle(
            &rounded_rect(bounds.left, bounds.top, bounds.right, bounds.bottom, 6.0),
            surface,
        );
        target.DrawRoundedRectangle(
            &rounded_rect(bounds.left, bounds.top, bounds.right, bounds.bottom, 6.0),
            border,
            if hovered { 1.5 } else { 1.0 },
            None,
        );
        draw_text(
            target,
            label,
            format,
            bounds,
            text,
            D2D1_DRAW_TEXT_OPTIONS_NONE,
        );
    }
}

unsafe fn draw_hover_help(
    state: &AppState,
    target: &ID2D1HwndRenderTarget,
    surface: &ID2D1SolidColorBrush,
    border: &ID2D1SolidColorBrush,
    text: &ID2D1SolidColorBrush,
) {
    let (width, _) = state.dimensions();
    let Some((help, anchor, top)) = (match state.hovered_target {
        Some(HitTarget::Browse) => Some((
            "Home and categories · Ctrl+G",
            header_button_rect(width, 2),
            31.0,
        )),
        Some(HitTarget::Settings) => {
            Some(("Settings · Ctrl+,", header_button_rect(width, 1), 31.0))
        }
        Some(HitTarget::Close) => Some((
            if state.view == View::Settings {
                "Save and go back · Esc"
            } else {
                "Close · Esc"
            },
            header_button_rect(width, 0),
            31.0,
        )),
        Some(HitTarget::SearchClear) => Some((
            "Clear search",
            search_clear_rect(width),
            (SEARCH_TOP + SEARCH_HEIGHT + 3) as f32,
        )),
        Some(HitTarget::Category(index)) => Some((
            BrowseCategory::ALL[index.min(BrowseCategory::ALL.len() - 1)].label(),
            category_rect(width, state.category_scroll, index),
            (CATEGORY_TOP + CATEGORY_HEIGHT + 2) as f32,
        )),
        Some(HitTarget::CategoryScrollLeft) => category_edge_rects(width)
            .map(|(left, _)| ("Earlier categories · scroll", left, left.bottom + 2.0)),
        Some(HitTarget::CategoryScrollRight) => category_edge_rects(width)
            .map(|(_, right)| ("More categories · scroll", right, right.bottom + 2.0)),
        Some(HitTarget::BrowseScrollbar) => {
            browse_scrollbar_rects(state).map(|(_, thumb)| ("Drag to scroll", thumb, thumb.top))
        }
        Some(HitTarget::Insert) => Some((
            "Insert and keep the picker open · Enter",
            footer_button_rects(width, state.footer_top()).0,
            state.footer_top() as f32 - 31.0,
        )),
        Some(HitTarget::InsertClose) => Some((
            "Insert and close · Ctrl+Enter",
            footer_button_rects(width, state.footer_top()).1,
            state.footer_top() as f32 - 31.0,
        )),
        _ => None,
    }) else {
        return;
    };
    let tooltip_width =
        (help.chars().count() as f32 * 6.2 + 20.0).clamp(74.0, (width as f32 - 16.0).max(74.0));
    let anchor_center = (anchor.left + anchor.right) / 2.0;
    let left = (anchor_center - tooltip_width / 2.0).clamp(8.0, width as f32 - tooltip_width - 8.0);
    let bounds = rect(left, top, left + tooltip_width, top + 25.0);
    unsafe {
        target.FillRoundedRectangle(
            &rounded_rect(bounds.left, bounds.top, bounds.right, bounds.bottom, 6.0),
            surface,
        );
        target.DrawRoundedRectangle(
            &rounded_rect(bounds.left, bounds.top, bounds.right, bounds.bottom, 6.0),
            border,
            1.0,
            None,
        );
        draw_text(
            target,
            help,
            &state.formats.center,
            bounds,
            text,
            D2D1_DRAW_TEXT_OPTIONS_NONE,
        );
    }
}

#[allow(clippy::too_many_arguments)]
unsafe fn draw_slider(
    target: &ID2D1HwndRenderTarget,
    bounds: D2D_RECT_F,
    value: i32,
    minimum: i32,
    maximum: i32,
    label: &str,
    track: &ID2D1SolidColorBrush,
    accent: &ID2D1SolidColorBrush,
    text: &ID2D1SolidColorBrush,
    format: &IDWriteTextFormat,
) {
    let line_left = bounds.left;
    let line_right = bounds.right - 56.0;
    let center = (bounds.top + bounds.bottom) / 2.0;
    let ratio = (value - minimum) as f32 / (maximum - minimum) as f32;
    let thumb = line_left + (line_right - line_left) * ratio;
    unsafe {
        target.DrawLine(
            Vector2 {
                X: line_left,
                Y: center,
            },
            Vector2 {
                X: line_right,
                Y: center,
            },
            track,
            3.0,
            None,
        );
        target.DrawLine(
            Vector2 {
                X: line_left,
                Y: center,
            },
            Vector2 {
                X: thumb,
                Y: center,
            },
            accent,
            3.0,
            None,
        );
        target.FillEllipse(
            &D2D1_ELLIPSE {
                point: Vector2 {
                    X: thumb,
                    Y: center,
                },
                radiusX: 5.0,
                radiusY: 5.0,
            },
            accent,
        );
        draw_text(
            target,
            label,
            format,
            rect(line_right + 6.0, bounds.top, bounds.right, bounds.bottom),
            text,
            D2D1_DRAW_TEXT_OPTIONS_NONE,
        );
    }
}

unsafe fn draw_glyph(
    state: &AppState,
    target: &ID2D1HwndRenderTarget,
    entry: &catalog::Entry,
    bounds: D2D_RECT_F,
    brush: &ID2D1SolidColorBrush,
) {
    if entry.kind == "Emoticon" {
        // Wide emoticons drop to a smaller face and clip to their tile
        // instead of spilling into the neighbours.
        let width = bounds.right - bounds.left;
        let characters = entry.glyph.chars().count() as f32;
        let format = if characters * 8.0 <= width {
            &state.formats.emoticon
        } else {
            &state.formats.emoticon_small
        };
        unsafe {
            draw_text(
                target,
                &entry.glyph,
                format,
                bounds,
                brush,
                D2D1_DRAW_TEXT_OPTIONS_CLIP,
            );
        }
        return;
    }
    let mathematical_alphanumeric = entry
        .glyph
        .chars()
        .next()
        .is_some_and(|character| (0x1d400..=0x1d7ff).contains(&(character as u32)));
    let format = if entry.kind == "Emoji" {
        &state.formats.glyph
    } else if entry.kind == "Math" || mathematical_alphanumeric {
        &state.formats.math
    } else {
        &state.formats.symbol
    };
    let options = if entry.kind == "Emoji" && state.config.emoji_font == EmojiFont::SegoeEmoji {
        D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT
    } else {
        D2D1_DRAW_TEXT_OPTIONS_NONE
    };
    unsafe {
        draw_text(target, &entry.glyph, format, bounds, brush, options);
    }
}

unsafe fn draw_entry_information(
    state: &AppState,
    target: &ID2D1HwndRenderTarget,
    bounds: D2D_RECT_F,
    brush: &ID2D1SolidColorBrush,
) {
    let Some(index) = state.hover_or_selected_entry() else {
        unsafe {
            draw_text(
                target,
                "Type to search or scroll to browse",
                &state.formats.center,
                bounds,
                brush,
                D2D1_DRAW_TEXT_OPTIONS_NONE,
            );
        }
        return;
    };
    let entry = &catalog::entries()[index];
    let detail = match state.config.details {
        DetailMode::None => entry.name.clone(),
        DetailMode::Type => format!("{}  {}", entry.name, entry.kind),
        DetailMode::Codepoint => format!("{}  {}", entry.name, codepoints(&entry.glyph)),
        DetailMode::Both => format!(
            "{}  {}  {}",
            entry.name,
            codepoints(&entry.glyph),
            entry.kind
        ),
    };
    unsafe {
        draw_text(
            target,
            &detail,
            &state.formats.metadata,
            bounds,
            brush,
            D2D1_DRAW_TEXT_OPTIONS_CLIP,
        );
    }
}

unsafe fn solid_brush(target: &ID2D1HwndRenderTarget, value: u32) -> Result<ID2D1SolidColorBrush> {
    unsafe { target.CreateSolidColorBrush(&color(value), None) }
}

fn color(value: u32) -> D2D1_COLOR_F {
    D2D1_COLOR_F {
        r: ((value >> 16) & 0xff) as f32 / 255.0,
        g: ((value >> 8) & 0xff) as f32 / 255.0,
        b: (value & 0xff) as f32 / 255.0,
        a: 1.0,
    }
}

fn rect(left: f32, top: f32, right: f32, bottom: f32) -> D2D_RECT_F {
    D2D_RECT_F {
        left,
        top,
        right,
        bottom,
    }
}

fn rounded_rect(left: f32, top: f32, right: f32, bottom: f32, radius: f32) -> D2D1_ROUNDED_RECT {
    D2D1_ROUNDED_RECT {
        rect: rect(left, top, right, bottom),
        radiusX: radius,
        radiusY: radius,
    }
}

unsafe fn draw_text(
    target: &ID2D1HwndRenderTarget,
    text: &str,
    format: &IDWriteTextFormat,
    bounds: D2D_RECT_F,
    brush: &ID2D1SolidColorBrush,
    options: windows::Win32::Graphics::Direct2D::D2D1_DRAW_TEXT_OPTIONS,
) {
    let wide: Vec<_> = text.encode_utf16().collect();
    unsafe {
        target.DrawText(
            &wide,
            format,
            &bounds,
            brush,
            options,
            DWRITE_MEASURING_MODE_NATURAL,
        );
    }
}

fn codepoints(value: &str) -> String {
    value
        .chars()
        .filter(|character| *character != '\u{fe0f}' && *character != '\u{200d}')
        .map(|character| format!("U+{:04X}", character as u32))
        .collect::<Vec<_>>()
        .join(" ")
}

fn scale(value: i32, dpi: u32) -> i32 {
    value * dpi as i32 / 96
}

fn smooth_scroll_step(current: f32, target: f32) -> f32 {
    current + (target - current) * 0.24
}

fn to_wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

unsafe fn focused_child_for(target: HWND) -> HWND {
    if target.is_invalid() || !unsafe { IsWindow(Some(target)).as_bool() } {
        return HWND::default();
    }
    let thread = unsafe { GetWindowThreadProcessId(target, None) };
    let mut information = GUITHREADINFO {
        cbSize: size_of::<GUITHREADINFO>() as u32,
        ..Default::default()
    };
    if thread != 0
        && unsafe { GetGUIThreadInfo(thread, &mut information).is_ok() }
        && valid_target_focus(target, information.hwndFocus)
    {
        information.hwndFocus
    } else {
        HWND::default()
    }
}

fn valid_target_focus(target: HWND, focus: HWND) -> bool {
    !focus.is_invalid()
        && unsafe { IsWindow(Some(focus)).as_bool() }
        && (focus == target || unsafe { IsChild(target, focus).as_bool() })
}

unsafe fn inject_unicode(target: HWND, target_focus: HWND, value: &str) -> Result<()> {
    if target.is_invalid() || !unsafe { IsWindow(Some(target)).as_bool() } {
        return Err(Error::new(
            HRESULT(0x80070006u32 as i32),
            "the captured target window is no longer available",
        ));
    }

    for _ in 0..50 {
        if commit_keys_released() {
            break;
        }
        unsafe {
            Sleep(10);
        }
    }
    if !commit_keys_released() {
        return Err(Error::new(
            HRESULT(0x800705AAu32 as i32),
            "modifier keys remained held; input was cancelled",
        ));
    }

    let current_thread = unsafe { GetCurrentThreadId() };
    let target_thread = unsafe { GetWindowThreadProcessId(target, None) };
    let attached = target_thread != 0
        && target_thread != current_thread
        && unsafe { AttachThreadInput(current_thread, target_thread, true).as_bool() };
    let activated =
        unsafe { GetForegroundWindow() == target || SetForegroundWindow(target).as_bool() };
    if valid_target_focus(target, target_focus) {
        unsafe {
            let _ = SetFocus(Some(target_focus));
        }
    }
    for _ in 0..20 {
        if unsafe { GetForegroundWindow() } == target {
            break;
        }
        unsafe {
            Sleep(10);
        }
    }
    if attached {
        unsafe {
            let _ = AttachThreadInput(current_thread, target_thread, false);
        }
    }
    if !activated && unsafe { GetForegroundWindow() } != target {
        return Err(Error::new(
            HRESULT(0x80070005u32 as i32),
            "Windows did not allow focus restoration; input was cancelled",
        ));
    }
    if unsafe { GetForegroundWindow() } != target {
        return Err(Error::new(
            HRESULT(0x80070005u32 as i32),
            "captured window did not regain focus; input was cancelled",
        ));
    }
    if valid_target_focus(target, target_focus)
        && unsafe { focused_child_for(target) } != target_focus
    {
        return Err(Error::new(
            HRESULT(0x80070005u32 as i32),
            "captured text control did not regain focus; input was cancelled",
        ));
    }

    let mut inputs = Vec::with_capacity(value.encode_utf16().count() * 2);
    for unit in value.encode_utf16() {
        inputs.push(unicode_input(unit, false));
        inputs.push(unicode_input(unit, true));
    }
    let sent = unsafe { SendInput(&inputs, size_of::<INPUT>() as i32) };
    if sent != inputs.len() as u32 {
        return Err(Error::from_win32());
    }
    Ok(())
}

fn unicode_input(unit: u16, key_up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(0),
                wScan: unit,
                dwFlags: if key_up {
                    KEYEVENTF_UNICODE | KEYEVENTF_KEYUP
                } else {
                    KEYEVENTF_UNICODE
                },
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn commit_keys_released() -> bool {
    [VK_CONTROL, VK_MENU, VK_SHIFT, VK_LWIN, VK_RWIN, VK_RETURN]
        .iter()
        .all(|key| unsafe { GetAsyncKeyState(key.0 as i32) } >= 0)
}

fn manage_startup(uninstall: bool, dry_run: bool) -> Result<()> {
    let source_executable = std::env::current_exe().map_err(io_error)?;
    let executable = installed_executable()?;
    let command = format!("\"{}\" --startup", executable.display());
    let action = if uninstall { "remove" } else { "set" };
    if dry_run {
        let mut message = format!(
            "Would {action} HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run\\WinMoji"
        );
        if !uninstall {
            message.push_str(&format!("\nValue: {command}"));
        }
        report_success(&message);
        return Ok(());
    }

    if !uninstall {
        stop_resident()?;
        let directory = executable.parent().ok_or_else(|| {
            Error::new(
                HRESULT(0x80004005u32 as i32),
                "the installation directory is unavailable",
            )
        })?;
        fs::create_dir_all(directory).map_err(io_error)?;
        let source = fs::canonicalize(&source_executable).unwrap_or(source_executable);
        let destination = fs::canonicalize(&executable).unwrap_or_else(|_| executable.clone());
        if source != destination {
            let staged = directory.join("winmoji.update.exe");
            let _ = fs::remove_file(&staged);
            fs::copy(source, &staged).map_err(io_error)?;
            let staged_wide: Vec<_> = staged.as_os_str().encode_wide().chain([0]).collect();
            let destination_wide: Vec<_> =
                executable.as_os_str().encode_wide().chain([0]).collect();
            if let Err(error) = unsafe {
                MoveFileExW(
                    PCWSTR(staged_wide.as_ptr()),
                    PCWSTR(destination_wide.as_ptr()),
                    MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
                )
            } {
                let _ = fs::remove_file(&staged);
                return Err(error);
            }
        }
    }

    unsafe {
        let mut key = HKEY::default();
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            RUN_KEY,
            None,
            None,
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            None,
            &mut key,
            None,
        )
        .ok()?;
        let result = if uninstall {
            let status = RegDeleteValueW(key, RUN_VALUE);
            if status == windows::Win32::Foundation::ERROR_SUCCESS
                || status == windows::Win32::Foundation::ERROR_FILE_NOT_FOUND
            {
                Ok(())
            } else {
                Err(Error::from_hresult(status.to_hresult()))
            }
        } else {
            let wide = to_wide(&command);
            let bytes = std::slice::from_raw_parts(
                wide.as_ptr().cast::<u8>(),
                wide.len() * size_of::<u16>(),
            );
            RegSetValueExW(key, RUN_VALUE, None, REG_SZ, Some(bytes)).ok()
        };
        let close_result = RegCloseKey(key);
        result?;
        close_result.ok()?;
    }
    if !uninstall {
        Command::new(&executable)
            .arg("--startup")
            .spawn()
            .map_err(io_error)?;
        let mut ready = false;
        for _ in 0..100 {
            if unsafe { FindWindowW(CLASS_NAME, PCWSTR::null()).is_ok() } {
                ready = true;
                break;
            }
            unsafe {
                Sleep(25);
            }
        }
        if !ready {
            return Err(Error::new(
                HRESULT(0x80004005u32 as i32),
                "the installed hotkey listener did not start",
            ));
        }
    }
    report_success(&format!(
        "WinMoji startup entry {}{}.",
        if uninstall { "removed" } else { "installed" },
        if uninstall {
            ""
        } else {
            " and listener started"
        }
    ));
    Ok(())
}

fn stop_resident() -> Result<()> {
    let Ok(window) = (unsafe { FindWindowW(CLASS_NAME, PCWSTR::null()) }) else {
        return Ok(());
    };
    let mut process_id = 0;
    unsafe {
        GetWindowThreadProcessId(window, Some(&mut process_id));
    }
    if process_id == 0 {
        return Err(Error::new(
            HRESULT(0x80004005u32 as i32),
            "the existing hotkey listener process could not be identified",
        ));
    }
    let process = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, false, process_id)? };
    if let Err(error) = unsafe { PostMessageW(Some(window), WM_CLOSE, WPARAM(0), LPARAM(0)) } {
        unsafe {
            CloseHandle(process)?;
        }
        return Err(error);
    }
    let wait = unsafe { WaitForSingleObject(process, 2_500) };
    unsafe {
        CloseHandle(process)?;
    }
    if wait == WAIT_OBJECT_0 {
        Ok(())
    } else {
        Err(Error::new(
            HRESULT(0x80004005u32 as i32),
            "the existing hotkey listener did not stop",
        ))
    }
}

fn installed_executable() -> Result<PathBuf> {
    let local_app_data = std::env::var_os("LOCALAPPDATA").ok_or_else(|| {
        Error::new(
            HRESULT(0x80004005u32 as i32),
            "LOCALAPPDATA is not available",
        )
    })?;
    Ok(PathBuf::from(local_app_data)
        .join("Programs")
        .join("WinMoji")
        .join("winmoji.exe"))
}

fn self_test() -> Result<()> {
    let checks = [
        ("pretzel", "🥨"),
        ("otter", "🦦"),
        ("snowman", "☃"),
        ("perpendicular", "⟂"),
        ("euro", "€"),
        ("'smi", "😊"),
    ];
    for (query, expected) in checks {
        let first = catalog::search(query, 1)
            .first()
            .map(|item| catalog::entries()[item.index].glyph.as_str());
        if first.map(|glyph| glyph.trim_end_matches('\u{fe0f}')) != Some(expected) {
            return Err(Error::new(
                HRESULT(0x80004005u32 as i32),
                format!("search self-test failed for {query}: found {first:?}"),
            ));
        }
    }
    println!(
        "search: PASS ({} offline entries, representative emoji and symbols)",
        catalog::entries().len()
    );

    let previous = unsafe { GetForegroundWindow() };
    let test_window = SelfTestWindow::create()?;
    let edit = test_window.hwnd;
    unsafe {
        println!("hotkey: PASS (Ctrl+Alt+Shift+F24, MOD_NOREPEAT)");

        if !previous.is_invalid() && IsWindow(Some(previous)).as_bool() {
            let _ = SetForegroundWindow(previous);
            for _ in 0..20 {
                if GetForegroundWindow() == previous {
                    break;
                }
                Sleep(10);
            }
        }
        if !previous.is_invalid() && GetForegroundWindow() != previous {
            return Err(Error::new(
                HRESULT(0x80070005u32 as i32),
                "self-test could not restore the original foreground window",
            ));
        }
        let clipboard_before = GetClipboardSequenceNumber();
        inject_unicode(edit, edit, "λ→🙂")?;
        let deadline = Instant::now() + Duration::from_millis(500);
        let mut message = MSG::default();
        while Instant::now() < deadline {
            while windows::Win32::UI::WindowsAndMessaging::PeekMessageW(
                &mut message,
                None,
                0,
                0,
                windows::Win32::UI::WindowsAndMessaging::PM_REMOVE,
            )
            .as_bool()
            {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
            if GetWindowTextLengthW(edit) >= 4 {
                break;
            }
            Sleep(5);
        }
        let length = GetWindowTextLengthW(edit);
        let mut buffer = vec![0u16; length as usize + 1];
        let copied = GetWindowTextW(edit, &mut buffer);
        let inserted = String::from_utf16_lossy(&buffer[..copied as usize]);
        if !previous.is_invalid() && IsWindow(Some(previous)).as_bool() {
            let _ = SetForegroundWindow(previous);
        }
        if inserted != "λ→🙂" {
            return Err(Error::new(
                HRESULT(0x80004005u32 as i32),
                format!("Unicode input self-test failed: received {inserted:?}"),
            ));
        }
        let clipboard_after = GetClipboardSequenceNumber();
        if clipboard_after != clipboard_before {
            return Err(Error::new(
                HRESULT(0x80004005u32 as i32),
                "Unicode input self-test changed the clipboard sequence number",
            ));
        }
        println!("unicode input: PASS (SendInput UTF-16 down/up pairs, clipboard unchanged)");
    }
    println!("self-test: PASS");
    #[cfg(not(feature = "console"))]
    report_success(
        "Self-test passed. Search, hotkey registration, Unicode input, and clipboard preservation are verified.",
    );
    Ok(())
}

struct SelfTestWindow {
    hwnd: HWND,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl SelfTestWindow {
    fn create() -> Result<Self> {
        let (sender, receiver) = std::sync::mpsc::sync_channel(1);
        let thread = std::thread::spawn(move || unsafe {
            let instance = match windows::Win32::System::LibraryLoader::GetModuleHandleW(None) {
                Ok(module) => HINSTANCE(module.0),
                Err(error) => {
                    let _ = sender.send(Err(error.to_string()));
                    return;
                }
            };
            let edit = match CreateWindowExW(
                WS_EX_TOOLWINDOW,
                w!("EDIT"),
                PCWSTR::null(),
                WS_OVERLAPPEDWINDOW | WS_VISIBLE | WINDOW_STYLE(ES_AUTOHSCROLL as u32),
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                360,
                100,
                None,
                None,
                Some(instance),
                None,
            ) {
                Ok(edit) => edit,
                Err(error) => {
                    let _ = sender.send(Err(error.to_string()));
                    return;
                }
            };
            let test_hotkey = HOT_KEY_MODIFIERS(
                MOD_CONTROL_VALUE | MOD_ALT_VALUE | MOD_SHIFT_VALUE | MOD_NOREPEAT_VALUE,
            );
            if let Err(error) = RegisterHotKey(Some(edit), HOTKEY_ID + 1, test_hotkey, 0x87) {
                let _ = sender.send(Err(format!(
                    "hotkey registration self-test failed: {error}"
                )));
                let _ = DestroyWindow(edit);
                return;
            }
            let _ = UnregisterHotKey(Some(edit), HOTKEY_ID + 1);
            let foreground = GetForegroundWindow();
            let current_thread = GetCurrentThreadId();
            let foreground_thread = if foreground.is_invalid() {
                0
            } else {
                GetWindowThreadProcessId(foreground, None)
            };
            let attached = foreground_thread != 0
                && foreground_thread != current_thread
                && AttachThreadInput(current_thread, foreground_thread, true).as_bool();
            let _ = ShowWindow(edit, SW_SHOW);
            let _ = SetForegroundWindow(edit);
            let _ = SetFocus(Some(edit));
            if attached {
                let _ = AttachThreadInput(current_thread, foreground_thread, false);
            }
            if GetForegroundWindow() != edit {
                let _ = sender.send(Err(
                    "temporary edit control could not obtain foreground focus".to_string(),
                ));
                let _ = DestroyWindow(edit);
                return;
            }
            if sender.send(Ok(edit.0 as usize)).is_err() {
                let _ = DestroyWindow(edit);
                return;
            }
            let mut message = MSG::default();
            while IsWindow(Some(edit)).as_bool() {
                while windows::Win32::UI::WindowsAndMessaging::PeekMessageW(
                    &mut message,
                    None,
                    0,
                    0,
                    windows::Win32::UI::WindowsAndMessaging::PM_REMOVE,
                )
                .as_bool()
                {
                    let _ = TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
                Sleep(2);
            }
        });
        let raw = receiver
            .recv_timeout(Duration::from_secs(2))
            .map_err(|error| {
                Error::new(
                    HRESULT(0x80004005u32 as i32),
                    format!("self-test window thread did not start: {error}"),
                )
            })?
            .map_err(|message| Error::new(HRESULT(0x80004005u32 as i32), message))?;
        Ok(Self {
            hwnd: HWND(raw as *mut c_void),
            thread: Some(thread),
        })
    }
}

impl Drop for SelfTestWindow {
    fn drop(&mut self) {
        unsafe {
            let _ = PostMessageW(Some(self.hwnd), WM_CLOSE, WPARAM(0), LPARAM(0));
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn benchmark() -> String {
    let queries = [
        "smile",
        "smiel",
        "pretzel",
        "right arrow",
        "right arorw",
        "integral",
        "euro",
        "lambda",
        "snowman",
        "perpendicular",
        "perpendiculr",
        "uniond",
        "copyright",
        "warning",
    ];
    let init_start = Instant::now();
    let catalog_size = catalog::entries().len();
    let catalog_init_ms = init_start.elapsed().as_secs_f64() * 1_000.0;
    let mut samples = Vec::with_capacity(queries.len() * 200);
    for _ in 0..200 {
        for query in queries {
            let start = Instant::now();
            std::hint::black_box(catalog::search(query, 7));
            samples.push(start.elapsed().as_nanos());
        }
    }
    samples.sort_unstable();
    let p50 = samples[samples.len() / 2] as f64 / 1_000_000.0;
    let p95 = samples[samples.len() * 95 / 100] as f64 / 1_000_000.0;
    let maximum = samples[samples.len() - 1] as f64 / 1_000_000.0;
    format!(
        "catalog={catalog_size} catalog_init_ms={catalog_init_ms:.3} queries={} p50={p50:.3}ms p95={p95:.3}ms max={maximum:.3}ms",
        samples.len()
    )
}

fn io_error(error: std::io::Error) -> Error {
    Error::new(HRESULT(0x80004005u32 as i32), error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_cli_modes() {
        assert_eq!(
            parse_mode([].into_iter()).unwrap(),
            Mode::Run { startup: false }
        );
        assert_eq!(
            parse_mode(["--startup".to_string()].into_iter()).unwrap(),
            Mode::Run { startup: true }
        );
        assert_eq!(
            parse_mode(["--install".to_string(), "--dry-run".to_string()].into_iter()).unwrap(),
            Mode::Install {
                uninstall: false,
                dry_run: true
            }
        );
        assert!(
            parse_mode(["--self-test".to_string(), "--dry-run".to_string()].into_iter()).is_err()
        );
    }

    #[test]
    fn formats_multi_codepoint_values() {
        assert_eq!(codepoints("❤️"), "U+2764");
        assert_eq!(codepoints("λ"), "U+03BB");
    }

    #[test]
    fn search_field_edits_words_and_characters() {
        let mut field = SearchField::default();
        field.insert("smiling face  ");
        field.backspace(true);
        assert_eq!(field.text, "smiling ");
        field.insert("🙂x");
        field.backspace(false);
        field.backspace(false);
        assert_eq!(field.text, "smiling ");
        let mut field = SearchField::default();
        field.insert("smiling");
        field.backspace(true);
        assert_eq!(field.text, "");
        field.backspace(true);
        assert_eq!(field.text, "");
    }

    #[test]
    fn search_field_selection_and_caret_moves() {
        let mut field = SearchField::default();
        field.insert("arrow");
        assert_eq!(field.caret, 5);
        field.move_caret(-1, false);
        field.move_caret(-1, true);
        assert_eq!(field.selection(), (3, 4));
        field.insert("0");
        assert_eq!(field.text, "arr0w");
        field.select_all();
        assert_eq!(field.selection(), (0, 5));
        field.backspace(false);
        assert_eq!(field.text, "");
        field.move_home(false);
        field.move_end(false);
        assert_eq!(field.caret, 0);
    }

    #[test]
    fn search_field_caret_collapses_selection_toward_direction() {
        let mut field = SearchField::default();
        field.insert("abc");
        field.move_home(false);
        field.move_caret(1, true);
        field.move_caret(1, true);
        assert_eq!(field.selection(), (0, 2));
        field.move_caret(-1, false);
        assert_eq!(field.caret, 0);
        assert!(!field.has_selection());
    }

    #[test]
    fn smooth_scroll_moves_by_sub_row_increments_and_converges() {
        let first = smooth_scroll_step(0.0, 76.0);
        assert!(first > 0.0 && first < GRID_CELL as f32);
        let mut position = first;
        for _ in 0..40 {
            position = smooth_scroll_step(position, 76.0);
        }
        assert!((76.0 - position).abs() < 0.01);
    }

    #[test]
    fn browser_categories_follow_cldr_groups_and_merge_symbols() {
        let entries = catalog::entries();
        let smile = entries
            .iter()
            .find(|entry| entry.glyph == "😀")
            .expect("grinning face exists");
        let summation = entries
            .iter()
            .find(|entry| entry.glyph == "∑")
            .expect("summation symbol exists");
        assert!(BrowseCategory::Smileys.contains(smile));
        assert!(BrowseCategory::Symbols.contains(summation));
        assert!(!BrowseCategory::Symbols.contains(smile));
    }

    #[test]
    fn emoticons_have_their_own_searchable_category() {
        let entries = catalog::entries();
        let table_flip = entries
            .iter()
            .find(|entry| entry.glyph == "(╯°□°)╯︵ ┻━┻")
            .expect("table flip emoticon exists");
        assert!(BrowseCategory::Emoticons.contains(table_flip));
        assert!(!BrowseCategory::Symbols.contains(table_flip));
        assert!(
            catalog::search("table flip", 7)
                .iter()
                .any(|found| { entries[found.index].glyph == "(╯°□°)╯︵ ┻━┻" })
        );
    }

    #[test]
    fn viewport_range_virtualizes_large_sections() {
        let layout = SectionLayout {
            top: 0,
            grid_top: 26,
            bottom: 10_000,
            columns: 8,
            cell_width: 48.0,
            cell_height: 48,
        };
        let visible = visible_item_range(layout, 1_500, 1_000.0, 1_260.0);
        assert!(visible.len() <= 56);
        assert!(visible.start > 0);
        assert!(visible.end < 1_500);
    }

    #[test]
    fn category_strip_overflows_and_scrolls_at_compact_widths() {
        let maximum = maximum_category_scroll(440);
        assert!(maximum > 0.0);
        let first = category_rect(440, maximum, 0);
        let last = category_rect(440, maximum, BrowseCategory::ALL.len() - 1);
        let viewport = category_viewport(440);
        assert!(first.right <= viewport.left);
        assert!(last.right <= viewport.right + f32::EPSILON);
    }

    #[test]
    fn captured_modifier_state_tracks_sided_keys_and_commit_release() {
        let mut keyboard = [0u8; 256];
        update_captured_keyboard_state(&mut keyboard, VK_LCONTROL, false);
        update_captured_keyboard_state(&mut keyboard, VK_RETURN, false);
        assert!(captured_key_down(&keyboard, VK_CONTROL));
        assert_eq!(
            captured_hotkey_modifiers(&keyboard),
            MOD_NOREPEAT_VALUE | MOD_CONTROL_VALUE
        );
        assert!(!captured_commit_keys_released(&keyboard));
        update_captured_keyboard_state(&mut keyboard, VK_RETURN, true);
        assert!(!captured_commit_keys_released(&keyboard));
        update_captured_keyboard_state(&mut keyboard, VK_LCONTROL, true);
        assert!(captured_commit_keys_released(&keyboard));
    }
}
