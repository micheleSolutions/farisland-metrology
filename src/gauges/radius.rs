/// # RadiusGauge
///
/// Measures the radius of a partial circular arc (fillet, rounded corner, etc.).
///
/// Unlike the DiameterGauge which expects a full circle, the RadiusGauge is
/// designed for partial arcs where only a fraction of the circumference is visible.
///
/// ## Algorithm
/// 1. Place calipers along the expected arc region
/// 2. Detect edge points on the arc contour
/// 3. Fit a circle to the partial arc points (Taubin + geometric refinement)
/// 4. Extract radius from the fitted circle
///
/// For short arcs (< 90°), geometric refinement is strongly recommended
/// as algebraic fits can be biased.

use crate::error::{MetrologyError, MetrologyResult};
use crate::fitting::{fit_circle_geometric, fit_circle_taubin};
use crate::gauges::caliper1d::{Caliper1D, Caliper1DConfig};
use crate::geometry::{Angle, Circle2D, Point2D};
use crate::image::GrayImageRef;
use crate::profile::EdgePolarity;

/// Configuration for radius measurement.
#[derive(Debug, Clone)]
pub struct RadiusGaugeConfig {
    /// Expected center of the arc.
    pub nominal_center: Point2D,
    /// Expected radius.
    pub nominal_radius: f64,
    /// Start angle of the arc (radians, 0 = positive X axis).
    pub start_angle: f64,
    /// End angle of the arc (radians). Arc goes counterclockwise from start to end.
    pub end_angle: f64,
    /// Extra search distance beyond nominal radius.
    pub search_margin: f64,
    /// Number of caliper scans distributed along the arc.
    pub num_calipers: u32,
    /// Caliper scan width.
    pub scan_width: u32,
    /// Gaussian smoothing sigma.
    pub smoothing_sigma: f64,
    /// Minimum edge strength.
    pub min_edge_strength: f64,
    /// Edge polarity.
    pub polarity: EdgePolarity,
    /// Use geometric refinement (recommended for arcs < 90°).
    pub geometric_refinement: bool,
    /// Max iterations for geometric refinement.
    pub max_iterations: u32,
}

impl Default for RadiusGaugeConfig {
    fn default() -> Self {
        Self {
            nominal_center: Point2D::new(0.0, 0.0),
            nominal_radius: 50.0,
            start_angle: 0.0,
            end_angle: std::f64::consts::FRAC_PI_2,
            search_margin: 15.0,
            num_calipers: 20,
            scan_width: 5,
            smoothing_sigma: 1.0,
            min_edge_strength: 10.0,
            polarity: EdgePolarity::Any,
            geometric_refinement: true,
            max_iterations: 100,
        }
    }
}

/// Result of a radius measurement.
#[derive(Debug, Clone)]
pub struct RadiusResult {
    /// Fitted circle.
    pub circle: Circle2D,
    /// Measured radius.
    pub radius: f64,
    /// Arc span actually covered (radians).
    pub arc_span: Angle,
    /// RMS fitting error.
    pub rms_error: f64,
    /// Number of edge points used.
    pub num_points: usize,
    /// Detected edge points.
    pub edge_points: Vec<Point2D>,
}

pub struct RadiusGauge;

impl RadiusGauge {
    pub fn measure(
        image: &GrayImageRef<'_>,
        config: &RadiusGaugeConfig,
    ) -> MetrologyResult<RadiusResult> {
        let mut edge_points = Vec::with_capacity(config.num_calipers as usize);

        let inner_r = (config.nominal_radius - config.search_margin).max(1.0);
        let outer_r = config.nominal_radius + config.search_margin;

        // Normalize angle span
        let mut span = config.end_angle - config.start_angle;
        if span <= 0.0 {
            span += 2.0 * std::f64::consts::PI;
        }

        for i in 0..config.num_calipers {
            let t = i as f64 / (config.num_calipers as f64 - 1.0).max(1.0);
            let angle = config.start_angle + t * span;
            let cos_a = angle.cos();
            let sin_a = angle.sin();

            let start = Point2D::new(
                config.nominal_center.x + inner_r * cos_a,
                config.nominal_center.y + inner_r * sin_a,
            );
            let end = Point2D::new(
                config.nominal_center.x + outer_r * cos_a,
                config.nominal_center.y + outer_r * sin_a,
            );

            let caliper_config = Caliper1DConfig {
                start,
                end,
                scan_width: config.scan_width,
                smoothing_sigma: config.smoothing_sigma,
                min_edge_strength: config.min_edge_strength,
                polarity: config.polarity,
                step: 1.0,
            };

            if let Ok(edge) = Caliper1D::find_strongest_edge(image, &caliper_config) {
                edge_points.push(edge.point);
            }
        }

        if edge_points.len() < 3 {
            return Err(MetrologyError::InsufficientData {
                needed: 3,
                got: edge_points.len(),
            });
        }

        let taubin = fit_circle_taubin(&edge_points)?;

        let fit_result = if config.geometric_refinement {
            fit_circle_geometric(&edge_points, taubin.circle, config.max_iterations, 1e-7)?
        } else {
            taubin
        };

        // Compute actual arc span from the edge points
        let actual_span = compute_arc_span(&edge_points, &fit_result.circle);

        Ok(RadiusResult {
            circle: fit_result.circle,
            radius: fit_result.circle.radius,
            arc_span: Angle::from_radians(actual_span),
            rms_error: fit_result.rms_error,
            num_points: edge_points.len(),
            edge_points,
        })
    }
}

/// Compute the angular span covered by points on a circle.
fn compute_arc_span(points: &[Point2D], circle: &Circle2D) -> f64 {
    if points.len() < 2 {
        return 0.0;
    }

    let mut angles: Vec<f64> = points
        .iter()
        .map(|p| (p.y - circle.center.y).atan2(p.x - circle.center.x))
        .collect();

    angles.sort_by(|a, b| a.partial_cmp(b).unwrap());

    // Find the largest gap — the arc is the complement of the largest gap
    let mut max_gap = 0.0_f64;
    for i in 1..angles.len() {
        let gap = angles[i] - angles[i - 1];
        max_gap = max_gap.max(gap);
    }
    // Wrap-around gap
    let wrap_gap = (angles[0] + 2.0 * std::f64::consts::PI) - angles[angles.len() - 1];
    max_gap = max_gap.max(wrap_gap);

    2.0 * std::f64::consts::PI - max_gap
}
