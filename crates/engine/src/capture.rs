// Original Windhawk mod by caliberda
// Ported to standalone Rust by Vigi Riyantanu

use windows::core::{Interface, Result, BOOL, PCWSTR};
use windows::Graphics::Capture::{
    Direct3D11CaptureFrame, Direct3D11CaptureFramePool, GraphicsCaptureItem, GraphicsCaptureSession,
};
use windows::Graphics::DirectX::Direct3D11::IDirect3DDevice;
use windows::Graphics::DirectX::DirectXPixelFormat;
use windows::Graphics::SizeInt32;
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Direct2D::Common::{
    D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_PIXEL_FORMAT,
};
use windows::Win32::Graphics::Direct2D::{
    ID2D1Bitmap1, ID2D1DeviceContext, D2D1_BITMAP_OPTIONS_NONE, D2D1_BITMAP_PROPERTIES1,
};
use windows::Win32::Graphics::Direct3D11::{
    ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D, D3D11_BIND_SHADER_RESOURCE, D3D11_BOX,
    D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT,
};
use windows::Win32::Graphics::DirectWrite::IDWriteTextLayout;
use windows::Win32::Graphics::Dwm::{
    DwmQueryThumbnailSourceSize, DwmRegisterThumbnail, DwmSetWindowAttribute,
    DwmUnregisterThumbnail, DwmUpdateThumbnailProperties, DWMWA_TRANSITIONS_FORCEDISABLED,
    DWM_THUMBNAIL_PROPERTIES, DWM_TNP_OPACITY, DWM_TNP_RECTDESTINATION,
    DWM_TNP_SOURCECLIENTAREAONLY, DWM_TNP_VISIBLE,
};
use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SAMPLE_DESC};
use windows::Win32::Graphics::Dxgi::IDXGISurface;
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, DeleteDC, DeleteObject, GetDC, ReleaseDC, SelectObject, BITMAPINFO,
    BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
};
use windows::Win32::System::WinRT::Direct3D11::IDirect3DDxgiInterfaceAccess;
use windows::Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, GetSystemMetrics, SetWindowPos, HWND_BOTTOM, SM_XVIRTUALSCREEN,
    SWP_NOACTIVATE, SWP_SHOWWINDOW, WS_EX_NOACTIVATE, WS_EX_NOREDIRECTIONBITMAP, WS_EX_TOOLWINDOW,
    WS_POPUP,
};

use crate::geometry::Pose;
use crate::render::EffectChain;

pub const PROXY_CLASS_NAME: &str = "WindhawkFlip3DSwitcherProxy";

pub struct Item {
    pub hwnd: HWND,
    pub title: String,
    pub rect: RECT,
    pub minimized: bool,
    pub hidden: bool,
    pub icon: Option<ID2D1Bitmap1>,
    pub placeholder: Option<ID2D1Bitmap1>,

    // Proxy window for minimized/hidden windows
    pub proxy: HWND,
    pub thumbnail: Option<isize>,

    // WGC capture
    pub capture_item: Option<GraphicsCaptureItem>,
    pub frame_pool: Option<Direct3D11CaptureFramePool>,
    pub session: Option<GraphicsCaptureSession>,
    pub pool_size: SizeInt32,
    pub texture: Option<ID3D11Texture2D>,
    pub content: Option<ID2D1Bitmap1>,

    // Rendering
    pub chains: [EffectChain; 2],
    pub label: Option<IDWriteTextLayout>,
    pub label_width: f32,

    pub close_requested_at: f64,
    pub last_pose: Pose,
    pub has_last_pose: bool,
    pub slide_from: Pose,
    pub slides: bool,
}

impl Item {
    pub fn new(hwnd: HWND, title: String, rect: RECT, minimized: bool, hidden: bool) -> Self {
        Self {
            hwnd,
            title,
            rect,
            minimized,
            hidden,
            icon: None,
            placeholder: None,
            proxy: HWND(std::ptr::null_mut()),
            thumbnail: None,
            capture_item: None,
            frame_pool: None,
            session: None,
            pool_size: SizeInt32 {
                Width: 0,
                Height: 0,
            },
            texture: None,
            content: None,
            chains: [EffectChain::default(), EffectChain::default()],
            label: None,
            label_width: 0.0,
            close_requested_at: 0.0,
            last_pose: Pose::default(),
            has_last_pose: false,
            slide_from: Pose::default(),
            slides: false,
        }
    }

    pub fn width(&self) -> f32 {
        (self.rect.right - self.rect.left) as f32
    }

