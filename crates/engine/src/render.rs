// Original Windhawk mod by caliberda
// Ported to standalone Rust by Vigi Riyantanu

use windows::core::Interface;
use windows::Win32::Graphics::Direct2D::Common::{
    D2D1_BORDER_MODE_SOFT, D2D1_COLOR_F, D2D1_COMPOSITE_MODE_SOURCE_OVER, D2D1_FIGURE_BEGIN_HOLLOW,
    D2D1_FIGURE_END_CLOSED, D2D_RECT_F,
};
use windows::Win32::Graphics::Direct2D::{
    CLSID_D2D13DTransform, CLSID_D2D1ColorMatrix, CLSID_D2D1Shadow, ID2D1DeviceContext,
    ID2D1Effect, ID2D1Factory1, ID2D1LinearGradientBrush, ID2D1RadialGradientBrush,
    ID2D1SolidColorBrush, ID2D1StrokeStyle, D2D1_CAP_STYLE_ROUND,
    D2D1_COLORMATRIX_PROP_COLOR_MATRIX, D2D1_INTERPOLATION_MODE_LINEAR, D2D1_LAYER_PARAMETERS1,
    D2D1_LINE_JOIN_ROUND, D2D1_PROPERTY_TYPE, D2D1_PROPERTY_TYPE_FLOAT,
    D2D1_PROPERTY_TYPE_MATRIX_4X4, D2D1_PROPERTY_TYPE_MATRIX_5X4, D2D1_PROPERTY_TYPE_UINT32,
    D2D1_PROPERTY_TYPE_VECTOR4, D2D1_SHADOW_PROP_BLUR_STANDARD_DEVIATION, D2D1_SHADOW_PROP_COLOR,
    D2D1_SHADOW_PROP_OPTIMIZATION, D2D1_STROKE_STYLE_PROPERTIES1,
};
use windows::Win32::Graphics::Dwm::DwmGetColorizationColor;
use windows_numerics::Vector2;

use crate::capture::Item;
use crate::geometry::{rect_to_quad_matrix, Quad};
use crate::layout::{project_pose, DrawEntry};

#[inline]
unsafe fn set_val<T: Sized>(
    effect: &ID2D1Effect,
    index: impl TryInto<u32>,
    prop_type: D2D1_PROPERTY_TYPE,
    val: &T,
) {
    if let Ok(idx) = index.try_into() {
        let bytes =
            std::slice::from_raw_parts(val as *const T as *const u8, std::mem::size_of::<T>());
        let _ = effect.SetValue(idx, prop_type, bytes);
    }
}

pub const TRANSFORM_3D_PROP_INTERPOLATION_MODE: u32 = 0;
pub const TRANSFORM_3D_PROP_BORDER_MODE: u32 = 1;
pub const TRANSFORM_3D_PROP_TRANSFORM_MATRIX: u32 = 2;
pub const TRANSFORM_3D_INTERPOLATION_ANISOTROPIC: u32 = 4;
pub const SHADOW_OPTIMIZATION_SPEED: u32 = 0;

#[derive(Default)]
pub struct EffectChain {
    pub transform: Option<ID2D1Effect>,
    pub color: Option<ID2D1Effect>,
    pub shadow: Option<ID2D1Effect>,
    pub input_ptr: usize,
}

pub fn get_accent_color() -> D2D1_COLOR_F {
    let mut color = 0u32;
    let mut opaque = windows_core::BOOL(0);
    // SAFETY: DwmGetColorizationColor retrieves current accent color.
    let hr = unsafe { DwmGetColorizationColor(&mut color, &mut opaque) };
    if hr.is_err() {
        return D2D1_COLOR_F {
            r: 0.30,
            g: 0.76,
            b: 1.0,
            a: 1.0,
        };
    }
    D2D1_COLOR_F {
        r: ((color >> 16) & 0xff) as f32 / 255.0,
        g: ((color >> 8) & 0xff) as f32 / 255.0,
        b: (color & 0xff) as f32 / 255.0,
        a: 1.0,
    }
}

