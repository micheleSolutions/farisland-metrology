use farisland_metrology::gauges::caliper1d::*;
use farisland_metrology::gauges::diameter::*;
use farisland_metrology::gauges::radius::*;
use farisland_metrology::gauges::thread_pitch::*;
use farisland_metrology::geometry::Point2D;
use farisland_metrology::image::GrayImage;
use farisland_metrology::profile::EdgePolarity;

// ── Test image generators ───────────────────────────────────────────────────

/// Create an image with a vertical step edge at column `edge_x`.
fn make_vertical_edge(width: u32, height: u32, edge_x: u32, low: u8, high: u8) -> GrayImage {
    let mut data = vec![0u8; (width * height) as usize];
    for y in 0..height {
        for x in 0..width {
            data[(y * width + x) as usize] = if x < edge_x { low } else { high };
        }
    }
    GrayImage::new(data, width, height).unwrap()
}

/// Create an image with a bright stripe (two vertical edges).
fn make_vertical_stripe(
    width: u32,
    height: u32,
    left: u32,
    right: u32,
    bg: u8,
    fg: u8,
) -> GrayImage {
    let mut data = vec![bg; (width * height) as usize];
    for y in 0..height {
        for x in left..right {
            data[(y * width + x) as usize] = fg;
        }
    }
    GrayImage::new(data, width, height).unwrap()
}

/// Create an image with a bright filled circle.
fn make_circle_image(
    width: u32,
    height: u32,
    cx: f64,
    cy: f64,
    radius: f64,
    bg: u8,
    fg: u8,
) -> GrayImage {
    let mut data = vec![bg; (width * height) as usize];
    for y in 0..height {
        for x in 0..width {
            let dx = x as f64 - cx;
            let dy = y as f64 - cy;
            if (dx * dx + dy * dy).sqrt() <= radius {
                data[(y * width + x) as usize] = fg;
            }
        }
    }
    GrayImage::new(data, width, height).unwrap()
}

/// Create an image with a sinusoidal vertical brightness pattern (simulates thread profile).
fn make_thread_image(width: u32, height: u32, pitch_px: f64, amplitude: f64) -> GrayImage {
    let mut data = vec![0u8; (width * height) as usize];
    for y in 0..height {
        for x in 0..width {
            let val = 128.0
                + amplitude
                    * (2.0 * std::f64::consts::PI * x as f64 / pitch_px).cos();
            data[(y * width + x) as usize] = val.clamp(0.0, 255.0) as u8;
        }
    }
    GrayImage::new(data, width, height).unwrap()
}

// ── Caliper1D tests ─────────────────────────────────────────────────────────

#[test]
fn test_caliper1d_find_edges_on_step() {
    let img = make_vertical_edge(200, 100, 100, 20, 220);
    let img_ref = img.as_ref();

    let config = Caliper1DConfig {
        start: Point2D::new(10.0, 50.0),
        end: Point2D::new(190.0, 50.0),
        scan_width: 5,
        smoothing_sigma: 1.0,
        min_edge_strength: 10.0,
        polarity: EdgePolarity::Any,
        step: 1.0,
    };

    let result = Caliper1D::find_edges(&img_ref, &config).unwrap();
    assert!(!result.edges.is_empty(), "should find at least one edge");

    // The edge should be near x=100
    let best = &result.edges[0];
    assert!(
        (best.point.x - 100.0).abs() < 3.0,
        "edge x={:.1}, expected near 100",
        best.point.x
    );
}

#[test]
fn test_caliper1d_measure_width() {
    let img = make_vertical_stripe(300, 100, 100, 200, 20, 220);
    let img_ref = img.as_ref();

    let config = Caliper1DConfig {
        start: Point2D::new(10.0, 50.0),
        end: Point2D::new(290.0, 50.0),
        scan_width: 5,
        smoothing_sigma: 1.0,
        min_edge_strength: 10.0,
        polarity: EdgePolarity::Any,
        step: 1.0,
    };

    let width = Caliper1D::measure_width(&img_ref, &config, 50.0, 200.0).unwrap();
    // Stripe is 100 pixels wide
    assert!(
        (width - 100.0).abs() < 5.0,
        "measured width={:.1}, expected ~100",
        width
    );
}

