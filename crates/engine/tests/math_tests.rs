// Original Windhawk mod by caliberda
// Ported to standalone Rust by Vigi Riyantanu
// Unit tests for Geometry and Layout matching C++ formulas bit-for-bit

use flip3d::geometry::D2D_POINT_2F;
use flip3d::geometry::{
    ease_in_out_cubic, ease_out_quart, quad_contains, rect_to_quad_matrix, Pose, Quad, Tween,
};
use flip3d::layout::{compute_layout_metrics, layout_pose, project_pose};
use flip3d::settings::{AnimationStyle, Settings};

#[test]
fn test_easing_functions() {
    // EaseOutQuart: u = 1 - t; 1 - u^4
    // t = 0.0 -> u = 1.0 -> 1 - 1 = 0.0
    assert!((ease_out_quart(0.0) - 0.0).abs() < 1e-6);
    // t = 1.0 -> u = 0.0 -> 1 - 0 = 1.0
    assert!((ease_out_quart(1.0) - 1.0).abs() < 1e-6);
    // t = 0.5 -> u = 0.5 -> 1 - 0.0625 = 0.9375
    assert!((ease_out_quart(0.5) - 0.9375).abs() < 1e-6);

    // EaseInOutCubic:
    // t < 0.5: 4 * t^3
    // t = 0.0 -> 0.0
    assert!((ease_in_out_cubic(0.0) - 0.0).abs() < 1e-6);
    // t = 0.25 -> 4 * (1/64) = 0.0625
    assert!((ease_in_out_cubic(0.25) - 0.0625).abs() < 1e-6);
    // t = 0.5 -> 4 * 0.125 = 0.5
    assert!((ease_in_out_cubic(0.5) - 0.5).abs() < 1e-6);
    // t = 1.0 -> u = 0 -> 1 - 0 = 1.0
    assert!((ease_in_out_cubic(1.0) - 1.0).abs() < 1e-6);
}

#[test]
fn test_tween() {
    let t = Tween::new(10.0, 20.0, 100.0, 2.0, false);
    // At start: now = 100.0 -> progress 0.0 -> value 10.0
    assert!((t.progress(100.0) - 0.0).abs() < 1e-6);
    assert!((t.value(100.0) - 10.0).abs() < 1e-6);
    assert!(!t.done(100.0));

    // At middle: now = 101.0 -> progress 0.5 -> ease_out_quart(0.5) = 0.9375
    // value = 10 + 10 * 0.9375 = 19.375
    assert!((t.progress(101.0) - 0.5).abs() < 1e-6);
    assert!((t.value(101.0) - 19.375).abs() < 1e-5);

    // At end: now = 102.0 -> progress 1.0 -> value 20.0
    assert!((t.progress(102.0) - 1.0).abs() < 1e-6);
    assert!((t.value(102.0) - 20.0).abs() < 1e-6);
    assert!(t.done(102.0));
}

#[test]
fn test_quad_contains() {
    let quad = Quad {
        p: [
            D2D_POINT_2F { x: 10.0, y: 10.0 },
            D2D_POINT_2F { x: 50.0, y: 10.0 },
            D2D_POINT_2F { x: 50.0, y: 50.0 },
            D2D_POINT_2F { x: 10.0, y: 50.0 },
        ],
    };

    assert!(quad_contains(&quad, D2D_POINT_2F { x: 30.0, y: 30.0 }));
    assert!(quad_contains(&quad, D2D_POINT_2F { x: 10.0, y: 10.0 }));
    assert!(!quad_contains(&quad, D2D_POINT_2F { x: 5.0, y: 30.0 }));
    assert!(!quad_contains(&quad, D2D_POINT_2F { x: 55.0, y: 30.0 }));
}

#[test]
fn test_rect_to_quad_matrix() {
    // Identity-like quad: maps (0,0)-(100,50) to (0,0)-(100,50)
    let q = Quad {
        p: [
            D2D_POINT_2F { x: 0.0, y: 0.0 },
            D2D_POINT_2F { x: 100.0, y: 0.0 },
            D2D_POINT_2F { x: 100.0, y: 50.0 },
            D2D_POINT_2F { x: 0.0, y: 50.0 },
        ],
    };

    let m = rect_to_quad_matrix(100.0, 50.0, &q);
    let slice = unsafe { std::slice::from_raw_parts(&m as *const _ as *const f32, 16) };
    // _11 = 1, _22 = 1, _33 = 1, _44 = 1
    assert!((slice[0] - 1.0).abs() < 1e-5);
    assert!((slice[5] - 1.0).abs() < 1e-5);
    assert!((slice[10] - 1.0).abs() < 1e-5);
    assert!((slice[15] - 1.0).abs() < 1e-5);
}

