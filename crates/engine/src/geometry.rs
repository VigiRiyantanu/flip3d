// Original Windhawk mod by caliberda
// Ported to standalone Rust by Antigravity

use windows_numerics::Vector2;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[repr(C)]
pub struct D2D_POINT_2F {
    pub x: f32,
    pub y: f32,
}

impl D2D_POINT_2F {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

impl From<D2D_POINT_2F> for Vector2 {
    fn from(p: D2D_POINT_2F) -> Self {
        Self { X: p.x, Y: p.y }
    }
}

impl From<Vector2> for D2D_POINT_2F {
    fn from(v: Vector2) -> Self {
        Self { x: v.X, y: v.Y }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct D2D_MATRIX_4X4_F {
    pub m: [f32; 16],
}

impl Default for D2D_MATRIX_4X4_F {
    fn default() -> Self {
        Self {
            m: [
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
        }
    }
}

pub fn now_seconds() -> f64 {
    use std::sync::OnceLock;
    use windows::Win32::System::Performance::{QueryPerformanceCounter, QueryPerformanceFrequency};

    static FREQUENCY: OnceLock<f64> = OnceLock::new();
    let freq = FREQUENCY.get_or_init(|| {
        let mut f = 0i64;
        // SAFETY: QueryPerformanceFrequency is safe with valid pointer.
        unsafe {
            let _ = QueryPerformanceFrequency(&mut f);
        }
        f as f64
    });

    let mut counter = 0i64;
    // SAFETY: QueryPerformanceCounter is safe with valid pointer.
    unsafe {
        let _ = QueryPerformanceCounter(&mut counter);
    }
    counter as f64 / *freq
}

#[inline(always)]
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[inline(always)]
pub fn positive_mod_f32(value: f32, modulus: f32) -> f32 {
    let result = value % modulus;
    if result < 0.0 {
        result + modulus
    } else {
        result
    }
}

#[inline(always)]
pub fn positive_mod_i64(value: i64, modulus: i32) -> i32 {
    let m = modulus as i64;
    let result = value % m;
    let r = if result < 0 { result + m } else { result };
    r as i32
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pose {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub scale: f32,
    pub angle_y: f32,
    pub angle_x: f32,
    pub angle_z: f32,
    pub opacity: f32,
    pub brightness: f32,
    pub highlight: f32,
    pub squeeze: f32,
}

impl Default for Pose {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            scale: 1.0,
            angle_y: 0.0,
            angle_x: 0.0,
            angle_z: 0.0,
            opacity: 1.0,
            brightness: 1.0,
            highlight: 0.0,
            squeeze: 1.0,
        }
    }
}

pub fn lerp_pose(a: &Pose, b: &Pose, t: f32) -> Pose {
    Pose {
        x: lerp(a.x, b.x, t),
        y: lerp(a.y, b.y, t),
        z: lerp(a.z, b.z, t),
        scale: lerp(a.scale, b.scale, t),
        angle_y: lerp(a.angle_y, b.angle_y, t),
        angle_x: lerp(a.angle_x, b.angle_x, t),
        angle_z: lerp(a.angle_z, b.angle_z, t),
        opacity: lerp(a.opacity, b.opacity, t),
        brightness: lerp(a.brightness, b.brightness, t),
        highlight: lerp(a.highlight, b.highlight, t),
        squeeze: lerp(a.squeeze, b.squeeze, t),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Quad {
    pub p: [D2D_POINT_2F; 4],
}

pub fn quad_contains(quad: &Quad, pt: D2D_POINT_2F) -> bool {
    let mut has_positive = false;
    let mut has_negative = false;
    for i in 0..4 {
        let a = quad.p[i];
        let b = quad.p[(i + 1) % 4];
        let cross = (b.x - a.x) * (pt.y - a.y) - (b.y - a.y) * (pt.x - a.x);
        has_positive |= cross > 0.0;
        has_negative |= cross < 0.0;
    }
    !(has_positive && has_negative)
}

pub fn rect_to_quad_matrix(width: f32, height: f32, q: &Quad) -> D2D_MATRIX_4X4_F {
    let x0 = q.p[0].x;
    let y0 = q.p[0].y;
    let x1 = q.p[1].x;
    let y1 = q.p[1].y;
    let x2 = q.p[2].x;
    let y2 = q.p[2].y;
    let x3 = q.p[3].x;
    let y3 = q.p[3].y;

    let sx = x0 - x1 + x2 - x3;
    let sy = y0 - y1 + y2 - y3;

    let (a, b, d, e, g, h) = if sx.abs() < 1e-3 && sy.abs() < 1e-3 {
        (x1 - x0, x3 - x0, y1 - y0, y3 - y0, 0.0, 0.0)
    } else {
        let dx1 = x1 - x2;
        let dx2 = x3 - x2;
        let dy1 = y1 - y2;
        let dy2 = y3 - y2;
        let mut den = dx1 * dy2 - dx2 * dy1;
        if den.abs() < 1e-6 {
            den = if den < 0.0 { -1e-6 } else { 1e-6 };
        }
        let g = (sx * dy2 - dx2 * sy) / den;
        let h = (dx1 * sy - sx * dy1) / den;
        let a = x1 - x0 + g * x1;
        let b = x3 - x0 + h * x3;
        let d = y1 - y0 + g * y1;
        let e = y3 - y0 + h * y3;
        (a, b, d, e, g, h)
    };

    let mut m = D2D_MATRIX_4X4_F::default();
    // Row-major 4x4 matrix for D2D:
    // _11, _12, _13, _14
    // _21, _22, _23, _24
    // _31, _32, _33, _34
    // _41, _42, _43, _44
    m.m[0] = a / width;
    m.m[1] = d / width;
    m.m[2] = 0.0;
    m.m[3] = g / width;

    m.m[4] = b / height;
    m.m[5] = e / height;
    m.m[6] = 0.0;
    m.m[7] = h / height;

    m.m[8] = 0.0;
    m.m[9] = 0.0;
    m.m[10] = 1.0;
    m.m[11] = 0.0;

    m.m[12] = x0;
    m.m[13] = y0;
    m.m[14] = 0.0;
    m.m[15] = 1.0;

    m
}

#[inline(always)]
pub fn ease_out_quart(t: f32) -> f32 {
    let u = 1.0 - t;
    1.0 - u * u * u * u
}

#[inline(always)]
pub fn ease_in_out_cubic(t: f32) -> f32 {
    let u = -2.0 * t + 2.0;
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - u * u * u / 2.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tween {
    pub from: f32,
    pub to: f32,
    pub start: f64,
    pub duration: f64,
    pub ease_in_out: bool,
}

impl Default for Tween {
    fn default() -> Self {
        Self {
            from: 0.0,
            to: 0.0,
            start: 0.0,
            duration: 0.0,
            ease_in_out: false,
        }
    }
}

impl Tween {
    pub fn new(from: f32, to: f32, start: f64, duration: f64, ease_in_out: bool) -> Self {
        Self {
            from,
            to,
            start,
            duration,
            ease_in_out,
        }
    }

    pub fn progress(&self, now: f64) -> f32 {
        if self.duration <= 0.0 {
            return 1.0;
        }
        ((now - self.start) / self.duration).clamp(0.0, 1.0) as f32
    }

    pub fn value(&self, now: f64) -> f32 {
        let p = self.progress(now);
        let eased = if self.ease_in_out {
            ease_in_out_cubic(p)
        } else {
            ease_out_quart(p)
        };
        lerp(self.from, self.to, eased)
    }

    pub fn done(&self, now: f64) -> bool {
        self.progress(now) >= 1.0
    }
}
