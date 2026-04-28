use farisland_metrology::geometry::{Point2D, Vec2D};
use farisland_metrology::image::GrayImage;
use farisland_metrology::profile::*;

/// Helper: create a horizontal gradient image (dark left, bright right).
fn make_gradient_image(width: u32, height: u32) -> GrayImage {
    let mut data = vec![0u8; (width * height) as usize];
    for y in 0..height {
        for x in 0..width {
            data[(y * width + x) as usize] = ((x as f64 / width as f64) * 255.0) as u8;
        }
    }
    GrayImage::new(data, width, height).unwrap()
}

#[test]
fn test_extract_profile_horizontal() {
    let img = make_gradient_image(100, 50);
    let img_ref = img.as_ref();
    let profile = extract_profile(
        &img_ref,
        Point2D::new(0.0, 25.0),
        Point2D::new(99.0, 25.0),
        1,
        1.0,
    );
    assert_eq!(profile.len(), 99);
    // Should be monotonically increasing
    for i in 1..profile.len() {
        assert!(profile.values[i] >= profile.values[i - 1]);
    }
}

#[test]
fn test_smooth_gaussian_preserves_length() {
    let profile = vec![0.0, 10.0, 20.0, 30.0, 40.0, 50.0];
    let smoothed = smooth_gaussian(&profile, 1.0);
    assert_eq!(smoothed.len(), profile.len());
}

#[test]
fn test_smooth_gaussian_zero_sigma_is_identity() {
    let profile = vec![1.0, 5.0, 3.0, 8.0, 2.0];
    let smoothed = smooth_gaussian(&profile, 0.0);
    assert_eq!(smoothed, profile);
}

#[test]
fn test_gradient_detects_step() {
    // Sharp step: [0, 0, 0, 100, 100, 100]
    let profile = vec![0.0, 0.0, 0.0, 100.0, 100.0, 100.0];
    let grad = gradient(&profile);
    assert_eq!(grad.len(), 6);
    // Maximum gradient should be at index 2 or 3 (the transition)
    let max_idx = grad
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.abs().partial_cmp(&b.abs()).unwrap())
        .unwrap()
        .0;
    assert!(max_idx == 2 || max_idx == 3);
}

#[test]
fn test_detect_edges_on_step() {
    let profile = vec![
        10.0, 10.0, 10.0, 10.0, 10.0, 200.0, 200.0, 200.0, 200.0, 200.0,
    ];
    let smoothed = smooth_gaussian(&profile, 0.5);
    let grad = gradient(&smoothed);
    let edges = detect_edges(&grad, 5.0, EdgePolarity::Any);
    assert!(!edges.is_empty(), "should detect at least one edge");
    // Edge should be near position 4-5
    let edge = &edges[0];
    assert!(
        edge.position > 3.0 && edge.position < 6.0,
        "edge at {}, expected near 4-5",
        edge.position
    );
    assert_eq!(edge.polarity, EdgePolarity::DarkToBright);
}

#[test]
fn test_detect_edges_polarity_filter() {
    let profile = vec![10.0, 10.0, 200.0, 200.0, 10.0, 10.0];
    let grad = gradient(&profile);

    let dark_to_bright = detect_edges(&grad, 5.0, EdgePolarity::DarkToBright);
    let bright_to_dark = detect_edges(&grad, 5.0, EdgePolarity::BrightToDark);

    assert!(
        !dark_to_bright.is_empty(),
        "should find dark-to-bright edge"
    );
    assert!(
        !bright_to_dark.is_empty(),
        "should find bright-to-dark edge"
    );
}

#[test]
fn test_find_edge_pairs() {
    let profile = vec![
        10.0, 10.0, 10.0, 200.0, 200.0, 200.0, 200.0, 10.0, 10.0, 10.0,
    ];
    let grad = gradient(&profile);
    let edges = detect_edges(&grad, 5.0, EdgePolarity::Any);
    let pairs = find_edge_pairs(&edges, 1.0, 10.0);
    assert!(!pairs.is_empty(), "should find at least one edge pair");
    assert!(pairs[0].width > 1.0 && pairs[0].width < 8.0);
}

#[test]
fn test_edge1d_to_image_point() {
    let profile = Profile1D {
        values: vec![0.0; 10],
        origin: Point2D::new(100.0, 50.0),
        direction: Vec2D::new(1.0, 0.0),
        step: 2.0,
    };
    let edge = Edge1D {
        position: 3.5,
        strength: 50.0,
        polarity: EdgePolarity::DarkToBright,
    };
    let pt = edge.to_image_point(&profile);
    // position 3.5 * step 2.0 = 7.0 pixels along the direction
    assert!((pt.x - 107.0).abs() < 1e-10);
    assert!((pt.y - 50.0).abs() < 1e-10);
}