#[test]
fn test_project_pose() {
    let pose = Pose {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        scale: 1.0,
        ..Default::default()
    };
    let screen_w = 1920.0;
    let screen_h = 1080.0;
    let focal = 1920.0 * 0.95;
    let quad = project_pose(&pose, 400.0, 300.0, focal, screen_w, screen_h);

    // Centered rectangle: half_w = 200, half_h = 150
    // top-left: 1920/2 - 200 = 760, 1080/2 - 150 = 390
    assert!((quad.p[0].x - 760.0).abs() < 1e-4);
    assert!((quad.p[0].y - 390.0).abs() < 1e-4);
    // top-right: 1920/2 + 200 = 1160, 1080/2 - 150 = 390
    assert!((quad.p[1].x - 1160.0).abs() < 1e-4);
    assert!((quad.p[1].y - 390.0).abs() < 1e-4);
}

#[test]
fn test_all_23_styles_at_rel_points() {
    let settings = Settings::default();
    let screen_w = 1920.0;
    let screen_h = 1080.0;
    let count = 8;
    let test_rels = [-1.0f32, -0.5f32, 0.0f32, 0.5f32, 1.0f32, 3.0f32];

    let styles = [
        AnimationStyle::Flip3D,
        AnimationStyle::Cascade,
        AnimationStyle::CoverFlow,
        AnimationStyle::ITab,
        AnimationStyle::ITabTitle,
        AnimationStyle::ITabBackground,
        AnimationStyle::ITabBackgroundTitle,
        AnimationStyle::Carousel,
        AnimationStyle::Grid,
        AnimationStyle::Helix,
        AnimationStyle::Fan,
        AnimationStyle::Panorama,
        AnimationStyle::Tunnel,
        AnimationStyle::Rolodex,
        AnimationStyle::Cube,
        AnimationStyle::Sphere,
        AnimationStyle::Shuffle,
        AnimationStyle::Domino,
        AnimationStyle::Windows11,
        AnimationStyle::Thumbnails,
        AnimationStyle::IconRow,
        AnimationStyle::List,
        AnimationStyle::Classic,
        AnimationStyle::Native,
    ];

    for &style in &styles {
        let metrics = compute_layout_metrics(style, &settings, screen_w, screen_h, count);
        for &rel in &test_rels {
            let pose = layout_pose(
                style, &settings, &metrics, 1200.0, 800.0, 0, rel, count, None, None,
            );

            // Verify numeric validity (finite, non-NaN)
            assert!(
                pose.x.is_finite(),
                "style {:?} rel {} x is not finite",
                style,
                rel
            );
            assert!(
                pose.y.is_finite(),
                "style {:?} rel {} y is not finite",
                style,
                rel
            );
            assert!(
                pose.z.is_finite(),
                "style {:?} rel {} z is not finite",
                style,
                rel
            );
            assert!(
                pose.scale.is_finite(),
                "style {:?} rel {} scale is not finite",
                style,
                rel
            );
            assert!(
                pose.angle_x.is_finite(),
                "style {:?} rel {} angle_x is not finite",
                style,
                rel
            );
            assert!(
                pose.angle_y.is_finite(),
                "style {:?} rel {} angle_y is not finite",
                style,
                rel
            );
            assert!(
                pose.angle_z.is_finite(),
                "style {:?} rel {} angle_z is not finite",
                style,
                rel
            );
            assert!(
                pose.opacity.is_finite(),
                "style {:?} rel {} opacity is not finite",
                style,
                rel
            );
            assert!(
                pose.brightness.is_finite(),
                "style {:?} rel {} brightness is not finite",
                style,
                rel
            );
            assert!(
                pose.highlight.is_finite(),
                "style {:?} rel {} highlight is not finite",
                style,
                rel
            );
            assert!(
                pose.squeeze.is_finite(),
                "style {:?} rel {} squeeze is not finite",
                style,
                rel
            );

            // Specific check for Flip3D:
            if style == AnimationStyle::Flip3D {
                let tilt = settings.tilt_radians;
                assert!((pose.angle_y - tilt).abs() < 1e-5);
                if rel == 0.0 {
                    // Front card: x = m_frontX, y = m_frontY, z = 0, brightness = 1
                    assert!((pose.x - metrics.front_x).abs() < 1e-4);
                    assert!((pose.y - metrics.front_y).abs() < 1e-4);
                    assert!((pose.z - 0.0).abs() < 1e-4);
                    assert!((pose.brightness - 1.0).abs() < 1e-4);
                } else if rel == 1.0 {
                    // One step behind:
                    assert!((pose.x - (metrics.front_x + metrics.step_x)).abs() < 1e-4);
                    assert!((pose.y - (metrics.front_y + metrics.step_y)).abs() < 1e-4);
                    assert!((pose.z - metrics.step_z).abs() < 1e-4);
                    assert!((pose.brightness - (1.0 - 0.07)).abs() < 1e-4);
                } else if rel == -1.0 {
                    // Leaving the front:
                    assert!((pose.opacity - 0.0).abs() < 1e-4);
                }
            }
        }
    }
}
