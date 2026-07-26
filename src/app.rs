use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_void;
use std::fs;
use std::mem::size_of;
use std::os::windows::ffi::OsStrExt;
use std::path::PathBuf;
use std::process::Command;
use std::rc::Rc;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{
    COLORREF, CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, GlobalFree, HANDLE, HGLOBAL,
    HINSTANCE, HMODULE, HWND, LPARAM, LRESULT, POINT, RECT, WAIT_OBJECT_0, WPARAM,
};
use windows::Win32::Graphics::Direct2D::Common::{
    D2D_RECT_F, D2D_SIZE_F, D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F, D2D1_PIXEL_FORMAT,
};
use windows::Win32::Graphics::Direct2D::{
    D2D1_ANTIALIAS_MODE_ALIASED, D2D1_BITMAP_INTERPOLATION_MODE_LINEAR,
    D2D1_BITMAP_OPTIONS_CANNOT_DRAW, D2D1_BITMAP_OPTIONS_TARGET, D2D1_BITMAP_PROPERTIES1,
    D2D1_COMPATIBLE_RENDER_TARGET_OPTIONS_NONE, D2D1_DEVICE_CONTEXT_OPTIONS_NONE,
    D2D1_DRAW_TEXT_OPTIONS_CLIP, D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT,
    D2D1_DRAW_TEXT_OPTIONS_NONE, D2D1_ELLIPSE, D2D1_FACTORY_TYPE_SINGLE_THREADED,
    D2D1_ROUNDED_RECT, D2D1CreateFactory, ID2D1Bitmap, ID2D1BitmapRenderTarget, ID2D1Device,
    ID2D1DeviceContext, ID2D1Factory1, ID2D1RenderTarget, ID2D1SolidColorBrush,
};
use windows::Win32::Graphics::Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_WARP};
use windows::Win32::Graphics::Direct3D11::{
    D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_SDK_VERSION, D3D11CreateDevice, ID3D11Device,
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
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_ALPHA_MODE_IGNORE, DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_FORMAT_UNKNOWN, DXGI_SAMPLE_DESC,
};
use windows::Win32::Graphics::Dxgi::{
    DXGI_MWA_NO_ALT_ENTER, DXGI_MWA_NO_WINDOW_CHANGES, DXGI_PRESENT, DXGI_SCALING_NONE,
    DXGI_SWAP_CHAIN_DESC1, DXGI_SWAP_CHAIN_FLAG_FRAME_LATENCY_WAITABLE_OBJECT,
    DXGI_SWAP_EFFECT_FLIP_DISCARD, DXGI_USAGE_RENDER_TARGET_OUTPUT, IDXGIDevice, IDXGIDevice3,
    IDXGIFactory2, IDXGISurface, IDXGISwapChain2,
};
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
    CloseClipboard, EmptyClipboard, GetClipboardData, GetClipboardSequenceNumber, OpenClipboard,
    SetClipboardData,
};
use windows::Win32::System::Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalUnlock};
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_SET_VALUE, REG_OPTION_NON_VOLATILE, REG_SZ, RegCloseKey,
    RegCreateKeyExW, RegDeleteValueW, RegSetValueExW,
};
use windows::Win32::System::Threading::{
    AttachThreadInput, CreateMutexW, GetCurrentThreadId, INFINITE, OpenProcess,
    PROCESS_SYNCHRONIZE, Sleep, WaitForSingleObject,
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
    GetWindowTextW, GetWindowThreadProcessId, HMENU, HTCLIENT, HWND_TOPMOST, IDC_ARROW,
    IDC_SIZENWSE, IsChild, IsWindow, IsWindowVisible, KBDLLHOOKSTRUCT, KillTimer, LB_ADDSTRING,
    LB_GETCURSEL, LB_RESETCONTENT, LB_SETCURSEL, LBN_DBLCLK, LBN_SELCHANGE, LBS_HASSTRINGS,
    LBS_NOINTEGRALHEIGHT, LBS_NOTIFY, LLKHF_INJECTED, LWA_ALPHA, LoadCursorW, MSG, MSLLHOOKSTRUCT,
    MWMO_INPUTAVAILABLE, MsgWaitForMultipleObjectsEx, OBJID_CLIENT, PM_NOREMOVE, PM_REMOVE,
    PeekMessageW, PostMessageW, PostQuitMessage, QS_ALLINPUT, RegisterClassW, SW_HIDE, SW_SHOW,
    SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SWP_SHOWWINDOW, SetCursor,
    SetForegroundWindow, SetLayeredWindowAttributes, SetTimer, SetWindowLongPtrW, SetWindowPos,
    SetWindowsHookExW, ShowWindow, TranslateMessage, UnhookWindowsHookEx, WH_KEYBOARD_LL,
    WH_MOUSE_LL, WINDOW_STYLE, WM_APP, WM_CLOSE, WM_COMMAND, WM_DESTROY, WM_DPICHANGED,
    WM_ERASEBKGND, WM_HOTKEY, WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDOWN,
    WM_MOUSEHWHEEL, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_NCCREATE, WM_NCDESTROY, WM_PAINT, WM_QUIT,
    WM_RBUTTONDOWN, WM_SETCURSOR, WM_SIZE, WM_SYSKEYDOWN, WM_SYSKEYUP, WM_TIMER, WM_XBUTTONDOWN,
    WNDCLASSW, WS_CHILD, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
    WS_EX_TRANSPARENT, WS_OVERLAPPEDWINDOW, WS_POPUP, WS_TABSTOP, WS_VISIBLE,
};
#[cfg(not(feature = "console"))]
use windows::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MESSAGEBOX_STYLE, MessageBoxW};
use windows::core::{Error, HRESULT, Interface, PCWSTR, Result, w};
use windows_numerics::Vector2;

use crate::catalog::{self, Match};
use crate::config::{
    Action, Binding, Config, DetailMode, EmojiFont, FONT_SCALE_STEP, Hotkey, Keybinds,
    MAX_FONT_SCALE, MAX_PICKER_HEIGHT, MAX_PICKER_WIDTH, MIN_FONT_SCALE, MIN_PICKER_HEIGHT,
    MIN_PICKER_WIDTH, MOD_ALT_VALUE, MOD_CONTROL_VALUE, MOD_NOREPEAT_VALUE, MOD_SHIFT_VALUE,
    MOD_WIN_VALUE, PickerDimensions, RecentGlyph, SkinTone, load_config, load_recents,
    remember_recent, save_config,
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
/// Geometry of the shortcut list.
const SHORTCUT_LIST_TOP: i32 = 42;
const SHORTCUT_ROW_HEIGHT: i32 = 32;
const SCROLLBAR_THUMB_WIDTH: f32 = 5.0;
/// Extra width the thumb takes on while hovered or dragged, giving the
/// pointer a larger target once it is already there.
const SCROLLBAR_GRIP_GROWTH: f32 = 4.0;
/// How long the grip takes to reach its full width, in seconds.
const SCROLLBAR_GRIP_SECONDS: f32 = 0.12;
/// How many matches a search keeps. The list scrolls, so this is a bound on
/// ranking work rather than on what the window can show; past a few hundred
/// rows the ranking is guesswork and scrolling to reach it is slower than
/// refining the query.
const SEARCH_MATCH_LIMIT: usize = 300;
const GRID_CELL: i32 = 48;
const SECTION_HEADING_HEIGHT: i32 = 26;
const SECTION_GAP: i32 = 10;
const RESULTS_ID: usize = 2;
const VK_A_VALUE: u16 = 0x41;
const VK_V_VALUE: u16 = 0x56;
const VK_J_VALUE: u16 = 0x4a;
const VK_K_VALUE: u16 = 0x4b;
const FOCUS_TIMER_ID: usize = 0x0057_4d02;
const FOCUS_FRAME_MS: u32 = 100;
const GLYPH_TILE: f32 = 44.0;
/// Glyphs are packed into shared atlas pages of this many tiles per side.
/// One render target per glyph would make Direct2D allocate a whole backing
/// texture per emoji, which costs both a GPU round trip to fill and orders
/// of magnitude more memory than the 44 pixel tile needs.
const ATLAS_SIDE: u32 = 16;
const ATLAS_SLOTS: u32 = ATLAS_SIDE * ATLAS_SIDE;
/// Glyphs rasterized per BeginDraw/EndDraw pair. Small enough that a slice
/// still checks the clock and the message queue often.
const ATLAS_BATCH: usize = 8;
/// Atlas pages kept while the picker is hidden. Enough that reopening on the
/// recent grid paints from cache; beyond it the tiles came from browsing the
/// catalog and are dropped.
const ATLAS_RESIDENT_PAGES: usize = 4;
/// Wall-clock ceiling on one glyph-warming slice while the picker is on
/// screen, and while it is hidden. Rasterizing a single color emoji costs
/// several milliseconds, so warming is only ever done in these bounded
/// slices between messages, never on the path from input to pixels.
const WARM_SLICE_VISIBLE_MS: u64 = 4;
const WARM_SLICE_HIDDEN_MS: u64 = 12;
/// Pause between background slices while the picker is hidden.
const WARM_IDLE_PAUSE_MS: u32 = 20;
/// Content distance one wheel notch scrolls, in DIPs.
const WHEEL_NOTCH_DIPS: f32 = 76.0;
/// Side of the square drag handle in the bottom-right corner, in DIPs.
const RESIZE_GRIP: f32 = 16.0;
// Marks our own SendInput batches so the keyboard hook can recognize them.
const INJECTION_TAG: usize = 0x574d_4f4a;

/// Snapshot of the capture state shared with the dedicated input thread.
/// The hook procedures may only touch this and stateless system calls; the
/// UI thread owns everything else.
struct HookState {
    active: AtomicBool,
    keep_visible: AtomicBool,
    capturing_shortcut: AtomicBool,
    hwnd: AtomicIsize,
    target: AtomicIsize,
}

static HOOK_STATE: HookState = HookState {
    active: AtomicBool::new(false),
    keep_visible: AtomicBool::new(false),
    capturing_shortcut: AtomicBool::new(false),
    hwnd: AtomicIsize::new(0),
    target: AtomicIsize::new(0),
};

/// Install the low-level hooks on their own thread. Windows serializes all
/// system input through installed low-level hooks, so they must never share
/// a thread with rendering: a slow frame there becomes system-wide input
/// lag. The thread only pumps hook callbacks; the hooks stay installed for
/// the process lifetime and pass everything through while capture is off.
fn ensure_hook_thread() -> std::result::Result<(), String> {
    static STARTED: OnceLock<std::result::Result<(), String>> = OnceLock::new();
    STARTED
        .get_or_init(|| {
            let (sender, receiver) = std::sync::mpsc::sync_channel(1);
            std::thread::spawn(move || unsafe {
                let instance = match windows::Win32::System::LibraryLoader::GetModuleHandleW(None) {
                    Ok(module) => HINSTANCE(module.0),
                    Err(error) => {
                        let _ = sender.send(Err(error.to_string()));
                        return;
                    }
                };
                let keyboard = match SetWindowsHookExW(
                    WH_KEYBOARD_LL,
                    Some(keyboard_hook_proc),
                    Some(instance),
                    0,
                ) {
                    Ok(hook) => hook,
                    Err(error) => {
                        let _ = sender.send(Err(error.to_string()));
                        return;
                    }
                };
                let mouse =
                    SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook_proc), Some(instance), 0).ok();
                let _ = sender.send(Ok(()));
                let mut message = MSG::default();
                while GetMessageW(&mut message, None, 0, 0).0 > 0 {
                    let _ = TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
                UnhookWindowsHookEx(keyboard).ok();
                if let Some(mouse) = mouse {
                    UnhookWindowsHookEx(mouse).ok();
                }
            });
            receiver
                .recv_timeout(Duration::from_secs(2))
                .map_err(|error| format!("input thread did not start: {error}"))?
        })
        .clone()
}

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
    Shortcuts,
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
    Characters,
    Emoticons,
}