    pub fn height(&self) -> f32 {
        (self.rect.bottom - self.rect.top) as f32
    }
}

pub fn create_proxy(item: &mut Item) -> bool {
    let proxy_class: Vec<u16> = format!("{}\0", PROXY_CLASS_NAME).encode_utf16().collect();
    let empty_title: Vec<u16> = "\0".encode_utf16().collect();

    // SAFETY: Creating proxy window to host DWM thumbnail off-screen.
    unsafe {
        let proxy = CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_NOREDIRECTIONBITMAP,
            PCWSTR::from_raw(proxy_class.as_ptr()),
            PCWSTR::from_raw(empty_title.as_ptr()),
            WS_POPUP,
            0,
            0,
            1,
            1,
            None,
            None,
            None,
            None,
        );

        let proxy_hwnd = match proxy {
            Ok(h) if !h.0.is_null() => h,
            _ => return false,
        };

        let mut disable_transitions = BOOL(1);
        let _ = DwmSetWindowAttribute(
            proxy_hwnd,
            DWMWA_TRANSITIONS_FORCEDISABLED,
            &mut disable_transitions as *mut _ as *mut _,
            std::mem::size_of::<BOOL>() as u32,
        );

        let thumbnail = match DwmRegisterThumbnail(proxy_hwnd, item.hwnd) {
            Ok(t) => t,
            Err(_) => {
                let _ = DestroyWindow(proxy_hwnd);
                return false;
            }
        };

        let size = match DwmQueryThumbnailSourceSize(thumbnail) {
            Ok(s) if s.cx > 0 && s.cy > 0 => s,
            _ => {
                let _ = DwmUnregisterThumbnail(thumbnail);
                let _ = DestroyWindow(proxy_hwnd);
                return false;
            }
        };

        let properties = DWM_THUMBNAIL_PROPERTIES {
            dwFlags: DWM_TNP_RECTDESTINATION
                | DWM_TNP_VISIBLE
                | DWM_TNP_OPACITY
                | DWM_TNP_SOURCECLIENTAREAONLY,
            rcDestination: RECT {
                left: 0,
                top: 0,
                right: size.cx,
                bottom: size.cy,
            },
            rcSource: RECT::default(),
            opacity: 255,
            fVisible: BOOL(1),
            fSourceClientAreaOnly: BOOL(0),
        };

        if DwmUpdateThumbnailProperties(thumbnail, &properties).is_err() {
            let _ = DwmUnregisterThumbnail(thumbnail);
            let _ = DestroyWindow(proxy_hwnd);
            return false;
        }

        let x = GetSystemMetrics(SM_XVIRTUALSCREEN) - size.cx - 100;
        let y = GetSystemMetrics(windows::Win32::UI::WindowsAndMessaging::SM_YVIRTUALSCREEN);
        let pos_ok = SetWindowPos(
            proxy_hwnd,
            Some(HWND_BOTTOM),
            x,
            y,
            size.cx,
            size.cy,
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        );

        if pos_ok.is_err() {
            let _ = DwmUnregisterThumbnail(thumbnail);
            let _ = DestroyWindow(proxy_hwnd);
            return false;
        }

        item.proxy = proxy_hwnd;
        item.thumbnail = Some(thumbnail);
        true
    }
}

pub fn stop_capture(item: &mut Item) {
    if let Some(session) = item.session.take() {
        let _ = session.Close();
    }
    if let Some(pool) = item.frame_pool.take() {
        let _ = pool.Close();
    }
    item.capture_item = None;

    if let Some(thumb) = item.thumbnail.take() {
        // SAFETY: Unregister thumbnail.
        unsafe {
            let _ = DwmUnregisterThumbnail(thumb);
        }
    }
    if !item.proxy.0.is_null() {
        // SAFETY: Destroy proxy window.
        unsafe {
            let _ = DestroyWindow(item.proxy);
        }
        item.proxy = HWND(std::ptr::null_mut());
    }
}

