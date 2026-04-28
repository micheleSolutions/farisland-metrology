/// 1D brightness profile extraction and processing.
///
/// A profile is a sequence of intensity samples taken along a scan path in an image.
/// This module handles:
/// - Extracting profiles along arbitrary lines (with configurable scan width for averaging)
/// - Gaussian smoothing
/// - Gradient (first derivative) computation
/// - Peak detection in the gradient for edge localization
/// - Sub-pixel refinement via parabolic interpolation

use crate::geometry::{Point2D, Vec2D};
use crate::image::GrayImageRef;

/// Raw 1D intensity profile with position metadata.
#[derive(Debug, Clone)]
pub struct Profile1D {
    /// Intensity values along the scan direction.
    pub values: Vec<f64>,
    /// Start point of the profile in image coordinates.
    pub origin: Point2D,
    /// Unit direction along the profile.
    pub direction: Vec2D,
    /// Step size in pixels between consecutive samples.
    pub step: f64,
}

impl Profile1D {
    /// World position of the i-th sample.
    #[inline]
    pub fn position_at(&self, i: usize) -> Point2D {
        let t = i as f64 * self.step;
        Point2D::new(
            self.origin.x + t * self.direction.dx,
            self.origin.y + t * self.direction.dy,
        )
    }

    /// Number of samples.
    #[inline]
    pub fn len(&self) -> usize {
        self.values.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

/// Extract a 1D brightness profile from an image along a line segment.
///
/// - `start`, `end`: endpoints of the scan line
/// - `scan_width`: number of parallel lines to average (1 = single line, odd values recommended)
/// - `step`: distance in pixels between consecutive samples (1.0 = every pixel)
///
/// The profile is sampled using bilinear interpolation. When `scan_width > 1`,
/// multiple parallel scan lines are averaged orthogonally to reduce noise.
pub fn extract_profile(
    image: &GrayImageRef<'_>,
    start: Point2D,
    end: Point2D,
    scan_width: u32,
    step: f64,
) -> Profile1D {
    let dir = Vec2D::new(end.x - start.x, end.y - start.y);
    let length = dir.length();
    let unit_dir = dir.normalized();
    let perp = unit_dir.perpendicular();

    let n_samples = ((length / step).floor() as usize).max(1);
    let half_w = (scan_width as f64 - 1.0) / 2.0;

    let mut values = Vec::with_capacity(n_samples);

    for i in 0..n_samples {
        let t = i as f64 * step;
        let cx = start.x + t * unit_dir.dx;
        let cy = start.y + t * unit_dir.dy;

        let mut sum = 0.0;
        let mut count = 0u32;
        for w in 0..scan_width {
            let offset = w as f64 - half_w;
            let sx = cx + offset * perp.dx;
            let sy = cy + offset * perp.dy;
            if sx >= 0.0
                && sx < (image.width() - 1) as f64
                && sy >= 0.0
                && sy < (image.height() - 1) as f64
            {
                sum += image.sample(sx, sy);
                count += 1;
            }
        }
        values.push(if count > 0 { sum / count as f64 } else { 0.0 });
    }

    Profile1D {
        values,
        origin: start,
        direction: unit_dir,
        step,
    }
}

/// Apply Gaussian smoothing to a profile.
///
/// `sigma` is in units of samples (not pixels). A kernel of radius `ceil(3*sigma)` is used.
pub fn smooth_gaussian(profile: &[f64], sigma: f64) -> Vec<f64> {
    if sigma < 0.01 || profile.len() < 2 {
        return profile.to_vec();
    }

    let radius = (3.0 * sigma).ceil() as usize;
    let kernel_size = 2 * radius + 1;
    let mut kernel = Vec::with_capacity(kernel_size);
    let mut ksum = 0.0;
    for i in 0..kernel_size {
        let x = i as f64 - radius as f64;
        let g = (-x * x / (2.0 * sigma * sigma)).exp();
        kernel.push(g);
        ksum += g;
    }
    for k in &mut kernel {
        *k /= ksum;
    }

    let n = profile.len();
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let mut val = 0.0;
        for (j, &kv) in kernel.iter().enumerate() {
            let idx = i as isize + j as isize - radius as isize;
            let idx = idx.clamp(0, n as isize - 1) as usize;
            val += profile[idx] * kv;
        }
        out.push(val);
    }
    out
}

/// Compute the first derivative (central differences) of a profile.
pub fn gradient(profile: &[f64]) -> Vec<f64> {
    let n = profile.len();
    if n < 2 {
        return vec![0.0; n];
    }
    let mut grad = Vec::with_capacity(n);
    grad.push(profile[1] - profile[0]);
    for i in 1..n - 1 {
        grad.push((profile[i + 1] - profile[i - 1]) * 0.5);
    }
    grad.push(profile[n - 1] - profile[n - 2]);
    grad
}

/// Edge transition direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub enum EdgePolarity {
    /// Dark to bright (positive gradient).
    DarkToBright,
    /// Bright to dark (negative gradient).
    BrightToDark,
    /// Either direction.
    Any,
}