impl BrowseCategory {
    const ALL: [Self; 12] = [
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
        Self::Characters,
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
            Self::Symbols => "Emoji symbols",
            Self::Characters => "Characters",
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
            Self::Characters => "Characters",
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
            Self::Symbols => entry.emoji_group == Some(Group::Symbols),
            // Everything the Unicode sweep contributed: arrows, math,
            // currency, punctuation, Greek and the technical ranges.
            Self::Characters => entry.emoji_group.is_none() && entry.kind != "Emoticon",
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
            Self::Symbols => "🔣",
            Self::Characters => "Ω",
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
                | Self::Symbols
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

/// One-time skin tone chooser opened by right-clicking an emoji that has
/// variants. A pick inserts that variant once; the permanent default lives
/// in settings.
#[derive(Clone, Copy, Debug)]
struct TonePicker {
    entry_index: usize,
    anchor_x: f32,
    anchor_y: f32,
}

/// Whether the browse scroll is animating toward `browse_scroll_target`.
/// Only programmatic jumps animate (category clicks, keyboard reveal,
/// paging): they head to a destination the user did not steer to, so an
/// ease-out aids orientation. Wheel input never animates; any smoothing
/// between the wheel and the content reads as a response curve, so wheel
/// deltas move the content instantly and 1:1.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ScrollAnimation {
    Idle,
    /// Exponential ease toward `browse_scroll_target`.
    Ease,
}

/// Value, minimum and maximum for the settings rows that use a slider.
fn slider_bounds(config: Config, index: usize) -> Option<(i32, i32, i32)> {
    match index {
        0 => Some((config.dimensions.width, MIN_PICKER_WIDTH, MAX_PICKER_WIDTH)),
        1 => Some((
            config.dimensions.height,
            MIN_PICKER_HEIGHT,
            MAX_PICKER_HEIGHT,
        )),
        2 => Some((config.font_scale, MIN_FONT_SCALE, MAX_FONT_SCALE)),
        _ => None,
    }
}

/// How many rows the settings view has.
const SETTINGS_ROWS: usize = 8;
const SETTINGS_LIST_TOP: i32 = 42;
const SETTINGS_ROW_HEIGHT: i32 = 38;
/// Room kept below the last row for the hint line, which scrolls with the rows
/// rather than floating above the footer.
const SETTINGS_HINT_HEIGHT: f32 = 30.0;

/// Rows that run an action instead of holding a value. Arrow keys and a click
/// both mean "do it", since there is nothing to step through.
fn setting_is_action(index: usize) -> bool {
    index >= 6
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
    Scrollbar,
    Copy,
    Insert,
    ShiftCap,
    ToneOption(usize),
    TonePopup,
    SettingRow(usize),
    SettingSlider(usize),
    SettingsDiscard,
    SettingsReset,
    SettingsBack,
    ShortcutRow(usize),
    ShortcutsReset,
    ShortcutsBack,
    ResizeGrip,
}

struct TextFormats {
    label: IDWriteTextFormat,
    brand: IDWriteTextFormat,
    title: IDWriteTextFormat,
    metadata: IDWriteTextFormat,
    search: IDWriteTextFormat,
    glyph: IDWriteTextFormat,
    /// The emoji face at preview size, following the configured emoji font so
    /// settings show the setting rather than describing it.
    glyph_small: IDWriteTextFormat,
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
    /// `scale` multiplies every size, so the whole picker reads larger or
    /// smaller together rather than the labels drifting away from the glyphs.
    unsafe fn new(factory: &IDWriteFactory, emoji_font: EmojiFont, scale: f32) -> Result<Self> {
        let emoji_family = match emoji_font {
            EmojiFont::SegoeEmoji => w!("Segoe UI Emoji"),
            EmojiFont::SegoeSymbol => w!("Segoe UI Symbol"),
        };
        let create_text_format = |factory: &IDWriteFactory, family, size: f32, bold| unsafe {
            create_text_format(factory, family, size * scale, bold)
        };
        let glyph = create_text_format(factory, emoji_family, 26.0, false)?;
        unsafe {
            glyph.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER)?;
        }
        let glyph_small = create_text_format(factory, emoji_family, 17.0, false)?;
        unsafe {
            glyph_small.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER)?;
        }
        let symbol = create_text_format(factory, w!("Segoe UI Symbol"), 23.0, false)?;
        unsafe {
            symbol.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER)?;
        }
        let math = create_text_format(factory, w!("Cambria Math"), 22.0, false)?;
        unsafe {
            math.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER)?;
        }
        let icon = create_text_format(factory, w!("Segoe UI Symbol"), 14.0, false)?;
        unsafe {
            icon.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER)?;
        }
        let center = create_text_format(factory, w!("Segoe UI Variable Text"), 12.0, false)?;
        unsafe {
            center.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER)?;
        }
        let center_title = create_text_format(factory, w!("Segoe UI Variable Text"), 16.0, true)?;
        unsafe {
            center_title.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER)?;
        }
        let brand = create_text_format(factory, w!("Segoe UI Variable Text"), 10.0, false)?;
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
            label: create_text_format(factory, w!("Segoe UI Variable Text"), 12.0, true)?,
            brand,
            title: create_text_format(factory, w!("Segoe UI Variable Text"), 14.0, true)?,
            metadata: create_text_format(factory, w!("Segoe UI Variable Text"), 11.0, false)?,
            search: create_text_format(factory, w!("Segoe UI Variable Text"), 14.0, false)?,
            glyph,
            glyph_small,
            symbol,
            math,
            emoticon: centered(create_text_format(
                factory,
                w!("Segoe UI Variable Text"),
                14.0,
                false,
            )?)?,
            emoticon_small: centered(create_text_format(
                factory,
                w!("Segoe UI Variable Text"),
                10.0,
                false,
            )?)?,
            emoticon_icon: centered(create_text_format(
                factory,
                w!("Segoe UI Variable Text"),
                8.0,
                false,
            )?)?,
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
            if entry.kind == "Emoticon" {
                return true;
            }
            // Joiners and variation selectors shape sequences and have no
            // standalone glyph requirement; every other code point must be
            // covered or the entry renders as a placeholder box.
            entry
                .glyph
                .chars()
                .filter(|character| !matches!(*character as u32, 0xfe0f | 0x200d))
                .all(|character| {
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
    recents: Vec<RecentGlyph>,
    /// Pick counts keyed by glyph, mirroring `recents` so scoring does not
    /// scan the list for every catalog entry on every keystroke.
    usage: catalog::UsageCounts,
    config: Config,
    display_dimensions: PickerDimensions,
    view: View,
    browse_sections: Vec<BrowseSection>,
    displayable_entries: Vec<bool>,
    browse_focus: (usize, usize),
    browse_scroll: f32,
    browse_scroll_target: f32,
    browse_animation: ScrollAnimation,
    /// Scroll offset of the search result list, in DIPs. The list holds more
    /// matches than the window can show, so anything ranked below the fold is
    /// still reachable.
    result_scroll: f32,
    /// Linear progress of the scrollbar grip between its resting and gripped
    /// widths. Eased where it is used, so the growth is symmetric in both
    /// directions.
    scrollbar_grip: f32,
    /// Whether Shift is down, and whether the footer's Shift cap has been
    /// clicked to latch it. Either one puts the footer actions into their
    /// keep-open form.
    shift_held: bool,
    shift_latched: bool,
    category_scroll: f32,
    active_category: usize,
    hovered_entry: Option<usize>,
    hovered_target: Option<HitTarget>,
    tone_picker: Option<TonePicker>,
    category_icon_entries: [Option<usize>; BrowseCategory::ALL.len()],
    settings_selected: usize,
    settings_scroll: f32,
    /// Whether `status` reports a failure. Only the shortcut list draws its
    /// status both ways; elsewhere a status is always a failure.
    status_error: bool,
    /// Row of the shortcut list that has focus, and how far the list is
    /// scrolled, in DIPs.
    shortcut_selected: usize,
    shortcut_scroll: f32,
    /// Which action a capture in progress will bind. `None` while capturing
    /// means the capture belongs to the global open shortcut.
    capturing_action: Option<Action>,
    settings_original: Config,
    /// Index of the settings row whose slider is being dragged.
    dragging_slider: Option<usize>,
    dragging_scrollbar: Option<f32>,
    /// Cursor offset from the window's bottom-right corner while the corner
    /// grip is being dragged, in physical pixels.
    dragging_resize: Option<(i32, i32)>,
    capturing_shortcut: bool,
    keyboard_state: [u8; 256],
    pending_commit: Option<bool>,
    capture_active: bool,
    registered_hotkey: Hotkey,
    dpi: u32,
    status: Option<String>,
    d2d_factory: ID2D1Factory1,
    dwrite_factory: IDWriteFactory,
    render: Option<RenderResources>,
    /// Set when something changed the picture. The message loop renders at
    /// most one frame per display refresh, so a burst of input coalesces into
    /// a single paced frame instead of one present per event.
    needs_render: bool,
    formats: TextFormats,
    keep_visible: bool,
    /// Timestamp of the last animation frame; drives time-based smoothing so
    /// scroll speed is identical at 60Hz and 165Hz.
    last_frame: Option<Instant>,
}

/// Device-bound rendering state: the swap chain, the brush set, and the
/// glyph atlas. Color emoji rasterization costs several milliseconds per
/// glyph; each glyph renders once into an atlas tile and every later frame
/// blits it. All of this dies together on device loss.
/// Cache key: catalog entry index plus the applied skin tone ordinal.
/// A cached `None` marks a glyph that failed to rasterize, so it is not
/// retried every frame.
type GlyphCache = HashMap<(usize, u8), Option<GlyphSlot>>;

/// Where a rasterized glyph lives: which atlas page, and the tile within it.
#[derive(Clone, Copy)]
struct GlyphSlot {
    page: usize,
    source: D2D_RECT_F,
}

/// One atlas texture holding `ATLAS_SLOTS` glyph tiles.
struct AtlasPage {
    target: ID2D1BitmapRenderTarget,
    bitmap: ID2D1Bitmap,
    used: u32,
}

fn atlas_slot_rect(slot: u32) -> D2D_RECT_F {
    let left = (slot % ATLAS_SIDE) as f32 * GLYPH_TILE;
    let top = (slot / ATLAS_SIDE) as f32 * GLYPH_TILE;
    rect(left, top, left + GLYPH_TILE, top + GLYPH_TILE)
}

/// Allocate an empty atlas page. Clearing once here means the per-glyph
/// batches never need to clear, so they only draw.
unsafe fn create_atlas_page(target: &ID2D1RenderTarget) -> Result<AtlasPage> {
    let side = GLYPH_TILE * ATLAS_SIDE as f32;
    let size = D2D_SIZE_F {
        width: side,
        height: side,
    };
    let format = D2D1_PIXEL_FORMAT {
        format: DXGI_FORMAT_B8G8R8A8_UNORM,
        alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
    };
    let page = unsafe {
        target.CreateCompatibleRenderTarget(
            Some(&size),
            None,
            Some(&format),
            D2D1_COMPATIBLE_RENDER_TARGET_OPTIONS_NONE,
        )?
    };
    unsafe {
        page.BeginDraw();
        page.Clear(Some(&D2D1_COLOR_F {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.0,
        }));
        page.EndDraw(None, None)?;
    }
    let bitmap = unsafe { page.GetBitmap()? };
    Ok(AtlasPage {
        target: page,
        bitmap,
        used: 0,
    })
}

/// Owns the swap chain's frame-latency waitable handle so it closes exactly
/// once when the device stack is torn down.
struct FrameLatencyGate(HANDLE);

impl Drop for FrameLatencyGate {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }
}

