//! DolphinOS squircle 形状共享库。
//!
//! 用超椭圆（superellipse）公式生成多边形点列，供 Slint 的 `Path` 直接吃点列绘制，
//! 与网站 Dolphin 主题的 clip-path 同一套算法（自编公式，不依赖平台 corner-shape）：
//!
//! ```text
//! x = a * sign(cos t) * |cos t|^(2/n)
//! y = b * sign(sin t) * |sin t|^(2/n)
//! ```
//!
//! 其中 `a`/`b` 为半宽/半高，`n` 为指数：n = 2 是椭圆，n 越大越接近矩形；
//! Dolphin 主题的 squircle 取 n ≈ 4.5。

/// 二维点 `(x, y)`，以形状中心为原点，单位与输入半宽/半高一致。
pub type Point = (f32, f32);

/// Dolphin squircle 的默认指数。
pub const SQUIRCLE_EXPONENT: f32 = 4.5;

/// 生成超椭圆多边形点列。
///
/// - `half_width` / `half_height`：半宽、半高（a、b），必须为正数。
/// - `exponent`：指数 n（2 = 椭圆；4~5 = squircle；越大越方），必须为正数。
/// - `segments`：点列点数（多边形顶点数），至少 4。
///
/// 返回的点列从 +x 轴起逆时针排列，不重复首尾点；闭合由调用方连回首点。
pub fn superellipse_points(
    half_width: f32,
    half_height: f32,
    exponent: f32,
    segments: usize,
) -> Vec<Point> {
    assert!(half_width > 0.0 && half_height > 0.0, "半宽半高必须为正数");
    assert!(exponent > 0.0, "指数必须为正数");
    assert!(segments >= 4, "点数至少为 4");

    let p = 2.0 / exponent;
    let mut pts = Vec::with_capacity(segments);
    for i in 0..segments {
        let t = (i as f32) / (segments as f32) * std::f32::consts::TAU;
        let (sin, cos) = t.sin_cos();
        // 数值吸附：轴附近的 sin/cos 残差（f32 约 1e-7）会被小于 1 的幂放大，
        // 小于阈值直接归零，保证轴向点精确落在轴上、点列严格对称。
        let snap = |v: f32| if v.abs() < 1e-6 { 0.0 } else { v };
        let (sin, cos) = (snap(sin), snap(cos));
        let x = half_width * cos.signum() * cos.abs().powf(p);
        let y = half_height * sin.signum() * sin.abs().powf(p);
        pts.push((x, y));
    }
    pts
}

/// Dolphin 主题 squircle：指数固定 [`SQUIRCLE_EXPONENT`] 的常用封装（中心原点）。
pub fn squircle_points(half_width: f32, half_height: f32, segments: usize) -> Vec<Point> {
    superellipse_points(half_width, half_height, SQUIRCLE_EXPONENT, segments)
}

/// 以左上角为原点、宽高为输入的 squircle 点列，方便 UI 布局坐标系（y 向下）直接使用。
pub fn squircle_points_xy(width: f32, height: f32, segments: usize) -> Vec<Point> {
    let (hw, hh) = (width / 2.0, height / 2.0);
    superellipse_points(hw, hh, SQUIRCLE_EXPONENT, segments)
        .into_iter()
        .map(|(x, y)| (x + hw, y + hh))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f32, b: f32, eps: f32) -> bool {
        (a - b).abs() <= eps
    }

    #[test]
    fn point_count_matches_segments() {
        assert_eq!(superellipse_points(100.0, 60.0, 4.5, 64).len(), 64);
        assert_eq!(squircle_points(50.0, 50.0, 8).len(), 8);
    }

    #[test]
    fn exponent_two_is_ellipse() {
        // n = 2 时每个点都应落在椭圆方程 (x/a)^2 + (y/b)^2 = 1 上
        let (a, b) = (100.0_f32, 60.0_f32);
        for (x, y) in superellipse_points(a, b, 2.0, 128) {
            let v = (x / a).powi(2) + (y / b).powi(2);
            assert!(approx(v, 1.0, 1e-3), "点 ({x}, {y}) 不在椭圆上: {v}");
        }
    }

    #[test]
    fn axis_points_hit_half_extents() {
        // segments 可被 4 整除时，四个轴向点应恰为 (±a, 0) 与 (0, ±b)
        let (a, b) = (80.0_f32, 40.0_f32);
        let pts = squircle_points(a, b, 64);
        assert!(approx(pts[0].0, a, 1e-3) && approx(pts[0].1, 0.0, 1e-3));
        assert!(approx(pts[16].0, 0.0, 1e-3) && approx(pts[16].1, b, 1e-3));
        assert!(approx(pts[32].0, -a, 1e-3) && approx(pts[32].1, 0.0, 1e-3));
        assert!(approx(pts[48].0, 0.0, 1e-3) && approx(pts[48].1, -b, 1e-3));
    }

    #[test]
    fn squircle_corner_is_fuller_than_ellipse() {
        // 45° 方向上 squircle 的点应比椭圆更靠外（更方）
        let n = 64;
        let e = superellipse_points(1.0, 1.0, 2.0, n);
        let s = superellipse_points(1.0, 1.0, SQUIRCLE_EXPONENT, n);
        let idx = n / 8; // t = 45°
        assert!(s[idx].0 > e[idx].0 + 0.05, "squircle 45° x={} 应明显大于椭圆 {}", s[idx].0, e[idx].0);
        assert!(s[idx].1 > e[idx].1 + 0.05);
    }

    #[test]
    fn opposite_points_are_symmetric() {
        let pts = squircle_points(70.0, 30.0, 64);
        for i in 0..32 {
            let (x1, y1) = pts[i];
            let (x2, y2) = pts[i + 32];
            assert!(approx(x1, -x2, 1e-3) && approx(y1, -y2, 1e-3));
        }
    }

    #[test]
    fn xy_variant_stays_inside_box() {
        let (w, h) = (200.0_f32, 120.0_f32);
        let pts = squircle_points_xy(w, h, 96);
        let max_x = pts.iter().map(|p| p.0).fold(f32::MIN, f32::max);
        let max_y = pts.iter().map(|p| p.1).fold(f32::MIN, f32::max);
        for (x, y) in &pts {
            assert!(*x >= -1e-4 && *x <= w + 1e-4 && *y >= -1e-4 && *y <= h + 1e-4);
        }
        assert!(approx(max_x, w, 1e-3) && approx(max_y, h, 1e-3));
    }
}
