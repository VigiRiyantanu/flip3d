// Original Windhawk mod by caliberda
// Ported to standalone Rust by Antigravity

use std::collections::HashMap;
use windows::core::{Result, GUID, PCWSTR};
use windows::Win32::Foundation::{HWND, PROPERTYKEY, SIZE};
use windows::Win32::Graphics::Direct2D::Common::{
    D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F, D2D1_PIXEL_FORMAT, D2D_RECT_F,
};
use windows::Win32::Graphics::Direct2D::{
    ID2D1Bitmap1, ID2D1DeviceContext, ID2D1SolidColorBrush, D2D1_BITMAP_OPTIONS_TARGET,
    D2D1_BITMAP_PROPERTIES1, D2D1_INTERPOLATION_MODE_HIGH_QUALITY_CUBIC, D2D1_ROUNDED_RECT,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::Graphics::Gdi::DeleteObject;
use windows::Win32::Graphics::Imaging::{
    GUID_WICPixelFormat32bppPBGRA, IWICBitmap, IWICBitmapSource, IWICImagingFactory,
    WICBitmapCacheOnLoad, WICBitmapDitherTypeNone, WICBitmapPaletteTypeMedianCut,
    WICBitmapUsePremultipliedAlpha,
};
use windows::Win32::System::Com::StructuredStorage::PropVariantClear;
use windows::Win32::UI::Shell::PropertiesSystem::{IPropertyStore, SHGetPropertyStoreForWindow};
use windows::Win32::UI::Shell::{
    IShellItem, IShellItem2, IShellItemImageFactory, SHCreateItemFromParsingName,
    SHCreateItemInKnownFolder, SIGDN_NORMALDISPLAY, SIIGBF_ICONONLY,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetClassLongPtrW, SendMessageTimeoutW, GCLP_HICON, GCLP_HICONSM, HICON, ICON_BIG, ICON_SMALL2,
    SMTO_ABORTIFHUNG, SMTO_BLOCK, WM_GETICON,
};

use crate::windows_list::get_window_process_path;

pub const FOLDERID_APPS_FOLDER: GUID = GUID::from_u128(0x1e87508d_89c2_42f0_8a7e_645a0f50ca58);

pub const PKEY_APP_USER_MODEL_ID: PROPERTYKEY = PROPERTYKEY {
    fmtid: GUID::from_u128(0x9f4c2855_9f79_4b39_a8d0_e1d42de1d5f3),
    pid: 5,
};

pub const PKEY_FILE_DESCRIPTION: PROPERTYKEY = PROPERTYKEY {
    fmtid: GUID::from_u128(0x0cef7d53_fa64_11d1_a203_0000f81fedee),
    pid: 3,
};

pub fn get_window_app_id(hwnd: HWND) -> String {
    // SAFETY: SHGetPropertyStoreForWindow gets property store for window.
    unsafe {
        let store: Result<IPropertyStore> = SHGetPropertyStoreForWindow(hwnd);
        let store = match store {
            Ok(s) => s,
            Err(_) => return String::new(),
        };

        if let Ok(mut val) = store.GetValue(&PKEY_APP_USER_MODEL_ID) {
            // VT_LPWSTR = 31
            if val.Anonymous.Anonymous.vt == windows::Win32::System::Variant::VARENUM(31)
                && !val.Anonymous.Anonymous.Anonymous.pwszVal.0.is_null()
            {
                let s = val
                    .Anonymous
                    .Anonymous
                    .Anonymous
                    .pwszVal
                    .to_string()
                    .unwrap_or_default();
                let _ = PropVariantClear(&mut val);
                return s;
            }
            let _ = PropVariantClear(&mut val);
        }
    }
    String::new()
}

pub fn get_app_name(hwnd: HWND) -> String {
    let app_id = get_window_app_id(hwnd);
    if !app_id.is_empty() {
        let app_id_w: Vec<u16> = format!("{}\0", app_id).encode_utf16().collect();
        // SAFETY: Query AppsFolder for packaged app display name.
        unsafe {
            let item_res: Result<IShellItem> = SHCreateItemInKnownFolder(
                &FOLDERID_APPS_FOLDER,
                windows::Win32::UI::Shell::KNOWN_FOLDER_FLAG(0),
                PCWSTR::from_raw(app_id_w.as_ptr()),
            );
            if let Ok(item) = item_res {
                if let Ok(name_pwstr) = item.GetDisplayName(SIGDN_NORMALDISPLAY) {
                    let name = name_pwstr.to_string().unwrap_or_default();
                    windows::Win32::System::Com::CoTaskMemFree(Some(name_pwstr.0 as *const _));
                    if !name.is_empty() {
                        return name;
                    }
                }
            }
        }
    }

    let path = get_window_process_path(hwnd);
    if path.is_empty() {
        return String::new();
    }

    let path_w: Vec<u16> = format!("{}\0", path).encode_utf16().collect();
    // SAFETY: Query file description property.
    unsafe {
        let file_res: Result<IShellItem2> =
            SHCreateItemFromParsingName(PCWSTR::from_raw(path_w.as_ptr()), None);
        if let Ok(file_item) = file_res {
            if let Ok(desc_pwstr) = file_item.GetString(&PKEY_FILE_DESCRIPTION) {
                let desc = desc_pwstr.to_string().unwrap_or_default();
                windows::Win32::System::Com::CoTaskMemFree(Some(desc_pwstr.0 as *const _));
                if !desc.is_empty() {
                    return desc;
                }
            }
        }
    }

    // Fallback: filename without extension
    let filename = match path.rfind('\\') {
        Some(pos) => &path[pos + 1..],
        None => &path,
    };
    match filename.rfind('.') {
        Some(pos) => filename[..pos].to_string(),
        None => filename.to_string(),
    }
}

pub fn get_window_icon(hwnd: HWND) -> Option<HICON> {
    // SAFETY: SendMessageTimeoutW to fetch WM_GETICON, fallback to class icon.
    unsafe {
        for icon_type in [ICON_BIG as usize, ICON_SMALL2 as usize] {
            let mut result = 0usize;
            if SendMessageTimeoutW(
                hwnd,
                WM_GETICON,
                windows::Win32::Foundation::WPARAM(icon_type),
                windows::Win32::Foundation::LPARAM(0),
                SMTO_ABORTIFHUNG | SMTO_BLOCK,
                50,
                Some(&mut result),
            )
            .0 != 0
                && result != 0
            {
                return Some(HICON(result as *mut _));
            }
        }

        let icon = GetClassLongPtrW(hwnd, GCLP_HICON);
        if icon != 0 {
            return Some(HICON(icon as *mut _));
        }

        let icon_sm = GetClassLongPtrW(hwnd, GCLP_HICONSM);
        if icon_sm != 0 {
            return Some(HICON(icon_sm as *mut _));
        }
    }
    None
}

pub fn copy_pixels(factory: &IWICImagingFactory, source: &IWICBitmapSource) -> Option<IWICBitmap> {
    // SAFETY: Convert image source to 32bppPBGRA and cache in WICBitmap.
    unsafe {
        let converter = factory.CreateFormatConverter().ok()?;
        converter
            .Initialize(
                source,
                &GUID_WICPixelFormat32bppPBGRA,
                WICBitmapDitherTypeNone,
                None,
                0.0,
                WICBitmapPaletteTypeMedianCut,
            )
            .ok()?;
        factory
            .CreateBitmapFromSource(&converter, WICBitmapCacheOnLoad)
            .ok()
    }
}

pub fn read_icon_pixels(factory: &IWICImagingFactory, icon: HICON) -> Option<IWICBitmap> {
    // SAFETY: CreateBitmapFromHICON converts HICON to WIC bitmap.
    unsafe {
        let wic_bmp = factory.CreateBitmapFromHICON(icon).ok()?;
        copy_pixels(factory, &wic_bmp)
    }
}

pub fn read_app_icon_pixels(
    factory: &IWICImagingFactory,
    hwnd: HWND,
    dpi_scale: f32,
) -> Option<IWICBitmap> {
    let app_id = get_window_app_id(hwnd);
    if app_id.is_empty() {
        return None;
    }

    let app_id_w: Vec<u16> = format!("{}\0", app_id).encode_utf16().collect();
    // SAFETY: Query IShellItemImageFactory for packaged app icon.
    unsafe {
        let img_factory: IShellItemImageFactory = SHCreateItemInKnownFolder(
            &FOLDERID_APPS_FOLDER,
            windows::Win32::UI::Shell::KNOWN_FOLDER_FLAG(0),
            PCWSTR::from_raw(app_id_w.as_ptr()),
        )
        .ok()?;

        let size = (64.0 * dpi_scale).round() as i32;
        let hbitmap = img_factory
            .GetImage(SIZE { cx: size, cy: size }, SIIGBF_ICONONLY)
            .ok()?;

        let wic_bmp = factory
            .CreateBitmapFromHBITMAP(
                hbitmap,
                windows::Win32::Graphics::Gdi::HPALETTE(std::ptr::null_mut()),
                WICBitmapUsePremultipliedAlpha,
            )
            .ok();
        let _ = DeleteObject(hbitmap.into());

        match wic_bmp {
            Some(wb) => copy_pixels(factory, &wb),
            None => None,
        }
    }
}

pub fn create_icon_bitmap(ctx: &ID2D1DeviceContext, pixels: &IWICBitmap) -> Option<ID2D1Bitmap1> {
    // SAFETY: CreateBitmapFromWicBitmap binds WIC bitmap into D2D bitmap.
    unsafe { ctx.CreateBitmapFromWicBitmap(pixels, None).ok() }
}

pub fn create_placeholder(
    ctx: &ID2D1DeviceContext,
    brush: &ID2D1SolidColorBrush,
    item_w: f32,
    item_h: f32,
    icon: Option<&ID2D1Bitmap1>,
) -> Option<ID2D1Bitmap1> {
    let scale = (1.0f32).min(640.0 / item_w.max(item_h));
    let width = ((item_w * scale).round() as u32).max(1);
    let height = ((item_h * scale).round() as u32).max(1);

    // SAFETY: Create local D2D target bitmap, draw rounded placeholder card and icon.
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

        let bitmap = ctx
            .CreateBitmap(
                windows::Win32::Graphics::Direct2D::Common::D2D_SIZE_U { width, height },
                None,
                0,
                &props,
            )
            .ok()?;

        ctx.SetTarget(&bitmap);
        ctx.BeginDraw();
        ctx.Clear(Some(&D2D1_COLOR_F {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.0,
        }));

        let card = D2D1_ROUNDED_RECT {
            rect: D2D_RECT_F {
                left: 0.5,
                top: 0.5,
                right: width as f32 - 0.5,
                bottom: height as f32 - 0.5,
            },
            radiusX: 8.0,
            radiusY: 8.0,
        };

        brush.SetColor(&D2D1_COLOR_F {
            r: 0.13,
            g: 0.13,
            b: 0.15,
            a: 0.96,
        });
        ctx.FillRoundedRectangle(&card, brush);

        brush.SetColor(&D2D1_COLOR_F {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 0.14,
        });
        ctx.DrawRoundedRectangle(&card, brush, 1.0, None);

        if let Some(ic) = icon {
            let icon_size = (width as f32 * 0.3)
                .min(height as f32 * 0.3)
                .min(96.0)
                .round();
            let x = ((width as f32 - icon_size) / 2.0).round();
            let y = ((height as f32 - icon_size) / 2.0).round();
            let icon_rect = D2D_RECT_F {
                left: x,
                top: y,
                right: x + icon_size,
                bottom: y + icon_size,
            };
            ctx.DrawBitmap(
                ic,
                Some(&icon_rect),
                1.0,
                D2D1_INTERPOLATION_MODE_HIGH_QUALITY_CUBIC,
                None,
                None,
            );
        }

        let hr = ctx.EndDraw(None, None);
        ctx.SetTarget(None);

        if hr.is_ok() {
            Some(bitmap)
        } else {
            None
        }
    }
}

