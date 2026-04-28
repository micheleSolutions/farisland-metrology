/// # DiameterGauge
///
/// Measures the diameter of a circular feature by placing multiple radial caliper
/// scans around a nominal center, detecting edge points on the contour, then
/// fitting a circle (Taubin + optional geometric refinement) to extract the diameter.
///
/// ## Algorithm
/// 1. Place N caliper scan lines radially from the nominal center outward
/// 2. On each scan line, detect the strongest edge (the circle contour)
/// 3. Collect all detected edge points
/// 4. Fit a circle using Taubin method (algebraic, fast, numerically stable)
/// 5. Optionally refine with geometric (iterative) circle fit
/// 6. Return diameter = 2 * fitted radius

use crate::error::{MetrologyError, MetrologyResult};
use crate::fitting::{fit_circle_geometric, fit_circle_taubin};
use crate::gauges::caliper1d::{Caliper1D, Caliper1DConfig};
use crate::geometry::{Circle2D, Point2D};
use crate::image::GrayImageRef;
use crate::profile::EdgePolarity;

/// Configuration for diameter measurement.
#[derive(Debug, Clone)]
pub struct DiameterGaugeConfig {
    /// Expected center of the circular feature.
    pub nominal_center: Point2D,
    /// Expected radius (scan lines extend from `nominal_radius - margin` to `+ margin`).
    pub nominal_radius: f64,
    /// Extra scan distance beyond the nominal radius on each side.
    pub search_margin: f64,
    /// Number of radial scan lines distributed evenly around 360°.
    pub num_calipers: u32,
    /// Width of each caliper scan (averaging width, in pixels).
    pub scan_width: u32,
    /// Gaussian smoothing sigma for each caliper.
    pub smoothing_sigma: f64,
    /// Minimum edge strength threshold.
    pub min_edge_strength: f64,
    /// Edge polarity to look for (typically DarkToBright for bright object on dark background).
    pub polarity: EdgePolarity,
    /// Whether to refine the algebraic fit with geometric (iterative) fitting.
    pub geometric_refinement: bool,
    /// Maximum iterations for geometric refinement.
    pub max_iterations: u32,
}

impl Default for DiameterGaugeConfig {
    fn default() -> Self {
        Self {
            nominal_center: Point2D::new(0.0, 0.0),
            nominal_radius: 100.0,
            search_margin: 20.0,
            num_calipers: 36,
            scan_width: 5,
            smoothing_sigma: 1.0,
            min_edge_strength: 10.0,
            polarity: EdgePolarity::Any,
            geometric_refinement: true,
            max_iterations: 50,
        }
    }
}

/// Result of a diameter measurement.
#[derive(Debug, Clone)]
pub struct DiameterResult {
    /// Fitted circle (center + radius).
    pub circle: Circle2D,
    /// Diameter = 2 * radius.
    pub diameter: f64,
    /// RMS fitting error (distance of edge points to fitted circle).
    pub rms_error: f64,
    /// Number of edge points used for fitting.
    pub num_points: usize,
    /// The edge points detected on the contour.
    pub edge_points: Vec<Point2D>,
}

/// The DiameterGauge.
pub struct DiameterGauge;

impl DiameterGauge {
    /// Measure the diameter of a circular feature.
    pub fn measure(
        image: &GrayImageRef<'_>,
        config: &DiameterGaugeConfig,
    ) -> MetrologyResult<DiameterResult> {
        let mut edge_points = Vec::with_capacity(config.num_calipers as usize);

        let inner_r = (config.nominal_radius - config.search_margin).max(1.0);
        let outer_r = config.nominal_radius + config.search_margin;

        for i in 0..config.num_calipers {
            let angle = 2.0 * std::f64::consts::PI * (i as f64) / (config.num_calipers as f64);
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

        // Taubin algebraic fit
        let taubin = fit_circle_taubin(&edge_points)?;

        let fit_result = if config.geometric_refinement {
            fit_circle_geometric(
                &edge_points,
                taubin.circle,
                config.max_iterations,
                1e-6,
            )?
        } else {
            taubin
        };

        Ok(DiameterResult {
            circle: fit_result.circle,
            diameter: 2.0 * fit_result.circle.radius,
            rms_error: fit_result.rms_error,
            num_points: edge_points.len(),
            edge_points,
        })
    }
}
