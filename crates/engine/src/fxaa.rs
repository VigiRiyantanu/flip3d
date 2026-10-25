// Original Windhawk mod by caliberda
// Ported to standalone Rust by Vigi Riyantanu & Ahmad Baihagi

use windows::core::{Interface, Result};
use windows::Win32::Graphics::Direct2D::Common::{D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_PIXEL_FORMAT};
use windows::Win32::Graphics::Direct2D::{
    ID2D1Bitmap1, ID2D1DeviceContext, D2D1_BITMAP_OPTIONS_CANNOT_DRAW, D2D1_BITMAP_OPTIONS_TARGET,
    D2D1_BITMAP_PROPERTIES1,
};
use windows::Win32::Graphics::Direct3D::D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST;
use windows::Win32::Graphics::Direct3D11::{
    ID3D11Buffer, ID3D11Device, ID3D11DeviceContext, ID3D11PixelShader, ID3D11RenderTargetView,
    ID3D11SamplerState, ID3D11ShaderResourceView, ID3D11Texture2D, ID3D11VertexShader,
    D3D11_BIND_CONSTANT_BUFFER, D3D11_BIND_RENDER_TARGET, D3D11_BIND_SHADER_RESOURCE,
    D3D11_BUFFER_DESC, D3D11_COMPARISON_NEVER, D3D11_FILTER_MIN_MAG_MIP_LINEAR,
    D3D11_SAMPLER_DESC, D3D11_TEXTURE2D_DESC, D3D11_TEXTURE_ADDRESS_CLAMP, D3D11_USAGE_DEFAULT,
    D3D11_VIEWPORT,
};
use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SAMPLE_DESC};
use windows::Win32::Graphics::Dxgi::{IDXGISurface, IDXGISwapChain1};

use settings_core::AntiAliasingMode;

const FXAA_VS_BYTECODE: &[u8] = include_bytes!("../shaders/fxaa_vs.cso");
const FXAA_PS_BYTECODE: &[u8] = include_bytes!("../shaders/fxaa_ps.cso");

#[repr(C)]
#[derive(Clone, Copy)]
struct FxaaConstants {
    rcp_frame: [f32; 2],
    subpixel_quality: f32,
    edge_threshold: f32,
    edge_threshold_min: f32,
    quality_preset: f32,
    _pad: [f32; 2],
}

pub struct FxaaPipeline {
    vertex_shader: ID3D11VertexShader,
    pixel_shader: ID3D11PixelShader,
    sampler_state: ID3D11SamplerState,
    constant_buffer: ID3D11Buffer,

    pub offscreen_tex: Option<ID3D11Texture2D>,
    pub offscreen_srv: Option<ID3D11ShaderResourceView>,
    pub offscreen_bitmap: Option<ID2D1Bitmap1>,
    pub back_buffer_rtv: Option<ID3D11RenderTargetView>,
    pub width: u32,
    pub height: u32,
}

impl FxaaPipeline {
    pub fn new(d3d_device: &ID3D11Device) -> Result<Self> {
        let mut vs_opt = None;
        // SAFETY: Create full-screen triangle vertex shader from embedded bytecode.
        unsafe {
            d3d_device.CreateVertexShader(FXAA_VS_BYTECODE, None, Some(&mut vs_opt))?;
        }
        let vertex_shader = vs_opt.ok_or_else(windows::core::Error::from_thread)?;

        let mut ps_opt = None;
        // SAFETY: Create FXAA 3.11 pixel shader from embedded bytecode.
        unsafe {
            d3d_device.CreatePixelShader(FXAA_PS_BYTECODE, None, Some(&mut ps_opt))?;
        }
        let pixel_shader = ps_opt.ok_or_else(windows::core::Error::from_thread)?;

        let sampler_desc = D3D11_SAMPLER_DESC {
            Filter: D3D11_FILTER_MIN_MAG_MIP_LINEAR,
            AddressU: D3D11_TEXTURE_ADDRESS_CLAMP,
            AddressV: D3D11_TEXTURE_ADDRESS_CLAMP,
            AddressW: D3D11_TEXTURE_ADDRESS_CLAMP,
            ComparisonFunc: D3D11_COMPARISON_NEVER,
            MinLOD: 0.0,
            MaxLOD: f32::MAX,
            ..Default::default()
        };

        let mut sampler_opt = None;
        // SAFETY: Create linear clamping sampler for subpixel filtering.
        unsafe {
            d3d_device.CreateSamplerState(&sampler_desc, Some(&mut sampler_opt))?;
        }
        let sampler_state = sampler_opt.ok_or_else(windows::core::Error::from_thread)?;

        let cb_desc = D3D11_BUFFER_DESC {
            ByteWidth: std::mem::size_of::<FxaaConstants>() as u32,
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: D3D11_BIND_CONSTANT_BUFFER.0 as u32,
            ..Default::default()
        };

        let mut cb_opt = None;
        // SAFETY: Create constant buffer for FXAA threshold and frame pacing params.
        unsafe {
            d3d_device.CreateBuffer(&cb_desc, None, Some(&mut cb_opt))?;
        }
        let constant_buffer = cb_opt.ok_or_else(windows::core::Error::from_thread)?;

        Ok(Self {
            vertex_shader,
            pixel_shader,
            sampler_state,
            constant_buffer,
            offscreen_tex: None,
            offscreen_srv: None,
            offscreen_bitmap: None,
            back_buffer_rtv: None,
            width: 0,
            height: 0,
        })
    }

