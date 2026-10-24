// Original Windhawk mod by caliberda
// Ported to standalone Rust by Antigravity

use windows::Win32::Foundation::RECT;
use windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F;

use crate::geometry::{
    ease_out_quart, lerp, lerp_pose, now_seconds, positive_mod_f32, positive_mod_i64, Pose, Quad,
    D2D_POINT_2F,
};
use crate::settings::{AnimationStyle, Settings};

pub const CLOSE_ANIMATION_SECONDS: f64 = 0.22;
pub const SLIDE_ANIMATION_SECONDS: f64 = 0.32;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CloseButton {
    pub center: D2D_POINT_2F,
    pub radius: f32,
}

impl CloseButton {
    pub fn contains(&self, point: D2D_POINT_2F) -> bool {
        let dx = point.x - self.center.x;
        let dy = point.y - self.center.y;
        self.radius > 0.0 && dx * dx + dy * dy <= self.radius * self.radius
    }
}

pub fn is_panel_style(style: AnimationStyle) -> bool {
    matches!(
        style,
        AnimationStyle::Windows11
            | AnimationStyle::Thumbnails
            | AnimationStyle::IconRow
            | AnimationStyle::List
            | AnimationStyle::Classic
    )
}

pub fn is_solid_style(style: AnimationStyle) -> bool {
    matches!(style, AnimationStyle::Cube | AnimationStyle::Sphere)
}

pub fn is_itab_style(style: AnimationStyle) -> bool {
    matches!(
        style,
        AnimationStyle::ITab
            | AnimationStyle::ITabTitle
            | AnimationStyle::ITabBackground
            | AnimationStyle::ITabBackgroundTitle
    )
}

pub fn has_black_floor(style: AnimationStyle) -> bool {
    matches!(style, AnimationStyle::ITab | AnimationStyle::ITabTitle)
}

pub fn shows_app_name(style: AnimationStyle) -> bool {
    matches!(style, AnimationStyle::ITab | AnimationStyle::ITabBackground)
}

#[derive(Debug, Clone, Copy)]
pub struct LayoutMetrics {
    pub width: f32,
    pub height: f32,
    pub focal: f32,
    pub box_width: f32,
    pub box_height: f32,
    pub front_x: f32,
    pub front_y: f32,
    pub step_x: f32,
    pub step_y: f32,
    pub step_z: f32,
    pub grid_columns: i32,
    pub grid_rows: i32,
    pub floor_y: f32,
}

impl Default for LayoutMetrics {
    fn default() -> Self {
        Self {
            width: 0.0,
            height: 0.0,
            focal: 0.0,
            box_width: 0.0,
            box_height: 0.0,
            front_x: 0.0,
            front_y: 0.0,
            step_x: 0.0,
            step_y: 0.0,
            step_z: 0.0,
            grid_columns: 1,
            grid_rows: 1,
            floor_y: 0.0,
        }
    }
}

