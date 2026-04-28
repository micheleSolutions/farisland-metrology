/// UniFFI-compatible wrapper API.
///
/// This module exposes a simplified, high-level API through UniFFI for
/// Java, Kotlin, Python, Swift, and Ruby consumers. The types here are
/// designed to be UniFFI-friendly (no raw pointers, no lifetimes).

use crate::error::MetrologyError;
use crate::gauges::caliper1d::{Caliper1D, Caliper1DConfig};
use crate::gauges::chamfer::{ChamferGauge, ChamferGaugeConfig, ScanRegion};
use crate::gauges::diameter::{DiameterGauge, DiameterGaugeConfig};
use crate::gauges::radius::{RadiusGauge, RadiusGaugeConfig};
use crate::gauges::thread_pitch::{ThreadPitchGauge, ThreadPitchGaugeConfig};
use crate::geometry::{Point2D, Vec2D};
use crate::image::GrayImage;
use crate::profile::EdgePolarity;

// Re-export error for UniFFI
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum MeasurementError {
    #[error("Insufficient data: need {needed}, got {got}")]
    InsufficientData { needed: u32, got: u32 },
    #[error("Empty profile")]
    EmptyProfile,
    #[error("No edge found")]
    NoEdgeFound,
    #[error("No edge pair found")]
    NoEdgePairFound,
    #[error("Fitting did not converge")]
    FittingDidNotConverge,
    #[error("Invalid image dimensions")]
    InvalidImage,
    #[error("Degenerate geometry: {reason}")]
    DegenerateGeometry { reason: String },
}

impl From<MetrologyError> for MeasurementError {
    fn from(e: MetrologyError) -> Self {
        match e {
            MetrologyError::InsufficientData { needed, got } => {
                MeasurementError::InsufficientData {
                    needed: needed as u32,
                    got: got as u32,
                }
            }
            MetrologyError::EmptyProfile => MeasurementError::EmptyProfile,
            MetrologyError::NoEdgeFound => MeasurementError::NoEdgeFound,
            MetrologyError::NoEdgePairFound => MeasurementError::NoEdgePairFound,
            MetrologyError::FittingDidNotConverge => MeasurementError::FittingDidNotConverge,
            MetrologyError::InvalidImageDimensions { .. } => MeasurementError::InvalidImage,
            MetrologyError::ScanOutOfBounds => MeasurementError::InvalidImage,
            MetrologyError::DegenerateGeometry(msg) => MeasurementError::DegenerateGeometry {
                reason: msg.to_string(),
            },
        }
    }
}

// ── UniFFI-exported records ─────────────────────────────────────────────────

