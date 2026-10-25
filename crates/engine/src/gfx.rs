// Original Windhawk mod by caliberda
// Ported to standalone Rust by Vigi Riyantanu

use windows::core::{Interface, Result, PCWSTR};
use windows::Graphics::DirectX::Direct3D11::IDirect3DDevice;
use windows::Win32::Foundation::{CloseHandle, HANDLE, HWND, LUID};
use windows::Win32::Graphics::Direct2D::Common::{
    D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F, D2D1_PIXEL_FORMAT,
};
use windows::Win32::Graphics::Direct2D::{
    D2D1CreateFactory, ID2D1Bitmap1, ID2D1Device, ID2D1DeviceContext, ID2D1Factory1,
    ID2D1SolidColorBrush, D2D1_BITMAP_OPTIONS_CANNOT_DRAW, D2D1_BITMAP_OPTIONS_TARGET,
    D2D1_BITMAP_PROPERTIES1, D2D1_DEVICE_CONTEXT_OPTIONS_NONE, D2D1_FACTORY_OPTIONS,
    D2D1_FACTORY_TYPE_MULTI_THREADED, D2D1_UNIT_MODE_PIXELS,
};
use windows::Win32::Graphics::Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_UNKNOWN};
use windows::Win32::Graphics::Direct3D11::{
    D3D11CreateDevice, ID3D11Device, ID3D11Device4, ID3D11DeviceContext, ID3D11Multithread,
    D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_SDK_VERSION,
};
use windows::Win32::Graphics::DirectComposition::{
    DCompositionCreateDevice, IDCompositionDevice, IDCompositionTarget, IDCompositionVisual,
};
use windows::Win32::Graphics::DirectWrite::{
    DWriteCreateFactory, IDWriteFactory, DWRITE_FACTORY_TYPE_SHARED,
};
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_ALPHA_MODE_PREMULTIPLIED, DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SAMPLE_DESC,
};
use windows::Win32::Graphics::Dxgi::{
    CreateDXGIFactory1, IDXGIAdapter, IDXGIAdapter1, IDXGIDevice, IDXGIDevice1, IDXGIFactory1,
    IDXGIFactory2, IDXGISurface, IDXGISwapChain1, DXGI_SCALING_STRETCH, DXGI_SWAP_CHAIN_DESC1,
    DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL, DXGI_USAGE_RENDER_TARGET_OUTPUT,
};
use windows::Win32::Graphics::Gdi::HMONITOR;
use windows::Win32::Graphics::Imaging::{CLSID_WICImagingFactory, IWICImagingFactory};
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};
use windows::Win32::System::WinRT::Direct3D11::CreateDirect3D11DeviceFromDXGIDevice;

pub fn find_monitor_adapter(monitor: HMONITOR) -> Option<IDXGIAdapter1> {
    if monitor.0.is_null() {
        return None;
    }
    // SAFETY: CreateDXGIFactory1 queries DXGI adapters.
    unsafe {
        let factory: IDXGIFactory1 = match CreateDXGIFactory1() {
            Ok(f) => f,
            Err(_) => return None,
        };

        let mut i = 0u32;
        while let Ok(adapter) = factory.EnumAdapters1(i) {
            let mut j = 0u32;
            while let Ok(output) = adapter.EnumOutputs(j) {
                if let Ok(desc) = output.GetDesc() {
                    if desc.Monitor == monitor {
                        return Some(adapter);
                    }
                }
                j += 1;
            }
            i += 1;
        }
    }
    None
}

pub fn get_monitor_adapter_luid(monitor: HMONITOR, luid: &mut LUID) -> bool {
    if let Some(adapter) = find_monitor_adapter(monitor) {
        // SAFETY: GetDesc1 extracts AdapterLuid.
        unsafe {
            if let Ok(desc) = adapter.GetDesc1() {
                *luid = desc.AdapterLuid;
                return true;
            }
        }
    }
    false
}