pub fn compute_layout_metrics(
    style: AnimationStyle,
    settings: &Settings,
    width: f32,
    height: f32,
    count: usize,
) -> LayoutMetrics {
    let mut m = LayoutMetrics {
        width,
        height,
        focal: width * 0.95,
        ..Default::default()
    };

    let count_i = count.max(1) as i32;
    let spacing = settings.stack_spacing;
    let density = (1.0f32).min(7.0 / (count_i - 1).max(1) as f32) * spacing;

    match style {
        AnimationStyle::Native => {}

        AnimationStyle::Flip3D => {
            m.box_width = m.width * 0.48;
            m.box_height = m.height * 0.50;
            m.front_x = m.width * 0.12;
            m.front_y = m.height * 0.05;
            m.step_x = -m.width * 0.065 * density;
            m.step_y = -m.height * 0.04 * density;
            m.step_z = m.width * 0.07 * density;
        }

        AnimationStyle::Cascade => {
            m.box_width = m.width * 0.52;
            m.box_height = m.height * 0.50;
            m.front_x = 0.0;
            m.front_y = m.height * 0.10;
            m.step_x = 0.0;
            m.step_y = -m.height * 0.075 * density;
            m.step_z = m.width * 0.09 * density;
        }

        AnimationStyle::CoverFlow => {
            m.box_width = m.width * 0.42;
            m.box_height = m.height * 0.55;
            m.front_x = m.width * 0.27;
            m.front_y = -m.height * 0.02;
            m.step_x = m.width * 0.075 * spacing;
            m.step_y = 0.0;
            m.step_z = m.width * 0.22;
        }

        AnimationStyle::ITab
        | AnimationStyle::ITabTitle
        | AnimationStyle::ITabBackground
        | AnimationStyle::ITabBackgroundTitle => {
            m.box_width = m.width * 0.30;
            m.box_height = m.height * 0.42;
            m.floor_y = m.height * 0.14;
            m.front_x = m.box_width * 0.80;
            m.front_y = 0.0;
            m.step_x = m.box_width * 0.19 * spacing;
            m.step_y = 0.0;
            m.step_z = m.box_width * 0.40;
        }

        AnimationStyle::Carousel => {
            m.box_width = m.width * 0.38;
            m.box_height = m.height * 0.45;
            m.front_x = 0.0;
            m.front_y = m.height * 0.08;
            m.step_x = m.width * 0.30 * spacing;
            m.step_y = 0.0;
            m.step_z = 2.0 * std::f32::consts::PI / count_i.max(5) as f32;
        }

        AnimationStyle::Grid => {
            let area_width = m.width * 0.92;
            let area_height = m.height * 0.78;
            m.grid_columns = ((count_i as f32 * area_width / area_height / 1.6)
                .sqrt()
                .ceil() as i32)
                .clamp(1, count_i);
            m.grid_rows = (count_i + m.grid_columns - 1) / m.grid_columns;
            m.box_width = area_width / m.grid_columns as f32;
            m.box_height = area_height / m.grid_rows as f32;
            m.front_x = 0.0;
            m.front_y = m.height * 0.05 - m.height / 2.0;
            m.step_x = 0.0;
            m.step_y = 0.0;
            m.step_z = 0.0;
        }

        AnimationStyle::Helix => {
            m.box_width = m.width * 0.36;
            m.box_height = m.height * 0.40;
            m.front_x = 0.0;
            m.front_y = m.height * 0.02;
            m.step_x = m.width * 0.28 * spacing;
            m.step_y = m.height * 0.09 * spacing;
            m.step_z = 0.75;
        }

        AnimationStyle::Fan => {
            m.box_width = m.width * 0.34;
            m.box_height = m.height * 0.42;
            m.front_x = 0.0;
            m.front_y = m.height * 0.95;
            m.step_x = m.height * 0.95;
            m.step_y = 0.0;
            m.step_z = 0.16 * spacing;
        }

        AnimationStyle::Panorama => {
            m.box_width = m.width * 0.26;
            m.box_height = m.height * 0.36;
            m.front_x = 0.0;
            m.front_y = -m.height * 0.02;
            m.step_x = m.width * 1.4;
            m.step_y = 0.0;
            m.step_z = m.width * 0.29 * spacing / m.step_x;
        }

        AnimationStyle::Tunnel => {
            m.box_width = m.width * 0.42;
            m.box_height = m.height * 0.44;
            m.front_x = 0.0;
            m.front_y = m.height * 0.08;
            m.step_x = 0.0;
            m.step_y = -m.height * 0.06 * density;
            m.step_z = m.width * 0.12 * density;
        }

        AnimationStyle::Windows11
        | AnimationStyle::Thumbnails
        | AnimationStyle::IconRow
        | AnimationStyle::List
        | AnimationStyle::Classic => {}

        AnimationStyle::Rolodex => {
            m.box_width = m.width * 0.36;
            m.box_height = m.height * 0.30;
            m.front_x = 0.0;
            m.front_y = -m.height * 0.04;
            m.step_x = m.height * 0.42 * spacing;
            m.step_y = 0.0;
            m.step_z = 0.75;
        }

        AnimationStyle::Cube => {
            m.box_width = m.width * 0.40;
            m.box_height = m.height * 0.46;
            m.front_x = 0.0;
            m.front_y = -m.height * 0.02;
            m.step_x = m.box_width / 2.0;
            m.step_y = 0.0;
            m.step_z = std::f32::consts::PI / 2.0;
        }

        AnimationStyle::Sphere => {
            m.box_width = m.width * 0.25;
            m.box_height = m.height * 0.30;
            m.front_x = 0.0;
            m.front_y = -m.height * 0.01;
            m.step_x = m.height * 0.40 * spacing;
            const MAX_LATITUDE: f32 = 1.15;
            m.step_z = (m.box_width * 0.95 / m.step_x)
                .clamp(0.3, 1.0)
                .min(2.0 * (std::f32::consts::PI * MAX_LATITUDE / count_i as f32).sqrt());
            m.step_y = m.step_z * m.step_z / (2.0 * std::f32::consts::PI);
        }

        AnimationStyle::Shuffle => {
            m.box_width = m.width * 0.46;
            m.box_height = m.height * 0.48;
            m.front_x = 0.0;
            m.front_y = m.height * 0.12;
            m.step_x = 0.0;
            m.step_y = -m.height * 0.02 * density;
            m.step_z = m.width * 0.035 * density;
        }

        AnimationStyle::Domino => {
            m.box_width = m.width * 0.30;
            m.box_height = m.height * 0.42;
            m.front_x = -m.width * 0.16;
            m.front_y = m.height * 0.06;
            m.step_x = m.width * 0.10 * density;
            m.step_y = 0.0;
            m.step_z = m.width * 0.15 * density;
        }
    }

    m
}

