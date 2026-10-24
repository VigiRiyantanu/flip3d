// Original Windhawk mod by caliberda
// Ported to standalone Rust by Antigravity

use windows::core::{Result, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, RECT};
use windows::Win32::Graphics::Direct2D::Common::{
    D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_BORDER_MODE_HARD, D2D1_COLOR_F, D2D1_PIXEL_FORMAT,
    D2D_RECT_F, D2D_SIZE_U,
};
use windows::Win32::Graphics::Direct2D::{
    CLSID_D2D1GaussianBlur, ID2D1Bitmap1, ID2D1DeviceContext, D2D1_BITMAP_OPTIONS_TARGET,
    D2D1_BITMAP_PROPERTIES1, D2D1_GAUSSIANBLUR_PROP_BORDER_MODE,
    D2D1_GAUSSIANBLUR_PROP_OPTIMIZATION, D2D1_GAUSSIANBLUR_PROP_STANDARD_DEVIATION,
    D2D1_INTERPOLATION_MODE_LINEAR,
};
use windows::Win32::Graphics::Dwm::{
    DwmRegisterThumbnail, DwmUnregisterThumbnail, DwmUpdateThumbnailProperties,
    DWM_THUMBNAIL_PROPERTIES, DWM_TNP_OPACITY, DWM_TNP_RECTDESTINATION, DWM_TNP_RECTSOURCE,
    DWM_TNP_SOURCECLIENTAREAONLY, DWM_TNP_VISIBLE,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::Graphics::Gdi::IntersectRect;
use windows::Win32::Graphics::Imaging::{
    GUID_WICPixelFormat32bppPBGRA, IWICImagingFactory, WICBitmapDitherTypeNone,
    WICBitmapInterpolationModeFant, WICBitmapPaletteTypeMedianCut, WICDecodeMetadataCacheOnDemand,
};
use windows::Win32::Storage::FileSystem::{
    GetFileAttributesExW, GetFileExInfoStandard, WIN32_FILE_ATTRIBUTE_DATA,
};
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};
use windows::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};
use windows::Win32::UI::Shell::{
    DesktopWallpaper, IDesktopWallpaper, DESKTOP_WALLPAPER_POSITION, DWPOS_CENTER, DWPOS_FILL,
    DWPOS_FIT, DWPOS_STRETCH,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetClassNameW, GetShellWindow, GetWindowRect, GetWindowThreadProcessId,
    IsWindowVisible, SetWindowPos, ShowWindow, SystemParametersInfoW, HWND_TOPMOST,
    SPI_GETDESKWALLPAPER, SWP_NOACTIVATE, SWP_SHOWWINDOW, SW_HIDE,
};

use crate::settings::{BackgroundMode, Settings};

pub const GAUSSIAN_BLUR_OPTIMIZATION_QUALITY: u32 = 2;

pub fn get_file_write_time(path: &str) -> u64 {
    if path.is_empty() {
        return 0;
    }
    let wide: Vec<u16> = format!("{}\0", path).encode_utf16().collect();
    let mut data = WIN32_FILE_ATTRIBUTE_DATA::default();
    // SAFETY: GetFileAttributesExW reads file timestamps.
    let ok = unsafe {
        GetFileAttributesExW(
            PCWSTR::from_raw(wide.as_ptr()),
            GetFileExInfoStandard,
            &mut data as *mut _ as *mut _,
        )
    };
    if ok.is_ok() {
        ((data.ftLastWriteTime.dwHighDateTime as u64) << 32)
            | (data.ftLastWriteTime.dwLowDateTime as u64)
    } else {
        0
    }
}

pub fn get_transcoded_wallpaper_path() -> String {
    if let Ok(appdata) = std::env::var("APPDATA") {
        format!(
            "{}\\Microsoft\\Windows\\Themes\\TranscodedWallpaper",
            appdata
        )
    } else {
        String::new()
    }
}

pub fn is_solid_color_background() -> bool {
    let mut val_type = 0u32;
    let mut size = std::mem::size_of::<u32>() as u32;
    let subkey: Vec<u16> = "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Wallpapers\0"
        .encode_utf16()
        .collect();
    let val_name: Vec<u16> = "BackgroundType\0".encode_utf16().collect();

    // SAFETY: RegGetValueW safely reads DWORD registry value.
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            PCWSTR::from_raw(subkey.as_ptr()),
            PCWSTR::from_raw(val_name.as_ptr()),
            RRF_RT_REG_DWORD,
            None,
            Some(&mut val_type as *mut _ as *mut _),
            Some(&mut size),
        )
    };
    status == windows::Win32::Foundation::ERROR_SUCCESS && val_type == 1
}

