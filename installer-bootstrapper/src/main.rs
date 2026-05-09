#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("The Stream Pad installer bootstrapper only runs on Windows.");
}

#[cfg(target_os = "windows")]
mod windows_installer {
    use std::{
        ffi::OsStr,
        fs,
        os::windows::ffi::OsStrExt,
        path::{Path, PathBuf},
        process::Command,
        sync::{Arc, Mutex, OnceLock},
        thread,
        time::{Duration, Instant},
    };

    use windows::{
        core::{w, PCWSTR},
        Win32::{
            Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM},
            Graphics::Gdi::{
                BeginPaint, CreateFontW, CreatePen, CreateSolidBrush, DeleteObject, DrawTextW,
                Ellipse, EndPaint, InvalidateRect, LineTo, MoveToEx, SelectObject, SetBkMode,
                SetTextColor, StretchDIBits, BITMAPINFO, BITMAPINFOHEADER, CLEARTYPE_QUALITY,
                CLIP_DEFAULT_PRECIS, DEFAULT_CHARSET, DEFAULT_PITCH, DIB_RGB_COLORS, DT_CENTER,
                DT_SINGLELINE, DT_VCENTER, FF_DONTCARE, FW_BOLD, FW_NORMAL, HDC,
                OUT_DEFAULT_PRECIS, PAINTSTRUCT, PS_SOLID, SRCCOPY, TRANSPARENT,
            },
            System::{
                Com::Urlmon::URLDownloadToFileW,
                LibraryLoader::GetModuleHandleW,
            },
            UI::{
                Input::KeyboardAndMouse::ReleaseCapture,
                WindowsAndMessaging::{
                    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW,
                    GetSystemMetrics, LoadCursorW, PostMessageW, PostQuitMessage, RegisterClassW,
                    SendMessageW, SetTimer, ShowWindow, TranslateMessage, CS_HREDRAW, CS_VREDRAW,
                    CW_USEDEFAULT, HTCAPTION, IDC_ARROW, MSG, SM_CXSCREEN, SM_CYSCREEN, SW_SHOW,
                    WINDOW_EX_STYLE, WM_CLOSE, WM_DESTROY, WM_LBUTTONDOWN, WM_NCLBUTTONDOWN,
                    WM_PAINT, WM_TIMER, WNDCLASSW, WS_POPUP, WS_VISIBLE,
                },
            },
        },
    };

    const WINDOW_WIDTH: i32 = 980;
    const WINDOW_HEIGHT: i32 = 640;
    const LOGO_SIZE: i32 = 160;
    const LOGO_BITMAP: &[u8] = include_bytes!("../../src-tauri/installer/app-logo-160.bgra");
    const TIMER_ID: usize = 1;
    const TIMER_MS: u32 = 16;
    const RELEASE_JSON_URL: &str =
        "https://github.com/Antonis-ge0/Stream-Deck/releases/latest/download/latest.json";
    const RELEASE_DOWNLOAD_BASE: &str = "https://github.com/Antonis-ge0/Stream-Deck/releases";

    static APP_STATE: OnceLock<Arc<AppState>> = OnceLock::new();
    static STARTED_AT: OnceLock<Instant> = OnceLock::new();
    static BACKGROUND: OnceLock<Vec<u8>> = OnceLock::new();

    #[derive(Clone)]
    struct InstallerView {
        status: String,
        detail: String,
        failed: bool,
    }

    struct AppState {
        view: Mutex<InstallerView>,
    }

    impl AppState {
        fn set_status(&self, status: impl Into<String>, detail: impl Into<String>) {
            if let Ok(mut view) = self.view.lock() {
                view.status = status.into();
                view.detail = detail.into();
                view.failed = false;
            }
        }

        fn set_error(&self, detail: impl Into<String>) {
            if let Ok(mut view) = self.view.lock() {
                view.status = "Install failed".to_string();
                view.detail = detail.into();
                view.failed = true;
            }
        }

        fn snapshot(&self) -> InstallerView {
            self.view.lock().map(|view| view.clone()).unwrap_or_else(|_| InstallerView {
                status: "Installing".to_string(),
                detail: "Please wait while we install Stream Pad.".to_string(),
                failed: false,
            })
        }
    }

    pub fn main() {
        let state = Arc::new(AppState {
            view: Mutex::new(InstallerView {
                status: "Preparing".to_string(),
                detail: "Please wait while we install Stream Pad.".to_string(),
                failed: false,
            }),
        });
        let _ = APP_STATE.set(Arc::clone(&state));
        let _ = STARTED_AT.set(Instant::now());
        let _ = BACKGROUND.set(generate_background(WINDOW_WIDTH, WINDOW_HEIGHT));

        unsafe {
            if let Some(hwnd) = create_window() {
                start_install_worker(hwnd, state);
                message_loop();
            }
        }
    }

    unsafe fn create_window() -> Option<HWND> {
        let module = GetModuleHandleW(None).ok()?;
        let instance = HINSTANCE(module.0);
        let class_name = w!("StreamPadVisualInstaller");
        let cursor = LoadCursorW(None, IDC_ARROW).ok();

        let window_class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(window_proc),
            hInstance: instance,
            hCursor: cursor.unwrap_or_default(),
            lpszClassName: class_name,
            ..Default::default()
        };

        if RegisterClassW(&window_class) == 0 {
            return None;
        }

        let screen_width = GetSystemMetrics(SM_CXSCREEN);
        let screen_height = GetSystemMetrics(SM_CYSCREEN);
        let x = if screen_width > WINDOW_WIDTH {
            (screen_width - WINDOW_WIDTH) / 2
        } else {
            CW_USEDEFAULT
        };
        let y = if screen_height > WINDOW_HEIGHT {
            (screen_height - WINDOW_HEIGHT) / 2
        } else {
            CW_USEDEFAULT
        };

        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            class_name,
            w!("Stream Pad Installer"),
            WS_POPUP | WS_VISIBLE,
            x,
            y,
            WINDOW_WIDTH,
            WINDOW_HEIGHT,
            None,
            None,
            Some(instance),
            None,
        )
        .ok()?;

        let _ = ShowWindow(hwnd, SW_SHOW);
        SetTimer(Some(hwnd), TIMER_ID, TIMER_MS, None);
        Some(hwnd)
    }

    unsafe fn message_loop() {
        let mut message = MSG::default();
        while GetMessageW(&mut message, None, 0, 0).into() {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }

    unsafe extern "system" fn window_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match msg {
            WM_PAINT => {
                paint(hwnd);
                LRESULT(0)
            }
            WM_TIMER => {
                let _ = InvalidateRect(Some(hwnd), None, false);
                LRESULT(0)
            }
            WM_LBUTTONDOWN => {
                let x = (lparam.0 & 0xffff) as i16 as i32;
                let y = ((lparam.0 >> 16) & 0xffff) as i16 as i32;

                if x >= WINDOW_WIDTH - 42 && y <= 42 {
                    PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0)).ok();
                } else {
                    ReleaseCapture().ok();
                    SendMessageW(
                        hwnd,
                        WM_NCLBUTTONDOWN,
                        Some(WPARAM(HTCAPTION as usize)),
                        Some(LPARAM(0)),
                    );
                }
                LRESULT(0)
            }
            WM_DESTROY => {
                PostQuitMessage(0);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }

    unsafe fn paint(hwnd: HWND) {
        let mut paint = PAINTSTRUCT::default();
        let hdc = BeginPaint(hwnd, &mut paint);

        draw_background(hdc);
        draw_close_button(hdc);
        draw_brand(hdc);
        draw_status(hdc);

        let _ = EndPaint(hwnd, &paint);
    }

    unsafe fn draw_background(hdc: HDC) {
        let Some(background) = BACKGROUND.get() else {
            return;
        };

        let mut info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: WINDOW_WIDTH,
                biHeight: -WINDOW_HEIGHT,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: 0,
                ..Default::default()
            },
            ..Default::default()
        };

        StretchDIBits(
            hdc,
            0,
            0,
            WINDOW_WIDTH,
            WINDOW_HEIGHT,
            0,
            0,
            WINDOW_WIDTH,
            WINDOW_HEIGHT,
            Some(background.as_ptr().cast()),
            &mut info,
            DIB_RGB_COLORS,
            SRCCOPY,
        );
    }

    unsafe fn draw_close_button(hdc: HDC) {
        let pen = CreatePen(PS_SOLID, 2, rgb(38, 42, 47));
        let old_pen = SelectObject(hdc, pen.into());

        let _ = MoveToEx(hdc, WINDOW_WIDTH - 24, 16, None);
        let _ = LineTo(hdc, WINDOW_WIDTH - 16, 24);
        let _ = MoveToEx(hdc, WINDOW_WIDTH - 16, 16, None);
        let _ = LineTo(hdc, WINDOW_WIDTH - 24, 24);

        SelectObject(hdc, old_pen);
        let _ = DeleteObject(pen.into());
    }

    unsafe fn draw_brand(hdc: HDC) {
        draw_app_logo(hdc, WINDOW_WIDTH / 2 - LOGO_SIZE / 2, 145);

        let title_font = CreateFontW(
            38,
            0,
            0,
            0,
            FW_BOLD.0 as i32,
            0,
            0,
            0,
            DEFAULT_CHARSET,
            OUT_DEFAULT_PRECIS,
            CLIP_DEFAULT_PRECIS,
            CLEARTYPE_QUALITY,
            font_pitch_family(),
            w!("Segoe UI"),
        );
        draw_centered_text(
            hdc,
            "STREAM PAD",
            RECT {
                left: 0,
                top: 320,
                right: WINDOW_WIDTH,
                bottom: 370,
            },
            title_font,
            rgb(10, 14, 18),
        );

        let version_font = CreateFontW(
            12,
            0,
            0,
            0,
            FW_BOLD.0 as i32,
            0,
            0,
            0,
            DEFAULT_CHARSET,
            OUT_DEFAULT_PRECIS,
            CLIP_DEFAULT_PRECIS,
            CLEARTYPE_QUALITY,
            font_pitch_family(),
            w!("Segoe UI"),
        );
        draw_centered_text(
            hdc,
            &format!("VERSION {}", env!("CARGO_PKG_VERSION")),
            RECT {
                left: 0,
                top: 372,
                right: WINDOW_WIDTH,
                bottom: 396,
            },
            version_font,
            rgb(76, 83, 92),
        );

        let _ = DeleteObject(title_font.into());
        let _ = DeleteObject(version_font.into());
    }

    unsafe fn draw_status(hdc: HDC) {
        let state = APP_STATE.get().map(|state| state.snapshot()).unwrap_or(InstallerView {
            status: "Installing".to_string(),
            detail: "Please wait while we install Stream Pad.".to_string(),
            failed: false,
        });

        let detail_font = CreateFontW(
            14,
            0,
            0,
            0,
            FW_NORMAL.0 as i32,
            0,
            0,
            0,
            DEFAULT_CHARSET,
            OUT_DEFAULT_PRECIS,
            CLIP_DEFAULT_PRECIS,
            CLEARTYPE_QUALITY,
            font_pitch_family(),
            w!("Segoe UI"),
        );
        draw_centered_text(
            hdc,
            &state.detail,
            RECT {
                left: 0,
                top: 490,
                right: WINDOW_WIDTH,
                bottom: 520,
            },
            detail_font,
            if state.failed {
                rgb(239, 68, 68)
            } else {
                rgb(92, 99, 109)
            },
        );

        draw_spinner(hdc, WINDOW_WIDTH / 2, 555, 17, state.failed);

        let status_font = CreateFontW(
            12,
            0,
            0,
            0,
            FW_BOLD.0 as i32,
            0,
            0,
            0,
            DEFAULT_CHARSET,
            OUT_DEFAULT_PRECIS,
            CLIP_DEFAULT_PRECIS,
            CLEARTYPE_QUALITY,
            font_pitch_family(),
            w!("Segoe UI"),
        );
        draw_centered_text(
            hdc,
            &state.status,
            RECT {
                left: 0,
                top: 580,
                right: WINDOW_WIDTH,
                bottom: 606,
            },
            status_font,
            rgb(10, 14, 18),
        );

        let _ = DeleteObject(detail_font.into());
        let _ = DeleteObject(status_font.into());
    }

    unsafe fn draw_app_logo(hdc: HDC, x: i32, y: i32) {
        let mut info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: LOGO_SIZE,
                biHeight: -LOGO_SIZE,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: 0,
                ..Default::default()
            },
            ..Default::default()
        };

        StretchDIBits(
            hdc,
            x,
            y,
            LOGO_SIZE,
            LOGO_SIZE,
            0,
            0,
            LOGO_SIZE,
            LOGO_SIZE,
            Some(LOGO_BITMAP.as_ptr().cast()),
            &mut info,
            DIB_RGB_COLORS,
            SRCCOPY,
        );
    }

    unsafe fn draw_spinner(hdc: HDC, center_x: i32, center_y: i32, radius: i32, failed: bool) {
        if failed {
            let pen = CreatePen(PS_SOLID, 3, rgb(239, 68, 68));
            let old_pen = SelectObject(hdc, pen.into());
            let _ = MoveToEx(hdc, center_x - 8, center_y - 8, None);
            let _ = LineTo(hdc, center_x + 8, center_y + 8);
            let _ = MoveToEx(hdc, center_x + 8, center_y - 8, None);
            let _ = LineTo(hdc, center_x - 8, center_y + 8);
            SelectObject(hdc, old_pen);
            let _ = DeleteObject(pen.into());
            return;
        }

        let elapsed = STARTED_AT
            .get()
            .map(|start| start.elapsed().as_millis() as f32 / 1000.0)
            .unwrap_or_default();
        let rotation = elapsed * 6.2;

        for index in 0..12 {
            let angle = rotation + index as f32 * std::f32::consts::TAU / 12.0;
            let alpha = 70 + index * 13;
            let color = 22 + (alpha.min(150) as u8 / 2);
            let x = center_x as f32 + angle.cos() * radius as f32;
            let y = center_y as f32 + angle.sin() * radius as f32;
            let brush = CreateSolidBrush(rgb(color, color, color));
            let old_brush = SelectObject(hdc, brush.into());
            let pen = CreatePen(PS_SOLID, 1, rgb(color, color, color));
            let old_pen = SelectObject(hdc, pen.into());

            let _ = Ellipse(
                hdc,
                x.round() as i32 - 3,
                y.round() as i32 - 3,
                x.round() as i32 + 3,
                y.round() as i32 + 3,
            );

            SelectObject(hdc, old_pen);
            SelectObject(hdc, old_brush);
            let _ = DeleteObject(pen.into());
            let _ = DeleteObject(brush.into());
        }
    }

    unsafe fn draw_centered_text(
        hdc: HDC,
        text: &str,
        mut rect: RECT,
        font: windows::Win32::Graphics::Gdi::HFONT,
        color: COLORREF,
    ) {
        let old_font = SelectObject(hdc, font.into());
        SetBkMode(hdc, TRANSPARENT);
        SetTextColor(hdc, color);

        let mut text = to_wide(text);
        DrawTextW(hdc, &mut text, &mut rect, DT_CENTER | DT_VCENTER | DT_SINGLELINE);

        SelectObject(hdc, old_font);
    }

    fn start_install_worker(hwnd: HWND, state: Arc<AppState>) {
        let hwnd_value = hwnd.0 as isize;
        thread::spawn(move || {
            state.set_status("Downloading", "Getting the latest Stream Pad installer.");

            let result = resolve_installer(&state).and_then(|installer| {
                state.set_status("Installing", "Please wait while we install Stream Pad.");
                run_inner_installer(&installer)
            });

            match result {
                Ok(()) => {
                    state.set_status("Ready", "Stream Pad has been installed.");
                    thread::sleep(Duration::from_millis(1500));
                    unsafe {
                        let hwnd = HWND(hwnd_value as *mut _);
                        PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0)).ok();
                    }
                }
                Err(error) => {
                    state.set_error(error);
                }
            }
        });
    }

    fn resolve_installer(state: &AppState) -> Result<PathBuf, String> {
        if let Some(local_installer) = find_companion_installer() {
            return Ok(local_installer);
        }

        let temp_dir = std::env::temp_dir().join("stream-pad-installer");
        fs::create_dir_all(&temp_dir)
            .map_err(|error| format!("Could not prepare installer cache: {error}"))?;

        let metadata_path = temp_dir.join("latest.json");
        download_file(RELEASE_JSON_URL, &metadata_path)
            .map_err(|_| "Could not reach the Stream Pad release endpoint.".to_string())?;

        let metadata = fs::read_to_string(&metadata_path)
            .map_err(|error| format!("Could not read update metadata: {error}"))?;
        let version = parse_json_string(&metadata, "version")
            .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string());
        let tag = format!("v{version}");
        let installer_name = format!("Stream.Pad_{version}_x64-setup.exe");
        let installer_url = format!("{RELEASE_DOWNLOAD_BASE}/download/{tag}/{installer_name}");
        let installer_path = temp_dir.join(installer_name);

        state.set_status(
            "Downloading",
            format!("Downloading Stream Pad {version}."),
        );
        download_file(&installer_url, &installer_path)
            .map_err(|_| "Could not download the Stream Pad setup file.".to_string())?;

        Ok(installer_path)
    }

    fn find_companion_installer() -> Option<PathBuf> {
        let exe_dir = std::env::current_exe().ok()?.parent()?.to_path_buf();
        let entries = fs::read_dir(exe_dir).ok()?;

        entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .find(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .map(|name| {
                        name.starts_with("Stream.Pad_") && name.ends_with("_x64-setup.exe")
                    })
                    .unwrap_or(false)
            })
    }

    fn run_inner_installer(installer: &Path) -> Result<(), String> {
        let status = Command::new(installer)
            .args(["/S", "/R"])
            .status()
            .map_err(|error| format!("Could not start the Stream Pad setup: {error}"))?;

        if status.success() {
            Ok(())
        } else {
            Err(format!(
                "The Stream Pad setup exited with code {}.",
                status
                    .code()
                    .map(|code| code.to_string())
                    .unwrap_or_else(|| "unknown".to_string())
            ))
        }
    }

    fn download_file(url: &str, destination: &Path) -> Result<(), windows::core::Error> {
        let url = to_wide(url);
        let destination = to_wide_path(destination);

        unsafe {
            URLDownloadToFileW(
                None,
                PCWSTR(url.as_ptr()),
                PCWSTR(destination.as_ptr()),
                0,
                None,
            )
        }
    }

    fn parse_json_string(source: &str, key: &str) -> Option<String> {
        let needle = format!("\"{key}\"");
        let start = source.find(&needle)?;
        let value_start = source[start + needle.len()..].find('"')? + start + needle.len() + 1;
        let value_end = source[value_start..].find('"')? + value_start;
        Some(source[value_start..value_end].to_string())
    }

    fn generate_background(width: i32, height: i32) -> Vec<u8> {
        let mut pixels = vec![255; (width * height * 4) as usize];

        for y in 0..height {
            for x in 0..width {
                let offset = ((y * width + x) * 4) as usize;
                pixels[offset] = 255;
                pixels[offset + 1] = 255;
                pixels[offset + 2] = 255;
                pixels[offset + 3] = 255;
            }
        }

        pixels
    }

    fn to_wide(value: &str) -> Vec<u16> {
        OsStr::new(value)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    fn to_wide_path(path: &Path) -> Vec<u16> {
        path.as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    fn rgb(red: u8, green: u8, blue: u8) -> COLORREF {
        COLORREF(red as u32 | ((green as u32) << 8) | ((blue as u32) << 16))
    }

    fn font_pitch_family() -> u32 {
        u32::from(DEFAULT_PITCH.0) | u32::from(FF_DONTCARE.0)
    }
}

#[cfg(target_os = "windows")]
fn main() {
    windows_installer::main();
}