pub fn start_capture(
    item: &mut Item,
    capture_device: &IDirect3DDevice,
    d3d_device: &ID3D11Device,
    d2d_context: &ID2D1DeviceContext,
    minimized_content: bool,
) {
    let mut source_hwnd = item.hwnd;
    if item.minimized || item.hidden {
        if item.minimized && !minimized_content {
            return;
        }
        if !create_proxy(item) {
            log::warn!("Proxy failed for {:?}", item.hwnd);
            // Fall back to PrintWindow fallback snapshot
            let _ = capture_print_window_fallback(item, d3d_device, d2d_context);
            return;
        }
        source_hwnd = item.proxy;
    }

    // Try Windows Graphics Capture
    let capture_res = (|| -> Result<()> {
        let interop: IGraphicsCaptureItemInterop =
            windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;
        // SAFETY: CreateForWindow creates GraphicsCaptureItem for HWND.
        let capture_item: GraphicsCaptureItem = unsafe { interop.CreateForWindow(source_hwnd)? };

        let size = capture_item.Size()?;
        if size.Width <= 0 || size.Height <= 0 {
            return Err(windows::core::Error::from_thread());
        }

        let frame_pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
            capture_device,
            DirectXPixelFormat::B8G8R8A8UIntNormalized,
            2,
            size,
        )?;
        let session = frame_pool.CreateCaptureSession(&capture_item)?;

        // Win10/11 compatibility: SetIsCursorCaptureEnabled & SetIsBorderRequired may fail on older builds
        let _ = session.SetIsCursorCaptureEnabled(false);
        let _ = session.SetIsBorderRequired(false);

        session.StartCapture()?;

        item.capture_item = Some(capture_item);
        item.frame_pool = Some(frame_pool);
        item.session = Some(session);
        item.pool_size = size;

        Ok(())
    })();

    if let Err(e) = capture_res {
        log::warn!(
            "WGC capture failed for {:?}: {:?}, falling back to PrintWindow",
            item.hwnd,
            e
        );
        stop_capture(item);
        let _ = capture_print_window_fallback(item, d3d_device, d2d_context);
    }
}

pub fn poll_frames(
    item: &mut Item,
    d3d_device: &ID3D11Device,
    d3d_context: &ID3D11DeviceContext,
    d2d_context: &ID2D1DeviceContext,
    capture_device: &IDirect3DDevice,
) -> bool {
    let pool = match &item.frame_pool {
        Some(p) => p,
        None => return false,
    };

    let poll_res = (|| -> Result<bool> {
        let mut latest: Option<Direct3D11CaptureFrame> = None;
        while let Ok(frame) = pool.TryGetNextFrame() {
            if let Some(f) = latest.take() {
                let _ = f.Close();
            }
            latest = Some(frame);
        }

        let frame = match latest {
            Some(f) => f,
            None => return Ok(false),
        };

        let content_size = frame.ContentSize()?;
        let surface = frame.Surface()?;
        let access: IDirect3DDxgiInterfaceAccess = surface.cast()?;
        // SAFETY: Access underlying D3D11 texture from WinRT surface.
        let source_texture: ID3D11Texture2D = unsafe { access.GetInterface()? };

        let mut source_desc = D3D11_TEXTURE2D_DESC::default();
        // SAFETY: Read source texture dimensions.
        unsafe {
            source_texture.GetDesc(&mut source_desc);
        }

        let width = (content_size.Width.max(1) as u32).min(source_desc.Width);
        let height = (content_size.Height.max(1) as u32).min(source_desc.Height);

        let mut current_desc = D3D11_TEXTURE2D_DESC::default();
        if let Some(tex) = &item.texture {
            // SAFETY: Read current texture dimensions.
            unsafe {
                tex.GetDesc(&mut current_desc);
            }
        }

        if item.texture.is_none() || current_desc.Width != width || current_desc.Height != height {
            let desc = D3D11_TEXTURE2D_DESC {
                Width: width,
                Height: height,
                MipLevels: 1,
                ArraySize: 1,
                Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                SampleDesc: DXGI_SAMPLE_DESC {
                    Count: 1,
                    Quality: 0,
                },
                Usage: D3D11_USAGE_DEFAULT,
                BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32,
                ..Default::default()
            };

            // SAFETY: Create local D3D11 texture and D2D bitmap.
            let mut texture_opt: Option<ID3D11Texture2D> = None;
            unsafe { d3d_device.CreateTexture2D(&desc, None, Some(&mut texture_opt))? };
            let texture = texture_opt.ok_or_else(windows::core::Error::from_thread)?;
            let dxgi_surface: IDXGISurface = texture.cast()?;

            let props = D2D1_BITMAP_PROPERTIES1 {
                pixelFormat: D2D1_PIXEL_FORMAT {
                    format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                },
                dpiX: 96.0,
                dpiY: 96.0,
                bitmapOptions: D2D1_BITMAP_OPTIONS_NONE,
                colorContext: std::mem::ManuallyDrop::new(None),
            };

            // SAFETY: CreateBitmapFromDxgiSurface wraps texture for Direct2D drawing.
            let bitmap =
                unsafe { d2d_context.CreateBitmapFromDxgiSurface(&dxgi_surface, Some(&props))? };

            item.texture = Some(texture);
            item.content = Some(bitmap);
        }

        if let Some(tex) = &item.texture {
            let box_region = D3D11_BOX {
                left: 0,
                top: 0,
                front: 0,
                right: width,
                bottom: height,
                back: 1,
            };
            // SAFETY: Copy frame content into item's D3D11 texture.
            unsafe {
                d3d_context.CopySubresourceRegion(
                    tex,
                    0,
                    0,
                    0,
                    0,
                    &source_texture,
                    0,
                    Some(&box_region),
                );
            }
        }

        let _ = frame.Close();

        if content_size.Width != item.pool_size.Width
            || content_size.Height != item.pool_size.Height
        {
            let _ = pool.Recreate(
                capture_device,
                DirectXPixelFormat::B8G8R8A8UIntNormalized,
                2,
                content_size,
            );
            item.pool_size = content_size;
        }

        Ok(true)
    })();

    match poll_res {
        Ok(has_new_frame) => has_new_frame,
        Err(e) => {
            log::warn!("Frame update failed for {:?}: {:?}", item.hwnd, e);
            stop_capture(item);
            false
        }
    }
}

