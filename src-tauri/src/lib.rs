use base64::{engine::general_purpose, Engine as _};
use futures_util::{SinkExt, StreamExt};
use rodio::{Decoder, OutputStream, Sink};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{BufReader, Cursor},
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex},
};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, WindowEvent,
};
use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct DeckConfig {
    active_profile_id: String,
    profiles: Vec<Profile>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Profile {
    id: String,
    name: String,
    buttons: Vec<DeckButton>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct DeckButton {
    id: String,
    label: String,
    icon: Option<String>,
    actions: Vec<Action>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ImportedButtonData {
    label: String,
    icon: Option<String>,
    actions: Vec<Action>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct NativeDropPosition {
    x: i32,
    y: i32,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct NativeDropImportPayload {
    position: NativeDropPosition,
    import_data: ImportedButtonData,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "camelCase")]
enum Action {
    OpenUrl {
        url: String,
    },
    LaunchApp {
        path: String,
        args: Option<Vec<String>>,
    },
    PlaySound {
        sound: String,
    },
    OpenFolder {
        path: String,
    },
}

#[derive(Clone)]
struct AppState {
    config: Arc<Mutex<DeckConfig>>,
    ws_clients: Arc<Mutex<Vec<tokio::sync::mpsc::UnboundedSender<String>>>>,
}

#[cfg(windows)]
mod native_drop {
    use super::NativeDropPosition;
    use std::{
        cell::UnsafeCell,
        ffi::{c_void, OsString},
        os::windows::ffi::OsStringExt,
        ptr,
        rc::Rc,
    };
    use windows::{
        core::{implement, w, Ref, Result as WinResult},
        Win32::{
            Foundation::{HWND, LPARAM, POINT, POINTL},
            Graphics::Gdi::ScreenToClient,
            System::{
                Com::{IDataObject, DVASPECT_CONTENT, FORMATETC, TYMED_HGLOBAL},
                DataExchange::RegisterClipboardFormatW,
                Memory::{GlobalLock, GlobalUnlock},
                Ole::{
                    IDropTarget, IDropTarget_Impl, OleInitialize, RegisterDragDrop,
                    ReleaseStgMedium, RevokeDragDrop, CF_HDROP, CF_TEXT, CF_UNICODETEXT,
                    CLIPBOARD_FORMAT, DROPEFFECT, DROPEFFECT_COPY, DROPEFFECT_NONE,
                },
                SystemServices::MODIFIERKEYS_FLAGS,
            },
            UI::{
                Shell::{DragQueryFileW, HDROP},
                WindowsAndMessaging::EnumChildWindows,
            },
        },
    };
    use windows_core::BOOL;

    pub struct NativeDropData {
        pub paths: Vec<String>,
        pub url: Option<String>,
        pub label: Option<String>,
        pub position: NativeDropPosition,
    }

    #[derive(Default)]
    pub struct NativeDropController {
        drop_targets: Vec<IDropTarget>,
    }

    impl NativeDropController {
        pub fn new(hwnd: HWND, handler: impl Fn(NativeDropData) + 'static) -> Self {
            let mut controller = Self::default();
            let handler = Rc::new(handler);

            let _ = unsafe { OleInitialize(None) };
            controller.inject_in_hwnd(hwnd, handler.clone());

            let mut callback = |child_hwnd| controller.inject_in_hwnd(child_hwnd, handler.clone());
            let mut trait_obj: &mut dyn FnMut(HWND) -> bool = &mut callback;
            let closure_pointer_pointer: *mut c_void =
                unsafe { std::mem::transmute(&mut trait_obj) };
            let lparam = LPARAM(closure_pointer_pointer as _);

            unsafe extern "system" fn enumerate_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
                let closure = &mut *(lparam.0 as *mut c_void as *mut &mut dyn FnMut(HWND) -> bool);
                closure(hwnd).into()
            }

            let _ = unsafe { EnumChildWindows(Some(hwnd), Some(enumerate_callback), lparam) };

            controller
        }

        fn inject_in_hwnd(&mut self, hwnd: HWND, handler: Rc<dyn Fn(NativeDropData)>) -> bool {
            let drop_target: IDropTarget = NativeDropTarget::new(hwnd, handler).into();
            let _ = unsafe { RevokeDragDrop(hwnd) };

            if unsafe { RegisterDragDrop(hwnd, &drop_target) }.is_ok() {
                self.drop_targets.push(drop_target);
            }

            true
        }
    }

    #[implement(IDropTarget)]
    struct NativeDropTarget {
        hwnd: HWND,
        listener: Rc<dyn Fn(NativeDropData)>,
        cursor_effect: UnsafeCell<DROPEFFECT>,
        enter_is_valid: UnsafeCell<bool>,
    }

    impl NativeDropTarget {
        fn new(hwnd: HWND, listener: Rc<dyn Fn(NativeDropData)>) -> Self {
            Self {
                hwnd,
                listener,
                cursor_effect: DROPEFFECT_NONE.into(),
                enter_is_valid: false.into(),
            }
        }

        fn client_position(&self, pt: &POINTL) -> NativeDropPosition {
            let mut point = POINT { x: pt.x, y: pt.y };
            let _ = unsafe { ScreenToClient(self.hwnd, &mut point) };

            NativeDropPosition {
                x: point.x,
                y: point.y,
            }
        }

        unsafe fn read_paths(data_obj: &Ref<'_, IDataObject>) -> Vec<String> {
            let drop_format = FORMATETC {
                cfFormat: CF_HDROP.0,
                ptd: ptr::null_mut(),
                dwAspect: DVASPECT_CONTENT.0,
                lindex: -1,
                tymed: TYMED_HGLOBAL.0 as u32,
            };

            let Some(data_obj) = data_obj.as_ref() else {
                return vec![];
            };

            let Ok(mut medium) = data_obj.GetData(&drop_format) else {
                return vec![];
            };

            let hdrop = HDROP(medium.u.hGlobal.0 as _);
            let item_count = DragQueryFileW(hdrop, 0xFFFFFFFF, None);
            let mut paths = Vec::with_capacity(item_count as usize);

            for index in 0..item_count {
                let character_count = DragQueryFileW(hdrop, index, None) as usize;
                let mut path_buf = vec![0; character_count + 1];

                DragQueryFileW(hdrop, index, Some(&mut path_buf));
                paths.push(
                    OsString::from_wide(&path_buf[..character_count])
                        .to_string_lossy()
                        .to_string(),
                );
            }

            ReleaseStgMedium(&mut medium);
            paths
        }

        unsafe fn read_utf16_format(
            data_obj: &Ref<'_, IDataObject>,
            format: CLIPBOARD_FORMAT,
        ) -> Option<String> {
            let Some(data_obj) = data_obj.as_ref() else {
                return None;
            };

            let drop_format = FORMATETC {
                cfFormat: format.0,
                ptd: ptr::null_mut(),
                dwAspect: DVASPECT_CONTENT.0,
                lindex: -1,
                tymed: TYMED_HGLOBAL.0 as u32,
            };
            let mut medium = data_obj.GetData(&drop_format).ok()?;
            let hglobal = medium.u.hGlobal;
            let locked = GlobalLock(hglobal);

            if locked.is_null() {
                ReleaseStgMedium(&mut medium);
                return None;
            }

            let data = locked as *const u16;
            let mut len = 0usize;

            while *data.add(len) != 0 {
                len += 1;
            }

            let text = String::from_utf16_lossy(std::slice::from_raw_parts(data, len));
            let _ = GlobalUnlock(hglobal);
            ReleaseStgMedium(&mut medium);

            Some(text)
        }

        unsafe fn read_ansi_format(
            data_obj: &Ref<'_, IDataObject>,
            format: CLIPBOARD_FORMAT,
        ) -> Option<String> {
            let Some(data_obj) = data_obj.as_ref() else {
                return None;
            };

            let drop_format = FORMATETC {
                cfFormat: format.0,
                ptd: ptr::null_mut(),
                dwAspect: DVASPECT_CONTENT.0,
                lindex: -1,
                tymed: TYMED_HGLOBAL.0 as u32,
            };
            let mut medium = data_obj.GetData(&drop_format).ok()?;
            let hglobal = medium.u.hGlobal;
            let locked = GlobalLock(hglobal);

            if locked.is_null() {
                ReleaseStgMedium(&mut medium);
                return None;
            }

            let data = locked as *const u8;
            let mut len = 0usize;

            while *data.add(len) != 0 {
                len += 1;
            }

            let text = String::from_utf8_lossy(std::slice::from_raw_parts(data, len)).to_string();
            let _ = GlobalUnlock(hglobal);
            ReleaseStgMedium(&mut medium);

            Some(text)
        }

        fn first_url_from_text(text: &str) -> Option<String> {
            for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
                if let Some(url) = super::normalize_web_url(line.trim_matches(['<', '>'])) {
                    return Some(url);
                }
            }

            let start = text.find("https://").or_else(|| text.find("http://"))?;
            let rest = &text[start..];
            let end = rest
                .find(|value: char| {
                    value.is_whitespace() || matches!(value, '<' | '>' | '"' | '\'')
                })
                .unwrap_or(rest.len());

            super::normalize_web_url(&rest[..end])
        }

        fn label_from_text(text: &str) -> Option<String> {
            text.lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .find(|line| Self::first_url_from_text(line).is_none())
                .map(str::to_string)
        }

        unsafe fn read_url(data_obj: &Ref<'_, IDataObject>) -> (Option<String>, Option<String>) {
            let uniform_url_w =
                CLIPBOARD_FORMAT(RegisterClipboardFormatW(w!("UniformResourceLocatorW")) as u16);
            let uniform_url =
                CLIPBOARD_FORMAT(RegisterClipboardFormatW(w!("UniformResourceLocator")) as u16);
            let text_uri_list =
                CLIPBOARD_FORMAT(RegisterClipboardFormatW(w!("text/uri-list")) as u16);
            let moz_url = CLIPBOARD_FORMAT(RegisterClipboardFormatW(w!("text/x-moz-url")) as u16);
            let moz_url_data =
                CLIPBOARD_FORMAT(RegisterClipboardFormatW(w!("text/x-moz-url-data")) as u16);
            let moz_url_desc =
                CLIPBOARD_FORMAT(RegisterClipboardFormatW(w!("text/x-moz-url-desc")) as u16);

            let candidates = [
                Self::read_utf16_format(data_obj, uniform_url_w),
                Self::read_ansi_format(data_obj, uniform_url),
                Self::read_ansi_format(data_obj, text_uri_list),
                Self::read_utf16_format(data_obj, moz_url),
                Self::read_utf16_format(data_obj, moz_url_data),
                Self::read_utf16_format(data_obj, CF_UNICODETEXT),
                Self::read_ansi_format(data_obj, CF_TEXT),
            ];
            let url_source = candidates.into_iter().flatten().find_map(|text| {
                Self::first_url_from_text(&text).map(|url| (url, Self::label_from_text(&text)))
            });

            let label = Self::read_utf16_format(data_obj, moz_url_desc)
                .map(|label| label.trim().to_string())
                .filter(|label| !label.is_empty());

            match url_source {
                Some((url, fallback_label)) => (Some(url), label.or(fallback_label)),
                None => (None, label),
            }
        }

        unsafe fn read_drop_data(
            &self,
            data_obj: &Ref<'_, IDataObject>,
            pt: &POINTL,
        ) -> NativeDropData {
            let paths = Self::read_paths(data_obj);
            let (url, label) = Self::read_url(data_obj);

            NativeDropData {
                paths,
                url,
                label,
                position: self.client_position(pt),
            }
        }

        unsafe fn has_supported_data(data_obj: &Ref<'_, IDataObject>) -> bool {
            !Self::read_paths(data_obj).is_empty() || Self::read_url(data_obj).0.is_some()
        }
    }

    #[allow(non_snake_case)]
    impl IDropTarget_Impl for NativeDropTarget_Impl {
        fn DragEnter(
            &self,
            pDataObj: Ref<'_, IDataObject>,
            _grfKeyState: MODIFIERKEYS_FLAGS,
            _pt: &POINTL,
            pdwEffect: *mut DROPEFFECT,
        ) -> WinResult<()> {
            let enter_is_valid = unsafe { NativeDropTarget::has_supported_data(&pDataObj) };
            let cursor_effect = if enter_is_valid {
                DROPEFFECT_COPY
            } else {
                DROPEFFECT_NONE
            };

            unsafe {
                *self.enter_is_valid.get() = enter_is_valid;
                *self.cursor_effect.get() = cursor_effect;
                *pdwEffect = cursor_effect;
            }

            Ok(())
        }

        fn DragOver(
            &self,
            _grfKeyState: MODIFIERKEYS_FLAGS,
            _pt: &POINTL,
            pdwEffect: *mut DROPEFFECT,
        ) -> WinResult<()> {
            unsafe {
                *pdwEffect = *self.cursor_effect.get();
            }

            Ok(())
        }

        fn DragLeave(&self) -> WinResult<()> {
            unsafe {
                *self.enter_is_valid.get() = false;
                *self.cursor_effect.get() = DROPEFFECT_NONE;
            }

            Ok(())
        }

        fn Drop(
            &self,
            pDataObj: Ref<'_, IDataObject>,
            _grfKeyState: MODIFIERKEYS_FLAGS,
            pt: &POINTL,
            pdwEffect: *mut DROPEFFECT,
        ) -> WinResult<()> {
            let data = unsafe { self.read_drop_data(&pDataObj, pt) };
            let is_valid = !data.paths.is_empty() || data.url.is_some();

            if is_valid {
                (self.listener)(data);
            }

            unsafe {
                *pdwEffect = if is_valid {
                    DROPEFFECT_COPY
                } else {
                    DROPEFFECT_NONE
                };
                *self.enter_is_valid.get() = false;
                *self.cursor_effect.get() = DROPEFFECT_NONE;
            }

            Ok(())
        }
    }
}

fn default_config() -> DeckConfig {
    DeckConfig {
        active_profile_id: "".into(),
        profiles: vec![],
    }
}

fn config_path(app: &AppHandle) -> PathBuf {
    let dir = app.path().app_config_dir().expect("config dir");
    fs::create_dir_all(&dir).ok();
    dir.join("deck.json")
}

fn read_config_from_disk(app: &AppHandle) -> Result<DeckConfig, String> {
    let path = config_path(app);

    if !path.exists() {
        let cfg = default_config();
        write_config_to_disk(app, &cfg)?;
        return Ok(cfg);
    }

    let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

fn write_config_to_disk(app: &AppHandle, config: &DeckConfig) -> Result<(), String> {
    let path = config_path(app);
    let text = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    fs::write(path, text).map_err(|e| e.to_string())
}

fn broadcast_config(state: &AppState, config: &DeckConfig) {
    let response = serde_json::json!({
        "type": "config",
        "config": config
    })
    .to_string();

    let mut clients = state.ws_clients.lock().unwrap();

    clients.retain(|client| client.send(response.clone()).is_ok());
}

#[tauri::command]
fn load_config(state: tauri::State<AppState>) -> Result<DeckConfig, String> {
    let config = state.config.lock().unwrap();
    Ok(config.clone())
}

#[tauri::command]
fn save_config(
    app: AppHandle,
    state: tauri::State<AppState>,
    config: DeckConfig,
) -> Result<(), String> {
    validate_config(&config)?;
    write_config_to_disk(&app, &config)?;

    {
        let mut cached_config = state.config.lock().unwrap();
        *cached_config = config.clone();
    }

    broadcast_config(&state, &config);

    Ok(())
}

fn validate_config(config: &DeckConfig) -> Result<(), String> {
    if config.profiles.len() > 15 {
        return Err("Only 15 profiles are allowed.".to_string());
    }

    for profile in &config.profiles {
        if profile.buttons.len() > 20 {
            return Err(format!(
                "Profile '{}' has too many buttons. Maximum allowed is {}.",
                profile.name, 20
            ));
        }
    }

    Ok(())
}

fn file_label(path: &Path) -> String {
    path.file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.trim().is_empty())
        .or_else(|| path.file_name().and_then(|value| value.to_str()))
        .unwrap_or("Dropped Item")
        .to_string()
}

fn file_extension(path: &Path) -> String {
    path.extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_lowercase()
}

fn is_audio_file(path: &Path) -> bool {
    matches!(
        file_extension(path).as_str(),
        "mp3" | "wav" | "ogg" | "m4a" | "flac" | "aac"
    )
}

fn is_supported_application(path: &Path) -> bool {
    matches!(file_extension(path).as_str(), "exe")
}

fn read_url_shortcut(path: &Path) -> Option<String> {
    let text = fs::read_to_string(path).ok()?;

    text.lines()
        .find_map(|line| line.trim().strip_prefix("URL=").map(str::trim))
        .filter(|url| !url.is_empty())
        .map(str::to_string)
}

fn normalize_web_url(value: &str) -> Option<String> {
    let trimmed = value.trim();

    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        Some(trimmed.to_string())
    } else {
        None
    }
}

fn label_for_url(url: &str) -> String {
    let host = url
        .split_once("://")
        .map(|(_, rest)| rest)
        .unwrap_or(url)
        .split('/')
        .next()
        .unwrap_or(url)
        .trim_start_matches("www.");

    if host.is_empty() {
        "Website".to_string()
    } else {
        host.to_string()
    }
}

fn favicon_for_url(url: &str) -> Option<String> {
    let (scheme, rest) = url.split_once("://")?;
    let host = rest.split('/').next()?.trim();

    if scheme.is_empty() || host.is_empty() {
        return None;
    }

    Some(format!(
        "https://www.google.com/s2/favicons?domain={host}&sz=64"
    ))
}

fn fallback_icon_for_path(path: &Path) -> String {
    if path.is_dir() {
        "\u{1F4C1}".to_string()
    } else if is_audio_file(path) {
        "\u{1F50A}".to_string()
    } else {
        "\u{1F3AE}".to_string()
    }
}

#[cfg(not(windows))]
fn file_icon_data_url(_path: &Path) -> Option<String> {
    None
}

#[cfg(windows)]
fn file_icon_data_url(path: &Path) -> Option<String> {
    use std::{ffi::c_void, os::windows::ffi::OsStrExt};
    use windows::{
        core::PCWSTR,
        Win32::{
            Graphics::Gdi::{
                CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, ReleaseDC,
                SelectObject, BITMAPINFO, BI_RGB, DIB_RGB_COLORS,
            },
            Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES,
            UI::{
                Shell::{SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON},
                WindowsAndMessaging::{DestroyIcon, DrawIconEx, DI_NORMAL},
            },
        },
    };

    let wide_path: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let mut shell_info = SHFILEINFOW::default();

    let result = unsafe {
        SHGetFileInfoW(
            PCWSTR(wide_path.as_ptr()),
            FILE_FLAGS_AND_ATTRIBUTES(0),
            Some(&mut shell_info),
            std::mem::size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON | SHGFI_LARGEICON,
        )
    };

    if result == 0 || shell_info.hIcon.is_invalid() {
        return None;
    }

    let width = 48;
    let height = 48;
    let mut bitmap_info = BITMAPINFO::default();

    bitmap_info.bmiHeader.biSize = std::mem::size_of_val(&bitmap_info.bmiHeader) as u32;
    bitmap_info.bmiHeader.biWidth = width;
    bitmap_info.bmiHeader.biHeight = -height;
    bitmap_info.bmiHeader.biPlanes = 1;
    bitmap_info.bmiHeader.biBitCount = 32;
    bitmap_info.bmiHeader.biCompression = BI_RGB.0;

    let mut bits: *mut c_void = std::ptr::null_mut();

    let data_url = unsafe {
        let screen_dc = GetDC(None);
        let memory_dc = CreateCompatibleDC(Some(screen_dc));
        let bitmap = CreateDIBSection(
            Some(screen_dc),
            &bitmap_info,
            DIB_RGB_COLORS,
            &mut bits,
            None,
            0,
        )
        .ok()?;
        let bitmap_object = bitmap.into();
        let old_bitmap = SelectObject(memory_dc, bitmap_object);

        DrawIconEx(
            memory_dc,
            0,
            0,
            shell_info.hIcon,
            width,
            height,
            0,
            None,
            DI_NORMAL,
        )
        .ok()?;

        SelectObject(memory_dc, old_bitmap);
        let _ = DeleteDC(memory_dc);
        let _ = ReleaseDC(None, screen_dc);
        let _ = DestroyIcon(shell_info.hIcon);

        let pixel_count = (width * height * 4) as usize;
        let pixels = std::slice::from_raw_parts(bits as *const u8, pixel_count).to_vec();
        let _ = DeleteObject(bitmap_object);

        Some(bmp_data_url(width, height, &pixels))
    };

    data_url
}

fn bmp_data_url(width: i32, height: i32, pixels: &[u8]) -> String {
    let pixel_data_offset = 54u32;
    let file_size = pixel_data_offset + pixels.len() as u32;
    let mut bytes = Vec::with_capacity(file_size as usize);

    bytes.extend_from_slice(b"BM");
    bytes.extend_from_slice(&file_size.to_le_bytes());
    bytes.extend_from_slice(&0u16.to_le_bytes());
    bytes.extend_from_slice(&0u16.to_le_bytes());
    bytes.extend_from_slice(&pixel_data_offset.to_le_bytes());
    bytes.extend_from_slice(&40u32.to_le_bytes());
    bytes.extend_from_slice(&width.to_le_bytes());
    bytes.extend_from_slice(&(-height).to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&32u16.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&(pixels.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&2835i32.to_le_bytes());
    bytes.extend_from_slice(&2835i32.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(pixels);

    format!(
        "data:image/bmp;base64,{}",
        general_purpose::STANDARD.encode(bytes)
    )
}

#[tauri::command]
fn describe_dropped_file(path: String) -> Result<ImportedButtonData, String> {
    if let Some(url) = normalize_web_url(&path) {
        return Ok(ImportedButtonData {
            label: label_for_url(&url),
            icon: favicon_for_url(&url),
            actions: vec![Action::OpenUrl { url }],
        });
    }

    let path_buf = PathBuf::from(&path);
    let label = file_label(&path_buf);
    let extension = file_extension(&path_buf);

    if path_buf.is_dir() {
        return Ok(ImportedButtonData {
            label,
            icon: Some("\u{1F4C1}".to_string()),
            actions: vec![Action::OpenFolder { path }],
        });
    }

    if extension == "url" || extension == "website" {
        let url = read_url_shortcut(&path_buf)
            .ok_or_else(|| "Could not read URL from shortcut.".to_string())?;

        return Ok(ImportedButtonData {
            label,
            icon: favicon_for_url(&url),
            actions: vec![Action::OpenUrl { url }],
        });
    }

    if extension == "lnk" {
        return Ok(ImportedButtonData {
            label,
            icon: file_icon_data_url(&path_buf).or_else(|| Some("\u{1F517}".to_string())),
            actions: vec![Action::OpenUrl { url: path }],
        });
    }

    if is_audio_file(&path_buf) {
        return Ok(ImportedButtonData {
            label,
            icon: Some("\u{1F50A}".to_string()),
            actions: vec![Action::PlaySound { sound: path }],
        });
    }

    if !is_supported_application(&path_buf) {
        return Err(
            "Unsupported item. Only folders, sound files, website URLs, shortcuts, and .exe applications can be dropped."
                .to_string(),
        );
    }

    let icon = file_icon_data_url(&path_buf).or_else(|| Some(fallback_icon_for_path(&path_buf)));

    Ok(ImportedButtonData {
        label,
        icon,
        actions: vec![Action::LaunchApp {
            path,
            args: Some(vec![]),
        }],
    })
}

fn play_sound(sound: &str) -> Result<(), String> {
    let (_stream, handle) = OutputStream::try_default().map_err(|e| e.to_string())?;

    let sink = Sink::try_new(&handle).map_err(|e| e.to_string())?;

    if sound.starts_with("data:audio") {
        let encoded = sound.split(',').nth(1).ok_or("Invalid audio data URL")?;

        let bytes = general_purpose::STANDARD
            .decode(encoded)
            .map_err(|e| e.to_string())?;

        let cursor = Cursor::new(bytes);

        let source = Decoder::new(BufReader::new(cursor)).map_err(|e| e.to_string())?;

        sink.append(source);
    } else {
        let file = std::fs::File::open(sound).map_err(|e| e.to_string())?;

        let source = Decoder::new(BufReader::new(file)).map_err(|e| e.to_string())?;

        sink.append(source);
    }

    sink.sleep_until_end();

    Ok(())
}

async fn run_action(action: Action) -> Result<(), String> {
    match action {
        Action::OpenUrl { url } => {
            open::that(url).map_err(|e| e.to_string())?;
        }

        Action::LaunchApp { path, args } => {
            let mut cmd = Command::new(path);

            if let Some(args) = args {
                cmd.args(args);
            }

            cmd.spawn().map_err(|e| e.to_string())?;
        }

        Action::PlaySound { sound } => {
            tauri::async_runtime::spawn_blocking(move || {
                let _ = play_sound(&sound);
            });
        }

        Action::OpenFolder { path } => {
            open::that(path).map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

#[tauri::command]
async fn trigger_button(
    state: tauri::State<'_, AppState>,
    button_id: String,
    profile_id: Option<String>,
) -> Result<(), String> {
    let actions = {
        let cfg = state.config.lock().unwrap();

        let profile = if let Some(profile_id) = profile_id {
            cfg.profiles
                .iter()
                .find(|p| p.id == profile_id)
                .ok_or("Profile not found")?
        } else {
            cfg.profiles
                .iter()
                .find(|p| p.id == cfg.active_profile_id)
                .ok_or("Active profile not found")?
        };

        let button = profile
            .buttons
            .iter()
            .find(|b| b.id == button_id)
            .ok_or("Button not found")?;

        button.actions.clone()
    };

    for action in actions {
        run_action(action).await?;
    }

    Ok(())
}

async fn websocket_server(state: AppState) {
    let listener = TcpListener::bind("0.0.0.0:37123")
        .await
        .expect("WebSocket server failed");

    loop {
        let Ok((stream, _)) = listener.accept().await else {
            continue;
        };

        let state = state.clone();

        tokio::spawn(async move {
            let Ok(ws) = accept_async(stream).await else {
                return;
            };

            let (mut write, mut read) = ws.split();
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();

            {
                let mut clients = state.ws_clients.lock().unwrap();
                clients.push(tx.clone());
            }

            tokio::spawn(async move {
                while let Some(msg) = rx.recv().await {
                    let _ = write
                        .send(tokio_tungstenite::tungstenite::Message::Text(msg.into()))
                        .await;
                }
            });

            while let Some(Ok(msg)) = read.next().await {
                if let Ok(text) = msg.to_text() {
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(text) {
                        if value["type"] == "triggerButton" {
                            if let Some(button_id) = value["buttonId"].as_str() {
                                let profile_id = value["profileId"].as_str().map(|s| s.to_string());

                                let button_id = button_id.to_string();
                                let state_for_action = state.clone();

                                tauri::async_runtime::spawn(async move {
                                    let _ = trigger_button_from_state(
                                        state_for_action,
                                        button_id,
                                        profile_id,
                                    )
                                    .await;
                                });
                            }
                        }

                        if value["type"] == "getConfig" {
                            let config = {
                                let cfg = state.config.lock().unwrap();
                                cfg.clone()
                            };

                            let response = serde_json::json!({
                                "type": "config",
                                "config": config
                            });

                            let _ = tx.send(response.to_string());
                        }
                    }
                }
            }
        });
    }
}

async fn trigger_button_from_state(
    state: AppState,
    button_id: String,
    profile_id: Option<String>,
) -> Result<(), String> {
    let actions = {
        let cfg = state.config.lock().unwrap();

        let profile = if let Some(profile_id) = profile_id {
            cfg.profiles
                .iter()
                .find(|p| p.id == profile_id)
                .ok_or("Profile not found")?
        } else {
            cfg.profiles
                .iter()
                .find(|p| p.id == cfg.active_profile_id)
                .ok_or("Active profile not found")?
        };

        let button = profile
            .buttons
            .iter()
            .find(|b| b.id == button_id)
            .ok_or("Button not found")?;

        button.actions.clone()
    };

    for action in actions {
        run_action(action).await?;
    }

    Ok(())
}

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

#[cfg(windows)]
fn install_native_drop_handler(app: &AppHandle) {
    use native_drop::{NativeDropController, NativeDropData};

    let Some(window) = app.get_webview_window("main") else {
        return;
    };

    let Ok(hwnd) = window.hwnd() else {
        return;
    };

    let app_handle = app.clone();
    let controller = NativeDropController::new(hwnd, move |drop: NativeDropData| {
        let source = drop.url.clone().or_else(|| drop.paths.first().cloned());

        let Some(source) = source else {
            return;
        };

        let Ok(mut import_data) = describe_dropped_file(source) else {
            return;
        };

        if let Some(label) = drop
            .label
            .as_ref()
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
        {
            if matches!(import_data.actions.first(), Some(Action::OpenUrl { .. })) {
                import_data.label = label.to_string();
            }
        }

        let _ = app_handle.emit(
            "native-drop-import",
            NativeDropImportPayload {
                position: drop.position,
                import_data,
            },
        );
    });

    Box::leak(Box::new(controller));
}

#[cfg(not(windows))]
fn install_native_drop_handler(_app: &AppHandle) {}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let initial_config = read_config_from_disk(app.handle())?;

            let state = AppState {
                config: Arc::new(Mutex::new(initial_config)),
                ws_clients: Arc::new(Mutex::new(vec![])),
            };

            app.manage(state.clone());

            tauri::async_runtime::spawn(websocket_server(state));
            install_native_drop_handler(app.handle());

            let show_item = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let tray_menu = Menu::with_items(app, &[&show_item, &quit_item])?;

            TrayIconBuilder::new()
                .menu(&tray_menu)
                .tooltip("Stream Deck")
                .icon(app.default_window_icon().unwrap().clone())
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "show" => {
                        show_main_window(app);
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();

                        show_main_window(app);
                    }
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();

                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            describe_dropped_file,
            load_config,
            save_config,
            trigger_button
        ])
        .run(tauri::generate_context!())
        .expect("error while running app");
}
