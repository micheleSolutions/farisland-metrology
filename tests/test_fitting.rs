use farisland_metrology::fitting::*;
use farisland_metrology::geometry::Point2D;

#[test]
fn test_fit_line_horizontal() {
    let points: Vec<Point2D> = (0..20)
        .map(|i| Point2D::new(i as f64, 5.0))
        .collect();
    let result = fit_line(&points).unwrap();
    assert!(result.rms_error < 1e-10, "RMS should be ~0 for collinear points");
    // Direction should be roughly horizontal
    assert!(result.line.direction.dx.abs() > 0.99);
}

#[test]
fn test_fit_line_diagonal_with_noise() {
    let points: Vec<Point2D> = (0..50)
        .map(|i| {
            let x = i as f64;
            // y = x + small deterministic perturbation
            let noise = ((i * 7 + 3) % 5) as f64 * 0.1 - 0.2;
            Point2D::new(x, x + noise)
        })
        .collect();
    let result = fit_line(&points).unwrap();
    assert!(result.rms_error < 0.5);
    // Direction should be roughly 45°
    let angle = result.line.direction.dy.atan2(result.line.direction.dx);
    assert!(
        (angle - std::f64::consts::FRAC_PI_4).abs() < 0.1,
        "angle should be ~45°, got {:.2}°",
        angle.to_degrees()
    );
}

#[test]
fn test_fit_line_insufficient_data() {
    let result = fit_line(&[Point2D::new(0.0, 0.0)]);
    assert!(result.is_err());
}

#[test]
fn test_fit_circle_taubin_perfect_circle() {
    let n = 36;
    let true_cx = 50.0;
    let true_cy = 50.0;
    let true_r = 30.0;

    let points: Vec<Point2D> = (0..n)
        .map(|i| {
            let angle = 2.0 * std::f64::consts::PI * i as f64 / n as f64;
            Point2D::new(
                true_cx + true_r * angle.cos(),
                true_cy + true_r * angle.sin(),
            )
        })
        .collect();

    let result = fit_circle_taubin(&points).unwrap();
    assert!(
        (result.circle.center.x - true_cx).abs() < 0.01,
        "center x: expected {true_cx}, got {}",
        result.circle.center.x
    );
    assert!(
        (result.circle.center.y - true_cy).abs() < 0.01,
        "center y: expected {true_cy}, got {}",
        result.circle.center.y
    );
    assert!(
        (result.circle.radius - true_r).abs() < 0.01,
        "radius: expected {true_r}, got {}",
        result.circle.radius
    );
    assert!(result.rms_error < 0.01);
}

#[test]
fn test_fit_circle_taubin_with_noise() {
    let n = 72;
    let true_cx = 100.0;
    let true_cy = 80.0;
    let true_r = 50.0;

    let points: Vec<Point2D> = (0..n)
        .map(|i| {
            let angle = 2.0 * std::f64::consts::PI * i as f64 / n as f64;
            // Deterministic noise
            let noise = ((i * 13 + 7) % 11) as f64 * 0.2 - 1.0;
            Point2D::new(
                true_cx + (true_r + noise) * angle.cos(),
                true_cy + (true_r + noise) * angle.sin(),
            )
        })
        .collect();

    let result = fit_circle_taubin(&points).unwrap();
    assert!(
        (result.circle.center.x - true_cx).abs() < 1.0,
        "center x off by {}",
        (result.circle.center.x - true_cx).abs()
    );
    assert!(
        (result.circle.center.y - true_cy).abs() < 1.0,
        "center y off by {}",
        (result.circle.center.y - true_cy).abs()
    );
    assert!(
        (result.circle.radius - true_r).abs() < 1.5,
        "radius off by {}",
        (result.circle.radius - true_r).abs()
    );
}

#[test]
fn test_fit_circle_geometric_refines_taubin() {
    let n = 36;
    let true_cx = 50.0;
    let true_cy = 50.0;
    let true_r = 30.0;

    let points: Vec<Point2D> = (0..n)
        .map(|i| {
            let angle = 2.0 * std::f64::consts::PI * i as f64 / n as f64;
            let noise = ((i * 13 + 7) % 11) as f64 * 0.2 - 1.0;
            Point2D::new(
                true_cx + (true_r + noise) * angle.cos(),
                true_cy + (true_r + noise) * angle.sin(),
            )
        })
        .collect();

    let taubin = fit_circle_taubin(&points).unwrap();
    let geometric = fit_circle_geometric(&points, taubin.circle, 100, 1e-8).unwrap();

    // Geometric should be at least as good as Taubin
    assert!(
        geometric.rms_error <= taubin.rms_error + 1e-10,
        "geometric RMS {} should be <= taubin RMS {}",
        geometric.rms_error,
        taubin.rms_error
    );
}

#[test]
fn test_fit_circle_partial_arc() {
    // Only a 90° arc — harder for circle fitting
    let n = 20;
    let true_r = 100.0;

    let points: Vec<Point2D> = (0..n)
        .map(|i| {
            let angle = std::f64::consts::FRAC_PI_2 * i as f64 / (n - 1) as f64;
            Point2D::new(true_r * angle.cos(), true_r * angle.sin())
        })
        .collect();

    let taubin = fit_circle_taubin(&points).unwrap();
    let geometric = fit_circle_geometric(&points, taubin.circle, 200, 1e-10).unwrap();

    assert!(
        (geometric.circle.radius - true_r).abs() < 0.5,
        "radius off by {} on 90° arc",
        (geometric.circle.radius - true_r).abs()
    );
}

#[test]
fn test_fit_circle_insufficient_points() {
    let points = vec![Point2D::new(0.0, 0.0), Point2D::new(1.0, 0.0)];
    assert!(fit_circle_taubin(&points).is_err());
}
