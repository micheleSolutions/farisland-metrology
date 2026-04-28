use farisland_metrology::calibration::PixelCalibration;

#[test]
fn test_uncalibrated_returns_zero() {
    let cal = PixelCalibration::UNCALIBRATED;
    assert!(!cal.is_calibrated());
    assert_eq!(cal.distance_px_to_mm(100.0, 0.0), 0.0);
    assert_eq!(cal.radius_to_mm(50.0), 0.0);
    assert_eq!(cal.point_to_mm(10.0, 20.0), (0.0, 0.0));
}

#[test]
fn test_square_pixels_horizontal() {
    let cal = PixelCalibration::new(0.01, 0.01); // 10 µm/px
    assert!(cal.is_calibrated());

    // 100 px horizontal → 1.0 mm
    let d = cal.distance_px_to_mm(100.0, 0.0);
    assert!((d - 1.0).abs() < 1e-10, "got {d}");
}

#[test]
fn test_square_pixels_diagonal() {
    let cal = PixelCalibration::new(0.01, 0.01);
    // 100 px at 45° → sqrt(2) * 100 * 0.01... wait, no.
    // dx=100, dy=100 → sqrt((100*0.01)^2 + (100*0.01)^2) = sqrt(2) ≈ 1.414 mm
    let d = cal.distance_px_to_mm(100.0, 100.0);
    assert!((d - std::f64::consts::SQRT_2).abs() < 1e-10);
}

#[test]
fn test_anisotropic_pixels_horizontal() {
    // h = 0.01 mm/px, v = 0.02 mm/px (rectangular pixels)
    let cal = PixelCalibration::new(0.01, 0.02);

    // Horizontal: 100 px → 100 * 0.01 = 1.0 mm
    let d = cal.distance_px_to_mm(100.0, 0.0);
    assert!((d - 1.0).abs() < 1e-10);

    // Vertical: 100 px → 100 * 0.02 = 2.0 mm
    let d = cal.distance_px_to_mm(0.0, 100.0);
    assert!((d - 2.0).abs() < 1e-10);
}

#[test]
fn test_anisotropic_pixels_diagonal() {
    let cal = PixelCalibration::new(0.01, 0.02);
    // dx=100, dy=100 → sqrt((100*0.01)^2 + (100*0.02)^2) = sqrt(1 + 4) = sqrt(5) ≈ 2.236
    let d = cal.distance_px_to_mm(100.0, 100.0);
    assert!((d - 5.0_f64.sqrt()).abs() < 1e-10);
}

#[test]
fn test_radius_to_mm_square() {
    let cal = PixelCalibration::new(0.01, 0.01);
    // r = 50 px → 50 * 0.01 = 0.5 mm
    let r = cal.radius_to_mm(50.0);
    assert!((r - 0.5).abs() < 1e-10);
}

#[test]
fn test_radius_to_mm_anisotropic() {
    let cal = PixelCalibration::new(0.01, 0.02);
    // Mean pixel size = (0.01 + 0.02) / 2 = 0.015
    // r = 100 px → 100 * 0.015 = 1.5 mm
    let r = cal.radius_to_mm(100.0);
    assert!((r - 1.5).abs() < 1e-10);
}

#[test]
fn test_scan_direction_scale_horizontal() {
    let cal = PixelCalibration::new(0.01, 0.02);
    // Horizontal scan: scale = h_px_mm = 0.01
    let scale = cal.scan_direction_scale(1.0, 0.0);
    assert!((scale - 0.01).abs() < 1e-10);
}

#[test]
fn test_scan_direction_scale_vertical() {
    let cal = PixelCalibration::new(0.01, 0.02);
    // Vertical scan: scale = v_px_mm = 0.02
    let scale = cal.scan_direction_scale(0.0, 1.0);
    assert!((scale - 0.02).abs() < 1e-10);
}

#[test]
fn test_scan_direction_scale_diagonal() {
    let cal = PixelCalibration::new(0.01, 0.02);
    // 45° scan: unit direction (1/√2, 1/√2)
    // scale = sqrt((1/√2 * 0.01)^2 + (1/√2 * 0.02)^2)
    //       = sqrt(0.01^2/2 + 0.02^2/2) = sqrt(0.0001/2 + 0.0004/2) = sqrt(0.00025) ≈ 0.01581
    let scale = cal.scan_direction_scale(1.0, 1.0);
    let expected = (0.01_f64.powi(2) / 2.0 + 0.02_f64.powi(2) / 2.0).sqrt();
    assert!(
        (scale - expected).abs() < 1e-10,
        "got {scale}, expected {expected}"
    );
}

#[test]
fn test_angle_to_real_square_pixels() {
    let cal = PixelCalibration::new(0.01, 0.01);
    // Square pixels: angles are preserved
    let real = cal.angle_to_real(std::f64::consts::FRAC_PI_4);
    assert!(
        (real - std::f64::consts::FRAC_PI_4).abs() < 1e-10,
        "45° should stay 45° with square pixels"
    );
}

#[test]
fn test_angle_to_real_anisotropic() {
    let cal = PixelCalibration::new(0.01, 0.02);
    // 45° in pixel space: dx = cos(45°)*h = 0.01/√2, dy = sin(45°)*v = 0.02/√2
    // real angle = atan2(0.02/√2, 0.01/√2) = atan2(0.02, 0.01) = atan(2) ≈ 63.43°
    let real = cal.angle_to_real(std::f64::consts::FRAC_PI_4);
    let expected = (0.02_f64).atan2(0.01);
    assert!(
        (real - expected).abs() < 1e-10,
        "got {:.2}°, expected {:.2}°",
        real.to_degrees(),
        expected.to_degrees()
    );
}

#[test]
fn test_default_is_uncalibrated() {
    let cal = PixelCalibration::default();
    assert!(!cal.is_calibrated());
}