    pub fn ensure_resources(
        &mut self,
        d3d_device: &ID3D11Device,
        d2d_context: &ID2D1DeviceContext,
        swap_chain: &IDXGISwapChain1,
        width: u32,
        height: u32,
    ) -> Result<()> {
        if self.width == width
            && self.height == height
            && self.offscreen_tex.is_some()
            && self.back_buffer_rtv.is_some()
        {
            return Ok(());
        }

        self.release_resources();

        // 1. Create intermediate offscreen texture for Direct2D rendering
        let tex_desc = D3D11_TEXTURE2D_DESC {
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
            BindFlags: (D3D11_BIND_RENDER_TARGET.0 | D3D11_BIND_SHADER_RESOURCE.0) as u32,
            ..Default::default()
        };

        let mut offscreen_tex_opt = None;
        // SAFETY: Create offscreen texture.
        unsafe {
            d3d_device.CreateTexture2D(&tex_desc, None, Some(&mut offscreen_tex_opt))?;
        }
        let offscreen_tex = offscreen_tex_opt.ok_or_else(windows::core::Error::from_thread)?;

        // 2. Create Shader Resource View (SRV) for the FXAA pixel shader pass
        let mut srv_opt = None;
        // SAFETY: Create SRV from offscreen texture.
        unsafe {
            d3d_device.CreateShaderResourceView(&offscreen_tex, None, Some(&mut srv_opt))?;
        }
        let offscreen_srv = srv_opt.ok_or_else(windows::core::Error::from_thread)?;

        // 3. Wrap offscreen surface as a Direct2D bitmap target
        let dxgi_surface: IDXGISurface = offscreen_tex.cast()?;
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

        // SAFETY: Direct2D target bitmap backed by offscreen DXGI surface.
        let offscreen_bitmap =
            unsafe { d2d_context.CreateBitmapFromDxgiSurface(&dxgi_surface, Some(&props))? };

        // 4. Create RenderTargetView for the swap chain back buffer (the final output)
        // SAFETY: Get back buffer from swap chain.
        let back_buffer: ID3D11Texture2D = unsafe { swap_chain.GetBuffer(0)? };
        let mut rtv_opt = None;
        // SAFETY: Create RTV for the back buffer.
        unsafe {
            d3d_device.CreateRenderTargetView(&back_buffer, None, Some(&mut rtv_opt))?;
        }
        let back_buffer_rtv = rtv_opt.ok_or_else(windows::core::Error::from_thread)?;

        self.offscreen_tex = Some(offscreen_tex);
        self.offscreen_srv = Some(offscreen_srv);
        self.offscreen_bitmap = Some(offscreen_bitmap);
        self.back_buffer_rtv = Some(back_buffer_rtv);
        self.width = width;
        self.height = height;

        log::debug!(
            "FXAA offscreen render target initialized at {}x{}",
            width,
            height
        );
        Ok(())
    }

    pub fn render(
        &self,
        d3d_context: &ID3D11DeviceContext,
        mode: AntiAliasingMode,
    ) -> Result<()> {
        if self.width == 0 || self.height == 0 {
            return Ok(());
        }

        let rtv = match &self.back_buffer_rtv {
            Some(r) => r,
            None => return Ok(()),
        };
        let srv = match &self.offscreen_srv {
            Some(s) => s,
            None => return Ok(()),
        };

        // Calibrate parameters based on mode
        let (subpixel_quality, edge_threshold, edge_threshold_min, quality_preset) =
            if mode == AntiAliasingMode::FxaaHigh {
                (0.85f32, 0.125f32, 0.0625f32, 1.0f32)
            } else {
                (0.75f32, 0.166f32, 0.0833f32, 0.0f32)
            };

        let constants = FxaaConstants {
            rcp_frame: [1.0 / self.width as f32, 1.0 / self.height as f32],
            subpixel_quality,
            edge_threshold,
            edge_threshold_min,
            quality_preset,
            _pad: [0.0, 0.0],
        };

        // SAFETY: Configure D3D11 pipeline state and execute full-screen FXAA pass.
        unsafe {
            // Update constant buffer
            d3d_context.UpdateSubresource(
                &self.constant_buffer,
                0,
                None,
                &constants as *const _ as *const _,
                0,
                0,
            );

            // Set render target to swap chain back buffer
            let rtvs = [Some(rtv.clone())];
            d3d_context.OMSetRenderTargets(Some(&rtvs), None);

            // Set full-screen viewport
            let viewport = D3D11_VIEWPORT {
                TopLeftX: 0.0,
                TopLeftY: 0.0,
                Width: self.width as f32,
                Height: self.height as f32,
                MinDepth: 0.0,
                MaxDepth: 1.0,
            };
            d3d_context.RSSetViewports(Some(&[viewport]));

            // Pipeline stages
            d3d_context.IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
            d3d_context.VSSetShader(&self.vertex_shader, None);
            d3d_context.PSSetShader(&self.pixel_shader, None);

            let cbs = [Some(self.constant_buffer.clone())];
            d3d_context.PSSetConstantBuffers(0, Some(&cbs));

            let srvs = [Some(srv.clone())];
            d3d_context.PSSetShaderResources(0, Some(&srvs));

            let samplers = [Some(self.sampler_state.clone())];
            d3d_context.PSSetSamplers(0, Some(&samplers));

            // Draw full-screen triangle
            d3d_context.Draw(3, 0);

            // Unbind input SRV and render target to prevent resource hazards on next frame
            let null_srvs: [Option<ID3D11ShaderResourceView>; 1] = [None];
            d3d_context.PSSetShaderResources(0, Some(&null_srvs));
            d3d_context.OMSetRenderTargets(None, None);
        }

        Ok(())
    }

    pub fn release_resources(&mut self) {
        self.offscreen_bitmap = None;
        self.offscreen_srv = None;
        self.offscreen_tex = None;
        self.back_buffer_rtv = None;
        self.width = 0;
        self.height = 0;
    }
}