pub struct GraphicsDeviceContext {
    pub d3d_device: ID3D11Device,
    pub d3d_context: ID3D11DeviceContext,
    pub dxgi_device: IDXGIDevice,
    pub capture_device: IDirect3DDevice,
    pub d2d_factory: ID2D1Factory1,
    pub d2d_device: ID2D1Device,
    pub ctx: ID2D1DeviceContext,
    pub dcomp_device: IDCompositionDevice,
    pub dcomp_target: IDCompositionTarget,
    pub dcomp_visual: IDCompositionVisual,
    pub swap_chain: Option<IDXGISwapChain1>,
    pub target_bitmap: Option<ID2D1Bitmap1>,
    pub brush: ID2D1SolidColorBrush,
    pub dwrite_factory: IDWriteFactory,
    pub wic_factory: IWICImagingFactory,
    pub swap_width: u32,
    pub swap_height: u32,
    pub adapter_luid: LUID,
    pub checked_monitor: HMONITOR,
    pub device_removed_event: Option<HANDLE>,
    pub device_removed_cookie: u32,
    pub fxaa: Option<crate::fxaa::FxaaPipeline>,
}

impl GraphicsDeviceContext {
    pub fn new(monitor: HMONITOR, overlay_hwnd: HWND) -> Result<Self> {
        let adapter_opt = find_monitor_adapter(monitor);
        let mut d3d_device_opt = None;
        let mut d3d_context_opt = None;

        // SAFETY: D3D11CreateDevice creates Direct3D 11 device with BGRA support.
        unsafe {
            let driver_type = if adapter_opt.is_some() {
                D3D_DRIVER_TYPE_UNKNOWN
            } else {
                D3D_DRIVER_TYPE_HARDWARE
            };

            let adapter_ptr = adapter_opt
                .as_ref()
                .map(|a| a as &IDXGIAdapter1 as &IDXGIAdapter);

            let hr = D3D11CreateDevice(
                adapter_ptr,
                driver_type,
                windows::Win32::Foundation::HMODULE(std::ptr::null_mut()),
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                None,
                D3D11_SDK_VERSION,
                Some(&mut d3d_device_opt),
                None,
                Some(&mut d3d_context_opt),
            );

            if hr.is_err() && adapter_opt.is_some() {
                // Fallback to default hardware adapter
                D3D11CreateDevice(
                    None,
                    D3D_DRIVER_TYPE_HARDWARE,
                    windows::Win32::Foundation::HMODULE(std::ptr::null_mut()),
                    D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                    None,
                    D3D11_SDK_VERSION,
                    Some(&mut d3d_device_opt),
                    None,
                    Some(&mut d3d_context_opt),
                )?;
            } else {
                hr?;
            }
        }

        let d3d_device = d3d_device_opt.ok_or_else(windows::core::Error::from_thread)?;
        let d3d_context = d3d_context_opt.ok_or_else(windows::core::Error::from_thread)?;

        let mut device_removed_event = None;
        let mut device_removed_cookie = 0u32;

        // Try ID3D11Device4 RegisterDeviceRemovedEvent
        if let Ok(device4) = d3d_device.cast::<ID3D11Device4>() {
            // SAFETY: CreateEventW creates unnamed auto-reset event.
            let event = unsafe {
                windows::Win32::System::Threading::CreateEventW(None, false, false, PCWSTR::null())
            };
            if let Ok(ev) = event {
                // SAFETY: RegisterDeviceRemovedEvent registers notification.
                match unsafe { device4.RegisterDeviceRemovedEvent(ev) } {
                    Ok(cookie) => {
                        device_removed_cookie = cookie;
                        device_removed_event = Some(ev);
                    }
                    Err(_) => {
                        // SAFETY: CloseHandle on failure.
                        unsafe {
                            let _ = CloseHandle(ev);
                        }
                    }
                }
            }
        }

        // Enable multithread protection for capture frame pools
        if let Ok(multithread) = d3d_device.cast::<ID3D11Multithread>() {
            // SAFETY: SetMultithreadProtected guarantees safe thread concurrency.
            unsafe {
                let _ = multithread.SetMultithreadProtected(true);
            }
        }

        let dxgi_device: IDXGIDevice = d3d_device.cast()?;
        if let Ok(device1) = dxgi_device.cast::<IDXGIDevice1>() {
            // SAFETY: Maximum frame latency set to 1 for lowest Alt+Tab latency.
            unsafe {
                let _ = device1.SetMaximumFrameLatency(1);
            }
        }

        let mut adapter_luid = LUID::default();
        // SAFETY: Read adapter description and LUID.
        unsafe {
            if let Ok(device_adapter) = dxgi_device.GetAdapter() {
                if let Ok(desc) = device_adapter.GetDesc() {
                    adapter_luid = desc.AdapterLuid;
                    let desc_str = String::from_utf16_lossy(&desc.Description);
                    log::info!("Rendering on {}", desc_str.trim_matches('\0'));
                }
            }
        }

        // Interop WinRT capture device from DXGI device
        // SAFETY: CreateDirect3D11DeviceFromDXGIDevice creates WinRT IDirect3DDevice.
        let inspectable = unsafe { CreateDirect3D11DeviceFromDXGIDevice(&dxgi_device)? };
        let capture_device: IDirect3DDevice = inspectable.cast()?;

        // Create Direct2D factory and device
        let options = D2D1_FACTORY_OPTIONS::default();
        // SAFETY: D2D1CreateFactory creates multi-threaded D2D factory.
        let d2d_factory: ID2D1Factory1 =
            unsafe { D2D1CreateFactory(D2D1_FACTORY_TYPE_MULTI_THREADED, Some(&options))? };
        // SAFETY: CreateDevice creates D2D device backed by DXGI device.
        let d2d_device = unsafe { d2d_factory.CreateDevice(&dxgi_device)? };
        // SAFETY: CreateDeviceContext creates D2D drawing context.
        let ctx = unsafe { d2d_device.CreateDeviceContext(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)? };

        // Everything is in physical pixels
        // SAFETY: Configure DPI and pixel unit mode.
        unsafe {
            ctx.SetDpi(96.0, 96.0);
            ctx.SetUnitMode(D2D1_UNIT_MODE_PIXELS);
        }

        // SAFETY: CreateSolidColorBrush initializes white brush.
        let brush = unsafe {
            ctx.CreateSolidColorBrush(
                &D2D1_COLOR_F {
                    r: 1.0,
                    g: 1.0,
                    b: 1.0,
                    a: 1.0,
                },
                None,
            )?
        };

        // DirectComposition
        // SAFETY: DCompositionCreateDevice creates composition device for HWND overlay.
        let dcomp_device: IDCompositionDevice = unsafe { DCompositionCreateDevice(&dxgi_device)? };
        // SAFETY: CreateTargetForHwnd binds composition to overlay window.
        let dcomp_target = unsafe { dcomp_device.CreateTargetForHwnd(overlay_hwnd, true)? };
        // SAFETY: CreateVisual creates root visual.
        let dcomp_visual = unsafe { dcomp_device.CreateVisual()? };
        // SAFETY: SetRoot sets the root visual on the target.
        unsafe {
            dcomp_target.SetRoot(&dcomp_visual)?;
        }

        // DirectWrite and WIC factories
        // SAFETY: DWriteCreateFactory initializes shared factory.
        let dwrite_factory: IDWriteFactory =
            unsafe { DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)? };
        // SAFETY: CoCreateInstance initializes WIC imaging factory.
        let wic_factory: IWICImagingFactory =
            unsafe { CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)? };

        // Post-process FXAA Anti-Aliasing pipeline
        let fxaa = match crate::fxaa::FxaaPipeline::new(&d3d_device) {
            Ok(pipe) => {
                log::info!("Hardware-accelerated FXAA 3.11 pipeline initialized successfully.");
                Some(pipe)
            }
            Err(e) => {
                log::warn!("FXAA pipeline initialization failed: {:?}, falling back to direct presentation.", e);
                None
            }
        };

        Ok(Self {
            d3d_device,
            d3d_context,
            dxgi_device,
            capture_device,
            d2d_factory,
            d2d_device,
            ctx,
            dcomp_device,
            dcomp_target,
            dcomp_visual,
            swap_chain: None,
            target_bitmap: None,
            brush,
            dwrite_factory,
            wic_factory,
            swap_width: 0,
            swap_height: 0,
            adapter_luid,
            checked_monitor: monitor,
            device_removed_event,
            device_removed_cookie,
            fxaa,
        })
    }

    pub fn device_removed(&self) -> bool {
        // SAFETY: GetDeviceRemovedReason queries hardware state.
        unsafe { self.d3d_device.GetDeviceRemovedReason().is_err() }
    }

    pub fn ensure_swap_chain(&mut self, width: u32, height: u32) -> Result<bool> {
        if self.swap_chain.is_some() && self.swap_width == width && self.swap_height == height {
            return Ok(true);
        }

        // Drop previous swap chain resources
        // SAFETY: Target must be cleared before dropping bitmap and swap chain.
        unsafe {
            self.ctx.SetTarget(None);
        }
        self.target_bitmap = None;
        self.swap_chain = None;
        self.swap_width = 0;
        self.swap_height = 0;

        // SAFETY: Query IDXGIFactory2 to create swap chain for composition.
        unsafe {
            let adapter = self.dxgi_device.GetAdapter()?;
            let factory: IDXGIFactory2 = adapter.GetParent()?;

            let desc = DXGI_SWAP_CHAIN_DESC1 {
                Width: width,
                Height: height,
                Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                SampleDesc: DXGI_SAMPLE_DESC {
                    Count: 1,
                    Quality: 0,
                },
                BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
                BufferCount: 2,
                Scaling: DXGI_SCALING_STRETCH,
                SwapEffect: DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
                AlphaMode: DXGI_ALPHA_MODE_PREMULTIPLIED,
                ..Default::default()
            };

            let swap_chain =
                factory.CreateSwapChainForComposition(&self.d3d_device, &desc, None)?;
            let surface: IDXGISurface = swap_chain.GetBuffer(0)?;

            let props = D2D1_BITMAP_PROPERTIES1 {
                pixelFormat: D2D1_PIXEL_FORMAT {
                    format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                },
                dpiX: 96.0,
                dpiY: 96.0,
                bitmapOptions: D2D1_BITMAP_OPTIONS_TARGET | D2D1_BITMAP_OPTIONS_CANNOT_DRAW,
                colorContext: std::mem::ManuallyDrop::new(None),
            };

            let target_bitmap = self
                .ctx
                .CreateBitmapFromDxgiSurface(&surface, Some(&props))?;
            self.dcomp_visual.SetContent(&swap_chain)?;
            self.dcomp_device.Commit()?;

            self.swap_chain = Some(swap_chain.clone());
            self.target_bitmap = Some(target_bitmap);
            self.swap_width = width;
            self.swap_height = height;

            if let Some(fxaa) = &mut self.fxaa {
                if let Err(e) = fxaa.ensure_resources(&self.d3d_device, &self.ctx, &swap_chain, width, height) {
                    log::warn!("FXAA resource allocation failed: {:?}, will fallback to standard render target.", e);
                }
            }
        }

        Ok(true)
    }

    pub fn release_device_resources(&mut self) {
        // SAFETY: Detach target bitmap before dropping swap chain.
        unsafe {
            self.ctx.SetTarget(None);
        }
        if let Some(fxaa) = &mut self.fxaa {
            fxaa.release_resources();
        }
        self.target_bitmap = None;
        self.swap_chain = None;
        self.swap_width = 0;
        self.swap_height = 0;

        if let Some(ev) = self.device_removed_event.take() {
            if let Ok(device4) = self.d3d_device.cast::<ID3D11Device4>() {
                // SAFETY: UnregisterDeviceRemoved before closing event.
                unsafe {
                    device4.UnregisterDeviceRemoved(self.device_removed_cookie);
                }
            }
            // SAFETY: Close event handle.
            unsafe {
                let _ = CloseHandle(ev);
            }
        }
    }
}
