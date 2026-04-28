/// # Caliper1D Gauge
///
/// Measures distance between edges along a 1D scan line in an image.
///
/// This is the fundamental building block: a virtual caliper placed on the image.
/// It extracts a brightness profile, detects edges, and returns measurements.
///
/// ## Algorithm
/// 1. Extract 1D brightness profile along the scan segment (with optional averaging width)
/// 2. Smooth with Gaussian filter to suppress noise
/// 3. Compute gradient (first derivative)
/// 4. Detect peaks in gradient → edge candidates
/// 5. Sub-pixel refinement via parabolic interpolation
/// 6. Return edge positions and/or edge-pair distances

use crate::error::{MetrologyError, MetrologyResult};
use crate::geometry::Point2D;
use crate::image::GrayImageRef;
use crate::profile::{self, detect_edges, find_edge_pairs, EdgePolarity, Profile1D};

/// Configuration for a Caliper1D measurement.
#[derive(Debug, Clone)]
#[repr(C)]
pub struct Caliper1DConfig {
    /// Start point of the scan line (image coordinates).
    pub start: Point2D,
    /// End point of the scan line (image coordinates).
    pub end: Point2D,
    /// Number of parallel scan lines to average (1 = no averaging). Odd values recommended.
    pub scan_width: u32,
    /// Gaussian smoothing sigma in sample units. 0 = no smoothing.
    pub smoothing_sigma: f64,
    /// Minimum gradient magnitude to consider as an edge.
    pub min_edge_strength: f64,
    /// Edge polarity filter.
    pub polarity: EdgePolarity,
    /// Sampling step in pixels (1.0 = every pixel). Smaller values give finer sampling.
    pub step: f64,
}

impl Default for Caliper1DConfig {
    fn default() -> Self {
        Self {
            start: Point2D::new(0.0, 0.0),
            end: Point2D::new(100.0, 0.0),
            scan_width: 5,
            smoothing_sigma: 1.0,
            min_edge_strength: 10.0,
            polarity: EdgePolarity::Any,
            step: 1.0,
        }
    }
}

/// Result of a single-edge caliper measurement.
#[derive(Debug, Clone)]
pub struct Caliper1DEdgeResult {
    /// All detected edges along the scan line, sorted by position.
    pub edges: Vec<Caliper1DEdge>,
    /// The extracted and smoothed profile (for diagnostics).
    pub profile: Profile1D,
}

/// A single detected edge with image-space coordinates.
#[derive(Debug, Clone, Copy)]
pub struct Caliper1DEdge {
    /// Position in image coordinates.
    pub point: Point2D,
    /// Sub-pixel position along the profile (sample units).
    pub profile_position: f64,
    /// Edge strength (gradient magnitude).
    pub strength: f64,
    /// Transition direction.
    pub polarity: EdgePolarity,
}

/// Result of an edge-pair caliper measurement (measures stripe width).
#[derive(Debug, Clone)]
pub struct Caliper1DPairResult {
    /// All detected edge pairs.
    pub pairs: Vec<Caliper1DPair>,
    /// Total number of individual edges detected.
    pub edge_count: usize,
    /// The profile used.
    pub profile: Profile1D,
}

/// A detected edge pair with distance measurement.
#[derive(Debug, Clone, Copy)]
pub struct Caliper1DPair {
    /// Leading edge position in image coordinates.
    pub leading: Point2D,
    /// Trailing edge position in image coordinates.
    pub trailing: Point2D,
    /// Distance between the two edges in pixels.
    pub distance_px: f64,
    /// Leading edge strength.
    pub leading_strength: f64,
    /// Trailing edge strength.
    pub trailing_strength: f64,
}

/// The Caliper1D gauge.
pub struct Caliper1D;

impl Caliper1D {
    /// Detect all edges along the scan line.
    pub fn find_edges(
        image: &GrayImageRef<'_>,
        config: &Caliper1DConfig,
    ) -> MetrologyResult<Caliper1DEdgeResult> {
        let profile = profile::extract_profile(
            image,
            config.start,
            config.end,
            config.scan_width,
            config.step,
        );

        if profile.is_empty() {
            return Err(MetrologyError::EmptyProfile);
        }

        let smoothed = profile::smooth_gaussian(&profile.values, config.smoothing_sigma);
        let grad = profile::gradient(&smoothed);
        let raw_edges = detect_edges(&grad, config.min_edge_strength, config.polarity);

        let smoothed_profile = Profile1D {
            values: smoothed,
            origin: profile.origin,
            direction: profile.direction,
            step: profile.step,
        };

        let edges = raw_edges
            .iter()
            .map(|e| Caliper1DEdge {
                point: e.to_image_point(&smoothed_profile),
                profile_position: e.position,
                strength: e.strength,
                polarity: e.polarity,
            })
            .collect();

        Ok(Caliper1DEdgeResult {
            edges,
            profile: smoothed_profile,
        })
    }

    /// Find the single strongest edge along the scan line.
    pub fn find_strongest_edge(
        image: &GrayImageRef<'_>,
        config: &Caliper1DConfig,
    ) -> MetrologyResult<Caliper1DEdge> {
        let result = Self::find_edges(image, config)?;
        result
            .edges
            .into_iter()
            .max_by(|a, b| a.strength.partial_cmp(&b.strength).unwrap())
            .ok_or(MetrologyError::NoEdgeFound)
    }

    /// Find edge pairs (stripe/gap measurements) along the scan line.
    ///
    /// `min_width` / `max_width` constrain pair width in pixels.
    pub fn find_pairs(
        image: &GrayImageRef<'_>,
        config: &Caliper1DConfig,
        min_width_px: f64,
        max_width_px: f64,
    ) -> MetrologyResult<Caliper1DPairResult> {
        let profile = profile::extract_profile(
            image,
            config.start,
            config.end,
            config.scan_width,
            config.step,
        );

        if profile.is_empty() {
            return Err(MetrologyError::EmptyProfile);
        }

        let smoothed = profile::smooth_gaussian(&profile.values, config.smoothing_sigma);
        let grad = profile::gradient(&smoothed);
        let raw_edges = detect_edges(&grad, config.min_edge_strength, EdgePolarity::Any);

        let smoothed_profile = Profile1D {
            values: smoothed,
            origin: profile.origin,
            direction: profile.direction,
            step: profile.step,
        };

        let min_samples = min_width_px / config.step;
        let max_samples = max_width_px / config.step;
        let raw_pairs = find_edge_pairs(&raw_edges, min_samples, max_samples);

        let pairs = raw_pairs
            .iter()
            .map(|pair| {
                let leading = pair.leading.to_image_point(&smoothed_profile);
                let trailing = pair.trailing.to_image_point(&smoothed_profile);
                Caliper1DPair {
                    leading,
                    trailing,
                    distance_px: leading.distance_to(&trailing),
                    leading_strength: pair.leading.strength,
                    trailing_strength: pair.trailing.strength,
                }
            })
            .collect();

        Ok(Caliper1DPairResult {
            pairs,
            edge_count: raw_edges.len(),
            profile: smoothed_profile,
        })
    }

    /// Convenience: measure the distance between the first edge pair found.
    /// Returns the distance in pixels.
    pub fn measure_width(
        image: &GrayImageRef<'_>,
        config: &Caliper1DConfig,
        min_width_px: f64,
        max_width_px: f64,
    ) -> MetrologyResult<f64> {
        let result = Self::find_pairs(image, config, min_width_px, max_width_px)?;
        result
            .pairs
            .first()
            .map(|p| p.distance_px)
            .ok_or(MetrologyError::NoEdgePairFound)
    }
}