pub fn ensure_chain(ctx: &ID2D1DeviceContext, chain: &mut EffectChain) -> bool {
    if chain.transform.is_some() {
        return true;
    }

    // SAFETY: Create effects for 3D transform, color matrix, and shadow.
    unsafe {
        let transform = match ctx.CreateEffect(&CLSID_D2D13DTransform) {
            Ok(e) => e,
            Err(_) => return false,
        };
        let color = match ctx.CreateEffect(&CLSID_D2D1ColorMatrix) {
            Ok(e) => e,
            Err(_) => return false,
        };
        let shadow = match ctx.CreateEffect(&CLSID_D2D1Shadow) {
            Ok(e) => e,
            Err(_) => return false,
        };

        let interp = TRANSFORM_3D_INTERPOLATION_ANISOTROPIC;
        set_val(
            &transform,
            TRANSFORM_3D_PROP_INTERPOLATION_MODE,
            D2D1_PROPERTY_TYPE_UINT32,
            &interp,
        );

        let border_mode = D2D1_BORDER_MODE_SOFT.0 as u32;
        set_val(
            &transform,
            TRANSFORM_3D_PROP_BORDER_MODE,
            D2D1_PROPERTY_TYPE_UINT32,
            &border_mode,
        );

        if let Ok(transform_out) = transform.GetOutput() {
            color.SetInput(0, &transform_out, true);
        }
        if let Ok(color_out) = color.GetOutput() {
            shadow.SetInput(0, &color_out, true);
        }

        let shadow_opt = SHADOW_OPTIMIZATION_SPEED;
        set_val(
            &shadow,
            D2D1_SHADOW_PROP_OPTIMIZATION.0,
            D2D1_PROPERTY_TYPE_UINT32,
            &shadow_opt,
        );

        chain.transform = Some(transform);
        chain.color = Some(color);
        chain.shadow = Some(shadow);
        true
    }
}