#[derive(Debug, Clone, uniffi::Record)]
pub struct UPoint {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct UEdge {
    pub x: f64,
    pub y: f64,
    pub strength: f64,
    pub is_dark_to_bright: bool,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct UEdgePair {
    pub leading_x: f64,
    pub leading_y: f64,
    pub trailing_x: f64,
    pub trailing_y: f64,
    pub distance_px: f64,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct UDiameterResult {
    pub center_x: f64,
    pub center_y: f64,
    pub diameter: f64,
    pub radius: f64,
    pub rms_error: f64,
    pub num_points: u32,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct UChamferResult {
    pub angle_a_degrees: f64,
    pub angle_b_degrees: f64,
    pub chamfer_width: f64,
    pub intersection_a: UPoint,
    pub intersection_b: UPoint,
    pub max_rms_error: f64,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct URadiusResult {
    pub center_x: f64,
    pub center_y: f64,
    pub radius: f64,
    pub arc_span_degrees: f64,
    pub rms_error: f64,
    pub num_points: u32,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct UThreadPitchResult {
    pub mean_pitch_px: f64,
    pub std_dev_px: f64,
    pub thread_count: u32,
    pub pitches: Vec<f64>,
}

// ── UniFFI-exported object ──────────────────────────────────────────────────

/// Main entry point for UniFFI consumers. Wraps a grayscale image and provides
/// measurement methods.
#[derive(uniffi::Object)]
pub struct Metrology {
    image: GrayImage,
}

#[uniffi::export]
impl Metrology {
    /// Create a new Metrology instance from raw grayscale pixel data.
    #[uniffi::constructor]
    pub fn new(data: Vec<u8>, width: u32, height: u32) -> Result<Self, MeasurementError> {
        let image = GrayImage::new(data, width, height).map_err(MeasurementError::from)?;
        Ok(Self { image })
    }

    /// Find edges along a scan line (Caliper1D).
    pub fn find_edges(
        &self,
        start_x: f64,
        start_y: f64,
        end_x: f64,
        end_y: f64,
        scan_width: u32,
        smoothing_sigma: f64,
        min_edge_strength: f64,
    ) -> Result<Vec<UEdge>, MeasurementError> {
        let config = Caliper1DConfig {
            start: Point2D::new(start_x, start_y),
            end: Point2D::new(end_x, end_y),
            scan_width,
            smoothing_sigma,
            min_edge_strength,
            polarity: EdgePolarity::Any,
            step: 1.0,
        };
        let img_ref = self.image.as_ref();
        let result = Caliper1D::find_edges(&img_ref, &config).map_err(MeasurementError::from)?;
        Ok(result
            .edges
            .iter()
            .map(|e| UEdge {
                x: e.point.x,
                y: e.point.y,
                strength: e.strength,
                is_dark_to_bright: e.polarity == EdgePolarity::DarkToBright,
            })
            .collect())
    }

    /// Measure caliper width (distance between first edge pair).
    pub fn measure_caliper_width(
        &self,
        start_x: f64,
        start_y: f64,
        end_x: f64,
        end_y: f64,
        scan_width: u32,
        smoothing_sigma: f64,
        min_edge_strength: f64,
        min_width_px: f64,
        max_width_px: f64,
    ) -> Result<f64, MeasurementError> {
        let config = Caliper1DConfig {
            start: Point2D::new(start_x, start_y),
            end: Point2D::new(end_x, end_y),
            scan_width,
            smoothing_sigma,
            min_edge_strength,
            polarity: EdgePolarity::Any,
            step: 1.0,
        };
        let img_ref = self.image.as_ref();
        Caliper1D::measure_width(&img_ref, &config, min_width_px, max_width_px)
            .map_err(MeasurementError::from)
    }

    /// Measure diameter of a circular feature.
    pub fn measure_diameter(
        &self,
        center_x: f64,
        center_y: f64,
        nominal_radius: f64,
        search_margin: f64,
        num_calipers: u32,
        min_edge_strength: f64,
    ) -> Result<UDiameterResult, MeasurementError> {
        let config = DiameterGaugeConfig {
            nominal_center: Point2D::new(center_x, center_y),
            nominal_radius,
            search_margin,
            num_calipers,
            min_edge_strength,
            geometric_refinement: true,
            ..Default::default()
        };
        let img_ref = self.image.as_ref();
        let r = DiameterGauge::measure(&img_ref, &config).map_err(MeasurementError::from)?;
        Ok(UDiameterResult {
            center_x: r.circle.center.x,
            center_y: r.circle.center.y,
            diameter: r.diameter,
            radius: r.circle.radius,
            rms_error: r.rms_error,
            num_points: r.num_points as u32,
        })
    }

    /// Measure radius of a partial arc.
    pub fn measure_radius(
        &self,
        center_x: f64,
        center_y: f64,
        nominal_radius: f64,
        start_angle_deg: f64,
        end_angle_deg: f64,
        search_margin: f64,
        num_calipers: u32,
        min_edge_strength: f64,
    ) -> Result<URadiusResult, MeasurementError> {
        let config = RadiusGaugeConfig {
            nominal_center: Point2D::new(center_x, center_y),
            nominal_radius,
            start_angle: start_angle_deg.to_radians(),
            end_angle: end_angle_deg.to_radians(),
            search_margin,
            num_calipers,
            min_edge_strength,
            geometric_refinement: true,
            ..Default::default()
        };
        let img_ref = self.image.as_ref();
        let r = RadiusGauge::measure(&img_ref, &config).map_err(MeasurementError::from)?;
        Ok(URadiusResult {
            center_x: r.circle.center.x,
            center_y: r.circle.center.y,
            radius: r.radius,
            arc_span_degrees: r.arc_span.degrees(),
            rms_error: r.rms_error,
            num_points: r.num_points as u32,
        })
    }

    /// Measure thread pitch by peak detection.
    pub fn measure_thread_pitch(
        &self,
        start_x: f64,
        start_y: f64,
        end_x: f64,
        end_y: f64,
        scan_width: u32,
        smoothing_sigma: f64,
        min_peak_prominence: f64,
        expected_pitch_min: f64,
        expected_pitch_max: f64,
    ) -> Result<UThreadPitchResult, MeasurementError> {
        let config = ThreadPitchGaugeConfig {
            start: Point2D::new(start_x, start_y),
            end: Point2D::new(end_x, end_y),
            scan_width,
            smoothing_sigma,
            min_peak_prominence,
            expected_pitch_range: (expected_pitch_min, expected_pitch_max),
            step: 1.0,
        };
        let img_ref = self.image.as_ref();
        let r = ThreadPitchGauge::measure_by_peaks(&img_ref, &config)
            .map_err(MeasurementError::from)?;
        Ok(UThreadPitchResult {
            mean_pitch_px: r.mean_pitch_px,
            std_dev_px: r.std_dev_px,
            thread_count: r.thread_count as u32,
            pitches: r.pitches,
        })
    }
}