pub struct CachedIcon {
    pub source: Option<HICON>,
    pub pixels: Option<IWICBitmap>,
    pub bitmap: Option<ID2D1Bitmap1>,
}

pub struct IconManager {
    pub cache: HashMap<isize, CachedIcon>,
    pub app_names: HashMap<isize, String>,
}

impl Default for IconManager {
    fn default() -> Self {
        Self::new()
    }
}

impl IconManager {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            app_names: HashMap::new(),
        }
    }

    pub fn get_icon_bitmap(
        &mut self,
        hwnd: HWND,
        ctx: &ID2D1DeviceContext,
        wic_factory: &IWICImagingFactory,
        dpi_scale: f32,
    ) -> Option<ID2D1Bitmap1> {
        let key = hwnd.0 as isize;
        if let Some(entry) = self.cache.get_mut(&key) {
            if entry.bitmap.is_none() {
                if let Some(pixels) = &entry.pixels {
                    entry.bitmap = create_icon_bitmap(ctx, pixels);
                }
            }
            return entry.bitmap.clone();
        }

        let mut entry = CachedIcon {
            source: None,
            pixels: None,
            bitmap: None,
        };

        if let Some(icon) = get_window_icon(hwnd) {
            entry.pixels = read_icon_pixels(wic_factory, icon);
            if entry.pixels.is_some() {
                entry.source = Some(icon);
            }
        }
        if entry.pixels.is_none() {
            entry.pixels = read_app_icon_pixels(wic_factory, hwnd, dpi_scale);
        }
        if let Some(pix) = &entry.pixels {
            entry.bitmap = create_icon_bitmap(ctx, pix);
        }

        let bmp = entry.bitmap.clone();
        self.cache.insert(key, entry);
        bmp
    }

    pub fn lookup_app_names(&mut self, windows: &[HWND]) {
        for &hwnd in windows {
            let key = hwnd.0 as isize;
            self.app_names
                .entry(key)
                .or_insert_with(|| get_app_name(hwnd));
        }
    }

    pub fn refresh_icons(
        &mut self,
        ctx: &ID2D1DeviceContext,
        wic_factory: &IWICImagingFactory,
        dpi_scale: f32,
    ) {
        // SAFETY: Purge invalid HWNDs.
        self.cache.retain(|&hwnd_val, _| unsafe {
            windows::Win32::UI::WindowsAndMessaging::IsWindow(Some(HWND(hwnd_val as *mut _)))
                .as_bool()
        });
        self.app_names.retain(|&hwnd_val, _| unsafe {
            windows::Win32::UI::WindowsAndMessaging::IsWindow(Some(HWND(hwnd_val as *mut _)))
                .as_bool()
        });

        for (&hwnd_val, entry) in self.cache.iter_mut() {
            let hwnd = HWND(hwnd_val as *mut _);
            let icon = get_window_icon(hwnd);
            if let Some(ic) = icon {
                if icon != entry.source {
                    if let Some(pixels) = read_icon_pixels(wic_factory, ic) {
                        let bitmap = create_icon_bitmap(ctx, &pixels);
                        *entry = CachedIcon {
                            source: Some(ic),
                            pixels: Some(pixels),
                            bitmap,
                        };
                    }
                }
            } else if icon.is_none() && entry.pixels.is_none() {
                entry.pixels = read_app_icon_pixels(wic_factory, hwnd, dpi_scale);
                if let Some(pix) = &entry.pixels {
                    entry.bitmap = create_icon_bitmap(ctx, pix);
                }
            }
        }
    }
}