#[allow(clippy::too_many_arguments)]
pub fn draw_card(
    ctx: &ID2D1DeviceContext,
    brush: &ID2D1SolidColorBrush,
    d2d_factory: &ID2D1Factory1,
    item: &mut Item,
    chain_idx: usize,
    entry: &DrawEntry,
    t: f32,
    shadows: bool,
    is_itab: bool,
    focal: f32,
    screen_h: f32,
) {
    let chain = &mut item.chains[chain_idx];
    if !ensure_chain(ctx, chain) {
        return;
    }

    let bitmap = match item.content.as_ref().or(item.placeholder.as_ref()) {
        Some(b) => b,
        None => return,
    };

    let bitmap_ptr = bitmap.as_raw() as usize;
    let transform = chain.transform.as_ref().unwrap();
    let color_effect = chain.color.as_ref().unwrap();
    let shadow_effect = chain.shadow.as_ref().unwrap();

    // SAFETY: Update effect inputs, matrix, and draw.
    unsafe {
        if chain.input_ptr != bitmap_ptr {
            transform.SetInput(0, bitmap, true);
            chain.input_ptr = bitmap_ptr;
        }

        let size = bitmap.GetSize();
        let matrix = rect_to_quad_matrix(size.width, size.height, &entry.quad);
        set_val(
            transform,
            TRANSFORM_3D_PROP_TRANSFORM_MATRIX,
            D2D1_PROPERTY_TYPE_MATRIX_4X4,
            &matrix,
        );

        let b = entry.pose.brightness
            * if item.close_requested_at > 0.0 {
                0.6
            } else {
                1.0
            };
        let color_matrix: [f32; 20] = [
            b,
            0.0,
            0.0,
            0.0,
            0.0,
            b,
            0.0,
            0.0,
            0.0,
            0.0,
            b,
            0.0,
            0.0,
            0.0,
            0.0,
            entry.pose.opacity,
            0.0,
            0.0,
            0.0,
            0.0,
        ];

        set_val(
            color_effect,
            D2D1_COLORMATRIX_PROP_COLOR_MATRIX.0,
            D2D1_PROPERTY_TYPE_MATRIX_5X4,
            &color_matrix,
        );

        if shadows && t > 0.01 && !is_itab {
            let depth = focal / (focal + entry.pose.z).max(focal * 0.05);
            let blur_std_dev = 14.0 * depth;
            set_val(
                shadow_effect,
                D2D1_SHADOW_PROP_BLUR_STANDARD_DEVIATION.0,
                D2D1_PROPERTY_TYPE_FLOAT,
                &blur_std_dev,
            );

            let shadow_color = [0.0f32, 0.0, 0.0, 0.55 * t];
            set_val(
                shadow_effect,
                D2D1_SHADOW_PROP_COLOR.0,
                D2D1_PROPERTY_TYPE_VECTOR4,
                &shadow_color,
            );

            let offset = Vector2 {
                X: 0.0,
                Y: 10.0 * depth * t,
            };
            if let Ok(shadow_out) = shadow_effect.GetOutput() {
                ctx.DrawImage(
                    &shadow_out,
                    Some(&offset),
                    None,
                    D2D1_INTERPOLATION_MODE_LINEAR,
                    D2D1_COMPOSITE_MODE_SOURCE_OVER,
                );
            }
        }

        if let Ok(color_out) = color_effect.GetOutput() {
            ctx.DrawImage(
                &color_out,
                None,
                None,
                D2D1_INTERPOLATION_MODE_LINEAR,
                D2D1_COMPOSITE_MODE_SOURCE_OVER,
            );
        }

        let highlight = entry.pose.highlight * entry.pose.opacity * t;
        if highlight > 0.01 {
            if let Ok(outline) = d2d_factory.CreatePathGeometry() {
                if let Ok(sink) = outline.Open() {
                    let p0: Vector2 = entry.quad.p[0].into();
                    let pts: [Vector2; 3] = [
                        entry.quad.p[1].into(),
                        entry.quad.p[2].into(),
                        entry.quad.p[3].into(),
                    ];
                    sink.BeginFigure(p0, D2D1_FIGURE_BEGIN_HOLLOW);
                    sink.AddLines(&pts);
                    sink.EndFigure(D2D1_FIGURE_END_CLOSED);
                    if sink.Close().is_ok() {
                        brush.SetColor(&D2D1_COLOR_F {
                            r: 1.0,
                            g: 1.0,
                            b: 1.0,
                            a: 0.9 * highlight,
                        });
                        let stroke_props = D2D1_STROKE_STYLE_PROPERTIES1 {
                            startCap: D2D1_CAP_STYLE_ROUND,
                            endCap: D2D1_CAP_STYLE_ROUND,
                            dashCap: D2D1_CAP_STYLE_ROUND,
                            lineJoin: D2D1_LINE_JOIN_ROUND,
                            miterLimit: 10.0,
                            ..Default::default()
                        };
                        let stroke_style = d2d_factory
                            .CreateStrokeStyle(&stroke_props, None)
                            .ok()
                            .and_then(|s| s.cast::<ID2D1StrokeStyle>().ok());
                        ctx.DrawGeometry(&outline, brush, (screen_h / 360.0).max(2.0), stroke_style.as_ref());
                    }
                }
            }
        }
    }
}

pub fn floor_screen_y(height: f32, floor_y: f32, window_size: f32) -> f32 {
    height / 2.0 + floor_y * window_size
}