#[test]
fn test_caliper1d_strongest_edge() {
    let img = make_vertical_edge(200, 100, 100, 20, 220);
    let img_ref = img.as_ref();

    let config = Caliper1DConfig {
        start: Point2D::new(10.0, 50.0),
        end: Point2D::new(190.0, 50.0),
        scan_width: 1,
        smoothing_sigma: 1.0,
        min_edge_strength: 5.0,
        polarity: EdgePolarity::DarkToBright,
        step: 1.0,
    };

    let edge = Caliper1D::find_strongest_edge(&img_ref, &config).unwrap();
    assert!(
        (edge.point.x - 100.0).abs() < 3.0,
        "strongest edge at x={:.1}",
        edge.point.x
    );
    assert_eq!(edge.polarity, EdgePolarity::DarkToBright);
}

// ── DiameterGauge tests ─────────────────────────────────────────────────────

#[test]
fn test_diameter_gauge_circle() {
    let cx = 150.0;
    let cy = 150.0;
    let true_r = 60.0;
    let img = make_circle_image(300, 300, cx, cy, true_r, 20, 220);
    let img_ref = img.as_ref();

    let config = DiameterGaugeConfig {
        nominal_center: Point2D::new(cx, cy),
        nominal_radius: true_r,
        search_margin: 20.0,
        num_calipers: 36,
        scan_width: 3,
        smoothing_sigma: 1.0,
        min_edge_strength: 10.0,
        polarity: EdgePolarity::Any,
        geometric_refinement: true,
        max_iterations: 50,
    };

    let result = DiameterGauge::measure(&img_ref, &config).unwrap();

    assert!(
        (result.diameter - 2.0 * true_r).abs() < 4.0,
        "diameter={:.1}, expected {:.1}",
        result.diameter,
        2.0 * true_r
    );
    assert!(
        (result.circle.center.x - cx).abs() < 2.0,
        "center x off by {:.1}",
        (result.circle.center.x - cx).abs()
    );
    assert!(
        (result.circle.center.y - cy).abs() < 2.0,
        "center y off by {:.1}",
        (result.circle.center.y - cy).abs()
    );
    assert!(result.num_points >= 20, "expected many edge points, got {}", result.num_points);
}

#[test]
fn test_diameter_gauge_off_center_nominal() {
    // Even with a slightly wrong nominal center, the gauge should still work
    let cx = 150.0;
    let cy = 150.0;
    let true_r = 50.0;
    let img = make_circle_image(300, 300, cx, cy, true_r, 10, 240);
    let img_ref = img.as_ref();

    let config = DiameterGaugeConfig {
        nominal_center: Point2D::new(cx + 5.0, cy - 3.0), // intentionally offset
        nominal_radius: true_r,
        search_margin: 25.0, // wider margin to compensate
        num_calipers: 48,
        scan_width: 5,
        smoothing_sigma: 1.0,
        min_edge_strength: 8.0,
        polarity: EdgePolarity::Any,
        geometric_refinement: true,
        max_iterations: 100,
    };

    let result = DiameterGauge::measure(&img_ref, &config).unwrap();
    assert!(
        (result.diameter - 2.0 * true_r).abs() < 6.0,
        "diameter={:.1}, expected ~{:.1}",
        result.diameter,
        2.0 * true_r
    );
}

// ── RadiusGauge tests ───────────────────────────────────────────────────────