/// A detected edge in a 1D profile.
#[derive(Debug, Clone, Copy)]
pub struct Edge1D {
    /// Sub-pixel position along the profile (in sample units).
    pub position: f64,
    /// Gradient magnitude at the edge (always positive).
    pub strength: f64,
    /// Direction of the transition.
    pub polarity: EdgePolarity,
}

impl Edge1D {
    /// Convert the sample-space position to image coordinates using the parent profile.
    pub fn to_image_point(&self, profile: &Profile1D) -> Point2D {
        let t = self.position * profile.step;
        Point2D::new(
            profile.origin.x + t * profile.direction.dx,
            profile.origin.y + t * profile.direction.dy,
        )
    }
}

/// Detect edges in a gradient profile using peak detection + parabolic sub-pixel refinement.
///
/// - `grad`: gradient profile (from `gradient()`)
/// - `min_magnitude`: minimum absolute gradient to consider as an edge
/// - `polarity`: filter by edge direction
///
/// Returns edges sorted by position along the profile.
pub fn detect_edges(grad: &[f64], min_magnitude: f64, polarity: EdgePolarity) -> Vec<Edge1D> {
    let n = grad.len();
    if n < 3 {
        return Vec::new();
    }

    let mut edges = Vec::new();

    for i in 1..n - 1 {
        let prev = grad[i - 1].abs();
        let curr = grad[i].abs();
        let next = grad[i + 1].abs();

        // Local maximum in absolute gradient
        if curr >= prev && curr >= next && curr >= min_magnitude {
            // Check polarity
            let edge_polarity = if grad[i] > 0.0 {
                EdgePolarity::DarkToBright
            } else {
                EdgePolarity::BrightToDark
            };

            if polarity != EdgePolarity::Any && polarity != edge_polarity {
                continue;
            }

            // Parabolic sub-pixel refinement on absolute gradient values
            let denom = prev - 2.0 * curr + next;
            let sub_offset = if denom.abs() > 1e-10 {
                0.5 * (prev - next) / denom
            } else {
                0.0
            };

            let refined_pos = i as f64 + sub_offset;
            // Interpolated strength at the refined position
            let refined_strength = curr - 0.25 * (prev - next) * sub_offset;

            edges.push(Edge1D {
                position: refined_pos,
                strength: refined_strength,
                polarity: edge_polarity,
            });
        }
    }

    edges.sort_by(|a, b| a.position.partial_cmp(&b.position).unwrap());
    edges
}

/// An edge pair (two edges forming a stripe or gap).
#[derive(Debug, Clone, Copy)]
pub struct EdgePair1D {
    pub leading: Edge1D,
    pub trailing: Edge1D,
    /// Distance between the two edges in sample units.
    pub width: f64,
}

/// Find edge pairs in a list of detected edges.
///
/// An edge pair consists of a leading edge followed by a trailing edge of opposite polarity.
/// `min_width` / `max_width` constrain the acceptable pair width (in sample units).
pub fn find_edge_pairs(
    edges: &[Edge1D],
    min_width: f64,
    max_width: f64,
) -> Vec<EdgePair1D> {
    let mut pairs = Vec::new();

    for (i, leading) in edges.iter().enumerate() {
        for trailing in edges.iter().skip(i + 1) {
            // Opposite polarity
            if leading.polarity == trailing.polarity {
                continue;
            }
            let w = trailing.position - leading.position;
            if w >= min_width && w <= max_width {
                pairs.push(EdgePair1D {
                    leading: *leading,
                    trailing: *trailing,
                    width: w,
                });
                break; // Greedy: take the first valid match for this leading edge
            }
        }
    }

    pairs
}