pub fn flat_pose(
    item_rect: RECT,
    monitor: RECT,
    width: f32,
    height: f32,
    off_screen: bool,
) -> Pose {
    Pose {
        x: (item_rect.left + item_rect.right) as f32 / 2.0 - monitor.left as f32 - width / 2.0,
        y: (item_rect.top + item_rect.bottom) as f32 / 2.0 - monitor.top as f32 - height / 2.0,
        scale: if off_screen { 0.85 } else { 1.0 },
        opacity: if off_screen { 0.0 } else { 1.0 },
        ..Default::default()
    }
}

pub fn project_pose(
    pose: &Pose,
    width: f32,
    height: f32,
    focal: f32,
    screen_w: f32,
    screen_h: f32,
) -> Quad {
    let half_width = width * pose.scale / 2.0;
    let half_height = height * pose.scale * pose.squeeze / 2.0;
    let cos_y = pose.angle_y.cos();
    let sin_y = pose.angle_y.sin();
    let cos_x = pose.angle_x.cos();
    let sin_x = pose.angle_x.sin();
    let cos_z = pose.angle_z.cos();
    let sin_z = pose.angle_z.sin();

    let corners = [
        [-half_width, -half_height],
        [half_width, -half_height],
        [half_width, half_height],
        [-half_width, half_height],
    ];

    let mut quad = Quad::default();
    for (i, corner) in corners.iter().enumerate() {
        let local_x = corner[0] * cos_z - corner[1] * sin_z;
        let local_y = corner[0] * sin_z + corner[1] * cos_z;
        let turned_z = local_x * sin_y;
        let x = pose.x + local_x * cos_y;
        let y = pose.y + local_y * cos_x + turned_z * sin_x;
        let z = pose.z + turned_z * cos_x - local_y * sin_x;
        let k = focal / (focal + z).max(focal * 0.05);
        quad.p[i] = D2D_POINT_2F {
            x: screen_w / 2.0 + x * k,
            y: screen_h / 2.0 + y * k,
        };
    }
    quad
}

