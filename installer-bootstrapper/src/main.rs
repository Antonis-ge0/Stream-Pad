#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("The Stream Pad installer bootstrapper only runs on Windows.");
}

#[cfg(target_os = "windows")]
mod windows_installer {
    use std::{
        env,
        ffi::OsStr,
        fs::{self, File},
        io::{Read, Write},
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
                Arc, BeginPaint, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateFontW,
                CreatePen, CreateSolidBrush, DeleteDC, DeleteObject, DrawTextW, Ellipse, EndPaint,
                InvalidateRect, LineTo, MoveToEx, RoundRect, SelectObject, SetBkMode,
                SetTextColor, StretchDIBits, BITMAPINFO,
                BITMAPINFOHEADER, CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, DEFAULT_CHARSET,
                DEFAULT_PITCH, DIB_RGB_COLORS, DT_CENTER, DT_SINGLELINE, DT_VCENTER,
                FF_DONTCARE, FW_BOLD, FW_NORMAL, HDC, OUT_DEFAULT_PRECIS, PAINTSTRUCT, PS_SOLID,
                SRCCOPY, TRANSPARENT,
            },
            Storage::FileSystem::GetDiskFreeSpaceExW,
            System::LibraryLoader::GetModuleHandleW,
            UI::{
                Input::KeyboardAndMouse::ReleaseCapture,
                WindowsAndMessaging::{
                    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW,
                    GetSystemMetrics, LoadCursorW, PostMessageW, PostQuitMessage, RegisterClassW,
                    SendMessageW, SetTimer, ShowWindow, TranslateMessage, CS_HREDRAW, CS_VREDRAW,
                    CW_USEDEFAULT, HTCAPTION, IDC_ARROW, MSG, SM_CXSCREEN, SM_CYSCREEN, SW_SHOW,
                    WINDOW_EX_STYLE, WM_CLOSE, WM_DESTROY, WM_ERASEBKGND, WM_LBUTTONDOWN,
                    WM_NCLBUTTONDOWN, WM_PAINT, WM_TIMER, WNDCLASSW, WS_POPUP, WS_VISIBLE,
                },
            },
        },
    };
    use winreg::{enums::*, RegKey};

    const WINDOW_WIDTH: i32 = 980;
    const WINDOW_HEIGHT: i32 = 640;
    const LOGO_SIZE: i32 = 160;
    const ACTION_BUTTON_WIDTH: i32 = 162;
    const ACTION_BUTTON_HEIGHT: i32 = 46;
    const ACTION_BUTTON_LEFT: i32 = WINDOW_WIDTH / 2 - ACTION_BUTTON_WIDTH / 2;
    const ACTION_BUTTON_TOP: i32 = 508;
    const LOGO_BITMAP: &[u8] = include_bytes!("../../src-tauri/installer/app-logo-160.bgra");
    const TIMER_ID: usize = 1;
    const TIMER_MS: u32 = 16;
    const RELEASE_JSON_URL: &str =
        "https://github.com/Antonis-ge0/Stream-Pad/releases/latest/download/latest.json";
    const RELEASE_DOWNLOAD_BASE: &str = "https://github.com/Antonis-ge0/Stream-Pad/releases";

    static APP_STATE: OnceLock<Arc<AppState>> = OnceLock::new();
    static STARTED_AT: OnceLock<Instant> = OnceLock::new();
    static BACKGROUND: OnceLock<Vec<u8>> = OnceLock::new();

    #[derive(Clone, Copy, PartialEq, Eq)]
    enum AppMode {
        Install,
        Uninstall,
    }

    #[derive(Clone, Copy, PartialEq, Eq)]
    enum ViewPhase {
        Ready,
        Preparing,
        Downloading,
        Installing,
        Removing,
        Complete,
        Failed,
    }

    #[derive(Clone)]
    struct DownloadView {
        downloaded: u64,
        total: Option<u64>,
        bytes_per_second: f64,
    }

    #[derive(Clone)]
    struct InstallerView {
        phase: ViewPhase,
        detail: String,
        status: String,
        install_path: String,
        free_space: String,
        download: Option<DownloadView>,
    }

    struct AppState {
        mode: AppMode,
        uninstall_path: Option<PathBuf>,
        view: Mutex<InstallerView>,
    }

    impl AppState {
        fn new(mode: AppMode, uninstall_path: Option<PathBuf>) -> Self {
            let install_path = match mode {
                AppMode::Install => display_install_path(),
                AppMode::Uninstall => uninstall_path
                    .as_deref()
                    .and_then(Path::parent)
                    .map(format_path)
                    .unwrap_or_else(display_install_path),
            };

            let detail = match mode {
                AppMode::Install => "Ready to install Stream Pad.".to_string(),
                AppMode::Uninstall => "Ready to uninstall Stream Pad.".to_string(),
            };

            Self {
                mode,
                uninstall_path,
                view: Mutex::new(InstallerView {
                    phase: ViewPhase::Ready,
                    detail,
                    status: "Ready".to_string(),
                    install_path,
                    free_space: c_drive_free_space(),
                    download: None,
                }),
            }
        }

        fn begin(&self) -> bool {
            if let Ok(mut view) = self.view.lock() {
                if view.phase != ViewPhase::Ready {
                    return false;
                }

                view.phase = match self.mode {
                    AppMode::Install => ViewPhase::Preparing,
                    AppMode::Uninstall => ViewPhase::Removing,
                };
                view.status = match self.mode {
                    AppMode::Install => "Preparing".to_string(),
                    AppMode::Uninstall => "Uninstalling".to_string(),
                };
                view.detail = match self.mode {
                    AppMode::Install => "Preparing Stream Pad setup.".to_string(),
                    AppMode::Uninstall => "Please wait while we remove Stream Pad.".to_string(),
                };
                view.download = None;
                return true;
            }

            false
        }

        fn set_phase(&self, phase: ViewPhase, status: impl Into<String>, detail: impl Into<String>) {
            if let Ok(mut view) = self.view.lock() {
                view.phase = phase;
                view.status = status.into();
                view.detail = detail.into();
                if phase != ViewPhase::Downloading {
                    view.download = None;
                }
            }
        }

        fn set_download_progress(
            &self,
            downloaded: u64,
            total: Option<u64>,
            bytes_per_second: f64,
        ) {
            if let Ok(mut view) = self.view.lock() {
                view.phase = ViewPhase::Downloading;
                view.status = "Downloading".to_string();
                view.detail = "Please wait while we install Stream Pad.".to_string();
                view.download = Some(DownloadView {
                    downloaded,
                    total,
                    bytes_per_second,
                });
            }
        }

        fn set_error(&self, detail: impl Into<String>) {
            self.set_phase(ViewPhase::Failed, "Failed", detail);
        }

        fn snapshot(&self) -> InstallerView {
            self.view.lock().map(|view| view.clone()).unwrap_or_else(|_| InstallerView {
                phase: ViewPhase::Failed,
                detail: "The installer state could not be read.".to_string(),
                status: "Failed".to_string(),
                install_path: display_install_path(),
                free_space: c_drive_free_space(),
                download: None,
            })
        }
    }

    enum LaunchMode {
        Install,
        Uninstall { uninstall_path: PathBuf },
        Relaunched,
    }

    pub fn main() {
        match resolve_launch_mode() {
            LaunchMode::Relaunched => return,
            LaunchMode::Install => run_window(AppMode::Install, None),
            LaunchMode::Uninstall { uninstall_path } => {
                run_window(AppMode::Uninstall, Some(uninstall_path))
            }
        }
    }

    fn run_window(mode: AppMode, uninstall_path: Option<PathBuf>) {
        let state = Arc::new(AppState::new(mode, uninstall_path));
        let _ = APP_STATE.set(Arc::clone(&state));
        let _ = STARTED_AT.set(Instant::now());
        let _ = BACKGROUND.set(generate_background(WINDOW_WIDTH, WINDOW_HEIGHT));

        unsafe {
            if create_window(mode).is_some() {
                message_loop();
            }
        }
    }

    fn resolve_launch_mode() -> LaunchMode {
        let mut args = env::args_os().skip(1);
        while let Some(arg) = args.next() {
            if arg == "--uninstall" {
                let uninstall_path = args
                    .next()
                    .map(PathBuf::from)
                    .unwrap_or_else(default_uninstaller_path);

                return relaunch_uninstaller_from_temp(uninstall_path);
            }

            if arg == "--uninstall-temp" {
                let uninstall_path = args
                    .next()
                    .map(PathBuf::from)
                    .unwrap_or_else(default_uninstaller_path);
                return LaunchMode::Uninstall { uninstall_path };
            }
        }

        LaunchMode::Install
    }

    fn relaunch_uninstaller_from_temp(uninstall_path: PathBuf) -> LaunchMode {
        let temp_dir = env::temp_dir().join("stream-pad-uninstall");
        let temp_exe = temp_dir.join("Stream Pad Uninstaller.exe");
        let current_exe = match env::current_exe() {
            Ok(path) => path,
            Err(_) => return LaunchMode::Uninstall { uninstall_path },
        };

        if fs::create_dir_all(&temp_dir).is_err() || fs::copy(&current_exe, &temp_exe).is_err() {
            return LaunchMode::Uninstall { uninstall_path };
        }

        if Command::new(&temp_exe)
            .arg("--uninstall-temp")
            .arg(uninstall_path)
            .spawn()
            .is_ok()
        {
            LaunchMode::Relaunched
        } else {
            LaunchMode::Uninstall {
                uninstall_path: default_uninstaller_path(),
            }
        }
    }

    unsafe fn create_window(mode: AppMode) -> Option<HWND> {
        let module = GetModuleHandleW(None).ok()?;
        let instance = HINSTANCE(module.0);
        let class_name = match mode {
            AppMode::Install => w!("StreamPadVisualInstaller"),
            AppMode::Uninstall => w!("StreamPadVisualUninstaller"),
        };
        let title = match mode {
            AppMode::Install => w!("Stream Pad Installer"),
            AppMode::Uninstall => w!("Stream Pad Uninstaller"),
        };
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
            title,
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
            WM_ERASEBKGND => LRESULT(1),
            WM_TIMER => {
                let _ = InvalidateRect(Some(hwnd), None, false);
                LRESULT(0)
            }
            WM_LBUTTONDOWN => {
                let x = (lparam.0 & 0xffff) as i16 as i32;
                let y = ((lparam.0 >> 16) & 0xffff) as i16 as i32;

                if x >= WINDOW_WIDTH - 42 && y <= 42 {
                    PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0)).ok();
                } else if action_button_contains(x, y) {
                    if let Some(state) = APP_STATE.get() {
                        if state.snapshot().phase == ViewPhase::Ready && state.begin() {
                            start_worker(hwnd, Arc::clone(state));
                        }
                    }
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

        let buffer_dc = CreateCompatibleDC(Some(hdc));
        let buffer_bitmap = CreateCompatibleBitmap(hdc, WINDOW_WIDTH, WINDOW_HEIGHT);

        if !buffer_dc.is_invalid() && !buffer_bitmap.is_invalid() {
            let old_bitmap = SelectObject(buffer_dc, buffer_bitmap.into());
            draw_background(buffer_dc);
            draw_close_button(buffer_dc);
            draw_brand(buffer_dc);
            draw_status(buffer_dc);
            let _ = BitBlt(
                hdc,
                0,
                0,
                WINDOW_WIDTH,
                WINDOW_HEIGHT,
                Some(buffer_dc),
                0,
                0,
                SRCCOPY,
            );
            SelectObject(buffer_dc, old_bitmap);
        } else {
            draw_background(hdc);
            draw_close_button(hdc);
            draw_brand(hdc);
            draw_status(hdc);
        }

        if !buffer_bitmap.is_invalid() {
            let _ = DeleteObject(buffer_bitmap.into());
        }
        if !buffer_dc.is_invalid() {
            let _ = DeleteDC(buffer_dc);
        }

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
        draw_app_logo(hdc, WINDOW_WIDTH / 2 - LOGO_SIZE / 2, 76);

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
                top: 256,
                right: WINDOW_WIDTH,
                bottom: 306,
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
                top: 308,
                right: WINDOW_WIDTH,
                bottom: 334,
            },
            version_font,
            rgb(76, 83, 92),
        );

        let _ = DeleteObject(title_font.into());
        let _ = DeleteObject(version_font.into());
    }

    unsafe fn draw_status(hdc: HDC) {
        let Some(state) = APP_STATE.get() else {
            return;
        };
        let view = state.snapshot();

        match view.phase {
            ViewPhase::Ready => draw_ready_state(hdc, state.mode, &view),
            ViewPhase::Downloading => draw_download_state(hdc, &view),
            ViewPhase::Preparing | ViewPhase::Installing | ViewPhase::Removing => {
                draw_spinner_state(hdc, &view, false)
            }
            ViewPhase::Complete => draw_done_state(hdc, &view),
            ViewPhase::Failed => draw_spinner_state(hdc, &view, true),
        }
    }

    unsafe fn draw_ready_state(hdc: HDC, mode: AppMode, view: &InstallerView) {
        let info_font = CreateFontW(
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

        let strong_font = CreateFontW(
            14,
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

        let path_label = match mode {
            AppMode::Install => format!("Install path: {}", view.install_path),
            AppMode::Uninstall => format!("Installed path: {}", view.install_path),
        };
        draw_centered_text(
            hdc,
            &path_label,
            RECT {
                left: 110,
                top: 406,
                right: WINDOW_WIDTH - 110,
                bottom: 432,
            },
            info_font,
            rgb(38, 42, 47),
        );

        let second_line = match mode {
            AppMode::Install => format!("C drive free space: {}", view.free_space),
            AppMode::Uninstall => "Stream Pad will be removed from this PC.".to_string(),
        };
        draw_centered_text(
            hdc,
            &second_line,
            RECT {
                left: 110,
                top: 442,
                right: WINDOW_WIDTH - 110,
                bottom: 468,
            },
            info_font,
            rgb(76, 83, 92),
        );

        draw_action_button(
            hdc,
            match mode {
                AppMode::Install => "Install",
                AppMode::Uninstall => "Uninstall",
            },
            strong_font,
        );

        let _ = DeleteObject(info_font.into());
        let _ = DeleteObject(strong_font.into());
    }

    unsafe fn draw_download_state(hdc: HDC, view: &InstallerView) {
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
            &view.detail,
            RECT {
                left: 0,
                top: 440,
                right: WINDOW_WIDTH,
                bottom: 470,
            },
            detail_font,
            rgb(76, 83, 92),
        );

        let download = view.download.clone().unwrap_or(DownloadView {
            downloaded: 0,
            total: None,
            bytes_per_second: 0.0,
        });
        draw_progress_bar(hdc, 290, 504, 400, 10, progress_fraction(&download));

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
            &format_download_status(&download),
            RECT {
                left: 0,
                top: 528,
                right: WINDOW_WIDTH,
                bottom: 552,
            },
            status_font,
            rgb(10, 14, 18),
        );

        let _ = DeleteObject(detail_font.into());
        let _ = DeleteObject(status_font.into());
    }

    unsafe fn draw_spinner_state(hdc: HDC, view: &InstallerView, failed: bool) {
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
            &view.detail,
            RECT {
                left: 0,
                top: 444,
                right: WINDOW_WIDTH,
                bottom: 474,
            },
            detail_font,
            if failed {
                rgb(239, 68, 68)
            } else {
                rgb(76, 83, 92)
            },
        );

        draw_spinner(hdc, WINDOW_WIDTH / 2, 520, 17, failed);

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
            &view.status,
            RECT {
                left: 0,
                top: 548,
                right: WINDOW_WIDTH,
                bottom: 574,
            },
            status_font,
            rgb(10, 14, 18),
        );

        let _ = DeleteObject(detail_font.into());
        let _ = DeleteObject(status_font.into());
    }

    unsafe fn draw_done_state(hdc: HDC, view: &InstallerView) {
        let detail_font = CreateFontW(
            14,
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
            &view.detail,
            RECT {
                left: 0,
                top: 484,
                right: WINDOW_WIDTH,
                bottom: 520,
            },
            detail_font,
            rgb(10, 14, 18),
        );
        let _ = DeleteObject(detail_font.into());
    }

    unsafe fn draw_action_button(hdc: HDC, label: &str, font: windows::Win32::Graphics::Gdi::HFONT) {
        let brush = CreateSolidBrush(rgb(10, 14, 18));
        let pen = CreatePen(PS_SOLID, 1, rgb(10, 14, 18));
        let old_brush = SelectObject(hdc, brush.into());
        let old_pen = SelectObject(hdc, pen.into());

        let _ = RoundRect(
            hdc,
            ACTION_BUTTON_LEFT,
            ACTION_BUTTON_TOP,
            ACTION_BUTTON_LEFT + ACTION_BUTTON_WIDTH,
            ACTION_BUTTON_TOP + ACTION_BUTTON_HEIGHT,
            14,
            14,
        );

        SelectObject(hdc, old_pen);
        SelectObject(hdc, old_brush);
        let _ = DeleteObject(pen.into());
        let _ = DeleteObject(brush.into());

        draw_centered_text(
            hdc,
            label,
            RECT {
                left: ACTION_BUTTON_LEFT,
                top: ACTION_BUTTON_TOP,
                right: ACTION_BUTTON_LEFT + ACTION_BUTTON_WIDTH,
                bottom: ACTION_BUTTON_TOP + ACTION_BUTTON_HEIGHT,
            },
            font,
            rgb(255, 255, 255),
        );
    }

    unsafe fn draw_progress_bar(hdc: HDC, x: i32, y: i32, width: i32, height: i32, progress: f64) {
        draw_pill(hdc, x, y, width, height, rgb(248, 248, 247), rgb(226, 226, 224));

        let fill_width = ((width as f64 * progress.clamp(0.0, 1.0)).round() as i32)
            .clamp(0, width);
        if fill_width > 0 {
            if fill_width <= height {
                let brush = CreateSolidBrush(rgb(10, 14, 18));
                let pen = CreatePen(PS_SOLID, 1, rgb(10, 14, 18));
                let old_brush = SelectObject(hdc, brush.into());
                let old_pen = SelectObject(hdc, pen.into());
                let _ = Ellipse(hdc, x, y, x + fill_width.max(2), y + height);
                SelectObject(hdc, old_pen);
                SelectObject(hdc, old_brush);
                let _ = DeleteObject(pen.into());
                let _ = DeleteObject(brush.into());
            } else {
                draw_pill(hdc, x, y, fill_width, height, rgb(10, 14, 18), rgb(10, 14, 18));
            }
        }
    }

    unsafe fn draw_pill(
        hdc: HDC,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        fill: COLORREF,
        stroke: COLORREF,
    ) {
        let brush = CreateSolidBrush(fill);
        let pen = CreatePen(PS_SOLID, 1, stroke);
        let old_brush = SelectObject(hdc, brush.into());
        let old_pen = SelectObject(hdc, pen.into());

        let _ = RoundRect(hdc, x, y, x + width, y + height, height, height);

        SelectObject(hdc, old_pen);
        SelectObject(hdc, old_brush);
        let _ = DeleteObject(pen.into());
        let _ = DeleteObject(brush.into());
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
        let rotation = elapsed * 4.6;
        let start = rotation;
        let end = rotation + std::f32::consts::TAU * 0.78;
        let stroke = 4;
        let box_radius = radius + stroke / 2;
        let white_brush = CreateSolidBrush(rgb(255, 255, 255));
        let track_pen = CreatePen(PS_SOLID, stroke, rgb(205, 207, 211));
        let old_brush = SelectObject(hdc, white_brush.into());
        let old_pen = SelectObject(hdc, track_pen.into());
        let _ = Ellipse(
            hdc,
            center_x - box_radius,
            center_y - box_radius,
            center_x + box_radius,
            center_y + box_radius,
        );
        SelectObject(hdc, old_pen);
        SelectObject(hdc, old_brush);
        let _ = DeleteObject(track_pen.into());
        let _ = DeleteObject(white_brush.into());

        let arc_pen = CreatePen(PS_SOLID, stroke, rgb(10, 14, 18));
        let old_pen = SelectObject(hdc, arc_pen.into());
        let (start_x, start_y) = arc_point(center_x, center_y, box_radius, start);
        let (end_x, end_y) = arc_point(center_x, center_y, box_radius, end);
        let _ = Arc(
            hdc,
            center_x - box_radius,
            center_y - box_radius,
            center_x + box_radius,
            center_y + box_radius,
            start_x,
            start_y,
            end_x,
            end_y,
        );
        SelectObject(hdc, old_pen);
        let _ = DeleteObject(arc_pen.into());

        draw_spinner_cap(hdc, start_x, start_y, stroke / 2, rgb(10, 14, 18));
        draw_spinner_cap(hdc, end_x, end_y, stroke / 2, rgb(10, 14, 18));
    }

    fn arc_point(center_x: i32, center_y: i32, radius: i32, angle: f32) -> (i32, i32) {
        (
            (center_x as f32 + angle.cos() * radius as f32).round() as i32,
            (center_y as f32 - angle.sin() * radius as f32).round() as i32,
        )
    }

    unsafe fn draw_spinner_cap(hdc: HDC, x: i32, y: i32, radius: i32, color: COLORREF) {
        let brush = CreateSolidBrush(color);
        let pen = CreatePen(PS_SOLID, 1, color);
        let old_brush = SelectObject(hdc, brush.into());
        let old_pen = SelectObject(hdc, pen.into());
        let _ = Ellipse(hdc, x - radius, y - radius, x + radius, y + radius);
        SelectObject(hdc, old_pen);
        SelectObject(hdc, old_brush);
        let _ = DeleteObject(pen.into());
        let _ = DeleteObject(brush.into());
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

    fn start_worker(hwnd: HWND, state: Arc<AppState>) {
        let hwnd_value = hwnd.0 as isize;
        thread::spawn(move || {
            let result = match state.mode {
                AppMode::Install => install_stream_pad(&state),
                AppMode::Uninstall => uninstall_stream_pad(&state),
            };

            match result {
                Ok(()) => {
                    let detail = match state.mode {
                        AppMode::Install => "Stream Pad has been installed.",
                        AppMode::Uninstall => "Stream Pad has been removed.",
                    };
                    state.set_phase(ViewPhase::Complete, "Done", detail);
                    thread::sleep(Duration::from_millis(1400));
                    unsafe {
                        let hwnd = HWND(hwnd_value as *mut _);
                        PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0)).ok();
                    }
                }
                Err(error) => state.set_error(error),
            }
        });
    }

    fn install_stream_pad(state: &AppState) -> Result<(), String> {
        state.set_phase(
            ViewPhase::Preparing,
            "Preparing",
            "Preparing Stream Pad setup.",
        );

        let installer = resolve_installer(state)?;
        state.set_phase(
            ViewPhase::Installing,
            "Installing",
            "Please wait while we install Stream Pad.",
        );
        run_inner_installer(&installer)
    }

    fn uninstall_stream_pad(state: &AppState) -> Result<(), String> {
        let uninstall_path = state
            .uninstall_path
            .clone()
            .unwrap_or_else(default_uninstaller_path);

        if !uninstall_path.exists() {
            return Err("Could not find the Stream Pad uninstaller.".to_string());
        }

        state.set_phase(
            ViewPhase::Removing,
            "Uninstalling",
            "Please wait while we remove Stream Pad.",
        );

        let status = Command::new(&uninstall_path)
            .arg("/S")
            .status()
            .map_err(|error| format!("Could not start the Stream Pad uninstaller: {error}"))?;

        if status.success() {
            Ok(())
        } else {
            Err(format!(
                "The Stream Pad uninstaller exited with code {}.",
                status
                    .code()
                    .map(|code| code.to_string())
                    .unwrap_or_else(|| "unknown".to_string())
            ))
        }
    }

    fn resolve_installer(state: &AppState) -> Result<PathBuf, String> {
        if let Some(local_installer) = find_companion_installer() {
            return Ok(local_installer);
        }

        let temp_dir = env::temp_dir().join("stream-pad-installer");
        fs::create_dir_all(&temp_dir)
            .map_err(|error| format!("Could not prepare installer cache: {error}"))?;

        let metadata = download_text(RELEASE_JSON_URL)
            .map_err(|_| "Could not reach the Stream Pad release endpoint.".to_string())?;
        let version = parse_json_string(&metadata, "version")
            .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string());
        let tag = format!("v{version}");
        let installer_name = format!("Stream.Pad_{version}_x64-setup.exe");
        let installer_url = format!("{RELEASE_DOWNLOAD_BASE}/download/{tag}/{installer_name}");
        let installer_path = temp_dir.join(installer_name);

        download_file(&installer_url, &installer_path, state)
            .map_err(|_| "Could not download the Stream Pad setup file.".to_string())?;

        Ok(installer_path)
    }

    fn find_companion_installer() -> Option<PathBuf> {
        let exe_dir = env::current_exe().ok()?.parent()?.to_path_buf();
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

    fn download_text(url: &str) -> Result<String, reqwest::Error> {
        reqwest::blocking::get(url)?.error_for_status()?.text()
    }

    fn download_file(url: &str, destination: &Path, state: &AppState) -> Result<(), String> {
        let mut response = reqwest::blocking::get(url)
            .map_err(|error| error.to_string())?
            .error_for_status()
            .map_err(|error| error.to_string())?;
        let total = response.content_length();
        let mut file = File::create(destination).map_err(|error| error.to_string())?;
        let mut buffer = [0_u8; 32 * 1024];
        let started = Instant::now();
        let mut downloaded = 0_u64;

        state.set_download_progress(0, total, 0.0);

        loop {
            let bytes_read = response
                .read(&mut buffer)
                .map_err(|error| error.to_string())?;
            if bytes_read == 0 {
                break;
            }

            file.write_all(&buffer[..bytes_read])
                .map_err(|error| error.to_string())?;
            downloaded += bytes_read as u64;

            let elapsed = started.elapsed().as_secs_f64().max(0.001);
            state.set_download_progress(downloaded, total, downloaded as f64 / elapsed);
        }

        Ok(())
    }

    fn parse_json_string(source: &str, key: &str) -> Option<String> {
        let needle = format!("\"{key}\"");
        let start = source.find(&needle)?;
        let value_start = source[start + needle.len()..].find('"')? + start + needle.len() + 1;
        let value_end = source[value_start..].find('"')? + value_start;
        Some(source[value_start..value_end].to_string())
    }

    fn default_uninstaller_path() -> PathBuf {
        env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(Path::to_path_buf))
            .unwrap_or_else(display_install_path_buf)
            .join("uninstall.exe")
    }

    fn display_install_path() -> String {
        format_path(&display_install_path_buf())
    }

    fn display_install_path_buf() -> PathBuf {
        installed_path_from_registry().unwrap_or_else(|| {
            let base = env::var_os("ProgramFiles")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from(r"C:\Program Files"));
            base.join("Stream Pad")
        })
    }

    fn installed_path_from_registry() -> Option<PathBuf> {
        let registry_locations = [
            (
                RegKey::predef(HKEY_LOCAL_MACHINE),
                r"Software\Microsoft\Windows\CurrentVersion\Uninstall\Stream Pad",
                "InstallLocation",
            ),
            (
                RegKey::predef(HKEY_CURRENT_USER),
                r"Software\Microsoft\Windows\CurrentVersion\Uninstall\Stream Pad",
                "InstallLocation",
            ),
            (
                RegKey::predef(HKEY_LOCAL_MACHINE),
                r"Software\Antonis Georgosopoulos\Stream Pad",
                "",
            ),
            (
                RegKey::predef(HKEY_CURRENT_USER),
                r"Software\Antonis Georgosopoulos\Stream Pad",
                "",
            ),
        ];

        for (root, key_path, value_name) in registry_locations {
            let Ok(key) = root.open_subkey(key_path) else {
                continue;
            };
            let Ok(value) = key.get_value::<String, _>(value_name) else {
                continue;
            };
            if let Some(path) = clean_registry_path(value) {
                return Some(path);
            }
        }

        None
    }

    fn clean_registry_path(value: String) -> Option<PathBuf> {
        let trimmed = value.trim().trim_matches('"');
        if trimmed.is_empty() {
            None
        } else {
            Some(PathBuf::from(trimmed))
        }
    }

    fn c_drive_free_space() -> String {
        let mut free_bytes = 0_u64;
        let root = to_wide(r"C:\");
        let result = unsafe {
            GetDiskFreeSpaceExW(
                PCWSTR(root.as_ptr()),
                Some(&mut free_bytes),
                None,
                None,
            )
        };

        if result.is_ok() {
            format!("{:.1} GB available", free_bytes as f64 / 1024.0 / 1024.0 / 1024.0)
        } else {
            "Unavailable".to_string()
        }
    }

    fn format_path(path: &Path) -> String {
        path.display().to_string()
    }

    fn progress_fraction(download: &DownloadView) -> f64 {
        match download.total {
            Some(total) if total > 0 => download.downloaded as f64 / total as f64,
            _ => 0.35,
        }
    }

    fn format_download_status(download: &DownloadView) -> String {
        let downloaded_mb = bytes_to_mb(download.downloaded);
        let speed_mb = bytes_to_mb(download.bytes_per_second.max(0.0) as u64);

        match download.total {
            Some(total) if total > 0 => {
                format!(
                    "{downloaded_mb:.1}MB of {:.1}MB ({speed_mb:.2}MB/s)",
                    bytes_to_mb(total)
                )
            }
            _ => format!("{downloaded_mb:.1}MB downloaded ({speed_mb:.2}MB/s)"),
        }
    }

    fn bytes_to_mb(bytes: u64) -> f64 {
        bytes as f64 / 1024.0 / 1024.0
    }

    fn action_button_contains(x: i32, y: i32) -> bool {
        x >= ACTION_BUTTON_LEFT
            && x <= ACTION_BUTTON_LEFT + ACTION_BUTTON_WIDTH
            && y >= ACTION_BUTTON_TOP
            && y <= ACTION_BUTTON_TOP + ACTION_BUTTON_HEIGHT
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