pub fn decode_wallpaper(
    factory: &IWICImagingFactory,
    ctx: &ID2D1DeviceContext,
    path: &str,
    width: u32,
    height: u32,
    position: DESKTOP_WALLPAPER_POSITION,
    rect: &mut D2D_RECT_F,
) -> Option<ID2D1Bitmap1> {
    if path.is_empty() {
        return None;
    }
    let path_w: Vec<u16> = format!("{}\0", path).encode_utf16().collect();

    // SAFETY: Decode wallpaper image and fit/scale according to position.
    unsafe {
        let decoder = factory
            .CreateDecoderFromFilename(
                PCWSTR::from_raw(path_w.as_ptr()),
                None,
                windows::Win32::Foundation::GENERIC_READ,
                WICDecodeMetadataCacheOnDemand,
            )
            .ok()?;

        let frame = decoder.GetFrame(0).ok()?;
        let mut img_w = 0u32;
        let mut img_h = 0u32;
        frame.GetSize(&mut img_w, &mut img_h).ok()?;
        if img_w == 0 || img_h == 0 {
            return None;
        }

        let mut draw_w = width as f32;
        let mut draw_h = height as f32;
        let scale_x = width as f32 / img_w as f32;
        let scale_y = height as f32 / img_h as f32;

        match position {
            DWPOS_FIT => {
                let s = scale_x.min(scale_y);
                draw_w = img_w as f32 * s;
                draw_h = img_h as f32 * s;
            }
            DWPOS_STRETCH => {}
            DWPOS_CENTER => {
                draw_w = img_w as f32;
                draw_h = img_h as f32;
            }
            _ => {
                // DWPOS_FILL, SPAN, TILE
                let s = scale_x.max(scale_y);
                draw_w = img_w as f32 * s;
                draw_h = img_h as f32 * s;
            }
        }

        *rect = D2D_RECT_F {
            left: (width as f32 - draw_w) / 2.0,
            top: (height as f32 - draw_h) / 2.0,
            right: (width as f32 + draw_w) / 2.0,
            bottom: (height as f32 + draw_h) / 2.0,
        };

        let scaled_w = (draw_w.round() as u32).max(1);
        let scaled_h = (draw_h.round() as u32).max(1);

        let scaler = factory.CreateBitmapScaler().ok()?;
        scaler
            .Initialize(&frame, scaled_w, scaled_h, WICBitmapInterpolationModeFant)
            .ok()?;

        let converter = factory.CreateFormatConverter().ok()?;
        converter
            .Initialize(
                &scaler,
                &GUID_WICPixelFormat32bppPBGRA,
                WICBitmapDitherTypeNone,
                None,
                0.0,
                WICBitmapPaletteTypeMedianCut,
            )
            .ok()?;

        ctx.CreateBitmapFromWicBitmap(&converter, None).ok()
    }
}

pub struct DesktopWindowsContext {
    pub windows: Vec<HWND>,
    pub shell_process_id: u32,
    pub monitor: RECT,
}

unsafe extern "system" fn desktop_windows_proc(hwnd: HWND, lparam: LPARAM) -> windows_core::BOOL {
    let ctx = &mut *(lparam.0 as *mut DesktopWindowsContext);
    let mut class_buf = [0u16; 16];
    let mut pid = 0u32;
    let mut rect = RECT::default();
    let mut part = RECT::default();

    if IsWindowVisible(hwnd).as_bool() {
        let len = GetClassNameW(hwnd, &mut class_buf);
        if len > 0 {
            let class_str = String::from_utf16_lossy(&class_buf[..len as usize]);
            if (class_str == "Progman" || class_str == "WorkerW")
                && (GetWindowThreadProcessId(hwnd, Some(&mut pid)) != 0
                    && pid == ctx.shell_process_id)
                && GetWindowRect(hwnd, &mut rect).is_ok()
                && IntersectRect(&mut part, &rect, &ctx.monitor).as_bool()
            {
                ctx.windows.push(hwnd);
            }
        }
    }
    windows_core::BOOL(1)
}