pub fn capture_print_window_fallback(
    item: &mut Item,
    d3d_device: &ID3D11Device,
    d2d_context: &ID2D1DeviceContext,
) -> Result<()> {
    let width = (item.rect.right - item.rect.left).max(1) as u32;
    let height = (item.rect.bottom - item.rect.top).max(1) as u32;

    // SAFETY: Use PrintWindow to snapshot content into GDI DIB section, then copy into D3D11 texture.
    unsafe {
        let hdc_screen = GetDC(None);
        let hdc_mem = CreateCompatibleDC(Some(hdc_screen));

        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width as i32,
                biHeight: -(height as i32), // top-down
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();
        let hbitmap = windows::Win32::Graphics::Gdi::CreateDIBSection(
            Some(hdc_screen),
            &bmi,
            DIB_RGB_COLORS,
            &mut bits,
            None,
            0,
        )?;

        let old_bmp = SelectObject(hdc_mem, hbitmap.into());

        const PW_RENDERFULLCONTENT: u32 = 0x00000002;
        windows_core::link!("user32.dll" "system" fn PrintWindow(hwnd: HWND, hdcblt: windows::Win32::Graphics::Gdi::HDC, nflags: u32) -> BOOL);
        let _ = PrintWindow(item.hwnd, hdc_mem, PW_RENDERFULLCONTENT);

        SelectObject(hdc_mem, old_bmp);
        let _ = DeleteDC(hdc_mem);
        ReleaseDC(None, hdc_screen);

        if !bits.is_null() {
            let desc = D3D11_TEXTURE2D_DESC {
                Width: width,
                Height: height,
                MipLevels: 1,
                ArraySize: 1,
                Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                SampleDesc: DXGI_SAMPLE_DESC {
                    Count: 1,
                    Quality: 0,
                },
                Usage: D3D11_USAGE_DEFAULT,
                BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32,
                ..Default::default()
            };

            let subresource = windows::Win32::Graphics::Direct3D11::D3D11_SUBRESOURCE_DATA {
                pSysMem: bits,
                SysMemPitch: width * 4,
                SysMemSlicePitch: 0,
            };

            let mut texture_opt: Option<ID3D11Texture2D> = None;
            d3d_device.CreateTexture2D(&desc, Some(&subresource), Some(&mut texture_opt))?;
            let texture = texture_opt.ok_or_else(windows::core::Error::from_thread)?;
            let dxgi_surface: IDXGISurface = texture.cast()?;

            let props = D2D1_BITMAP_PROPERTIES1 {
                pixelFormat: D2D1_PIXEL_FORMAT {
                    format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                },
                dpiX: 96.0,
                dpiY: 96.0,
                bitmapOptions: D2D1_BITMAP_OPTIONS_NONE,
                colorContext: std::mem::ManuallyDrop::new(None),
            };

            let bitmap = d2d_context.CreateBitmapFromDxgiSurface(&dxgi_surface, Some(&props))?;
            item.texture = Some(texture);
            item.content = Some(bitmap);
        }

        let _ = DeleteObject(hbitmap.into());
    }

    Ok(())
}