#[derive(Clone)]
struct RenderResources {
    /// The device context viewed through its render-target interface; all
    /// drawing code works against this.
    target: ID2D1RenderTarget,
    context: ID2D1DeviceContext,
    device: ID2D1Device,
    dxgi_device: IDXGIDevice,
    swapchain: IDXGISwapChain2,
    /// Signals when the compositor can accept a new frame; waiting on it
    /// paces animation to the monitor refresh rate with one frame of latency.
    frame_gate: Rc<FrameLatencyGate>,
    brushes: Brushes,
    glyphs: Rc<RefCell<GlyphCache>>,
    atlas: Rc<RefCell<Vec<AtlasPage>>>,
    /// Entries a frame wanted but could not draw because they were not
    /// rasterized yet. Drawing never rasterizes; it records the miss here and
    /// leaves the tile empty, and the idle warmer fills it for a later frame.
    wanted: Rc<RefCell<Vec<usize>>>,
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

unsafe fn create_brushes(target: &ID2D1RenderTarget) -> Result<Brushes> {
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
        let d2d_factory: ID2D1Factory1 =
            unsafe { D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)? };
        let dwrite_factory: IDWriteFactory =
            unsafe { DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)? };
        let formats =
            unsafe { TextFormats::new(&dwrite_factory, config.emoji_font, config.scale())? };
        let displayable_entries = unsafe { build_displayable_entry_index(&dwrite_factory)? };
        let recents = load_recents();
        let mut state = Self {
            hwnd: HWND::default(),
            accessible_results: HWND::default(),
            target: HWND::default(),
            target_focus: HWND::default(),
            search: SearchField::default(),
            matches: Vec::new(),
            selected: 0,
            usage: usage_counts(&recents),
            recents,
            config,
            display_dimensions: config.dimensions,
            view: View::Search,
            browse_sections: Vec::new(),
            displayable_entries,
            browse_focus: (0, 0),
            browse_scroll: 0.0,
            browse_scroll_target: 0.0,
            result_scroll: 0.0,
            scrollbar_grip: 0.0,
            shift_held: false,
            shift_latched: false,
            browse_animation: ScrollAnimation::Idle,
            category_scroll: 0.0,
            active_category: 0,
            hovered_entry: None,
            hovered_target: None,
            tone_picker: None,
            category_icon_entries: BrowseCategory::ALL.map(|category| {
                category
                    .uses_color_icon()
                    .then(|| {
                        catalog::entries()
                            .iter()
                            .position(|entry| entry.glyph == category.icon())
                    })
                    .flatten()
            }),
            settings_selected: 0,
            settings_scroll: 0.0,
            status_error: true,
            shortcut_selected: 0,
            shortcut_scroll: 0.0,
            capturing_action: None,
            settings_original: config,
            dragging_slider: None,
            dragging_scrollbar: None,
            dragging_resize: None,
            capturing_shortcut: false,
            keyboard_state: [0; 256],
            pending_commit: None,
            capture_active: false,
            registered_hotkey: config.hotkey,
            dpi: 96,
            status: None,
            d2d_factory,
            dwrite_factory,
            render: None,
            needs_render: false,
            formats,
            keep_visible,
            last_frame: None,
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
        (RESULT_ROW_HEIGHT as f32 * self.config.scale()).round() as i32
    }

    fn grid_cell(&self) -> i32 {
        (GRID_CELL as f32 * self.config.scale()).round() as i32
    }

    fn footer_top(&self) -> i32 {
        self.dimensions().1 - FOOTER_HEIGHT
    }

    /// Height of the result list viewport, in DIPs.
    fn result_viewport(&self) -> f32 {
        (self.footer_top() - SEARCH_RESULTS_TOP).max(1) as f32
    }

    fn total_result_height(&self) -> f32 {
        self.matches.len() as f32 * self.row_height() as f32
    }

    fn maximum_result_scroll(&self) -> f32 {
        (self.total_result_height() - self.result_viewport()).max(0.0)
    }

    fn clamp_result_scroll(&mut self) {
        self.result_scroll = self.result_scroll.clamp(0.0, self.maximum_result_scroll());
    }

    /// Bring the selected row fully inside the viewport, scrolling the least
    /// distance that does so.
    fn ensure_selected_result_visible(&mut self) {
        let row_height = self.row_height() as f32;
        let top = self.selected as f32 * row_height;
        let bottom = top + row_height;
        if top < self.result_scroll {
            self.result_scroll = top;
        } else if bottom > self.result_scroll + self.result_viewport() {
            self.result_scroll = bottom - self.result_viewport();
        }
        self.clamp_result_scroll();
    }

    unsafe fn set_result_scroll_immediate(&mut self, position: f32) {
        self.result_scroll = position;
        self.clamp_result_scroll();
        unsafe {
            let _ = InvalidateRect(Some(self.hwnd), None, false);
        }
    }

    fn grid_columns(&self) -> usize {
        ((self.dimensions().0 - 24) / self.grid_cell()).max(1) as usize
    }

    fn query(&self) -> &str {
        &self.search.text
    }

    fn browsing(&self) -> bool {
        self.query().trim().is_empty()
    }

    unsafe fn update_results(&mut self) {
        self.tone_picker = None;
        // Typing takes over from any scroll in flight.
        self.browse_scroll_target = self.browse_scroll;
        self.browse_animation = ScrollAnimation::Idle;
        self.matches = if self.browsing() {
            Vec::new()
        } else {
            catalog::search(&self.search.text, SEARCH_MATCH_LIMIT, &self.usage)
        };
        self.result_scroll = 0.0;
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
        self.ensure_selected_result_visible();
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
        let before = self.browse_scroll_target;
        if item_top < self.browse_scroll_target {
            self.browse_scroll_target = item_top;
        } else if item_bottom > self.browse_scroll_target + viewport_height {
            self.browse_scroll_target = item_bottom - viewport_height;
        }
        self.clamp_browse_scroll();
        if self.browse_scroll_target != before {
            self.browse_animation = ScrollAnimation::Ease;
        }
    }

    fn rebuild_browse_sections(&mut self) {
        let entries = catalog::entries();
        let mut recent = self
            .recents
            .iter()
            .take(RECENT_GRID_LIMIT)
            .filter_map(|recent| entries.iter().position(|entry| entry.glyph == recent.glyph))
            .collect::<Vec<_>>();
        if recent.is_empty() {
            recent.extend(
                catalog::search("", 24, &self.usage)
                    .into_iter()
                    .map(|found| found.index),
            );
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
        self.browse_animation = ScrollAnimation::Idle;
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
                        (
                            self.grid_columns(),
                            self.grid_cell() as f32,
                            self.grid_cell(),
                        )
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

    /// Page-sized keyboard scrolling: a discrete jump eased toward its
    /// destination.
    fn scroll_browse(&mut self, delta: f32) {
        self.browse_scroll_target += delta;
        self.clamp_browse_scroll();
        if self.browse_scroll_target != self.browse_scroll {
            self.browse_animation = ScrollAnimation::Ease;
        }
    }

    /// Top of whichever list the search view is showing.
    /// Record a pick. It feeds both the Recent grid order and the usage
    /// weight search ranking applies.
    fn record_use(&mut self, glyph: &str) {
        if let Err(error) = remember_recent(&mut self.recents, glyph) {
            eprintln!("winmoji: could not save recent item: {error}");
        }
        self.usage = usage_counts(&self.recents);
    }

    fn settings_viewport(&self) -> f32 {
        (self.footer_top() - SETTINGS_LIST_TOP).max(1) as f32
    }

    fn total_settings_height(&self) -> f32 {
        SETTINGS_ROWS as f32 * SETTINGS_ROW_HEIGHT as f32 + SETTINGS_HINT_HEIGHT
    }

    fn maximum_settings_scroll(&self) -> f32 {
        (self.total_settings_height() - self.settings_viewport()).max(0.0)
    }

    fn clamp_settings_scroll(&mut self) {
        self.settings_scroll = self
            .settings_scroll
            .clamp(0.0, self.maximum_settings_scroll());
    }

    unsafe fn set_settings_scroll_immediate(&mut self, position: f32) {
        self.settings_scroll = position;
        self.clamp_settings_scroll();
        unsafe {
            let _ = InvalidateRect(Some(self.hwnd), None, false);
        }
    }

    /// Keep the focused settings row inside the viewport after it moves, so
    /// keyboard navigation never leaves focus on a row that is off screen.
    fn ensure_selected_setting_visible(&mut self) {
        let row = SETTINGS_ROW_HEIGHT as f32;
        let top = self.settings_selected as f32 * row;
        let viewport = self.settings_viewport();
        if top < self.settings_scroll {
            self.settings_scroll = top;
        } else if top + row > self.settings_scroll + viewport {
            self.settings_scroll = top + row - viewport;
        }
        self.clamp_settings_scroll();
    }

    fn shortcut_viewport(&self) -> f32 {
        (self.footer_top() - SHORTCUT_LIST_TOP).max(1) as f32
    }

    fn total_shortcut_height(&self) -> f32 {
        Action::ALL.len() as f32 * SHORTCUT_ROW_HEIGHT as f32
    }

    fn maximum_shortcut_scroll(&self) -> f32 {
        (self.total_shortcut_height() - self.shortcut_viewport()).max(0.0)
    }

    fn clamp_shortcut_scroll(&mut self) {
        self.shortcut_scroll = self
            .shortcut_scroll
            .clamp(0.0, self.maximum_shortcut_scroll());
    }

    unsafe fn set_shortcut_scroll_immediate(&mut self, position: f32) {
        self.shortcut_scroll = position;
        self.clamp_shortcut_scroll();
        unsafe {
            let _ = InvalidateRect(Some(self.hwnd), None, false);
        }
    }

    /// Whether the footer actions are in their keep-open form. Holding Shift
    /// and clicking the Shift cap are the same switch reached two ways.
    fn keep_open(&self) -> bool {
        self.shift_held || self.shift_latched
    }

    fn list_content_top(&self) -> i32 {
        if self.browsing() {
            BROWSE_CONTENT_TOP
        } else {
            SEARCH_RESULTS_TOP
        }
    }

    fn scrollbar_grip_target(&self) -> f32 {
        let gripped = self.dragging_scrollbar.is_some()
            || matches!(self.hovered_target, Some(HitTarget::Scrollbar));
        if gripped { 1.0 } else { 0.0 }
    }

    /// True while anything on screen needs animation frames: the browse
    /// scroll easing to a destination, or the scrollbar grip easing between
    /// widths. The message loop polls this after handling input and renders
    /// vsync-paced frames until everything settles.
    fn animation_active(&self) -> bool {
        if self.view != View::Search || !unsafe { IsWindowVisible(self.hwnd) }.as_bool() {
            return false;
        }
        (self.browse_animation != ScrollAnimation::Idle && self.browsing())
            || self.scrollbar_grip != self.scrollbar_grip_target()
    }

    /// Advance the grip toward its target at a fixed rate, so the growth
    /// takes the same time however far it has left to travel.
    fn tick_scrollbar_grip(&mut self, dt: f32) {
        let target = self.scrollbar_grip_target();
        let step = dt / SCROLLBAR_GRIP_SECONDS;
        if (target - self.scrollbar_grip).abs() <= step {
            self.scrollbar_grip = target;
        } else if target > self.scrollbar_grip {
            self.scrollbar_grip += step;
        } else {
            self.scrollbar_grip -= step;
        }
    }

    /// Cancel any scroll animation, leaving the content where it stands.
    unsafe fn settle_scroll(&mut self) {
        self.browse_scroll_target = self.browse_scroll;
        self.browse_animation = ScrollAnimation::Idle;
        self.last_frame = None;
        unsafe {
            self.sync_accessible_results();
        }
    }

    unsafe fn tick_browse_scroll(&mut self, dt: f32) {
        match self.browse_animation {
            ScrollAnimation::Idle => {}
            ScrollAnimation::Ease => {
                // Long jumps teleport to within one viewport of the
                // destination so the animation never renders the entire
                // catalog in between.
                let viewport = (self.footer_top() - BROWSE_CONTENT_TOP).max(1) as f32;
                let span = self.browse_scroll_target - self.browse_scroll;
                if span.abs() > viewport * 2.0 {
                    self.browse_scroll = self.browse_scroll_target - viewport * span.signum();
                }
                let distance = self.browse_scroll_target - self.browse_scroll;
                if distance.abs() < 0.35 {
                    self.browse_scroll = self.browse_scroll_target;
                    unsafe {
                        self.settle_scroll();
                    }
                } else {
                    self.browse_scroll =
                        smooth_scroll_step(self.browse_scroll, self.browse_scroll_target, dt);
                }
            }
        }
        self.update_active_category();
    }

    unsafe fn set_browse_scroll_immediate(&mut self, position: f32) {
        let position = position.clamp(0.0, self.maximum_browse_scroll());
        self.browse_scroll = position;
        self.browse_scroll_target = position;
        self.browse_animation = ScrollAnimation::Idle;
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
            if self.browse_scroll_target != self.browse_scroll {
                self.browse_animation = ScrollAnimation::Ease;
            }
            unsafe {
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

    /// Entries within the viewport plus a viewport of lookahead either way.
    /// Warming this window means scrolling runs into tiles that are already
    /// rasterized, without paying to rasterize a catalog the user may never
    /// look at.
    fn prefetch_entries(&self) -> Vec<usize> {
        let viewport = (self.footer_top() - BROWSE_CONTENT_TOP).max(1) as f32;
        let top = self.browse_scroll - viewport;
        let bottom = self.browse_scroll + viewport * 2.0;
        let layouts = self.section_layouts();
        let mut found = Vec::new();
        for (section, layout) in self.browse_sections.iter().zip(layouts.iter()) {
            if (layout.bottom as f32) < top || (layout.top as f32) > bottom {
                continue;
            }
            for item in visible_item_range(*layout, section.indices.len(), top, bottom) {
                found.push(section.indices[item]);
            }
        }
        found
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
        self.formats = unsafe {
            TextFormats::new(
                &self.dwrite_factory,
                self.config.emoji_font,
                self.config.scale(),
            )?
        };
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
                format!("Text size, {} percent", self.config.font_scale),
                format!("Hover details, {}", self.config.details),
                format!("Emoji font, {}", self.config.emoji_font),
                format!("Skin tone, {}", self.config.skin_tone),
                format!("Keyboard shortcuts, {} actions", Action::ALL.len()),
                format!("Open shortcut, {}", self.config.hotkey),
            ],
            View::Shortcuts => Action::ALL
                .iter()
                .map(|action| format!("{}, {}", action.label(), self.config.keys.get(*action)))
                .collect::<Vec<_>>(),
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
        } else if self.view == View::Shortcuts {
            self.shortcut_selected
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

        // The loop separates input from frame pacing: every pending message
        // is drained first, then, while a scroll animation is live, one frame
        // renders paced by the swap chain's latency waitable (the compositor
        // clock). Idle, the thread blocks in MsgWaitForMultipleObjectsEx and
        // costs nothing. Animation never rides WM_TIMER or WM_PAINT, whose
        // lowest-priority coalesced delivery is what made scrolling stutter.
        let mut message = MSG::default();
        'run: loop {
            while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
                if message.message == WM_QUIT {
                    break 'run;
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
                    let shift = GetKeyState(VK_SHIFT.0 as i32) < 0;
                    let handled = match state.view {
                        View::Settings => handle_settings_key(state, key, control),
                        View::Shortcuts => handle_shortcuts_key(state, key, control),
                        View::Search => handle_picker_key(state, key, control, shift),
                    };
                    if handled {
                        continue;
                    }
                }
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
            let state = &mut *state_pointer;
            if state.animation_active() {
                render_animation_frame(state);
            } else if state.needs_render {
                render_frame(state);
                // A frame costs a fraction of the refresh interval; spend
                // the slack filling tiles so rows revealed by a continuous
                // scroll are not left blank until the gesture stops.
                let _ = warm_glyph_slice(state);
            } else {
                state.last_frame = None;
                // Warming ahead for a picker nobody is looking at is never
                // urgent; duty-cycle it so the resident process does not
                // monopolise a core.
                let wait = match warm_glyph_slice(state) {
                    WarmOutcome::Worked if IsWindowVisible(state.hwnd).as_bool() => 0,
                    WarmOutcome::Worked => WARM_IDLE_PAUSE_MS,
                    WarmOutcome::Done => INFINITE,
                };
                if wait > 0 {
                    let _ =
                        MsgWaitForMultipleObjectsEx(None, wait, QS_ALLINPUT, MWMO_INPUTAVAILABLE);
                }
            }
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
    if code < 0 || !HOOK_STATE.active.load(Ordering::Acquire) {
        return unsafe { CallNextHookEx(None, code, wparam, lparam) };
    }
    let message = wparam.0 as u32;
    if !matches!(message, WM_KEYDOWN | WM_SYSKEYDOWN | WM_KEYUP | WM_SYSKEYUP) {
        return unsafe { CallNextHookEx(None, code, wparam, lparam) };
    }
    let event = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
    // Our own SendInput batches pass untouched: eating one of their UTF-16
    // halves would corrupt the inserted character. Other injected input
    // (software remappers and similar) is treated like typing.
    if event.flags.contains(LLKHF_INJECTED) && event.dwExtraInfo == INJECTION_TAG {
        return unsafe { CallNextHookEx(None, code, wparam, lparam) };
    }
    // Alt chords belong to the system: eating TAB while Alt is held would
    // block Alt+Tab (and Alt+Esc, Alt+F4) for as long as the picker is open.
    // Alt-modified keys arrive as WM_SYSKEY* events, while AltGr chords come
    // through as plain WM_KEYDOWN because the driver reports the paired Ctrl,
    // so layout-level AltGr typing still captures normally. Modifier keys are
    // exempt so the captured keyboard state keeps tracking Alt itself, and
    // shortcut recording keeps the whole chord.
    if matches!(message, WM_SYSKEYDOWN | WM_SYSKEYUP)
        && !is_modifier_key(VIRTUAL_KEY(event.vkCode as u16))
        && !HOOK_STATE.capturing_shortcut.load(Ordering::Acquire)
    {
        return unsafe { CallNextHookEx(None, code, wparam, lparam) };
    }
    let hwnd = HWND(HOOK_STATE.hwnd.load(Ordering::Acquire) as *mut c_void);
    let target = HWND(HOOK_STATE.target.load(Ordering::Acquire) as *mut c_void);
    let keep_visible = HOOK_STATE.keep_visible.load(Ordering::Acquire);
    if !keep_visible && unsafe { GetForegroundWindow() } != target {
        unsafe {
            let _ = PostMessageW(Some(hwnd), WM_CAPTURE_TARGET_LOST, WPARAM(0), LPARAM(0));
        }
        return unsafe { CallNextHookEx(None, code, wparam, lparam) };
    }
    // Preview mode has no target window to scope the capture, so it captures
    // only while the cursor is over the picker; everything else keeps typing
    // into the rest of the desktop normally.
    if keep_visible && !cursor_over_window(hwnd) {
        return unsafe { CallNextHookEx(None, code, wparam, lparam) };
    }
    let key_up = matches!(message, WM_KEYUP | WM_SYSKEYUP);
    let packed = event.scanCode as u64 | ((key_up as u64) << 32);
    unsafe {
        let _ = PostMessageW(
            Some(hwnd),
            WM_CAPTURED_KEY,
            WPARAM(event.vkCode as usize),
            LPARAM(packed as isize),
        );
    }
    // Modifier events must reach the system: discarding them here freezes
    // the system key-state tables, which leaves Ctrl/Shift stuck down after
    // the picker closes and blocks SendInput, whose preflight waits for all
    // modifiers to be released. PrintScreen and Win-modified shortcuts
    // (screenshots, OS shortcuts) also stay with the system.
    let virtual_key = VIRTUAL_KEY(event.vkCode as u16);
    if is_modifier_key(virtual_key)
        || virtual_key == VK_SNAPSHOT
        || unsafe { GetAsyncKeyState(VK_LWIN.0 as i32) } < 0
        || unsafe { GetAsyncKeyState(VK_RWIN.0 as i32) } < 0
    {
        return unsafe { CallNextHookEx(None, code, wparam, lparam) };
    }
    LRESULT(1)
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
    if code >= 0
        && HOOK_STATE.active.load(Ordering::Acquire)
        && !HOOK_STATE.keep_visible.load(Ordering::Acquire)
    {
        let message = wparam.0 as u32;
        if matches!(
            message,
            WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN | WM_XBUTTONDOWN
        ) {
            let event = unsafe { &*(lparam.0 as *const MSLLHOOKSTRUCT) };
            let hwnd = HWND(HOOK_STATE.hwnd.load(Ordering::Acquire) as *mut c_void);
            let mut window = RECT::default();
            let inside = unsafe { GetWindowRect(hwnd, &mut window) }.is_ok()
                && event.pt.x >= window.left
                && event.pt.x < window.right
                && event.pt.y >= window.top
                && event.pt.y < window.bottom;
            if !inside {
                unsafe {
                    let _ = PostMessageW(Some(hwnd), WM_CAPTURE_TARGET_LOST, WPARAM(0), LPARAM(0));
                }
            }
        }
    }
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

unsafe fn start_keyboard_capture(state: &mut AppState) -> Result<()> {
    if state.capture_active {
        return Ok(());
    }
    ensure_hook_thread().map_err(|message| Error::new(HRESULT(0x80004005u32 as i32), message))?;
    unsafe {
        GetKeyboardState(&mut state.keyboard_state)?;
    }
    state.pending_commit = None;
    state.capture_active = true;
    HOOK_STATE
        .hwnd
        .store(state.hwnd.0 as isize, Ordering::Release);
    HOOK_STATE
        .target
        .store(state.target.0 as isize, Ordering::Release);
    HOOK_STATE
        .keep_visible
        .store(state.keep_visible, Ordering::Release);
    HOOK_STATE.active.store(true, Ordering::Release);
    Ok(())
}

unsafe fn stop_keyboard_capture(state: &mut AppState) {
    HOOK_STATE.active.store(false, Ordering::Release);
    state.capture_active = false;
    state.pending_commit = None;
}

/// Shortcut recording needs the whole chord, including Alt-modified keys the
/// hook otherwise leaves to the system; the flag mirrors into the hook state
/// so the input thread sees it.
fn set_capturing_shortcut(state: &mut AppState, value: bool) {
    state.capturing_shortcut = value;
    HOOK_STATE
        .capturing_shortcut
        .store(value, Ordering::Release);
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
                    View::Settings => SETTINGS_ROWS,
                    View::Shortcuts => Action::ALL.len(),
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
                        state.settings_selected = state.selected.min(SETTINGS_ROWS - 1);
                        state.ensure_selected_setting_visible();
                    }
                    if state.view == View::Shortcuts {
                        state.shortcut_selected = state.selected.min(Action::ALL.len() - 1);
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
                if state.dragging_resize.is_some() {
                    update_dragged_resize(state);
                } else if state.dragging_slider.is_some() {
                    update_dragged_slider(state, x);
                } else if state.dragging_scrollbar.is_some() {
                    update_dragged_scrollbar(state, y);
                } else {
                    update_hover(state, x, y);
                }
            }
            LRESULT(0)
        }
        WM_SETCURSOR if (lparam.0 as u32 & 0xffff) == HTCLIENT => {
            let over_grip = matches!(state.hovered_target, Some(HitTarget::ResizeGrip))
                || state.dragging_resize.is_some();
            if over_grip {
                unsafe {
                    if let Ok(cursor) = LoadCursorW(None, IDC_SIZENWSE) {
                        SetCursor(Some(cursor));
                    }
                }
                LRESULT(1)
            } else {
                unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
            }
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
            let was_dragging = state.dragging_slider.is_some()
                || was_dragging_scrollbar
                || state.dragging_resize.is_some();
            if state.dragging_slider.take().is_some() {
                unsafe {
                    resize_window_in_place(state);
                }
            }
            if state.dragging_resize.take().is_some() {
                // The drag is the authoritative size; keep it across restarts.
                let _ = save_config(state.config);
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
        WM_RBUTTONDOWN => {
            let (x, y) = mouse_point_dip(lparam, state.dpi);
            unsafe {
                open_tone_picker(state, x, y);
            }
            LRESULT(0)
        }
        WM_MOUSEWHEEL | WM_MOUSEHWHEEL => {
            unsafe {
                route_wheel(state, message == WM_MOUSEHWHEEL, wparam, lparam);
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
            unsafe {
                resize_swapchain(state);
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

/// How many recent glyphs the Recent grid shows. The store keeps more than
/// this so search ranking has a longer memory than the grid does.
const RECENT_GRID_LIMIT: usize = 32;

fn usage_counts(recents: &[RecentGlyph]) -> catalog::UsageCounts {
    recents
        .iter()
        .map(|recent| (recent.glyph.clone(), recent.uses))
        .collect()
}

/// Make the window visible without activating it and put it back at the front
/// of the topmost band. WS_EX_TOPMOST only decides which band the window
/// belongs to; its position inside that band is wherever it was last left, and
/// a picker that never takes activation is never raised by activation either.
/// Anything that displaces it once displaces it permanently, and ShowWindow
/// alone then returns it to exactly that spot, behind the window the user is
/// typing into: visible to the API, invisible on screen, and passing clicks
/// through to the application underneath.
unsafe fn raise_picker_window(hwnd: HWND) {
    unsafe {
        SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW,
        )
        .ok();
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
        // Positioning settles the DPI for the monitor the picker lands on;
        // make the device agree before anything is drawn.
        apply_device_dpi(state);
        // Render before showing: the window otherwise appears as an empty
        // frame until the first WM_PAINT lands.
        render_frame(state);
        raise_picker_window(state.hwnd);
        arm_focus_watch(state);
        if let Err(error) = start_keyboard_capture(state) {
            state.status = Some(format!("Keyboard capture unavailable: {error}"));
            let _ = InvalidateRect(Some(state.hwnd), None, false);
            return;
        }
        let _ = InvalidateRect(Some(state.hwnd), None, false);
    }
}

/// Point the device at the window's current DPI and resize the swap chain to
/// match the client area.
unsafe fn apply_device_dpi(state: &mut AppState) {
    let dpi = unsafe { GetDpiForWindow(state.hwnd) }.max(96);
    state.dpi = dpi;
    unsafe {
        resize_swapchain(state);
    }
}

unsafe fn arm_focus_watch(state: &AppState) {
    if !state.keep_visible {
        unsafe {
            let _ = SetTimer(Some(state.hwnd), FOCUS_TIMER_ID, FOCUS_FRAME_MS, None);
        }
    }
}

/// Release the glyph and driver caches that browsing accumulates. Rendering
/// the whole catalog leaves Direct2D and DirectWrite holding hundreds of
/// megabytes of rasterized glyphs, which a picker that spends most of its
/// life hidden has no business keeping.
unsafe fn release_browsing_caches(state: &mut AppState) {
    let Some(resources) = &state.render else {
        return;
    };
    // Tiles from a long browse are not worth keeping; a handful of pages is,
    // so the common open-glance-pick cycle still paints from a warm cache.
    if resources.atlas.borrow().len() > ATLAS_RESIDENT_PAGES {
        resources.glyphs.borrow_mut().clear();
        resources.atlas.borrow_mut().clear();
    }
    resources.wanted.borrow_mut().clear();
    unsafe {
        resources.device.ClearResources(0);
        if let Ok(dxgi) = resources.dxgi_device.cast::<IDXGIDevice3>() {
            dxgi.Trim();
        }
    }
}

unsafe fn hide_picker(state: &mut AppState) {
    state.tone_picker = None;
    // Cancel any scroll in flight so the loop never animates a hidden window.
    state.browse_scroll_target = state.browse_scroll;
    state.browse_animation = ScrollAnimation::Idle;
    state.scrollbar_grip = 0.0;
    state.shift_held = false;
    state.shift_latched = false;
    state.last_frame = None;
    unsafe {
        stop_keyboard_capture(state);
        KillTimer(Some(state.hwnd), FOCUS_TIMER_ID).ok();
        let _ = ShowWindow(state.hwnd, SW_HIDE);
        release_browsing_caches(state);
    }
}

unsafe fn enter_search(state: &mut AppState) {
    state.view = View::Search;
    set_capturing_shortcut(state, false);
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

/// Row geometry of the shortcut list, before scrolling is applied.
fn shortcut_row_rect(width: i32, index: usize) -> D2D_RECT_F {
    let top = SHORTCUT_LIST_TOP as f32 + index as f32 * SHORTCUT_ROW_HEIGHT as f32;
    rect(
        12.0,
        top,
        width as f32 - 16.0,
        top + SHORTCUT_ROW_HEIGHT as f32 - 4.0,
    )
}

unsafe fn enter_shortcuts(state: &mut AppState) {
    state.view = View::Shortcuts;
    state.status = None;
    state.shortcut_selected = 0;
    state.shortcut_scroll = 0.0;
    state.capturing_action = None;
    set_capturing_shortcut(state, false);
    unsafe {
        state.sync_accessible_results();
        let _ = InvalidateRect(Some(state.hwnd), None, false);
    }
}

/// Leave the list and go back to the settings page that opened it.
unsafe fn leave_shortcuts(state: &mut AppState) {
    state.capturing_action = None;
    set_capturing_shortcut(state, false);
    state.status = None;
    unsafe {
        enter_settings(state);
    }
    state.settings_selected = 6;
    state.selected = 6;
    state.ensure_selected_setting_visible();
    unsafe {
        state.sync_accessible_results();
        let _ = InvalidateRect(Some(state.hwnd), None, false);
    }
}

unsafe fn begin_capture(state: &mut AppState, action: Action) {
    state.capturing_action = Some(action);
    set_capturing_shortcut(state, true);
    state.status = Some(format!("Press the new shortcut for {}", action.label()));
    state.status_error = false;
    unsafe {
        let _ = InvalidateRect(Some(state.hwnd), None, false);
    }
}

unsafe fn reset_shortcuts(state: &mut AppState) {
    state.config.keys = Keybinds::default();
    state.capturing_action = None;
    set_capturing_shortcut(state, false);
    // Rebinding on this page writes straight through, so resetting has to as
    // well: defaults that only live in memory would come back changed.
    match save_config(state.config) {
        Ok(()) => {
            state.status = Some("Shortcuts reset".to_string());
            state.status_error = false;
        }
        Err(error) => {
            state.status = Some(format!("Could not save shortcuts: {error}"));
            state.status_error = true;
        }
    }
    unsafe {
        state.sync_accessible_results();
        let _ = InvalidateRect(Some(state.hwnd), None, false);
    }
}

unsafe fn move_shortcut_selection(state: &mut AppState, delta: isize) {
    state.shortcut_selected = state
        .shortcut_selected
        .saturating_add_signed(delta)
        .min(Action::ALL.len() - 1);
    let row = SHORTCUT_ROW_HEIGHT as f32;
    let top = state.shortcut_selected as f32 * row;
    let viewport = state.shortcut_viewport();
    if top < state.shortcut_scroll {
        state.shortcut_scroll = top;
    } else if top + row > state.shortcut_scroll + viewport {
        state.shortcut_scroll = top + row - viewport;
    }
    state.clamp_shortcut_scroll();
    unsafe {
        state.sync_accessible_results();
        let _ = InvalidateRect(Some(state.hwnd), None, false);
    }
}

/// Record a captured chord against the action being rebound. A chord another
/// action owns is refused rather than stolen, so no action is left
/// unreachable by a rebind the user cannot see.
unsafe fn apply_captured_binding(state: &mut AppState, action: Action, key: VIRTUAL_KEY) {
    let modifiers = if state.capture_active {
        captured_hotkey_modifiers(&state.keyboard_state)
    } else {
        current_hotkey_modifiers()
    };
    let result = Binding::from_parts(modifiers & !MOD_NOREPEAT_VALUE, key.0 as u32)
        .and_then(|binding| state.config.keys.set(action, binding));
    match result {
        Ok(()) => {
            state.capturing_action = None;
            set_capturing_shortcut(state, false);
            state.status = None;
            state.status_error = false;
            if let Err(error) = save_config(state.config) {
                state.status = Some(format!("Could not save shortcuts: {error}"));
                state.status_error = true;
            }
        }
        Err(error) => {
            state.status = Some(error);
            state.status_error = true;
        }
    }
    unsafe {
        state.sync_accessible_results();
        let _ = InvalidateRect(Some(state.hwnd), None, false);
    }
}

unsafe fn handle_shortcuts_key(state: &mut AppState, key: VIRTUAL_KEY, control: bool) -> bool {
    if state.capturing_shortcut {
        if key == VK_ESCAPE {
            state.capturing_action = None;
            set_capturing_shortcut(state, false);
            state.status = None;
            unsafe {
                let _ = InvalidateRect(Some(state.hwnd), None, false);
            }
            return true;
        }
        if matches!(key, VK_CONTROL | VK_SHIFT | VK_MENU | VK_LWIN | VK_RWIN) {
            return true;
        }
        if let Some(action) = state.capturing_action {
            unsafe {
                apply_captured_binding(state, action, key);
            }
        }
        return true;
    }
    if key == VK_ESCAPE {
        unsafe {
            leave_shortcuts(state);
        }
        return true;
    }
    if key == VK_RETURN {
        let action = Action::ALL[state.shortcut_selected.min(Action::ALL.len() - 1)];
        unsafe {
            begin_capture(state, action);
        }
        return true;
    }
    if key == VK_UP || (control && key.0 == VK_K_VALUE) {
        unsafe {
            move_shortcut_selection(state, -1);
        }
        return true;
    }
    if key == VK_DOWN || (control && key.0 == VK_J_VALUE) {
        unsafe {
            move_shortcut_selection(state, 1);
        }
        return true;
    }
    if key == VK_PRIOR || key == VK_NEXT {
        let rows = (state.shortcut_viewport() / SHORTCUT_ROW_HEIGHT as f32).max(1.0) as isize;
        unsafe {
            move_shortcut_selection(state, if key == VK_PRIOR { -rows } else { rows });
        }
        return true;
    }
    key == VK_TAB
}

unsafe fn enter_settings(state: &mut AppState) {
    if state.view != View::Settings {
        state.settings_original = state.config;
    }
    state.browse_scroll_target = state.browse_scroll;
    state.browse_animation = ScrollAnimation::Idle;
    state.view = View::Settings;
    state.status = None;
    state.settings_selected = 0;
    state.selected = 0;
    state.settings_scroll = 0.0;
    set_capturing_shortcut(state, false);
    unsafe {
        state.sync_accessible_results();
        let _ = InvalidateRect(Some(state.hwnd), None, false);
    }
}

/// Change the text scale by `steps` and keep it. Every size the picker draws
/// derives from this, so the formats are rebuilt and the window re-laid out.
unsafe fn adjust_font_scale(state: &mut AppState, steps: i32) {
    let scaled =
        (state.config.font_scale + steps * FONT_SCALE_STEP).clamp(MIN_FONT_SCALE, MAX_FONT_SCALE);
    if scaled == state.config.font_scale {
        return;
    }
    state.config.font_scale = scaled;
    unsafe {
        if let Err(error) = state.rebuild_formats() {
            state.status = Some(format!("Could not resize the text: {error}"));
            return;
        }
        state.rebuild_browse_sections_preserving_view();
        state.clamp_result_scroll();
        state.ensure_selected_result_visible();
        state.sync_accessible_results();
        let _ = InvalidateRect(Some(state.hwnd), None, false);
    }
    if let Err(error) = save_config(state.config) {
        eprintln!("winmoji: could not save the text size: {error}");
    }
}

unsafe fn adjust_setting(state: &mut AppState, delta: isize) {
    if setting_is_action(state.settings_selected) {
        // Nothing to step through, so either direction runs the row.
        unsafe {
            activate_setting(state);
        }
        return;
    }
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
        2 => unsafe {
            adjust_font_scale(state, delta as i32);
        },
        3 => state.config.details = state.config.details.next(delta),
        4 => {
            state.config.emoji_font = state.config.emoji_font.next(delta);
            if let Err(error) = unsafe { state.rebuild_formats() } {
                state.status = Some(format!("Could not change emoji font: {error}"));
            }
        }
        5 => {
            state.config.skin_tone = state.config.skin_tone.next(delta);
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
        2 => unsafe {
            // Wrap back to the smallest once the largest is reached, the way
            // the other cycling rows do.
            let steps = if state.config.font_scale >= MAX_FONT_SCALE {
                (MIN_FONT_SCALE - MAX_FONT_SCALE) / FONT_SCALE_STEP
            } else {
                1
            };
            adjust_font_scale(state, steps);
        },
        3 => {
            state.config.details = if state.config.details == DetailMode::Both {
                DetailMode::None
            } else {
                state.config.details.next(1)
            };
        }
        4 => {
            state.config.emoji_font = match state.config.emoji_font {
                EmojiFont::SegoeEmoji => EmojiFont::SegoeSymbol,
                EmojiFont::SegoeSymbol => EmojiFont::SegoeEmoji,
            };
            if let Err(error) = unsafe { state.rebuild_formats() } {
                state.status = Some(format!("Could not change emoji font: {error}"));
            }
        }
        5 => {
            state.config.skin_tone = state.config.skin_tone.cycled();
        }
        6 => unsafe {
            enter_shortcuts(state);
            return;
        },
        7 => {
            state.capturing_action = None;
            set_capturing_shortcut(state, true);
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
    // The footer follows Shift as it is held, so a change repaints even
    // though modifiers themselves run nothing.
    let shift_now = captured_key_down(&state.keyboard_state, VK_SHIFT);
    if shift_now != state.shift_held {
        state.shift_held = shift_now;
        unsafe {
            let _ = InvalidateRect(Some(state.hwnd), None, false);
        }
    }
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
    let shift = captured_key_down(&state.keyboard_state, VK_SHIFT);
    if state.view == View::Settings {
        unsafe {
            handle_settings_key(state, key, control);
        }
        return;
    }
    if state.view == View::Shortcuts {
        unsafe {
            handle_shortcuts_key(state, key, control);
        }
        return;
    }
    if key == VK_RETURN {
        // Enter finishes; Shift keeps the picker open for another pick, the
        // same way Ctrl+C and Ctrl+Shift+C differ.
        state.pending_commit = Some(!shift);
        return;
    }
    if unsafe { handle_picker_key(state, key, control, shift) } {
        return;
    }
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

unsafe fn handle_picker_key(
    state: &mut AppState,
    key: VIRTUAL_KEY,
    control: bool,
    shift: bool,
) -> bool {
    // The tone chooser is a transient popup layered over the picker, so the
    // dismiss key closes it before it closes anything else.
    if state
        .config
        .keys
        .get(Action::Dismiss)
        .matches(key.0 as u32, control, shift)
        && state.tone_picker.is_some()
    {
        state.tone_picker = None;
        unsafe {
            let _ = InvalidateRect(Some(state.hwnd), None, false);
        }
        return true;
    }
    if let Some(action) = state.config.keys.action_for(key.0 as u32, control, shift) {
        return unsafe { run_action(state, action) };
    }
    // The numeric keypad's own plus and minus follow the text size binding
    // whenever that binding is on the main row's plus or minus.
    if control && matches!(key.0, 0x6b | 0x6d) {
        let action = if key.0 == 0x6b {
            Action::TextBigger
        } else {
            Action::TextSmaller
        };
        return unsafe { run_action(state, action) };
    }
    if key == VK_TAB {
        return true;
    }
    // Arrows are the primitive the rebindable motions are shorthand for, so
    // they always move the selection.
    let arrow = match key {
        VK_UP => Some(Action::SelectUp),
        VK_DOWN => Some(Action::SelectDown),
        VK_LEFT => Some(Action::SelectLeft),
        VK_RIGHT => Some(Action::SelectRight),
        _ => None,
    };
    if let Some(action) = arrow {
        return unsafe { run_action(state, action) };
    }
    false
}

/// Run one bound action. Motion means different things in the two views: the
/// grid moves by a row of cells, the result list by a row of text.
unsafe fn run_action(state: &mut AppState, action: Action) -> bool {
    let browsing = state.browsing();
    let columns = state
        .section_layouts()
        .get(state.browse_focus.0)
        .map_or(1, |layout| layout.columns) as isize;
    unsafe {
        match action {
            Action::Insert => commit_selection(state, true),
            Action::InsertKeep => commit_selection(state, false),
            Action::Copy => copy_selection(state, true),
            Action::CopyKeep => copy_selection(state, false),
            Action::Dismiss => hide_picker(state),
            Action::Settings => enter_settings(state),
            Action::Browse => focus_browser(state),
            Action::TextBigger => adjust_font_scale(state, 1),
            Action::TextSmaller => adjust_font_scale(state, -1),
            Action::SelectUp => {
                if browsing {
                    state.move_browse_selection(-columns);
                } else {
                    state.move_selection(-1);
                }
            }
            Action::SelectDown => {
                if browsing {
                    state.move_browse_selection(columns);
                } else {
                    state.move_selection(1);
                }
            }
            Action::SelectLeft => {
                if browsing {
                    state.move_browse_selection(-1);
                }
            }
            Action::SelectRight => {
                if browsing {
                    state.move_browse_selection(1);
                }
            }
            Action::HalfPageUp | Action::HalfPageDown => {
                let back = action == Action::HalfPageUp;
                if browsing {
                    let viewport = (state.footer_top() - BROWSE_CONTENT_TOP) as f32;
                    let cell = state
                        .section_layouts()
                        .get(state.browse_focus.0)
                        .map_or(state.grid_cell(), |layout| layout.cell_height)
                        .max(1);
                    let rows = ((viewport * 0.5) / cell as f32).max(1.0) as isize;
                    let jump = rows * columns;
                    state.move_browse_selection(if back { -jump } else { jump });
                } else {
                    let rows = ((state.result_viewport() * 0.5) / state.row_height() as f32)
                        .max(1.0) as isize;
                    state.move_selection(if back { -rows } else { rows });
                }
            }
            Action::PageUp | Action::PageDown => {
                let back = action == Action::PageUp;
                if browsing {
                    let viewport = (state.footer_top() - BROWSE_CONTENT_TOP) as f32;
                    state.scroll_browse(if back {
                        -viewport * 0.88
                    } else {
                        viewport * 0.88
                    });
                } else {
                    let step = state.result_viewport() * 0.88;
                    state.set_result_scroll_immediate(if back {
                        state.result_scroll - step
                    } else {
                        state.result_scroll + step
                    });
                }
            }
        }
    }
    true
}

unsafe fn handle_settings_key(state: &mut AppState, key: VIRTUAL_KEY, control: bool) -> bool {
    if state.capturing_shortcut {
        if key == VK_ESCAPE {
            set_capturing_shortcut(state, false);
            state.status = None;
            unsafe {
                let _ = InvalidateRect(Some(state.hwnd), None, false);
            }
            return true;
        }
        if matches!(key, VK_CONTROL | VK_SHIFT | VK_MENU | VK_LWIN | VK_RWIN) {
            return true;
        }
        if let Some(action) = state.capturing_action {
            unsafe {
                apply_captured_binding(state, action, key);
            }
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
                set_capturing_shortcut(state, false);
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
        state.ensure_selected_setting_visible();
        unsafe {
            state.sync_accessible_results();
            let _ = InvalidateRect(Some(state.hwnd), None, false);
        }
        return true;
    }
    if key == VK_DOWN || (control && key.0 == VK_J_VALUE) || key == VK_TAB {
        state.settings_selected = (state.settings_selected + 1).min(SETTINGS_ROWS - 1);
        state.selected = state.settings_selected;
        state.ensure_selected_setting_visible();
        unsafe {
            state.sync_accessible_results();
            let _ = InvalidateRect(Some(state.hwnd), None, false);
        }
        return true;
    }
    if key == VK_PRIOR || key == VK_NEXT {
        let rows = ((state.settings_viewport() / SETTINGS_ROW_HEIGHT as f32) as usize).max(1);
        state.settings_selected = if key == VK_PRIOR {
            state.settings_selected.saturating_sub(rows)
        } else {
            (state.settings_selected + rows).min(SETTINGS_ROWS - 1)
        };
        state.selected = state.settings_selected;
        state.ensure_selected_setting_visible();
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
    if key.0 == 0x20 && state.settings_selected == SETTINGS_ROWS - 1 {
        set_capturing_shortcut(state, true);
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
    let Some(index) = state.selected_entry_index() else {
        return;
    };
    let base = catalog::entries()[index].glyph.clone();
    let text = catalog::toned(&base, state.config.skin_tone)
        .map(str::to_owned)
        .unwrap_or_else(|| base.clone());
    unsafe {
        commit_text(state, text, base, close_after);
    }
}

/// Put the selection on the clipboard rather than typing it into the target
/// window. This is the path that still works where injection cannot reach:
/// an elevated application, or a control that ignores KEYEVENTF_UNICODE.
unsafe fn copy_selection(state: &mut AppState, close_after: bool) {
    let Some(index) = state.selected_entry_index() else {
        return;
    };
    let base = catalog::entries()[index].glyph.clone();
    let text = catalog::toned(&base, state.config.skin_tone)
        .map(str::to_owned)
        .unwrap_or_else(|| base.clone());
    if let Err(error) = unsafe { set_clipboard_text(state.hwnd, &text) } {
        eprintln!("winmoji: clipboard write failed: {error}");
        state.status = Some("Could not write to the clipboard.".to_string());
        unsafe {
            let _ = InvalidateRect(Some(state.hwnd), None, false);
        }
        return;
    }
    state.record_use(&base);
    if close_after {
        unsafe {
            hide_picker(state);
        }
        state.rebuild_browse_sections();
        return;
    }
    state.status = Some(format!("Copied {text}"));
    state.rebuild_browse_sections_preserving_view();
    unsafe {
        state.sync_accessible_results();
        let _ = InvalidateRect(Some(state.hwnd), None, false);
    }
}

/// Replace the clipboard contents with `text`. The clipboard takes ownership
/// of the moveable global on a successful SetClipboardData, so the handle is
/// only freed on the paths that never hand it over.
unsafe fn set_clipboard_text(hwnd: HWND, text: &str) -> Result<()> {
    const CF_UNICODETEXT: u32 = 13;
    let utf16: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        OpenClipboard(Some(hwnd))?;
        let result = (|| -> Result<()> {
            EmptyClipboard()?;
            let bytes = std::mem::size_of_val(utf16.as_slice());
            let global = GlobalAlloc(GMEM_MOVEABLE, bytes)?;
            let pointer = GlobalLock(global) as *mut u16;
            if pointer.is_null() {
                let _ = GlobalFree(Some(global));
                return Err(Error::new(
                    HRESULT(0x8007000Eu32 as i32),
                    "could not lock the clipboard buffer",
                ));
            }
            std::ptr::copy_nonoverlapping(utf16.as_ptr(), pointer, utf16.len());
            let _ = GlobalUnlock(global);
            if let Err(error) = SetClipboardData(CF_UNICODETEXT, Some(HANDLE(global.0))) {
                let _ = GlobalFree(Some(global));
                return Err(error);
            }
            Ok(())
        })();
        let _ = CloseClipboard();
        result
    }
}

/// Insert `text` into the captured target; `recent_glyph` is the catalog
/// entry recorded in recents (the base glyph, so the Recent grid always
/// shows catalog entries and follows the configured tone).
unsafe fn commit_text(state: &mut AppState, text: String, recent_glyph: String, close_after: bool) {
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
            state.record_use(&recent_glyph);
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
        raise_picker_window(state.hwnd);
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
    state.clamp_settings_scroll();
    state.clamp_shortcut_scroll();
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
    let notches = ((wparam.0 >> 16) as u16 as i16 as f32) / 120.0;
    if state.view == View::Shortcuts {
        unsafe {
            state.set_shortcut_scroll_immediate(state.shortcut_scroll - notches * WHEEL_NOTCH_DIPS);
        }
        return;
    }
    if state.view == View::Settings {
        unsafe {
            state.set_settings_scroll_immediate(state.settings_scroll - notches * WHEEL_NOTCH_DIPS);
        }
        return;
    }
    if state.view != View::Search {
        return;
    }
    if !state.browsing() {
        // The result list is a plain row list with no category rail, so the
        // wheel always means the list, in either axis.
        unsafe {
            state.set_result_scroll_immediate(state.result_scroll - notches * WHEEL_NOTCH_DIPS);
        }
        return;
    }
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
        // Direct manipulation: the content tracks the wheel 1:1 with no
        // animation between input and pixels.
        unsafe {
            state.set_browse_scroll_immediate(state.browse_scroll - notches * WHEEL_NOTCH_DIPS);
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

/// The footer actions, left to right: the Shift cap, copy, insert. Both
/// buttons show whichever action the current Shift state would run, so the
/// cap and the labels always agree.
fn footer_button_rects(width: i32, footer_top: i32) -> (D2D_RECT_F, D2D_RECT_F, D2D_RECT_F) {
    let top = footer_top as f32 + 8.0;
    let bottom = footer_top as f32 + 34.0;
    let insert_right = width as f32 - 26.0;
    let insert_left = insert_right - 108.0;
    let copy_right = insert_left - 8.0;
    let copy_left = copy_right - 96.0;
    let cap_right = copy_left - 8.0;
    let cap_left = cap_right - 26.0;
    (
        rect(cap_left, top, cap_right, bottom),
        rect(copy_left, top, copy_right, bottom),
        rect(insert_left, top, insert_right, bottom),
    )
}

/// Track and thumb for whichever list the current view is showing. `None`
/// means the content fits, so nothing is drawn or hit-tested.
fn list_scrollbar_rects(state: &AppState) -> Option<(D2D_RECT_F, D2D_RECT_F)> {
    if state.view == View::Shortcuts {
        let viewport = state.shortcut_viewport();
        return scrollbar_rects(
            state,
            SHORTCUT_LIST_TOP,
            viewport,
            state.total_shortcut_height().max(viewport),
            state.shortcut_scroll,
            state.maximum_shortcut_scroll(),
        );
    }
    if state.view == View::Settings {
        let viewport = state.settings_viewport();
        return scrollbar_rects(
            state,
            SETTINGS_LIST_TOP,
            viewport,
            state.total_settings_height().max(viewport),
            state.settings_scroll,
            state.maximum_settings_scroll(),
        );
    }
    let content_top = state.list_content_top();
    let viewport = (state.footer_top() - content_top).max(1) as f32;
    let (total, scroll, maximum) = if state.browsing() {
        (
            state.total_browse_height().max(viewport as i32) as f32,
            state.browse_scroll,
            state.maximum_browse_scroll(),
        )
    } else {
        (
            state.total_result_height().max(viewport),
            state.result_scroll,
            state.maximum_result_scroll(),
        )
    };
    if total <= viewport {
        return None;
    }
    scrollbar_rects(state, content_top, viewport, total, scroll, maximum)
}

fn scrollbar_rects(
    state: &AppState,
    content_top: i32,
    viewport: f32,
    total: f32,
    scroll: f32,
    maximum: f32,
) -> Option<(D2D_RECT_F, D2D_RECT_F)> {
    if total <= viewport {
        return None;
    }
    let width = state.dimensions().0;
    let track = rect(
        width as f32 - 13.0,
        content_top as f32 + 4.0,
        width as f32 - 2.0,
        state.footer_top() as f32 - 4.0,
    );
    let track_height = track.bottom - track.top;
    let thumb_height = (track_height * viewport / total).max(24.0);
    let thumb_top = track.top + (track_height - thumb_height) * scroll / maximum.max(1.0);
    // The grip widens toward the left so its right edge stays put; a handle
    // that moved under the cursor as it grew would be harder to grab, not
    // easier.
    let grip = SCROLLBAR_THUMB_WIDTH + ease_in_out(state.scrollbar_grip) * SCROLLBAR_GRIP_GROWTH;
    Some((
        track,
        rect(
            width as f32 - 3.0 - grip,
            thumb_top,
            width as f32 - 3.0,
            thumb_top + thumb_height,
        ),
    ))
}

/// Symmetric acceleration and deceleration, so the grip neither jumps at the
/// start nor arrives abruptly.
fn ease_in_out(progress: f32) -> f32 {
    let progress = progress.clamp(0.0, 1.0);
    if progress < 0.5 {
        2.0 * progress * progress
    } else {
        1.0 - (-2.0 * progress + 2.0).powi(2) / 2.0
    }
}

fn settings_row_rect(width: i32, index: usize, scroll: f32) -> D2D_RECT_F {
    let top = SETTINGS_LIST_TOP as f32 + index as f32 * SETTINGS_ROW_HEIGHT as f32 - scroll;
    rect(12.0, top, width as f32 - 12.0, top + 34.0)
}

fn slider_rect(width: i32, index: usize, scroll: f32) -> D2D_RECT_F {
    let row = settings_row_rect(width, index, scroll);
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
        // The right edge stops short of the corner so the resize grip has
        // room of its own.
        rect(
            width as f32 - 82.0,
            footer_top as f32 + 7.0,
            width as f32 - 26.0,
            footer_top as f32 + 35.0,
        ),
    )
}

unsafe fn open_tone_picker(state: &mut AppState, x: f32, y: f32) {
    if state.view != View::Search {
        return;
    }
    let entry_index = match hit_test(state, x, y) {
        Some(HitTarget::SearchResult(row)) => state.matches.get(row).map(|found| found.index),
        Some(HitTarget::BrowseItem { section, item }) => state
            .browse_sections
            .get(section)
            .and_then(|section| section.indices.get(item))
            .copied(),
        _ => None,
    };
    let Some(entry_index) = entry_index else {
        return;
    };
    if !catalog::supports_tones(&catalog::entries()[entry_index].glyph) {
        return;
    }
    state.tone_picker = Some(TonePicker {
        entry_index,
        anchor_x: x,
        anchor_y: y,
    });
    unsafe {
        let _ = InvalidateRect(Some(state.hwnd), None, false);
    }
}

const TONE_TILE: f32 = 40.0;

fn tone_picker_layout(state: &AppState, picker: &TonePicker) -> (D2D_RECT_F, [D2D_RECT_F; 6]) {
    let (width, height) = state.dimensions();
    let popup_width = TONE_TILE * 6.0 + 16.0;
    let popup_height = TONE_TILE + 34.0;
    let left = (picker.anchor_x - popup_width / 2.0).clamp(8.0, width as f32 - popup_width - 8.0);
    let top = if picker.anchor_y - popup_height - 6.0 > SEARCH_RESULTS_TOP as f32 {
        picker.anchor_y - popup_height - 6.0
    } else {
        (picker.anchor_y + 6.0).min(height as f32 - popup_height - 8.0)
    };
    let popup = rect(left, top, left + popup_width, top + popup_height);
    let mut tiles = [popup; 6];
    for (index, tile) in tiles.iter_mut().enumerate() {
        let tile_left = left + 8.0 + index as f32 * TONE_TILE;
        *tile = rect(
            tile_left,
            top + 6.0,
            tile_left + TONE_TILE,
            top + 6.0 + TONE_TILE,
        );
    }
    (popup, tiles)
}

/// The drag handle in the bottom-right corner, in layout coordinates.
fn resize_grip_rect(state: &AppState) -> D2D_RECT_F {
    let (width, height) = state.dimensions();
    rect(
        width as f32 - RESIZE_GRIP - 2.0,
        height as f32 - RESIZE_GRIP - 2.0,
        width as f32 - 2.0,
        height as f32 - 2.0,
    )
}

fn hit_test(state: &AppState, x: f32, y: f32) -> Option<HitTarget> {
    let (width, _) = state.dimensions();
    if let Some(picker) = &state.tone_picker {
        let (popup, tiles) = tone_picker_layout(state, picker);
        for (index, tile) in tiles.iter().enumerate() {
            if contains(*tile, x, y) {
                return Some(HitTarget::ToneOption(index));
            }
        }
        if contains(popup, x, y) {
            return Some(HitTarget::TonePopup);
        }
    }
    if contains(header_button_rect(width, 0), x, y) {
        return Some(HitTarget::Close);
    }
    if state.view == View::Shortcuts {
        if list_scrollbar_rects(state).is_some_and(|(track, _)| contains(track, x, y)) {
            return Some(HitTarget::Scrollbar);
        }
        let (reset, _, back) = settings_footer_rects(width, state.footer_top());
        if contains(reset, x, y) {
            return Some(HitTarget::ShortcutsReset);
        }
        if contains(back, x, y) {
            return Some(HitTarget::ShortcutsBack);
        }
        if y >= SHORTCUT_LIST_TOP as f32 && y < state.footer_top() as f32 {
            let content_y = y - SHORTCUT_LIST_TOP as f32 + state.shortcut_scroll;
            let index = (content_y / SHORTCUT_ROW_HEIGHT as f32) as usize;
            if index < Action::ALL.len() {
                return Some(HitTarget::ShortcutRow(index));
            }
        }
        return contains(resize_grip_rect(state), x, y).then_some(HitTarget::ResizeGrip);
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
        if list_scrollbar_rects(state).is_some_and(|(track, _)| contains(track, x, y)) {
            return Some(HitTarget::Scrollbar);
        }
        if y >= SETTINGS_LIST_TOP as f32 && y < state.footer_top() as f32 {
            for index in 0..SETTINGS_ROWS {
                if contains(settings_row_rect(width, index, state.settings_scroll), x, y) {
                    if slider_bounds(state.config, index).is_some()
                        && contains(slider_rect(width, index, state.settings_scroll), x, y)
                    {
                        return Some(HitTarget::SettingSlider(index));
                    }
                    return Some(HitTarget::SettingRow(index));
                }
            }
        }
        let (discard, reset, back) = settings_footer_rects(width, state.footer_top());
        if contains(discard, x, y) {
            return Some(HitTarget::SettingsDiscard);
        }
        if contains(reset, x, y) {
            return Some(HitTarget::SettingsReset);
        }
        if contains(back, x, y) {
            return Some(HitTarget::SettingsBack);
        }
        return contains(resize_grip_rect(state), x, y).then_some(HitTarget::ResizeGrip);
    }
    let (cap, copy, insert) = footer_button_rects(width, state.footer_top());
    if contains(cap, x, y) {
        return Some(HitTarget::ShiftCap);
    }
    if contains(copy, x, y) {
        return Some(HitTarget::Copy);
    }
    if contains(insert, x, y) {
        return Some(HitTarget::Insert);
    }
    if contains(resize_grip_rect(state), x, y) {
        return Some(HitTarget::ResizeGrip);
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
        if list_scrollbar_rects(state).is_some_and(|(track, _)| contains(track, x, y)) {
            return Some(HitTarget::Scrollbar);
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
        let content_y = y - SEARCH_RESULTS_TOP as f32 + state.result_scroll;
        let row = (content_y / RESULT_ROW_HEIGHT as f32) as usize;
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

/// Resize the window to follow the cursor while the corner grip is dragged.
/// The picker draws its own frame, so sizing is done here rather than by
/// handing the system a resize border it would draw over.
unsafe fn update_dragged_resize(state: &mut AppState) {
    let Some((offset_x, offset_y)) = state.dragging_resize else {
        return;
    };
    let mut cursor = POINT::default();
    let mut window = RECT::default();
    if unsafe { GetCursorPos(&mut cursor) }.is_err()
        || unsafe { GetWindowRect(state.hwnd, &mut window) }.is_err()
    {
        return;
    }
    let width = ((cursor.x + offset_x - window.left) * 96 / state.dpi as i32)
        .clamp(MIN_PICKER_WIDTH, MAX_PICKER_WIDTH);
    let height = ((cursor.y + offset_y - window.top) * 96 / state.dpi as i32)
        .clamp(MIN_PICKER_HEIGHT, MAX_PICKER_HEIGHT);
    if (width, height) == state.dimensions() {
        return;
    }
    state.config.dimensions.width = width;
    state.config.dimensions.height = height;
    state.display_dimensions = state.config.dimensions;
    unsafe {
        SetWindowPos(
            state.hwnd,
            None,
            0,
            0,
            scale(width, state.dpi),
            scale(height, state.dpi),
            SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .ok();
        layout(state);
    }
    state.clamp_browse_scroll();
    state.clamp_settings_scroll();
    state.clamp_shortcut_scroll();
    state.needs_render = true;
}

unsafe fn update_dragged_slider(state: &mut AppState, x: f32) {
    let Some(index) = state.dragging_slider else {
        return;
    };
    let Some((_, minimum, maximum)) = slider_bounds(state.config, index) else {
        return;
    };
    let track = slider_rect(state.dimensions().0, index, state.settings_scroll);
    let ratio = ((x - track.left) / (track.right - track.left)).clamp(0.0, 1.0);
    let value = minimum + ((maximum - minimum) as f32 * ratio) as i32;
    match index {
        0 => state.config.dimensions.width = value,
        1 => state.config.dimensions.height = value,
        2 => {
            // The text size takes effect as it is dragged; width and height
            // wait for the drag to end because they resize the window.
            let steps = (value - state.config.font_scale) / FONT_SCALE_STEP;
            unsafe {
                adjust_font_scale(state, steps);
            }
        }
        _ => return,
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
    let Some((track, thumb)) = list_scrollbar_rects(state) else {
        return;
    };
    let thumb_height = thumb.bottom - thumb.top;
    let available = (track.bottom - track.top - thumb_height).max(1.0);
    let thumb_top = (y - offset).clamp(track.top, track.bottom - thumb_height);
    let ratio = (thumb_top - track.top) / available;
    unsafe {
        if state.view == View::Shortcuts {
            state.set_shortcut_scroll_immediate(ratio * state.maximum_shortcut_scroll());
        } else if state.view == View::Settings {
            state.set_settings_scroll_immediate(ratio * state.maximum_settings_scroll());
        } else if state.browsing() {
            state.set_browse_scroll_immediate(ratio * state.maximum_browse_scroll());
        } else {
            state.set_result_scroll_immediate(ratio * state.maximum_result_scroll());
        }
    }
}

unsafe fn handle_click(state: &mut AppState, x: f32, y: f32) {
    let target = hit_test(state, x, y);
    if state.tone_picker.is_some()
        && !matches!(
            target,
            Some(HitTarget::ToneOption(_)) | Some(HitTarget::TonePopup)
        )
    {
        state.tone_picker = None;
        unsafe {
            let _ = InvalidateRect(Some(state.hwnd), None, false);
        }
        return;
    }
    let Some(target) = target else {
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
        HitTarget::Scrollbar => {
            if let Some((_, thumb)) = list_scrollbar_rects(state) {
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
        HitTarget::ShiftCap => {
            state.shift_latched = !state.shift_latched;
            unsafe {
                let _ = InvalidateRect(Some(state.hwnd), None, false);
            }
        }
        HitTarget::Copy => unsafe { copy_selection(state, !state.keep_open()) },
        HitTarget::Insert => unsafe { commit_selection(state, !state.keep_open()) },
        HitTarget::TonePopup => {}
        HitTarget::ToneOption(index) => {
            if let Some(picker) = state.tone_picker.take() {
                let base = catalog::entries()[picker.entry_index].glyph.clone();
                let tone = SkinTone::ALL[index.min(SkinTone::ALL.len() - 1)];
                let text = catalog::toned(&base, tone)
                    .map(str::to_owned)
                    .unwrap_or_else(|| base.clone());
                unsafe {
                    commit_text(state, text, base, false);
                }
            }
        }
        HitTarget::SettingRow(index) => {
            state.settings_selected = index;
            state.selected = index;
            if setting_is_action(index) {
                unsafe {
                    activate_setting(state);
                }
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
        HitTarget::SettingSlider(index) => {
            state.dragging_slider = Some(index);
            unsafe {
                let _ = SetCapture(state.hwnd);
                update_dragged_slider(state, x);
            }
        }
        HitTarget::ShortcutRow(index) => {
            state.shortcut_selected = index.min(Action::ALL.len() - 1);
            let action = Action::ALL[state.shortcut_selected];
            unsafe {
                begin_capture(state, action);
            }
        }
        HitTarget::ShortcutsReset => unsafe { reset_shortcuts(state) },
        HitTarget::ShortcutsBack => unsafe { leave_shortcuts(state) },
        HitTarget::ResizeGrip => {
            let mut cursor = POINT::default();
            let mut window = RECT::default();
            if unsafe { GetCursorPos(&mut cursor) }.is_ok()
                && unsafe { GetWindowRect(state.hwnd, &mut window) }.is_ok()
            {
                state.dragging_resize = Some((window.right - cursor.x, window.bottom - cursor.y));
                unsafe {
                    let _ = SetCapture(state.hwnd);
                }
            }
        }
        HitTarget::SettingsDiscard => unsafe { discard_settings(state) },
        HitTarget::SettingsReset => unsafe { reset_settings(state) },
        HitTarget::SettingsBack => unsafe { save_settings(state) },
    }
}

/// Validate the update region and mark the frame dirty. Rendering itself
/// happens once per display refresh in the message loop: presenting straight
/// from here would put one present on the queue per input event, which the
/// driver has to buffer.
unsafe fn paint(state: &mut AppState) {
    let mut paint = PAINTSTRUCT::default();
    unsafe {
        BeginPaint(state.hwnd, &mut paint);
        let _ = EndPaint(state.hwnd, &paint);
    }
    state.needs_render = true;
}

/// Wait for the compositor to accept a frame, then draw and present one.
/// Every present in the process goes through here, so the frame-latency
/// waitable object is always honoured.
unsafe fn render_frame(state: &mut AppState) {
    if unsafe { ensure_render_target(state) }.is_err() {
        state.needs_render = false;
        return;
    }
    if let Some(resources) = &state.render {
        unsafe {
            let _ = WaitForSingleObject(resources.frame_gate.0, 33);
        }
    }
    state.needs_render = false;
    unsafe {
        render_and_present(state);
    }
}

/// Build the device stack: D3D11 device, DXGI flip-model swap chain with a
/// frame-latency waitable object, and a D2D device context targeting the
/// back buffer. Flip-model presentation hands frames to the compositor
/// without a copy, and the waitable object lets the animation loop pace
/// itself to the monitor refresh rate instead of a coarse timer.
unsafe fn ensure_render_target(state: &mut AppState) -> Result<RenderResources> {
    if let Some(resources) = &state.render {
        return Ok(resources.clone());
    }
    let mut client = RECT::default();
    unsafe {
        GetClientRect(state.hwnd, &mut client)?;
    }

    let mut device: Option<ID3D11Device> = None;
    let hardware = unsafe {
        D3D11CreateDevice(
            None,
            D3D_DRIVER_TYPE_HARDWARE,
            HMODULE::default(),
            D3D11_CREATE_DEVICE_BGRA_SUPPORT,
            None,
            D3D11_SDK_VERSION,
            Some(&mut device),
            None,
            None,
        )
    };
    if hardware.is_err() {
        device = None;
        unsafe {
            D3D11CreateDevice(
                None,
                D3D_DRIVER_TYPE_WARP,
                HMODULE::default(),
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                None,
                D3D11_SDK_VERSION,
                Some(&mut device),
                None,
                None,
            )?;
        }
    }
    let device = device.ok_or_else(|| {
        Error::new(
            HRESULT(0x80004005u32 as i32),
            "D3D11CreateDevice returned no device",
        )
    })?;
    let dxgi_device: IDXGIDevice = device.cast()?;
    let d2d_device = unsafe { state.d2d_factory.CreateDevice(&dxgi_device)? };
    let context = unsafe { d2d_device.CreateDeviceContext(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)? };
    unsafe {
        context.SetDpi(state.dpi as f32, state.dpi as f32);
    }

    let adapter = unsafe { dxgi_device.GetAdapter()? };
    let dxgi_factory: IDXGIFactory2 = unsafe { adapter.GetParent()? };
    let descriptor = DXGI_SWAP_CHAIN_DESC1 {
        Width: client.right.max(1) as u32,
        Height: client.bottom.max(1) as u32,
        Format: DXGI_FORMAT_B8G8R8A8_UNORM,
        SampleDesc: DXGI_SAMPLE_DESC {
            Count: 1,
            Quality: 0,
        },
        BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
        BufferCount: 2,
        Scaling: DXGI_SCALING_NONE,
        SwapEffect: DXGI_SWAP_EFFECT_FLIP_DISCARD,
        AlphaMode: DXGI_ALPHA_MODE_IGNORE,
        Flags: DXGI_SWAP_CHAIN_FLAG_FRAME_LATENCY_WAITABLE_OBJECT.0 as u32,
        ..Default::default()
    };
    let swapchain: IDXGISwapChain2 = unsafe {
        dxgi_factory
            .CreateSwapChainForHwnd(&device, state.hwnd, &descriptor, None, None)?
            .cast()?
    };
    unsafe {
        // Alt chords pass through the keyboard hook to the system; keep DXGI
        // from claiming Alt+Enter for a fullscreen toggle.
        let _ = dxgi_factory.MakeWindowAssociation(
            state.hwnd,
            DXGI_MWA_NO_ALT_ENTER | DXGI_MWA_NO_WINDOW_CHANGES,
        );
        swapchain.SetMaximumFrameLatency(1)?;
    }
    let frame_gate = Rc::new(FrameLatencyGate(unsafe {
        swapchain.GetFrameLatencyWaitableObject()
    }));
    unsafe {
        attach_swapchain_target(&context, &swapchain, state.dpi)?;
    }
    let target: ID2D1RenderTarget = context.cast()?;
    let brushes = unsafe { create_brushes(&target)? };
    let resources = RenderResources {
        target,
        context,
        device: d2d_device,
        dxgi_device,
        swapchain,
        frame_gate,
        brushes,
        glyphs: Rc::new(RefCell::new(HashMap::new())),
        atlas: Rc::new(RefCell::new(Vec::new())),
        wanted: Rc::new(RefCell::new(Vec::new())),
    };
    state.render = Some(resources.clone());
    Ok(resources)
}

/// Point the device context at the swap chain's current back buffer.
unsafe fn attach_swapchain_target(
    context: &ID2D1DeviceContext,
    swapchain: &IDXGISwapChain2,
    dpi: u32,
) -> Result<()> {
    let surface: IDXGISurface = unsafe { swapchain.GetBuffer(0)? };
    let properties = D2D1_BITMAP_PROPERTIES1 {
        pixelFormat: D2D1_PIXEL_FORMAT {
            format: DXGI_FORMAT_B8G8R8A8_UNORM,
            alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
        },
        dpiX: dpi as f32,
        dpiY: dpi as f32,
        bitmapOptions: D2D1_BITMAP_OPTIONS_TARGET | D2D1_BITMAP_OPTIONS_CANNOT_DRAW,
        ..Default::default()
    };
    let bitmap = unsafe { context.CreateBitmapFromDxgiSurface(&surface, Some(&properties))? };
    unsafe {
        context.SetTarget(&bitmap);
    }
    Ok(())
}

/// Resize the swap chain to the current client area, keeping the device and
/// the glyph cache alive; only the back-buffer binding is rebuilt.
unsafe fn resize_swapchain(state: &mut AppState) {
    let Some(resources) = state.render.clone() else {
        return;
    };
    let mut client = RECT::default();
    if unsafe { GetClientRect(state.hwnd, &mut client) }.is_err() {
        return;
    }
    unsafe {
        resources.context.SetTarget(None);
        if resources
            .swapchain
            .ResizeBuffers(
                0,
                client.right.max(1) as u32,
                client.bottom.max(1) as u32,
                DXGI_FORMAT_UNKNOWN,
                DXGI_SWAP_CHAIN_FLAG_FRAME_LATENCY_WAITABLE_OBJECT,
            )
            .is_err()
            || attach_swapchain_target(&resources.context, &resources.swapchain, state.dpi).is_err()
        {
            state.render = None;
            return;
        }
        // The device may have been built while the window was still hidden
        // at the default DPI; without this the layout is drawn at the wrong
        // scale and leaves the frame partly empty.
        resources.context.SetDpi(state.dpi as f32, state.dpi as f32);
    }
}

/// Draw the current view and hand the frame to the compositor. Present(1)
/// queues at most one frame (SetMaximumFrameLatency), so this never blocks
/// the thread for more than a frame even without the waitable gate.
unsafe fn render_and_present(state: &mut AppState) {
    if let Err(error) = unsafe { draw_picker(state) } {
        state.render = None;
        eprintln!("winmoji: rendering failed: {error}");
        return;
    }
    if let Some(resources) = &state.render {
        let presented = unsafe { resources.swapchain.Present(1, DXGI_PRESENT(0)) };
        if presented.is_err() {
            // Device removed or reset: rebuild the stack on the next frame.
            state.render = None;
            unsafe {
                let _ = InvalidateRect(Some(state.hwnd), None, false);
            }
        }
    }
}

/// One vsync-paced animation frame: wait until the compositor can take a new
/// frame, advance the scroll by the elapsed wall-clock time, and render. The
/// wait is bounded so a stalled compositor never wedges the loop.
unsafe fn render_animation_frame(state: &mut AppState) {
    if unsafe { ensure_render_target(state) }.is_err() {
        // No device: snap to the destination instead of spinning.
        state.browse_scroll = state.browse_scroll_target;
        state.last_frame = None;
        return;
    }
    let now = Instant::now();
    let dt = state.last_frame.map_or(1.0 / 120.0, |last| {
        now.duration_since(last).as_secs_f32().clamp(0.001, 0.05)
    });
    state.last_frame = Some(now);
    unsafe {
        state.tick_browse_scroll(dt);
        state.tick_scrollbar_grip(dt);
        render_frame(state);
    }
}

fn glyph_key(state: &AppState, entry_index: usize) -> (usize, u8) {
    let entry = &catalog::entries()[entry_index];
    let toned = catalog::toned(&entry.glyph, state.config.skin_tone);
    (
        entry_index,
        toned.map_or(0, |_| state.config.skin_tone.index() as u8),
    )
}

/// Look up an already-rasterized glyph. Drawing never rasterizes: a color
/// emoji costs several milliseconds to raster, and doing that on the way to
/// the screen is what made scrolling stall behind the wheel. A miss is
/// recorded for the idle warmer and the tile stays empty for this frame.
fn glyph_slot(
    state: &AppState,
    resources: &RenderResources,
    entry_index: usize,
) -> Option<GlyphSlot> {
    let key = glyph_key(state, entry_index);
    if let Some(cached) = resources.glyphs.borrow().get(&key) {
        return *cached;
    }
    resources.wanted.borrow_mut().push(entry_index);
    None
}

/// Rasterize a batch of glyphs into atlas tiles. Everything landing on one
/// page shares a single BeginDraw/EndDraw pair, so the GPU round trip that
/// dominates the cost is paid once per batch rather than once per glyph.
/// Only ever called from the idle warmer.
unsafe fn rasterize_batch(state: &AppState, resources: &RenderResources, batch: &[usize]) {
    let mut atlas = resources.atlas.borrow_mut();
    let mut position = 0;
    while position < batch.len() {
        if atlas.last().is_none_or(|page| page.used >= ATLAS_SLOTS) {
            match unsafe { create_atlas_page(&resources.target) } {
                Ok(page) => atlas.push(page),
                Err(_) => return,
            }
        }
        let page_index = atlas.len() - 1;
        let page = &mut atlas[page_index];
        unsafe {
            page.target.BeginDraw();
        }
        while position < batch.len() && page.used < ATLAS_SLOTS {
            let entry_index = batch[position];
            position += 1;
            let entry = &catalog::entries()[entry_index];
            let glyph =
                catalog::toned(&entry.glyph, state.config.skin_tone).unwrap_or(&entry.glyph);
            let source = atlas_slot_rect(page.used);
            page.used += 1;
            unsafe {
                // Clip so a glyph that overflows its advance cannot bleed
                // into the neighbouring tiles.
                page.target
                    .PushAxisAlignedClip(&source, D2D1_ANTIALIAS_MODE_ALIASED);
                draw_text(
                    &page.target,
                    glyph,
                    &state.formats.glyph,
                    source,
                    &resources.brushes.primary,
                    D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT,
                );
                page.target.PopAxisAlignedClip();
            }
            resources.glyphs.borrow_mut().insert(
                glyph_key(state, entry_index),
                Some(GlyphSlot {
                    page: page_index,
                    source,
                }),
            );
        }
        let _ = unsafe { page.target.EndDraw(None, None) };
    }
}

/// True when a message is already waiting, so warming can yield instantly.
unsafe fn input_pending() -> bool {
    let mut message = MSG::default();
    unsafe { PeekMessageW(&mut message, None, 0, 0, PM_NOREMOVE) }.as_bool()
}

/// Glyphs worth rasterizing now: whatever the last frame could not draw,
/// then the prefetch window around the current scroll position.
fn cold_glyphs(state: &AppState, resources: &RenderResources) -> Vec<usize> {
    let entries = catalog::entries();
    let cache = resources.glyphs.borrow();
    let mut seen = HashMap::new();
    resources
        .wanted
        .borrow_mut()
        .drain(..)
        .chain(state.prefetch_entries())
        .filter(|entry_index| {
            entries[*entry_index].kind == "Emoji"
                && state.displayable_entries[*entry_index]
                && !cache.contains_key(&glyph_key(state, *entry_index))
                && seen.insert(*entry_index, ()).is_none()
        })
        .collect()
}

/// What the idle warmer wants the message loop to do next.
enum WarmOutcome {
    /// Rasterized at least one glyph; come back as soon as input allows.
    Worked,
    /// Nothing left to rasterize; the loop may sleep until input arrives.
    Done,
}

/// Rasterize a bounded slice of missing glyphs during idle time. The slice
/// is capped by wall clock and abandoned the instant a message arrives, so
/// rasterization can never delay input by more than a single glyph.
unsafe fn warm_glyph_slice(state: &mut AppState) -> WarmOutcome {
    if state.config.emoji_font != EmojiFont::SegoeEmoji {
        return WarmOutcome::Done;
    }
    let visible = unsafe { IsWindowVisible(state.hwnd) }.as_bool();
    let Ok(resources) = (unsafe { ensure_render_target(state) }) else {
        return WarmOutcome::Done;
    };
    let slice = Duration::from_millis(if visible {
        WARM_SLICE_VISIBLE_MS
    } else {
        WARM_SLICE_HIDDEN_MS
    });
    let started = Instant::now();
    let mut warmed = 0usize;
    let cold = cold_glyphs(state, &resources);
    for batch in cold.chunks(ATLAS_BATCH) {
        if started.elapsed() >= slice || unsafe { input_pending() } {
            break;
        }
        unsafe {
            rasterize_batch(state, &resources, batch);
        }
        warmed += batch.len();
    }
    if warmed == 0 {
        return WarmOutcome::Done;
    }
    if visible {
        state.needs_render = true;
    }
    WarmOutcome::Worked
}

unsafe fn draw_picker(state: &mut AppState) -> Result<()> {
    match state.view {
        View::Search => unsafe { draw_search_picker(state) },
        View::Settings => unsafe { draw_settings_picker(state) },
        View::Shortcuts => unsafe { draw_shortcuts_picker(state) },
    }
}

/// Every action and the chord that runs it, one row each. The list scrolls
/// because it is longer than the shortest allowed window.
unsafe fn draw_shortcuts_picker(state: &mut AppState) -> Result<()> {
    let resources = unsafe { ensure_render_target(state)? };
    let target = resources.target.clone();
    let brushes = resources.brushes.clone();
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
            "Keyboard shortcuts",
            &state.formats.label,
            rect(18.0, 7.0, 240.0, 34.0),
            &brushes.primary,
            D2D1_DRAW_TEXT_OPTIONS_NONE,
        );
        draw_header_button(
            &target,
            state,
            width,
            0,
            "×",
            &brushes.selection,
            &brushes.secondary,
        );
        target.PushAxisAlignedClip(
            &rect(0.0, SHORTCUT_LIST_TOP as f32, width as f32, footer_top),
            D2D1_ANTIALIAS_MODE_ALIASED,
        );
    }
    for (index, action) in Action::ALL.iter().enumerate() {
        let row = shortcut_row_rect(width, index);
        let top = row.top - state.shortcut_scroll;
        let bottom = row.bottom - state.shortcut_scroll;
        if bottom <= SHORTCUT_LIST_TOP as f32 || top >= footer_top {
            continue;
        }
        let focused = index == state.shortcut_selected;
        let hovered =
            matches!(state.hovered_target, Some(HitTarget::ShortcutRow(row)) if row == index);
        let capturing = state.capturing_action == Some(*action);
        unsafe {
            if focused || hovered || capturing {
                let bounds = rounded_rect(row.left, top, row.right, bottom, 8.0);
                target.FillRoundedRectangle(&bounds, &brushes.selection);
                if focused || capturing {
                    target.DrawRoundedRectangle(&bounds, &brushes.selection_border, 1.0, None);
                }
            }
            draw_text(
                &target,
                action.label(),
                &state.formats.label,
                rect(24.0, top, width as f32 * 0.55, bottom),
                &brushes.primary,
                D2D1_DRAW_TEXT_OPTIONS_CLIP,
            );
            let binding = if capturing {
                "Press a shortcut…".to_string()
            } else {
                state.config.keys.get(*action).to_string()
            };
            draw_text(
                &target,
                &binding,
                &state.formats.metadata,
                rect(width as f32 * 0.55, top, width as f32 - 26.0, bottom),
                if capturing {
                    &brushes.accent
                } else {
                    &brushes.secondary
                },
                D2D1_DRAW_TEXT_OPTIONS_CLIP,
            );
        }
    }
    unsafe {
        target.PopAxisAlignedClip();
        draw_list_scrollbar(state, &resources);
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
        let (reset, _, back) = settings_footer_rects(width, state.footer_top());
        if let Some(status) = &state.status {
            draw_text(
                &target,
                status,
                &state.formats.metadata,
                rect(140.0, footer_top, width as f32 - 74.0, height as f32 - 2.0),
                if state.status_error {
                    &brushes.danger
                } else {
                    &brushes.secondary
                },
                D2D1_DRAW_TEXT_OPTIONS_CLIP,
            );
        } else {
            draw_text(
                &target,
                "Enter rebinds the focused action",
                &state.formats.center,
                rect(140.0, footer_top, width as f32 - 74.0, height as f32 - 2.0),
                &brushes.secondary,
                D2D1_DRAW_TEXT_OPTIONS_CLIP,
            );
        }
        draw_button(
            &target,
            reset,
            "Reset",
            matches!(state.hovered_target, Some(HitTarget::ShortcutsReset)),
            &brushes.selection,
            &brushes.selection_border,
            &brushes.primary,
            &state.formats.center,
        );
        draw_button(
            &target,
            back,
            "Back",
            matches!(state.hovered_target, Some(HitTarget::ShortcutsBack)),
            &brushes.selection,
            &brushes.selection_border,
            &brushes.primary,
            &state.formats.center,
        );
        draw_resize_grip(state, &target, &brushes);
        target.EndDraw(None, None)?;
    }
    Ok(())
}

unsafe fn draw_search_picker(state: &mut AppState) -> Result<()> {
    let resources = unsafe { ensure_render_target(state)? };
    resources.wanted.borrow_mut().clear();
    let target = resources.target.clone();
    let brushes = resources.brushes.clone();
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
                &resources,
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
                &resources,
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
        let (cap, copy, insert) = footer_button_rects(width, state.footer_top());
        let information = rect(14.0, footer_top, cap.left - 10.0, height as f32 - 2.0);
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
        draw_shift_cap(state, &target, cap, &brushes);
        let keep = state.keep_open();
        draw_button(
            &target,
            copy,
            if keep { "Copy + keep" } else { "Copy" },
            matches!(state.hovered_target, Some(HitTarget::Copy)),
            &brushes.surface,
            &brushes.selection_border,
            &brushes.primary,
            &state.formats.center,
        );
        draw_button(
            &target,
            insert,
            if keep { "Insert + keep" } else { "Insert" },
            matches!(state.hovered_target, Some(HitTarget::Insert)),
            &brushes.surface,
            &brushes.selection_border,
            &brushes.primary,
            &state.formats.center,
        );
        draw_resize_grip(state, &target, &brushes);
        draw_tone_picker(state, &target, &brushes);
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
    target: &ID2D1RenderTarget,
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
    resources: &RenderResources,
    border: &ID2D1SolidColorBrush,
    selection: &ID2D1SolidColorBrush,
    selection_border: &ID2D1SolidColorBrush,
    glyph_surface: &ID2D1SolidColorBrush,
    primary: &ID2D1SolidColorBrush,
    secondary: &ID2D1SolidColorBrush,
    accent: &ID2D1SolidColorBrush,
) {
    let target = &resources.target;
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
            if let Some(entry_index) = state.category_icon_entries[index] {
                draw_glyph(
                    state,
                    resources,
                    entry_index,
                    rect(
                        bounds.left + 4.0,
                        bounds.top + 3.0,
                        bounds.right - 4.0,
                        bounds.bottom - 5.0,
                    ),
                    if active { primary } else { secondary },
                );
            } else {
                let format = if *category == BrowseCategory::Emoticons {
                    &state.formats.emoticon_icon
                } else {
                    &state.formats.symbol
                };
                draw_text(
                    target,
                    category.icon(),
                    format,
                    bounds,
                    if active { primary } else { secondary },
                    D2D1_DRAW_TEXT_OPTIONS_NONE,
                );
            }
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
                draw_glyph(state, resources, entry_index, glyph_bounds, primary);
            }
        }
    }
    unsafe {
        target.PopAxisAlignedClip();
        draw_list_scrollbar(state, resources);
    }
}

/// The Shift key cap beside the footer actions. It carries no words: it is
/// lit exactly when the actions are in their keep-open form, so holding
/// Shift visibly moves it and the labels together. Clicking it latches the
/// same state for people who never find the key.
unsafe fn draw_shift_cap(
    state: &AppState,
    target: &ID2D1RenderTarget,
    bounds: D2D_RECT_F,
    brushes: &Brushes,
) {
    let active = state.keep_open();
    let hovered = matches!(state.hovered_target, Some(HitTarget::ShiftCap));
    unsafe {
        let cap = rounded_rect(
            bounds.left,
            bounds.top + 3.0,
            bounds.right,
            bounds.bottom - 3.0,
            6.0,
        );
        if active {
            target.FillRoundedRectangle(&cap, &brushes.selection);
        }
        target.DrawRoundedRectangle(
            &cap,
            if active || hovered {
                &brushes.selection_border
            } else {
                &brushes.surface_border
            },
            1.0,
            None,
        );
        draw_text(
            target,
            "⇧",
            &state.formats.icon,
            rect(bounds.left, bounds.top + 1.0, bounds.right, bounds.bottom),
            if active {
                &brushes.primary
            } else {
                &brushes.secondary
            },
            D2D1_DRAW_TEXT_OPTIONS_NONE,
        );
    }
}

/// Draw the scrollbar for whichever list is on screen. The grip carries both
/// the hover colour and the eased width, so the two read as one response.
unsafe fn draw_list_scrollbar(state: &AppState, resources: &RenderResources) {
    let Some((track, thumb)) = list_scrollbar_rects(state) else {
        return;
    };
    let (width, _) = state.dimensions();
    let brushes = &resources.brushes;
    let gripped = matches!(state.hovered_target, Some(HitTarget::Scrollbar))
        || state.dragging_scrollbar.is_some();
    unsafe {
        resources.target.FillRoundedRectangle(
            &rounded_rect(
                width as f32 - 7.0,
                track.top,
                width as f32 - 4.0,
                track.bottom,
                1.5,
            ),
            &brushes.surface,
        );
        resources.target.FillRoundedRectangle(
            &rounded_rect(
                thumb.left,
                thumb.top,
                thumb.right,
                thumb.bottom,
                (thumb.right - thumb.left) / 2.0,
            ),
            if gripped {
                &brushes.selection_border
            } else {
                &brushes.surface_border
            },
        );
    }
}

#[allow(clippy::too_many_arguments)]
unsafe fn draw_search_results(
    state: &AppState,
    resources: &RenderResources,
    selection: &ID2D1SolidColorBrush,
    selection_border: &ID2D1SolidColorBrush,
    glyph_surface: &ID2D1SolidColorBrush,
    primary: &ID2D1SolidColorBrush,
    secondary: &ID2D1SolidColorBrush,
    accent: &ID2D1SolidColorBrush,
) {
    let target = &resources.target;
    let (width, _) = state.dimensions();
    let viewport_top = SEARCH_RESULTS_TOP as f32;
    let viewport_bottom = state.footer_top() as f32;
    unsafe {
        target.PushAxisAlignedClip(
            &rect(0.0, viewport_top, width as f32, viewport_bottom),
            D2D1_ANTIALIAS_MODE_ALIASED,
        );
    }
    for (row, found) in state.matches.iter().enumerate() {
        let top = viewport_top + row as f32 * RESULT_ROW_HEIGHT as f32 - state.result_scroll;
        if top + RESULT_ROW_HEIGHT as f32 <= viewport_top {
            continue;
        }
        if top >= viewport_bottom {
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
                resources,
                found.index,
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
    unsafe {
        target.PopAxisAlignedClip();
        draw_list_scrollbar(state, resources);
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
    let resources = unsafe { ensure_render_target(state)? };
    let target = resources.target.clone();
    let brushes = resources.brushes.clone();
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
        target.PushAxisAlignedClip(
            &rect(0.0, SETTINGS_LIST_TOP as f32, width as f32, footer_top),
            D2D1_ANTIALIAS_MODE_ALIASED,
        );
    }

    let settings = [
        ("Width", format!("{} px", state.config.dimensions.width)),
        ("Height", format!("{} px", state.config.dimensions.height)),
        ("Text size", format!("{}%", state.config.font_scale)),
        ("Hover details", state.config.details.to_string()),
        (
            "Emoji font",
            match state.config.emoji_font {
                EmojiFont::SegoeEmoji => "Color emoji".to_string(),
                EmojiFont::SegoeSymbol => "Monochrome".to_string(),
            },
        ),
        ("Skin tone", state.config.skin_tone.to_string()),
        (
            "Keyboard shortcuts",
            format!("{} actions", Action::ALL.len()),
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
        let bounds = settings_row_rect(width, index, state.settings_scroll);
        if bounds.bottom <= SETTINGS_LIST_TOP as f32 || bounds.top >= footer_top {
            continue;
        }
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
            if let Some((current, minimum, maximum)) = slider_bounds(state.config, index) {
                draw_slider(
                    &target,
                    slider_rect(width, index, state.settings_scroll),
                    current,
                    minimum,
                    maximum,
                    value,
                    &selection_border,
                    &accent,
                    &primary,
                    &state.formats.metadata,
                );
            } else {
                draw_setting_value(
                    state,
                    &target,
                    &brushes,
                    index,
                    value,
                    rect(
                        width as f32 * 0.42,
                        bounds.top,
                        width as f32 - 24.0,
                        bounds.bottom,
                    ),
                );
            }
        }
    }

    let hint_top = settings_row_rect(width, SETTINGS_ROWS - 1, state.settings_scroll).bottom + 8.0;
    unsafe {
        draw_text(
            &target,
            "Arrow keys adjust. Enter changes the focused value.",
            &state.formats.center,
            rect(24.0, hint_top, width as f32 - 24.0, hint_top + 22.0),
            &secondary,
            D2D1_DRAW_TEXT_OPTIONS_CLIP,
        );
        target.PopAxisAlignedClip();
        draw_list_scrollbar(state, &resources);
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
        draw_resize_grip(state, &target, &brushes);
        target.EndDraw(None, None)?;
    }
    Ok(())
}

unsafe fn draw_header_button(
    target: &ID2D1RenderTarget,
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
    target: &ID2D1RenderTarget,
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

/// The sample entry every settings preview is drawn from.
const PREVIEW_GLYPH: &str = "👋";

/// Draw a settings value between its adjust arrows. Settings that change how
/// something looks show that thing rather than naming it: the hover line is
/// rendered as it will appear, the emoji font and skin tone are shown on a
/// sample glyph.
#[allow(clippy::too_many_arguments)]
unsafe fn draw_setting_value(
    state: &AppState,
    target: &ID2D1RenderTarget,
    brushes: &Brushes,
    index: usize,
    value: &str,
    bounds: D2D_RECT_F,
) {
    let primary = &brushes.primary;
    let secondary = &brushes.secondary;
    let selection_border = &brushes.selection_border;
    unsafe {
        draw_text(
            target,
            "‹",
            &state.formats.brand,
            rect(bounds.left, bounds.top, bounds.left + 14.0, bounds.bottom),
            secondary,
            D2D1_DRAW_TEXT_OPTIONS_NONE,
        );
        draw_text(
            target,
            "›",
            &state.formats.brand,
            rect(bounds.right - 14.0, bounds.top, bounds.right, bounds.bottom),
            secondary,
            D2D1_DRAW_TEXT_OPTIONS_NONE,
        );
    }
    let inner = rect(
        bounds.left + 16.0,
        bounds.top,
        bounds.right - 16.0,
        bounds.bottom,
    );
    match index {
        // Hover details: a hovered row drawn the way the picker draws one,
        // carrying the footer line the mode produces.
        3 => unsafe {
            let row = rounded_rect(
                inner.left,
                inner.top + 1.0,
                inner.right,
                inner.bottom - 1.0,
                7.0,
            );
            target.FillRoundedRectangle(&row, &brushes.selection);
            target.DrawRoundedRectangle(&row, selection_border, 1.0, None);
            let tile = rect(
                inner.left + 5.0,
                inner.top + 5.0,
                inner.left + 31.0,
                inner.bottom - 5.0,
            );
            target.FillRoundedRectangle(
                &rounded_rect(tile.left, tile.top, tile.right, tile.bottom, 6.0),
                &brushes.glyph_surface,
            );
            draw_text(
                target,
                PREVIEW_GLYPH,
                &state.formats.glyph_small,
                tile,
                primary,
                D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT,
            );
            draw_text(
                target,
                &preview_details_line(state),
                &state.formats.metadata,
                rect(tile.right + 7.0, inner.top, inner.right - 6.0, inner.bottom),
                secondary,
                D2D1_DRAW_TEXT_OPTIONS_CLIP,
            );
        },
        // Emoji font: the same glyphs under the chosen face.
        4 => unsafe {
            let sample = rect(inner.left, inner.top, inner.left + 74.0, inner.bottom);
            draw_text(
                target,
                "😀 ✋ 🚀",
                &state.formats.glyph_small,
                sample,
                primary,
                if state.config.emoji_font == EmojiFont::SegoeEmoji {
                    D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT
                } else {
                    D2D1_DRAW_TEXT_OPTIONS_NONE
                },
            );
            draw_text(
                target,
                value,
                &state.formats.metadata,
                rect(sample.right + 6.0, inner.top, inner.right, inner.bottom),
                secondary,
                D2D1_DRAW_TEXT_OPTIONS_CLIP,
            );
        },
        // Skin tone: every tone on one glyph, the active one ringed.
        5 => {
            let step = ((inner.right - inner.left) / SkinTone::ALL.len() as f32).min(26.0);
            let strip =
                inner.left + ((inner.right - inner.left) - step * SkinTone::ALL.len() as f32) / 2.0;
            for (position, tone) in SkinTone::ALL.iter().enumerate() {
                let left = strip + position as f32 * step;
                let cell = rect(left, inner.top + 2.0, left + step, inner.bottom - 2.0);
                if *tone == state.config.skin_tone {
                    unsafe {
                        target.DrawRoundedRectangle(
                            &rounded_rect(cell.left, cell.top, cell.right, cell.bottom, 6.0),
                            selection_border,
                            1.2,
                            None,
                        );
                    }
                }
                let toned = catalog::toned(PREVIEW_GLYPH, *tone).unwrap_or(PREVIEW_GLYPH);
                unsafe {
                    draw_text(
                        target,
                        toned,
                        &state.formats.glyph_small,
                        cell,
                        primary,
                        D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT,
                    );
                }
            }
        }
        _ => unsafe {
            draw_text(
                target,
                value,
                &state.formats.brand,
                inner,
                secondary,
                D2D1_DRAW_TEXT_OPTIONS_CLIP,
            );
        },
    }
}

/// The footer line for the sample glyph under the current detail mode.
/// The footer line for one entry under the chosen mode. The footer and the
/// settings preview both build their text here so the preview cannot drift
/// from what the picker actually shows.
fn detail_line(mode: DetailMode, name: &str, glyph: &str, kind: &str) -> String {
    match mode {
        DetailMode::None => name.to_string(),
        DetailMode::Type => format!("{name}  {kind}"),
        DetailMode::Codepoint => format!("{name}  {}", codepoints(glyph)),
        DetailMode::Both => format!("{name}  {}  {kind}", codepoints(glyph)),
    }
}

fn preview_details_line(state: &AppState) -> String {
    let entry = catalog::entries()
        .iter()
        .find(|entry| entry.glyph == PREVIEW_GLYPH);
    let name = entry.map_or("Waving Hand", |entry| entry.name.as_str());
    let kind = entry.map_or("Emoji", |entry| entry.kind);
    detail_line(state.config.details, name, PREVIEW_GLYPH, kind)
}

/// Three stacked diagonals in the bottom-right corner marking the drag
/// handle, brighter while hovered or dragging.
unsafe fn draw_resize_grip(state: &AppState, target: &ID2D1RenderTarget, brushes: &Brushes) {
    let grip = resize_grip_rect(state);
    let active = matches!(state.hovered_target, Some(HitTarget::ResizeGrip))
        || state.dragging_resize.is_some();
    let brush = if active {
        &brushes.primary
    } else {
        &brushes.surface_border
    };
    for step in 0..3 {
        let inset = 3.0 + step as f32 * 4.5;
        unsafe {
            target.DrawLine(
                Vector2 {
                    X: grip.right - inset,
                    Y: grip.bottom,
                },
                Vector2 {
                    X: grip.right,
                    Y: grip.bottom - inset,
                },
                brush,
                1.4,
                None,
            );
        }
    }
}

unsafe fn draw_tone_picker(state: &AppState, target: &ID2D1RenderTarget, brushes: &Brushes) {
    let Some(picker) = &state.tone_picker else {
        return;
    };
    let (popup, tiles) = tone_picker_layout(state, picker);
    let base = &catalog::entries()[picker.entry_index].glyph;
    unsafe {
        target.FillRoundedRectangle(
            &rounded_rect(popup.left, popup.top, popup.right, popup.bottom, 8.0),
            &brushes.surface,
        );
        target.DrawRoundedRectangle(
            &rounded_rect(popup.left, popup.top, popup.right, popup.bottom, 8.0),
            &brushes.selection_border,
            1.0,
            None,
        );
        for (index, tile) in tiles.iter().enumerate() {
            let tone = SkinTone::ALL[index];
            let hovered = matches!(
                state.hovered_target,
                Some(HitTarget::ToneOption(hovered)) if hovered == index
            );
            if hovered {
                target.FillRoundedRectangle(
                    &rounded_rect(
                        tile.left + 1.0,
                        tile.top + 1.0,
                        tile.right - 1.0,
                        tile.bottom - 1.0,
                        7.0,
                    ),
                    &brushes.selection,
                );
            }
            let glyph = catalog::toned(base, tone).unwrap_or(base);
            draw_text(
                target,
                glyph,
                &state.formats.glyph,
                *tile,
                &brushes.primary,
                D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT,
            );
        }
        draw_text(
            target,
            "Inserts once · default is in Settings",
            &state.formats.center,
            rect(
                popup.left,
                popup.bottom - 24.0,
                popup.right,
                popup.bottom - 4.0,
            ),
            &brushes.secondary,
            D2D1_DRAW_TEXT_OPTIONS_CLIP,
        );
    }
}

unsafe fn draw_hover_help(
    state: &AppState,
    target: &ID2D1RenderTarget,
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
        // The scrollbar has no tooltip: the grip grows under the pointer,
        // which says what it is without covering the content beside it.
        Some(HitTarget::ShiftCap) => Some((
            "Hold Shift, or click, to keep the picker open",
            footer_button_rects(width, state.footer_top()).0,
            state.footer_top() as f32 - 31.0,
        )),
        Some(HitTarget::Copy) => Some((
            if state.keep_open() {
                "Copy and keep the picker open · Ctrl+Shift+C"
            } else {
                "Copy to the clipboard and close · Ctrl+C"
            },
            footer_button_rects(width, state.footer_top()).1,
            state.footer_top() as f32 - 31.0,
        )),
        Some(HitTarget::Insert) => Some((
            if state.keep_open() {
                "Insert and keep the picker open · Shift+Enter"
            } else {
                "Insert and close · Enter"
            },
            footer_button_rects(width, state.footer_top()).2,
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
    target: &ID2D1RenderTarget,
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
    resources: &RenderResources,
    entry_index: usize,
    bounds: D2D_RECT_F,
    brush: &ID2D1SolidColorBrush,
) {
    let target = &resources.target;
    let entry = &catalog::entries()[entry_index];
    if entry.kind == "Emoji" && state.config.emoji_font == EmojiFont::SegoeEmoji {
        // Blit the cached atlas tile. A glyph that has not been rasterized
        // yet stays blank for this frame and the idle warmer fills it in;
        // rasterizing here instead would put several milliseconds of GPU work
        // directly between the wheel and the pixels.
        if let Some(slot) = glyph_slot(state, resources, entry_index) {
            let side = (bounds.right - bounds.left).min(bounds.bottom - bounds.top);
            let center_x = (bounds.left + bounds.right) / 2.0;
            let center_y = (bounds.top + bounds.bottom) / 2.0;
            let destination = rect(
                center_x - side / 2.0,
                center_y - side / 2.0,
                center_x + side / 2.0,
                center_y + side / 2.0,
            );
            let atlas = resources.atlas.borrow();
            if let Some(page) = atlas.get(slot.page) {
                unsafe {
                    target.DrawBitmap(
                        &page.bitmap,
                        Some(&destination),
                        1.0,
                        D2D1_BITMAP_INTERPOLATION_MODE_LINEAR,
                        Some(&slot.source),
                    );
                }
            }
        }
        return;
    }
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
    target: &ID2D1RenderTarget,
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
    let detail = detail_line(state.config.details, &entry.name, &entry.glyph, entry.kind);
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

unsafe fn solid_brush(target: &ID2D1RenderTarget, value: u32) -> Result<ID2D1SolidColorBrush> {
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
    target: &ID2D1RenderTarget,
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

/// Frame-rate-independent exponential approach: the same fraction of the
/// remaining distance is covered per unit of wall-clock time regardless of
/// how many frames the monitor delivers in that time.
fn smooth_scroll_step(current: f32, target: f32, dt: f32) -> f32 {
    const RATE: f32 = 17.0;
    current + (target - current) * (1.0 - (-dt * RATE).exp())
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
                dwExtraInfo: INJECTION_TAG,
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
    // Stock ranking, with no usage history in play: this checks the catalog,
    // not whatever the machine running the test has picked before.
    let usage = catalog::UsageCounts::new();
    for (query, expected) in checks {
        let first = catalog::search(query, 1, &usage)
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
    let usage = catalog::UsageCounts::new();
    let mut samples = Vec::with_capacity(queries.len() * 200);
    for _ in 0..200 {
        for query in queries {
            let start = Instant::now();
            std::hint::black_box(catalog::search(query, 7, &usage));
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
        const FRAME: f32 = 1.0 / 60.0;
        let first = smooth_scroll_step(0.0, 76.0, FRAME);
        assert!(first > 0.0 && first < GRID_CELL as f32);
        let mut position = first;
        for _ in 0..40 {
            position = smooth_scroll_step(position, 76.0, FRAME);
        }
        assert!((76.0 - position).abs() < 0.01);
    }

    #[test]
    fn smooth_scroll_speed_is_frame_rate_independent() {
        // One 60Hz step covers the same distance as two 120Hz steps.
        let coarse = smooth_scroll_step(0.0, 100.0, 1.0 / 60.0);
        let fine = smooth_scroll_step(
            smooth_scroll_step(0.0, 100.0, 1.0 / 120.0),
            100.0,
            1.0 / 120.0,
        );
        assert!((coarse - fine).abs() < 0.001);
    }

    #[test]
    fn browser_categories_follow_cldr_groups_and_split_symbols() {
        let entries = catalog::entries();
        let find = |glyph: &str| {
            entries
                .iter()
                .find(|entry| entry.glyph == glyph)
                .unwrap_or_else(|| panic!("{glyph} exists"))
        };
        let smile = find("😀");
        let summation = find("∑");
        let input_symbols = find("🔣");
        assert!(BrowseCategory::Smileys.contains(smile));
        assert!(!BrowseCategory::Symbols.contains(smile));
        // The CLDR emoji symbols group and the Unicode text catalog are
        // separate categories, and neither holds the other's entries.
        assert!(BrowseCategory::Symbols.contains(input_symbols));
        assert!(!BrowseCategory::Characters.contains(input_symbols));
        assert!(BrowseCategory::Characters.contains(summation));
        assert!(!BrowseCategory::Symbols.contains(summation));
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
            catalog::search("table flip", 7, &catalog::UsageCounts::new())
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