pub struct BackgroundManager {
    pub background: Option<ID2D1Bitmap1>,
    pub background_key: String,
    pub backdrop: HWND,
    pub desktop_thumbnails: Vec<isize>,
    pub backdrop_opacity: u8,
}

impl BackgroundManager {
    pub fn new(backdrop: HWND) -> Self {
        Self {
            background: None,
            background_key: String::new(),
            backdrop,
            desktop_thumbnails: Vec::new(),
            backdrop_opacity: 0,
        }
    }

    pub fn show_backdrop(&mut self, monitor: RECT, overlay_hwnd: HWND) {
        self.hide_backdrop();
        let shell = unsafe { GetShellWindow() };
        if self.backdrop.0.is_null() || shell.0.is_null() {
            return;
        }

        let mut shell_pid = 0u32;
        unsafe {
            GetWindowThreadProcessId(shell, Some(&mut shell_pid));
        }

        let mut ctx = DesktopWindowsContext {
            windows: Vec::new(),
            shell_process_id: shell_pid,
            monitor,
        };

        // SAFETY: EnumWindows to gather Progman/WorkerW desktop windows.
        unsafe {
            let _ = EnumWindows(
                Some(desktop_windows_proc),
                LPARAM(&mut ctx as *mut _ as isize),
            );
        }

        // Register bottom one first so thumbnails stack identical to windows
        for &hwnd in ctx.windows.iter().rev() {
            let mut rect = RECT::default();
            let mut part = RECT::default();

            // SAFETY: Register thumbnails for backdrop window.
            unsafe {
                if GetWindowRect(hwnd, &mut rect).is_err()
                    || !IntersectRect(&mut part, &rect, &monitor).as_bool()
                {
                    continue;
                }

                let thumbnail = match DwmRegisterThumbnail(self.backdrop, hwnd) {
                    Ok(t) => t,
                    Err(_) => continue,
                };

                let props = DWM_THUMBNAIL_PROPERTIES {
                    dwFlags: DWM_TNP_RECTDESTINATION
                        | DWM_TNP_RECTSOURCE
                        | DWM_TNP_VISIBLE
                        | DWM_TNP_OPACITY
                        | DWM_TNP_SOURCECLIENTAREAONLY,
                    rcSource: RECT {
                        left: part.left - rect.left,
                        top: part.top - rect.top,
                        right: part.right - rect.left,
                        bottom: part.bottom - rect.top,
                    },
                    rcDestination: RECT {
                        left: part.left - monitor.left,
                        top: part.top - monitor.top,
                        right: part.right - monitor.left,
                        bottom: part.bottom - monitor.top,
                    },
                    opacity: 0,
                    fVisible: windows_core::BOOL(1),
                    fSourceClientAreaOnly: windows_core::BOOL(0),
                };

                if DwmUpdateThumbnailProperties(thumbnail, &props).is_ok() {
                    self.desktop_thumbnails.push(thumbnail);
                } else {
                    let _ = DwmUnregisterThumbnail(thumbnail);
                }
            }
        }

        if self.desktop_thumbnails.is_empty() {
            log::info!("Live desktop thumbnails not available, using wallpaper");
            return;
        }

        self.backdrop_opacity = 0;
        // SAFETY: Position backdrop right under overlay window.
        unsafe {
            let insert_after = if IsWindowVisible(overlay_hwnd).as_bool() {
                overlay_hwnd
            } else {
                HWND_TOPMOST
            };
            let _ = SetWindowPos(
                self.backdrop,
                Some(insert_after),
                monitor.left,
                monitor.top,
                monitor.right - monitor.left,
                monitor.bottom - monitor.top,
                SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );
        }
    }

    pub fn set_backdrop_opacity(&mut self, opacity: f32) {
        let value = (opacity.clamp(0.0, 1.0) * 255.0).round() as u8;
        if value == self.backdrop_opacity {
            return;
        }
        self.backdrop_opacity = value;

        let props = DWM_THUMBNAIL_PROPERTIES {
            dwFlags: DWM_TNP_OPACITY,
            opacity: value,
            ..Default::default()
        };

        // SAFETY: DwmUpdateThumbnailProperties for all backdrop thumbnails.
        for &thumb in &self.desktop_thumbnails {
            unsafe {
                let _ = DwmUpdateThumbnailProperties(thumb, &props);
            }
        }
    }