#[allow(clippy::too_many_arguments)]
pub fn layout_pose(
    style: AnimationStyle,
    settings: &Settings,
    m: &LayoutMetrics,
    item_w: f32,
    item_h: f32,
    index: usize,
    rel: f32,
    count: usize,
    thumbnail_rect: Option<D2D_RECT_F>,
    flat_pose: Option<Pose>,
) -> Pose {
    let mut pose = Pose::default();
    let fit = (m.box_width / item_w).min(m.box_height / item_h).min(1.0);
    pose.scale = fit;
    let tilt = settings.tilt_radians;
    let distance = rel.abs();
    let side = if rel < 0.0 { -1.0 } else { 1.0 };

    match style {
        AnimationStyle::Native => {}

        AnimationStyle::Flip3D | AnimationStyle::Cascade => {
            let flip = style != AnimationStyle::Cascade;
            if flip {
                pose.angle_y = tilt;
            } else {
                pose.angle_x = tilt;
            }
            if rel >= 0.0 {
                pose.x = m.front_x + rel * m.step_x;
                pose.y = m.front_y + rel * m.step_y;
                pose.z = rel * m.step_z;
                pose.brightness = 1.0 - (rel * 0.07).min(0.55);
            } else if flip {
                pose.x = m.front_x - rel * m.width * 0.22;
                pose.y = m.front_y - rel * m.height * 0.10;
                pose.z = rel * m.focal * 0.45;
                pose.opacity = (1.0 + rel) * (1.0 + rel);
            } else {
                pose.x = m.front_x;
                pose.y = m.front_y - rel * m.height * 0.45;
                pose.z = rel * m.focal * 0.25;
                pose.opacity = (1.0 + rel) * (1.0 + rel);
            }
        }

        AnimationStyle::CoverFlow => {
            let inner = distance.min(1.0);
            let outer = (distance - 1.0).max(0.0);
            pose.x = side * (inner * m.front_x + outer * m.step_x);
            pose.y = m.front_y;
            pose.z = inner * m.step_z + outer * m.width * 0.01;
            pose.angle_y = rel.clamp(-1.0, 1.0) * (tilt * 2.2).min(1.3);
            pose.brightness = 1.0 - inner * 0.2 - (outer * 0.04).min(0.3);
            pose.opacity = (8.0 - distance).clamp(0.0, 1.0);
        }

        AnimationStyle::ITab
        | AnimationStyle::ITabTitle
        | AnimationStyle::ITabBackground
        | AnimationStyle::ITabBackgroundTitle => {
            let inner = distance.min(1.0);
            let outer = (distance - 1.0).max(0.0);
            pose.x = side * (inner * m.front_x + outer * m.step_x);
            pose.y = m.floor_y - item_h * fit / 2.0;
            pose.z = inner * m.step_z + outer * m.width * 0.012;
            pose.angle_y = -rel.clamp(-1.0, 1.0) * 1.05;
            pose.brightness = 1.0 - inner * 0.1 - (outer * 0.03).min(0.3);
            pose.opacity = (10.0 - distance).clamp(0.0, 1.0);
        }

        AnimationStyle::Carousel => {
            let angle = rel * m.step_z;
            pose.x = m.step_x * angle.sin();
            pose.z = m.step_x * (1.0 - angle.cos());
            pose.y = m.front_y - pose.z * 0.18;
            pose.angle_y = angle * 0.55;
            pose.brightness = 1.0 - (1.0 - angle.cos()) * 0.22;
            pose.opacity = (3.5 - angle.abs()).clamp(0.0, 1.0);
        }

        AnimationStyle::Grid => {
            let row = index as i32 / m.grid_columns;
            let column = index as i32 % m.grid_columns;
            let in_row = m.grid_columns.min(count as i32 - row * m.grid_columns);
            let selected = (1.0 - distance).max(0.0);
            let cell_fit = (m.box_width * 0.88 / item_w).min(m.box_height * 0.84 / item_h);
            pose.scale =
                cell_fit.min(1.0) * settings.window_size.min(cell_fit) * (1.0 + selected * 0.05);
            pose.x = (column as f32 - (in_row - 1) as f32 / 2.0) * m.box_width;
            pose.y = m.front_y + (row as f32 + 0.5) * m.box_height;
            pose.z = -selected * m.width * 0.02;
            pose.brightness = 0.72 + selected * 0.28;
            pose.highlight = selected;
        }

        AnimationStyle::Helix => {
            let angle = rel * m.step_z;
            pose.x = m.step_x * angle.sin();
            pose.z = m.step_x * (1.0 - angle.cos());
            pose.y = m.front_y + rel * m.step_y;
            pose.angle_y = angle * 0.6;
            pose.brightness = 1.0 - (1.0 - angle.cos()) * 0.22;
            pose.opacity = (3.6 - angle.abs()).clamp(0.0, 1.0);
        }

        AnimationStyle::Fan => {
            let angle = rel * m.step_z;
            let selected = (1.0 - distance).max(0.0);
            pose.x = m.step_x * angle.sin();
            pose.y = m.front_y - m.step_x * angle.cos() - selected * m.height * 0.05;
            pose.z = distance * m.width * 0.004 - selected * m.width * 0.02;
            pose.angle_z = angle;
            pose.brightness = 0.8 + selected * 0.2;
            pose.opacity = ((1.45 - angle.abs()) * 3.0).clamp(0.0, 1.0);
        }

        AnimationStyle::Panorama => {
            let angle = rel * m.step_z;
            let selected = (1.0 - distance).max(0.0);
            pose.x = m.step_x * angle.sin();
            pose.y = m.front_y;
            pose.z = -m.step_x * (1.0 - angle.cos()) * 0.25 - selected * m.width * 0.02;
            pose.angle_y = -angle * 0.8;
            pose.scale = fit * (1.0 + selected * 0.08);
            pose.brightness = 0.7 + selected * 0.3;
            pose.highlight = selected;
            pose.opacity = (4.5 - distance).clamp(0.0, 1.0);
        }

        AnimationStyle::Tunnel => {
            pose.x = m.front_x;
            pose.y = m.front_y;
            if rel >= 0.0 {
                pose.y += rel * m.step_y;
                pose.z = rel * m.step_z;
                pose.angle_z = rel * 0.3;
                pose.brightness = 1.0 - (rel * 0.09).min(0.6);
            } else {
                pose.z = rel * m.focal * 0.4;
                pose.angle_z = rel * 0.6;
                pose.opacity = (1.0 + rel) * (1.0 + rel);
            }
        }

        AnimationStyle::Windows11 | AnimationStyle::Thumbnails => {
            if let Some(thumbnail) = thumbnail_rect {
                let selected = (1.0 - distance).max(0.0);
                pose.x = (thumbnail.left + thumbnail.right) / 2.0 - m.width / 2.0;
                pose.y = (thumbnail.top + thumbnail.bottom) / 2.0 - m.height / 2.0;
                pose.z = -selected * m.width * 0.002;
                pose.scale = ((thumbnail.right - thumbnail.left) / item_w)
                    .min((thumbnail.bottom - thumbnail.top) / item_h)
                    * (1.0 + selected * 0.03);
            }
        }

        AnimationStyle::IconRow | AnimationStyle::List | AnimationStyle::Classic => {
            if let Some(flat) = flat_pose {
                pose = flat;
                pose.opacity = 0.0;
            }
        }

        AnimationStyle::Rolodex => {
            let angle = rel * m.step_z;
            pose.x = m.front_x;
            pose.y = m.front_y + m.step_x * angle.sin();
            pose.z = m.step_x * (1.0 - angle.cos());
            pose.angle_x = -angle;
            pose.brightness = 1.0 - (1.0 - angle.cos()) * 0.45;
            pose.opacity = ((1.9 - angle.abs()) * 2.0).clamp(0.0, 1.0);
        }

        AnimationStyle::Cube => {
            let angle = rel * m.step_z;
            let half = m.step_x;
            pose.scale = 2.0 * half / item_w;
            pose.squeeze = (1.0f32).min(m.box_height / (item_h * pose.scale));
            pose.x = half * angle.sin();
            pose.y = m.front_y;
            pose.z = half * (1.0 - angle.cos()) + half * 0.35;
            pose.angle_y = angle;
            pose.brightness = 0.45 + 0.55 * angle.cos().max(0.0);
            pose.opacity = ((1.0 - distance) * 30.0).clamp(0.0, 1.0);
        }

        AnimationStyle::Sphere => {
            let selected = (1.0 - distance).max(0.0);
            let latitude = -rel * m.step_y;
            let longitude = rel * m.step_z;
            let facing = longitude.cos() * latitude.cos();
            pose.x = m.step_x * longitude.sin() * latitude.cos();
            pose.y = m.front_y - m.step_x * latitude.sin();
            pose.z = m.step_x * (1.0 - facing) - selected * m.step_x * 0.2;
            pose.angle_y = longitude;
            pose.angle_x = latitude;

            let cell = m.step_x * m.step_z;
            pose.scale = fit.min(cell * 0.92 / item_w).min(cell * 0.85 / item_h)
                * latitude.cos().max(0.5)
                * (1.0 + selected * 0.25);
            pose.brightness = 0.45 + 0.55 * facing.max(0.0);
            pose.highlight = selected;
            pose.opacity = ((facing - 0.3) * 4.0).clamp(0.0, 1.0);
            if count > 4 {
                let ends = count as f32 / 2.0 - distance;
                pose.opacity *= (ends * 1.5).clamp(0.0, 1.0);
            }
        }

        AnimationStyle::Shuffle => {
            if rel >= 0.0 {
                pose.x = m.front_x;
                pose.y = m.front_y + rel * m.step_y;
                pose.z = rel * m.step_z;
                pose.angle_z = ((index % 3) as i32 - 1) as f32 * 0.035 * rel.min(1.0);
                pose.brightness = 1.0 - (rel * 0.06).min(0.5);
            } else {
                let progress = -rel;
                let arc = (progress * std::f32::consts::PI).sin();
                let back = (count - 1).max(1) as f32;
                pose.x = m.front_x + arc * m.width * 0.16;
                pose.y = m.front_y + progress * back * m.step_y - arc * m.height * 0.62;
                pose.z = progress * back * m.step_z;
                pose.angle_x = arc * 0.8;
                pose.angle_z = -arc * 0.35;
                pose.brightness = 1.0 - (progress * back * 0.06).min(0.5);
            }
        }

        AnimationStyle::Domino => {
            let half_height = item_h * fit / 2.0;
            if rel >= 0.0 {
                pose.x = m.front_x + rel * m.step_x;
                pose.y = m.front_y;
                pose.z = rel * m.step_z;
                pose.angle_y = -0.35;
                pose.brightness = 1.0 - (rel * 0.08).min(0.55);
            } else {
                let progress = -rel;
                let fall = progress * 1.45;
                pose.x = m.front_x;
                pose.y = m.front_y + half_height - half_height * fall.cos();
                pose.z = -half_height * settings.window_size * fall.sin();
                pose.angle_x = -fall;
                pose.angle_y = -0.35 * (1.0 - progress);
                pose.opacity = 1.0 - progress * progress;
            }
        }
    }

    pose
}