#[test]
fn test_radius_gauge_quarter_circle() {
    let cx = 150.0;
    let cy = 150.0;
    let true_r = 70.0;
    let img = make_circle_image(300, 300, cx, cy, true_r, 20, 220);
    let img_ref = img.as_ref();

    let config = RadiusGaugeConfig {
        nominal_center: Point2D::new(cx, cy),
        nominal_radius: true_r,
        start_angle: 0.0,
        end_angle: std::f64::consts::FRAC_PI_2,
        search_margin: 15.0,
        num_calipers: 20,
        scan_width: 3,
        smoothing_sigma: 1.0,
        min_edge_strength: 10.0,
        polarity: EdgePolarity::Any,
        geometric_refinement: true,
        max_iterations: 100,
    };

    let result = RadiusGauge::measure(&img_ref, &config).unwrap();

    assert!(
        (result.radius - true_r).abs() < 3.0,
        "radius={:.1}, expected {:.1}",
        result.radius,
        true_r
    );
    assert!(result.num_points >= 10);
}

// ── ThreadPitchGauge tests ──────────────────────────────────────────────────

#[test]
fn test_thread_pitch_by_peaks() {
    let pitch = 25.0;
    let img = make_thread_image(500, 50, pitch, 80.0);
    let img_ref = img.as_ref();

    let config = ThreadPitchGaugeConfig {
        start: Point2D::new(10.0, 25.0),
        end: Point2D::new(490.0, 25.0),
        scan_width: 10,
        smoothing_sigma: 1.5,
        min_peak_prominence: 10.0,
        expected_pitch_range: (15.0, 40.0),
        step: 1.0,
    };

    let result = ThreadPitchGauge::measure_by_peaks(&img_ref, &config).unwrap();

    assert!(
        (result.mean_pitch_px - pitch).abs() < 2.0,
        "measured pitch={:.1}, expected {:.1}",
        result.mean_pitch_px,
        pitch
    );
    assert!(result.std_dev_px < 2.0, "std_dev should be small for clean signal");
    assert!(result.thread_count >= 15, "should detect many threads, got {}", result.thread_count);
}

#[test]
fn test_thread_pitch_by_fft() {
    let pitch = 30.0;
    let img = make_thread_image(600, 50, pitch, 90.0);
    let img_ref = img.as_ref();

    let config = ThreadPitchGaugeConfig {
        start: Point2D::new(5.0, 25.0),
        end: Point2D::new(595.0, 25.0),
        scan_width: 10,
        smoothing_sigma: 1.0,
        min_peak_prominence: 5.0,
        expected_pitch_range: (15.0, 50.0),
        step: 1.0,
    };

    let result = ThreadPitchGauge::measure_by_fft(&img_ref, &config).unwrap();

    assert!(
        (result.pitch_px - pitch).abs() < 2.0,
        "FFT pitch={:.1}, expected {:.1}",
        result.pitch_px,
        pitch
    );
    assert!(result.dominant_magnitude > 100.0, "should have strong dominant peak");
}

#[test]
fn test_thread_pitch_consistency() {
    // Both methods should agree on the same image
    let pitch = 20.0;
    let img = make_thread_image(400, 50, pitch, 70.0);
    let img_ref = img.as_ref();

    let config = ThreadPitchGaugeConfig {
        start: Point2D::new(5.0, 25.0),
        end: Point2D::new(395.0, 25.0),
        scan_width: 10,
        smoothing_sigma: 1.5,
        min_peak_prominence: 8.0,
        expected_pitch_range: (10.0, 40.0),
        step: 1.0,
    };

    let peaks = ThreadPitchGauge::measure_by_peaks(&img_ref, &config).unwrap();
    let fft = ThreadPitchGauge::measure_by_fft(&img_ref, &config).unwrap();

    assert!(
        (peaks.mean_pitch_px - fft.pitch_px).abs() < 3.0,
        "peaks ({:.1}) and FFT ({:.1}) should agree",
        peaks.mean_pitch_px,
        fft.pitch_px
    );
}
