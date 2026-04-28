/// # ChamferGauge
///
/// Measures chamfer geometry: width, height, and angle at the intersection of
/// two surfaces. A chamfer is a beveled edge connecting two planar surfaces.
///
/// ## Algorithm
/// 1. Three scan regions are defined:
///    - Surface A: the first flat surface (before the chamfer)
///    - Chamfer: the angled transitional surface
///    - Surface B: the second flat surface (after the chamfer)
/// 2. For each region, multiple parallel caliper scans detect edge points
/// 3. Lines are fitted to each set of edge points (total least squares)
/// 4. Chamfer measurements are derived from the line geometry:
///    - Angle: between the chamfer line and surface A (or B)
///    - Width: distance along the chamfer between intersection points
///    - Height: perpendicular projection components

use crate::error::{MetrologyError, MetrologyResult};
use crate::fitting::fit_line;
use crate::gauges::caliper1d::{Caliper1D, Caliper1DConfig};
use crate::geometry::{Angle, Line2D, Point2D, Vec2D};
use crate::image::GrayImageRef;
use crate::profile::EdgePolarity;

/// Defines a scan region as a set of parallel caliper lines.
#[derive(Debug, Clone)]
pub struct ScanRegion {
    /// Start point of the first scan line.
    pub start: Point2D,
    /// End point of the first scan line.
    pub end: Point2D,
    /// Offset direction for parallel scan lines.
    pub step_direction: Vec2D,
    /// Distance between parallel scan lines.
    pub step_size: f64,
    /// Number of parallel scan lines.
    pub num_lines: u32,
}

/// Configuration for chamfer measurement.
#[derive(Debug, Clone)]
pub struct ChamferGaugeConfig {
    /// Scan region for surface A (first flat surface).
    pub surface_a: ScanRegion,
    /// Scan region for the chamfer surface.
    pub chamfer_surface: ScanRegion,
    /// Scan region for surface B (second flat surface).
    pub surface_b: ScanRegion,
    /// Caliper scan width for edge detection.
    pub scan_width: u32,
    /// Gaussian smoothing sigma.
    pub smoothing_sigma: f64,
    /// Minimum edge strength.
    pub min_edge_strength: f64,
    /// Edge polarity to detect.
    pub polarity: EdgePolarity,
}

/// Result of a chamfer measurement.
#[derive(Debug, Clone)]
pub struct ChamferResult {
    /// Fitted line for surface A.
    pub line_a: Line2D,
    /// Fitted line for the chamfer surface.
    pub line_chamfer: Line2D,
    /// Fitted line for surface B.
    pub line_b: Line2D,
    /// Angle between chamfer and surface A.
    pub angle_a: Angle,
    /// Angle between chamfer and surface B.
    pub angle_b: Angle,
    /// Chamfer width: distance along the chamfer line between its intersection
    /// with surface A and surface B.
    pub chamfer_width: f64,
    /// Intersection point of chamfer with surface A.
    pub intersection_a: Point2D,
    /// Intersection point of chamfer with surface B.
    pub intersection_b: Point2D,
    /// RMS error of line fits (max of the three).
    pub max_rms_error: f64,
    /// Number of points used for each line fit.
    pub points_per_surface: [usize; 3],
}

pub struct ChamferGauge;

impl ChamferGauge {
    /// Measure chamfer geometry from three scan regions.
    pub fn measure(
        image: &GrayImageRef<'_>,
        config: &ChamferGaugeConfig,
    ) -> MetrologyResult<ChamferResult> {
        // Detect edges on each surface
        let pts_a = Self::scan_region_edges(image, &config.surface_a, config)?;
        let pts_chamfer = Self::scan_region_edges(image, &config.chamfer_surface, config)?;
        let pts_b = Self::scan_region_edges(image, &config.surface_b, config)?;

        // Fit lines
        let fit_a = fit_line(&pts_a)?;
        let fit_chamfer = fit_line(&pts_chamfer)?;
        let fit_b = fit_line(&pts_b)?;

        // Intersection points
        let intersection_a = fit_a
            .line
            .intersect(&fit_chamfer.line)
            .ok_or(MetrologyError::DegenerateGeometry(
                "surface A parallel to chamfer",
            ))?;

        let intersection_b = fit_chamfer
            .line
            .intersect(&fit_b.line)
            .ok_or(MetrologyError::DegenerateGeometry(
                "chamfer parallel to surface B",
            ))?;

        // Chamfer width
        let chamfer_width = intersection_a.distance_to(&intersection_b);

        // Angles between lines
        let angle_a = angle_between_lines(&fit_a.line, &fit_chamfer.line);
        let angle_b = angle_between_lines(&fit_chamfer.line, &fit_b.line);

        let max_rms_error = fit_a
            .rms_error
            .max(fit_chamfer.rms_error)
            .max(fit_b.rms_error);

        Ok(ChamferResult {
            line_a: fit_a.line,
            line_chamfer: fit_chamfer.line,
            line_b: fit_b.line,
            angle_a,
            angle_b,
            chamfer_width,
            intersection_a,
            intersection_b,
            max_rms_error,
            points_per_surface: [pts_a.len(), pts_chamfer.len(), pts_b.len()],
        })
    }

    fn scan_region_edges(
        image: &GrayImageRef<'_>,
        region: &ScanRegion,
        config: &ChamferGaugeConfig,
    ) -> MetrologyResult<Vec<Point2D>> {
        let mut points = Vec::new();

        for i in 0..region.num_lines {
            let offset = i as f64 * region.step_size;
            let start = Point2D::new(
                region.start.x + offset * region.step_direction.dx,
                region.start.y + offset * region.step_direction.dy,
            );
            let end = Point2D::new(
                region.end.x + offset * region.step_direction.dx,
                region.end.y + offset * region.step_direction.dy,
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
                points.push(edge.point);
            }
        }

        if points.len() < 2 {
            return Err(MetrologyError::InsufficientData {
                needed: 2,
                got: points.len(),
            });
        }

        Ok(points)
    }
}

/// Compute the acute angle between two lines.
fn angle_between_lines(a: &Line2D, b: &Line2D) -> Angle {
    let dot = a.direction.dot(&b.direction).abs();
    let clamped = dot.clamp(0.0, 1.0);
    Angle::from_radians(clamped.acos())
}
