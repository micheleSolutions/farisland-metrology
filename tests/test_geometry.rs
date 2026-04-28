use farisland_metrology::geometry::*;

#[test]
fn test_point_distance() {
    let a = Point2D::new(0.0, 0.0);
    let b = Point2D::new(3.0, 4.0);
    assert!((a.distance_to(&b) - 5.0).abs() < 1e-10);
}

#[test]
fn test_vec_normalized() {
    let v = Vec2D::new(3.0, 4.0);
    let n = v.normalized();
    assert!((n.length() - 1.0).abs() < 1e-10);
    assert!((n.dx - 0.6).abs() < 1e-10);
    assert!((n.dy - 0.8).abs() < 1e-10);
}

#[test]
fn test_vec_perpendicular() {
    let v = Vec2D::new(1.0, 0.0);
    let p = v.perpendicular();
    assert!((p.dx - 0.0).abs() < 1e-10);
    assert!((p.dy - 1.0).abs() < 1e-10);
    // Perpendicular should be orthogonal
    assert!(v.dot(&p).abs() < 1e-10);
}

#[test]
fn test_line_intersection() {
    // Horizontal line y=5
    let h = Line2D {
        origin: Point2D::new(0.0, 5.0),
        direction: Vec2D::new(1.0, 0.0),
    };
    // Vertical line x=3
    let v = Line2D {
        origin: Point2D::new(3.0, 0.0),
        direction: Vec2D::new(0.0, 1.0),
    };
    let p = h.intersect(&v).unwrap();
    assert!((p.x - 3.0).abs() < 1e-10);
    assert!((p.y - 5.0).abs() < 1e-10);
}

#[test]
fn test_line_parallel_no_intersection() {
    let a = Line2D {
        origin: Point2D::new(0.0, 0.0),
        direction: Vec2D::new(1.0, 0.0),
    };
    let b = Line2D {
        origin: Point2D::new(0.0, 5.0),
        direction: Vec2D::new(1.0, 0.0),
    };
    assert!(a.intersect(&b).is_none());
}

#[test]
fn test_line_signed_distance() {
    let line = Line2D {
        origin: Point2D::new(0.0, 0.0),
        direction: Vec2D::new(1.0, 0.0),
    };
    // Point above the line
    assert!((line.signed_distance(&Point2D::new(5.0, 3.0)) - 3.0).abs() < 1e-10);
    // Point below the line
    assert!((line.signed_distance(&Point2D::new(5.0, -3.0)) - (-3.0)).abs() < 1e-10);
}

#[test]
fn test_segment_length() {
    let s = Segment2D {
        start: Point2D::new(1.0, 1.0),
        end: Point2D::new(4.0, 5.0),
    };
    assert!((s.length() - 5.0).abs() < 1e-10);
}

#[test]
fn test_angle_conversions() {
    let a = Angle::from_degrees(90.0);
    assert!((a.radians - std::f64::consts::FRAC_PI_2).abs() < 1e-10);
    assert!((a.degrees() - 90.0).abs() < 1e-10);
}

#[test]
fn test_line_from_two_points() {
    let a = Point2D::new(0.0, 0.0);
    let b = Point2D::new(10.0, 0.0);
    let line = Line2D::from_two_points(a, b);
    assert!((line.direction.dx - 1.0).abs() < 1e-10);
    assert!((line.direction.dy).abs() < 1e-10);
}