    pub fn hide_backdrop(&mut self) {
        // SAFETY: Unregister all thumbnails and hide backdrop.
        for &thumb in &self.desktop_thumbnails {
            unsafe {
                let _ = DwmUnregisterThumbnail(thumb);
            }
        }
        self.desktop_thumbnails.clear();
        if !self.backdrop.0.is_null() {
            unsafe {
                let _ = ShowWindow(self.backdrop, SW_HIDE);
            }
        }
    }

    pub fn update_background(
        &mut self,
        settings: &Settings,
        monitor: RECT,
        width: u32,
        height: u32,
        ctx: &ID2D1DeviceContext,
        wic_factory: &IWICImagingFactory,
    ) {
        if settings.background == BackgroundMode::Dim || !self.desktop_thumbnails.is_empty() {
            self.background = None;
            self.background_key.clear();
            return;
        }

        let mut path = String::new();
        let mut color = COLORREF(0);
        let mut position = DWPOS_FILL;

        // SAFETY: Query IDesktopWallpaper COM interface.
        unsafe {
            let wallpaper_res: Result<IDesktopWallpaper> =
                CoCreateInstance(&DesktopWallpaper, None, CLSCTX_ALL);
            if let Ok(wp) = wallpaper_res {
                if let Ok(c) = wp.GetBackgroundColor() {
                    color = c;
                }
                if let Ok(pos) = wp.GetPosition() {
                    position = pos;
                }

                let count = wp.GetMonitorDevicePathCount().unwrap_or(0);
                let center_x = (monitor.left + monitor.right) / 2;
                let center_y = (monitor.top + monitor.bottom) / 2;
                let mut best_dist = i64::MAX;

                for i in 0..count {
                    if let Ok(dev_path) = wp.GetMonitorDevicePathAt(i) {
                        let dev_pcwstr = PCWSTR(dev_path.0);
                        if let (Ok(m_rect), Ok(mon_path)) =
                            (wp.GetMonitorRECT(dev_pcwstr), wp.GetWallpaper(dev_pcwstr))
                        {
                            let dx = ((m_rect.left + m_rect.right) / 2 - center_x) as i64;
                            let dy = ((m_rect.top + m_rect.bottom) / 2 - center_y) as i64;
                            let dist = dx * dx + dy * dy;
                            if dist < best_dist {
                                best_dist = dist;
                                path = mon_path.to_string().unwrap_or_default();
                            }
                            windows::Win32::System::Com::CoTaskMemFree(Some(
                                mon_path.0 as *const _,
                            ));
                        }
                        windows::Win32::System::Com::CoTaskMemFree(Some(dev_path.0 as *const _));
                    }
                }
            }
        }

        if path.is_empty() {
            let mut desktop_path = [0u16; 512];
            // SAFETY: SystemParametersInfoW fallback.
            unsafe {
                if SystemParametersInfoW(
                    SPI_GETDESKWALLPAPER,
                    desktop_path.len() as u32,
                    Some(desktop_path.as_mut_ptr() as *mut _),
                    windows::Win32::UI::WindowsAndMessaging::SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
                )
                .is_ok()
                {
                    path = String::from_utf16_lossy(&desktop_path)
                        .trim_matches('\0')
                        .to_string();
                }
            }
        }

        let solid_color = path.is_empty() && is_solid_color_background();
        let transcoded = if solid_color {
            String::new()
        } else {
            get_transcoded_wallpaper_path()
        };

        let key = format!(
            "{}|{}|{}|{}|{}x{}|{}|{}|{}|{}",
            path,
            get_file_write_time(&path),
            transcoded,
            get_file_write_time(&transcoded),
            width,
            height,
            color.0,
            position.0,
            settings.background as usize,
            settings.blur_amount as i32
        );

        if key == self.background_key {
            return;
        }
        self.background = None;
        self.background_key = key;

        let mut image = None;
        let mut image_rect = D2D_RECT_F::default();
        if !solid_color {
            image = decode_wallpaper(
                wic_factory,
                ctx,
                &path,
                width,
                height,
                position,
                &mut image_rect,
            );
            if image.is_none() {
                image = decode_wallpaper(
                    wic_factory,
                    ctx,
                    &transcoded,
                    width,
                    height,
                    position,
                    &mut image_rect,
                );
            }
            if image.is_none() {
                return;
            }
        }

        // SAFETY: Compose wallpaper image with background color on a target bitmap.
        unsafe {
            let props = D2D1_BITMAP_PROPERTIES1 {
                pixelFormat: D2D1_PIXEL_FORMAT {
                    format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                },
                dpiX: 96.0,
                dpiY: 96.0,
                bitmapOptions: D2D1_BITMAP_OPTIONS_TARGET,
                colorContext: std::mem::ManuallyDrop::new(None),
            };

            let composed = match ctx.CreateBitmap(D2D_SIZE_U { width, height }, None, 0, &props) {
                Ok(b) => b,
                Err(_) => return,
            };

            ctx.SetTarget(&composed);
            ctx.BeginDraw();
            ctx.Clear(Some(&D2D1_COLOR_F {
                r: (color.0 & 0xff) as f32 / 255.0,
                g: ((color.0 >> 8) & 0xff) as f32 / 255.0,
                b: ((color.0 >> 16) & 0xff) as f32 / 255.0,
                a: 1.0,
            }));

            if let Some(img) = &image {
                ctx.DrawBitmap(
                    img,
                    Some(&image_rect),
                    1.0,
                    D2D1_INTERPOLATION_MODE_LINEAR,
                    None,
                    None,
                );
            }
            let _ = ctx.EndDraw(None, None);
            ctx.SetTarget(None);

            if settings.background != BackgroundMode::BlurredWallpaper {
                self.background = Some(composed);
                return;
            }

            // Apply Gaussian Blur effect
            if let Ok(blur) = ctx.CreateEffect(&CLSID_D2D1GaussianBlur) {
                if let Ok(blurred) = ctx.CreateBitmap(D2D_SIZE_U { width, height }, None, 0, &props)
                {
                    blur.SetInput(0, &composed, true);
                    let std_dev = settings.blur_amount;
                    let bytes = std::slice::from_raw_parts(
                        &std_dev as *const _ as *const u8,
                        std::mem::size_of::<f32>(),
                    );
                    let _ = blur.SetValue(
                        D2D1_GAUSSIANBLUR_PROP_STANDARD_DEVIATION.0 as u32,
                        windows::Win32::Graphics::Direct2D::D2D1_PROPERTY_TYPE_FLOAT,
                        bytes,
                    );
                    let border_mode = D2D1_BORDER_MODE_HARD.0 as u32;
                    let bytes = std::slice::from_raw_parts(
                        &border_mode as *const _ as *const u8,
                        std::mem::size_of::<u32>(),
                    );
                    let _ = blur.SetValue(
                        D2D1_GAUSSIANBLUR_PROP_BORDER_MODE.0 as u32,
                        windows::Win32::Graphics::Direct2D::D2D1_PROPERTY_TYPE_UINT32,
                        bytes,
                    );
                    let opt = GAUSSIAN_BLUR_OPTIMIZATION_QUALITY;
                    let bytes = std::slice::from_raw_parts(
                        &opt as *const _ as *const u8,
                        std::mem::size_of::<u32>(),
                    );
                    let _ = blur.SetValue(
                        D2D1_GAUSSIANBLUR_PROP_OPTIMIZATION.0 as u32,
                        windows::Win32::Graphics::Direct2D::D2D1_PROPERTY_TYPE_UINT32,
                        bytes,
                    );

                    ctx.SetTarget(&blurred);
                    ctx.BeginDraw();
                    ctx.Clear(Some(&D2D1_COLOR_F {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 1.0,
                    }));
                    if let Ok(blur_out) = blur.GetOutput() {
                        ctx.DrawImage(&blur_out, None, None, windows::Win32::Graphics::Direct2D::D2D1_INTERPOLATION_MODE_LINEAR, windows::Win32::Graphics::Direct2D::Common::D2D1_COMPOSITE_MODE_SOURCE_OVER);
                    }
                    if ctx.EndDraw(None, None).is_ok() {
                        self.background = Some(blurred);
                    } else {
                        self.background = Some(composed);
                    }
                    ctx.SetTarget(None);
                    return;
                }
            }

            self.background = Some(composed);
        }
    }
}