#[derive(Debug, Clone, Default)]
pub struct PanelLayout {
    pub cells: Vec<D2D_RECT_F>,
    pub thumbnails: Vec<D2D_RECT_F>,
    pub icons: Vec<D2D_RECT_F>,
    pub labels: Vec<D2D_RECT_F>,
    pub panel: D2D_RECT_F,
    pub caption: D2D_RECT_F,
    pub label_font_size: f32,
}

#[derive(Clone, Copy)]
struct CellLayout {
    size_w: f32,
    size_h: f32,
    thumbnail: D2D_RECT_F,
    icon: D2D_RECT_F,
    label: D2D_RECT_F,
}

pub fn compute_panel_layout(
    style: AnimationStyle,
    settings: &Settings,
    items: &[(f32, f32)], // (width, height)
    screen_w: f32,
    screen_h: f32,
    dpi_scale: f32,
) -> PanelLayout {
    let count = items.len();
    let mut out = PanelLayout {
        cells: vec![D2D_RECT_F::default(); count],
        thumbnails: vec![D2D_RECT_F::default(); count],
        icons: vec![D2D_RECT_F::default(); count],
        labels: vec![D2D_RECT_F::default(); count],
        panel: D2D_RECT_F::default(),
        caption: D2D_RECT_F::default(),
        label_font_size: (14.0 * dpi_scale).round(),
    };

    if count == 0 {
        return out;
    }

    let vertical = style == AnimationStyle::List;
    let spacing = settings.stack_spacing;
    let font = out.label_font_size;

    let cell_for = |idx: usize, scale: f32| -> CellLayout {
        let (item_w, item_h) = items[idx];
        match style {
            AnimationStyle::Windows11 | AnimationStyle::Thumbnails => {
                let height = (screen_h * 0.17 * scale).round();
                let width = (height * item_w / item_h)
                    .clamp(height * 0.75, height * 1.9)
                    .round();
                let pad = (height * 0.07).round();
                let label = (font * 2.1).round();
                let icon = (font * 1.35).round();
                let size_w = width + 2.0 * pad;
                let size_h = label + height + 2.0 * pad;

                let thumbnail_top = if style == AnimationStyle::Windows11 {
                    pad + label
                } else {
                    pad
                };
                let label_top = if style == AnimationStyle::Windows11 {
                    pad
                } else {
                    pad + height
                };
                let thumbnail = D2D_RECT_F {
                    left: pad,
                    top: thumbnail_top,
                    right: pad + width,
                    bottom: thumbnail_top + height,
                };
                let icon_top = (label_top + (label - icon) / 2.0).round();
                let icon_rect = D2D_RECT_F {
                    left: pad,
                    top: icon_top,
                    right: pad + icon,
                    bottom: icon_top + icon,
                };
                let label_rect = D2D_RECT_F {
                    left: pad + icon + (font * 0.55).round(),
                    top: label_top,
                    right: pad + width,
                    bottom: label_top + label,
                };
                CellLayout {
                    size_w,
                    size_h,
                    thumbnail,
                    icon: icon_rect,
                    label: label_rect,
                }
            }

            AnimationStyle::IconRow => {
                let icon = (font * 3.0 * scale).round();
                let width = (font * 11.0 * scale).round();
                let pad = (font * 0.9 * scale).round();
                let label = (font * 2.1).round();
                let icon_left = ((width - icon) / 2.0).round();
                let size_w = width;
                let size_h = pad + icon + font * 0.5 + label + pad;
                let icon_rect = D2D_RECT_F {
                    left: icon_left,
                    top: pad,
                    right: icon_left + icon,
                    bottom: pad + icon,
                };
                let label_rect = D2D_RECT_F {
                    left: pad,
                    top: pad + icon + font * 0.5,
                    right: width - pad,
                    bottom: pad + icon + font * 0.5 + label,
                };
                CellLayout {
                    size_w,
                    size_h,
                    thumbnail: D2D_RECT_F::default(),
                    icon: icon_rect,
                    label: label_rect,
                }
            }

            AnimationStyle::List => {
                let height = (font * 2.7 * scale).round();
                let width = (screen_w * 0.34).min(font * 30.0).round();
                let icon = (font * 1.45).round();
                let pad = (font * 0.8).round();
                let icon_top = ((height - icon) / 2.0).round();
                let size_w = width;
                let size_h = height;
                let icon_rect = D2D_RECT_F {
                    left: pad,
                    top: icon_top,
                    right: pad + icon,
                    bottom: icon_top + icon,
                };
                let label_rect = D2D_RECT_F {
                    left: pad + icon + (font * 0.7).round(),
                    top: 0.0,
                    right: width - pad,
                    bottom: height,
                };
                CellLayout {
                    size_w,
                    size_h,
                    thumbnail: D2D_RECT_F::default(),
                    icon: icon_rect,
                    label: label_rect,
                }
            }

            _ => {
                let size = (font * 3.6 * scale).round();
                let icon = (font * 2.4 * scale).round();
                let offset = ((size - icon) / 2.0).round();
                let icon_rect = D2D_RECT_F {
                    left: offset,
                    top: offset,
                    right: offset + icon,
                    bottom: offset + icon,
                };
                CellLayout {
                    size_w: size,
                    size_h: size,
                    thumbnail: D2D_RECT_F::default(),
                    icon: icon_rect,
                    label: D2D_RECT_F::default(),
                }
            }
        }
    };

    let gap = match style {
        AnimationStyle::Windows11 | AnimationStyle::Thumbnails => {
            (screen_h * 0.01 * spacing).round()
        }
        AnimationStyle::IconRow => (font * 0.4 * spacing).round(),
        AnimationStyle::List => (font * 0.25 * spacing).round(),
        _ => (font * 0.15 * spacing).round(),
    };

    let max_per_line = if style == AnimationStyle::Classic {
        7
    } else {
        count
    };
    let max_line = if vertical {
        screen_h * 0.78
    } else {
        screen_w * 0.86
    };
    let max_across = if vertical {
        screen_w * 0.86
    } else {
        screen_h * 0.78
    };

    let mut cells = vec![
        CellLayout {
            size_w: 0.0,
            size_h: 0.0,
            thumbnail: D2D_RECT_F::default(),
            icon: D2D_RECT_F::default(),
            label: D2D_RECT_F::default(),
        };
        count
    ];
    let mut line_of = vec![0usize; count];
    let mut line_lengths = Vec::new();
    let mut line_thickness = Vec::new();
    let mut scale = 1.0f32;

    for _ in 0..10 {
        line_lengths.clear();
        line_thickness.clear();
        let mut in_line = 0;
        for i in 0..count {
            cells[i] = cell_for(i, scale);
            let along = if vertical {
                cells[i].size_h
            } else {
                cells[i].size_w
            };
            let across = if vertical {
                cells[i].size_w
            } else {
                cells[i].size_h
            };
            if line_lengths.is_empty()
                || in_line >= max_per_line
                || line_lengths.last().copied().unwrap_or(0.0) + gap + along > max_line
            {
                line_lengths.push(along);
                line_thickness.push(across);
                in_line = 1;
            } else {
                if let Some(last) = line_lengths.last_mut() {
                    *last += gap + along;
                }
                if let Some(last) = line_thickness.last_mut() {
                    *last = last.max(across);
                }
                in_line += 1;
            }
            line_of[i] = line_lengths.len() - 1;
        }

        let total = gap * (line_thickness.len().saturating_sub(1) as f32)
            + line_thickness.iter().sum::<f32>();
        if total <= max_across {
            break;
        }
        scale *= 0.85;
    }

    let center_lines = style != AnimationStyle::Classic && style != AnimationStyle::List;
    let total_across =
        gap * (line_thickness.len().saturating_sub(1) as f32) + line_thickness.iter().sum::<f32>();
    let longest = line_lengths.iter().copied().fold(0.0f32, f32::max);

    let mut along = 0.0f32;
    let mut across = (((if vertical { screen_w } else { screen_h }) - total_across) / 2.0).round();

    for i in 0..count {
        let line = line_of[i];
        if i > 0 && line_of[i - 1] != line {
            across += line_thickness[line - 1] + gap;
        }
        if i == 0 || line_of[i - 1] != line {
            let length = if center_lines {
                line_lengths[line]
            } else {
                longest
            };
            let line_start =
                (((if vertical { screen_h } else { screen_w }) - length) / 2.0).round();
            along = line_start;
        }

        let cell = cells[i];
        let x = if vertical { across } else { along };
        let y = if vertical { along } else { across };
        let offset = |r: D2D_RECT_F| D2D_RECT_F {
            left: x + r.left,
            top: y + r.top,
            right: x + r.right,
            bottom: y + r.bottom,
        };

        out.cells[i] = D2D_RECT_F {
            left: x,
            top: y,
            right: x + cell.size_w,
            bottom: y + cell.size_h,
        };
        out.thumbnails[i] = offset(cell.thumbnail);
        out.icons[i] = offset(cell.icon);
        out.labels[i] = offset(cell.label);
        along += (if vertical { cell.size_h } else { cell.size_w }) + gap;
    }

    out.panel = out.cells[0];
    for cell in &out.cells {
        out.panel.left = out.panel.left.min(cell.left);
        out.panel.top = out.panel.top.min(cell.top);
        out.panel.right = out.panel.right.max(cell.right);
        out.panel.bottom = out.panel.bottom.max(cell.bottom);
    }
    let margin = (font
        * if style == AnimationStyle::Classic {
            0.9
        } else {
            1.1
        })
    .round();
    out.panel = D2D_RECT_F {
        left: out.panel.left - margin,
        top: out.panel.top - margin,
        right: out.panel.right + margin,
        bottom: out.panel.bottom + margin,
    };

    if style == AnimationStyle::Classic {
        let height = (font * 2.2).round();
        out.caption = D2D_RECT_F {
            left: out.panel.left + margin,
            top: out.panel.bottom,
            right: out.panel.right - margin,
            bottom: out.panel.bottom + height,
        };
        out.panel.bottom = out.caption.bottom + margin;
    }

    out
}