pub fn draw_floor(
    ctx: &ID2D1DeviceContext,
    brush: &ID2D1SolidColorBrush,
    floor_light: Option<&ID2D1RadialGradientBrush>,
    width: f32,
    height: f32,
    floor_y: f32,
    cover: f32,
) {
    let screen = D2D_RECT_F {
        left: 0.0,
        top: 0.0,
        right: width,
        bottom: height,
    };

    // SAFETY: Fill solid black floor and radial floor light.
    unsafe {
        brush.SetColor(&D2D1_COLOR_F {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: cover,
        });
        ctx.FillRectangle(&screen, brush);

        if let Some(light) = floor_light {
            light.SetCenter(Vector2 {
                X: width / 2.0,
                Y: floor_y,
            });
            light.SetRadiusX(width * 0.55);
            light.SetRadiusY(height * 0.16);
            light.SetOpacity(cover);
            ctx.FillRectangle(&screen, light);
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn draw_reflections(
    ctx: &ID2D1DeviceContext,
    draw_list: &[DrawEntry],
    items: &mut [Item],
    reflection_fade: Option<&ID2D1LinearGradientBrush>,
    width: f32,
    height: f32,
    floor_y: f32,
    t: f32,
    focal: f32,
) {
    let opacity = t * t;
    if opacity < 0.01 {
        return;
    }
    let fade = match reflection_fade {
        Some(f) => f,
        None => return,
    };

    // SAFETY: PushLayer with reflection fade gradient brush, draw mirrored cards, pop layer.
    unsafe {
        fade.SetStartPoint(Vector2 { X: 0.0, Y: floor_y });
        fade.SetEndPoint(Vector2 {
            X: 0.0,
            Y: floor_y + height * 0.20,
        });

        let layer_params = D2D1_LAYER_PARAMETERS1 {
            contentBounds: windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F {
                left: f32::NEG_INFINITY,
                top: f32::NEG_INFINITY,
                right: f32::INFINITY,
                bottom: f32::INFINITY,
            },
            opacityBrush: std::mem::ManuallyDrop::new(Some(fade.cast().unwrap())),
            opacity: 1.0,
            ..Default::default()
        };

        ctx.PushLayer(&layer_params, None);

        for entry in draw_list {
            if entry.item_index >= items.len() {
                continue;
            }
            let item = &mut items[entry.item_index];
            let item_width = item.width();
            let item_height = item.height();
            let chain = &mut item.chains[1];
            let bitmap = match item.content.as_ref().or(item.placeholder.as_ref()) {
                Some(b) => b,
                None => continue,
            };
            if !ensure_chain(ctx, chain) {
                continue;
            }

            let bitmap_ptr = bitmap.as_raw() as usize;
            let transform = chain.transform.as_ref().unwrap();
            let color_effect = chain.color.as_ref().unwrap();

            if chain.input_ptr != bitmap_ptr {
                transform.SetInput(0, bitmap, true);
                chain.input_ptr = bitmap_ptr;
            }

            let mut mirror = entry.pose;
            mirror.y += item_height * mirror.scale * mirror.squeeze;
            let quad = project_pose(&mirror, item_width, item_height, focal, width, height);
            let flipped = Quad {
                p: [quad.p[3], quad.p[2], quad.p[1], quad.p[0]],
            };

            let size = bitmap.GetSize();
            let matrix = rect_to_quad_matrix(size.width, size.height, &flipped);
            set_val(
                transform,
                TRANSFORM_3D_PROP_TRANSFORM_MATRIX,
                D2D1_PROPERTY_TYPE_MATRIX_4X4,
                &matrix,
            );

            let b = entry.pose.brightness * 0.9;
            let color_matrix: [f32; 20] = [
                b,
                0.0,
                0.0,
                0.0,
                0.0,
                b,
                0.0,
                0.0,
                0.0,
                0.0,
                b,
                0.0,
                0.0,
                0.0,
                0.0,
                entry.pose.opacity * opacity,
                0.0,
                0.0,
                0.0,
                0.0,
            ];

            set_val(
                color_effect,
                D2D1_COLORMATRIX_PROP_COLOR_MATRIX.0,
                D2D1_PROPERTY_TYPE_MATRIX_5X4,
                &color_matrix,
            );

            if let Ok(color_out) = color_effect.GetOutput() {
                ctx.DrawImage(
                    &color_out,
                    None,
                    None,
                    D2D1_INTERPOLATION_MODE_LINEAR,
                    D2D1_COMPOSITE_MODE_SOURCE_OVER,
                );
            }
        }

        ctx.PopLayer();
    }
}