pub fn selection_cell(cells: &[D2D_RECT_F], scroll: f64) -> (D2D_RECT_F, f32) {
    if cells.is_empty() {
        return (D2D_RECT_F::default(), 0.0);
    }
    let count = cells.len();
    let base = scroll.floor();
    let fraction = (scroll - base) as f32;
    let from = positive_mod_i64(base as i64, count as i32) as usize;
    let to = positive_mod_i64(base as i64 + 1, count as i32) as usize;

    if to == 0 && count > 2 {
        let opacity = (fraction - 0.5).abs() * 2.0;
        let cell = cells[if fraction < 0.5 { from } else { to }];
        return (cell, opacity);
    }

    let a = cells[from];
    let b = cells[to];
    let cell = D2D_RECT_F {
        left: lerp(a.left, b.left, fraction),
        top: lerp(a.top, b.top, fraction),
        right: lerp(a.right, b.right, fraction),
        bottom: lerp(a.bottom, b.bottom, fraction),
    };
    (cell, 1.0)
}

#[derive(Debug, Clone)]
pub struct DrawEntry {
    pub item_index: usize,
    pub chain: usize,
    pub pose: Pose,
    pub order: f32,
    pub quad: Quad,
}

#[allow(clippy::too_many_arguments)]
pub fn build_draw_list(
    style: AnimationStyle,
    settings: &Settings,
    m: &LayoutMetrics,
    items: &[(f32, f32, RECT, bool)], // (w, h, rect, is_minimized_or_hidden)
    scroll: f64,
    t: f32,
    monitor: RECT,
    selected_on_top_when_flat: bool,
    slide_start: f64,
    item_last_poses: &mut [(Option<Pose>, Option<Pose>, bool)], // (last_pose, slide_from, slides)
    closing_items: &[(usize, Pose, f64, f32, f32)], // (original_idx, last_pose, start, w, h)
    thumbnails: &[D2D_RECT_F],
) -> Vec<DrawEntry> {
    let mut draw_list = Vec::new();
    let count = items.len();
    if count == 0 {
        return draw_list;
    }

    let selected = positive_mod_i64(scroll.round() as i64, count as i32) as usize;
    let panel = is_panel_style(style);
    let now = now_seconds();
    let slide = ((now - slide_start) / SLIDE_ANIMATION_SECONDS).clamp(0.0, 1.0) as f32;

    let mut add = |index: usize, chain: usize, rel: f32, fade: f32| {
        let (item_w, item_h, item_rect, off_screen) = items[index];
        let thumb = if index < thumbnails.len() {
            Some(thumbnails[index])
        } else {
            None
        };
        let flat = flat_pose(item_rect, monitor, m.width, m.height, off_screen);
        let mut layout = layout_pose(
            style,
            settings,
            m,
            item_w,
            item_h,
            index,
            rel,
            count,
            thumb,
            Some(flat),
        );
        layout.opacity *= fade;

        if !panel {
            if style != AnimationStyle::Grid {
                layout.x *= settings.window_size;
                layout.y *= settings.window_size;
                layout.scale *= settings.window_size;
            }
            if is_solid_style(style) {
                layout.z *= settings.window_size;
            } else if layout.z > 0.0 {
                layout.z *= settings.depth;
            }
        }

        let mut flat_order = index as f32;
        if selected_on_top_when_flat && index == selected {
            flat_order = -1.0;
        }
        let depth_order = layout.z / (m.width * 0.05);

        let mut pose = lerp_pose(&flat, &layout, t);
        if chain == 0 && index < item_last_poses.len() {
            let (_, slide_from, slides) = item_last_poses[index];
            if slides && slide < 1.0 {
                if let Some(sf) = slide_from {
                    pose = lerp_pose(&sf, &pose, ease_out_quart(slide));
                }
            }
            item_last_poses[index].0 = Some(pose);
        }

        let order = lerp(flat_order, depth_order, t);
        let quad = project_pose(&pose, item_w, item_h, m.focal, m.width, m.height);
        if pose.opacity > 0.003 {
            draw_list.push(DrawEntry {
                item_index: index,
                chain,
                pose,
                order,
                quad,
            });
        }
    };

    let stacked = matches!(
        style,
        AnimationStyle::Flip3D
            | AnimationStyle::Cascade
            | AnimationStyle::Tunnel
            | AnimationStyle::Shuffle
            | AnimationStyle::Domino
    );

    for i in 0..count {
        if count == 1 {
            add(i, 0, 0.0, 1.0);
            continue;
        }
        let slot = positive_mod_f32((i as f64 - scroll) as f32, count as f32);
        if !stacked {
            let rel = if slot >= count as f32 / 2.0 {
                slot - count as f32
            } else {
                slot
            };
            add(i, 0, rel, 1.0);
        } else if slot <= count as f32 - 1.0 {
            add(i, 0, slot, 1.0);
        } else if style == AnimationStyle::Shuffle {
            add(i, 0, slot - count as f32, 1.0);
        } else {
            let leaving = slot - count as f32;
            add(i, 0, leaving, 1.0);
            add(i, 1, slot, -leaving);
        }
    }

    // Add closing items
    for &(_orig_idx, last_pose, start, item_w, item_h) in closing_items {
        let progress = ((now - start) / CLOSE_ANIMATION_SECONDS).clamp(0.0, 1.0) as f32;
        let mut pose = last_pose;
        pose.scale *= 1.0 - 0.25 * ease_out_quart(progress);
        pose.opacity *= (1.0 - progress) * (1.0 - progress);
        pose.highlight = 0.0;
        let order = pose.z / (m.width * 0.05) - 0.01;
        let quad = project_pose(&pose, item_w, item_h, m.focal, m.width, m.height);
        draw_list.push(DrawEntry {
            item_index: usize::MAX, // closing sentinel
            chain: 0,
            pose,
            order,
            quad,
        });
    }

    draw_list.sort_by(|a, b| {
        b.order
            .partial_cmp(&a.order)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    draw_list
}
