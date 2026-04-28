/// C ABI exports for farisland-metrology.
///
/// All functions use `#[no_mangle]` + `extern "C"` for consumption via
/// cbindgen-generated headers. Opaque pointers and flat C structs are used
/// to avoid exposing Rust internals.
///
/// Naming convention: `fm_<module>_<function>`
///
/// Memory convention:
/// - Functions returning `*mut T` transfer ownership to the caller
/// - Caller frees via corresponding `fm_*_free` function
/// - Functions taking `*const T` borrow without taking ownership

use std::ptr;

use crate::error::MetrologyError;
use crate::gauges::caliper1d::{Caliper1D, Caliper1DConfig};
use crate::geometry::Point2D;
use crate::image::GrayImage;
use crate::profile::EdgePolarity;

// ── Error handling ──────────────────────────────────────────────────────────

/// Status codes for C API.
#[repr(C)]
pub enum FmStatus {
    Ok = 0,
    InsufficientData = 1,
    EmptyProfile = 2,
    NoEdgeFound = 3,
    NoEdgePairFound = 4,
    FittingDidNotConverge = 5,
    InvalidImageDimensions = 6,
    ScanOutOfBounds = 7,
    DegenerateGeometry = 8,
    NullPointer = 9,
}

impl From<&MetrologyError> for FmStatus {
    fn from(e: &MetrologyError) -> Self {
        match e {
            MetrologyError::InsufficientData { .. } => FmStatus::InsufficientData,
            MetrologyError::EmptyProfile => FmStatus::EmptyProfile,
            MetrologyError::NoEdgeFound => FmStatus::NoEdgeFound,
            MetrologyError::NoEdgePairFound => FmStatus::NoEdgePairFound,
            MetrologyError::FittingDidNotConverge => FmStatus::FittingDidNotConverge,
            MetrologyError::InvalidImageDimensions { .. } => FmStatus::InvalidImageDimensions,
            MetrologyError::ScanOutOfBounds => FmStatus::ScanOutOfBounds,
            MetrologyError::DegenerateGeometry(_) => FmStatus::DegenerateGeometry,
        }
    }
}

// ── Image ───────────────────────────────────────────────────────────────────

/// Create a GrayImage from raw pixel data. Caller takes ownership of the returned pointer.
///
/// # Safety
/// `data` must point to `width * height` valid bytes.
#[no_mangle]
pub unsafe extern "C" fn fm_image_create(
    data: *const u8,
    width: u32,
    height: u32,
) -> *mut GrayImage {
    if data.is_null() {
        return ptr::null_mut();
    }
    let len = width as usize * height as usize;
    let slice = unsafe { std::slice::from_raw_parts(data, len) };
    match GrayImage::new(slice.to_vec(), width, height) {
        Ok(img) => Box::into_raw(Box::new(img)),
        Err(_) => ptr::null_mut(),
    }
}

/// Free a GrayImage previously created by `fm_image_create`.
///
/// # Safety
/// `img` must be a valid pointer from `fm_image_create` or null.
#[no_mangle]
pub unsafe extern "C" fn fm_image_free(img: *mut GrayImage) {
    if !img.is_null() {
        drop(unsafe { Box::from_raw(img) });
    }
}

// ── Caliper1D ───────────────────────────────────────────────────────────────

/// C-compatible caliper config.
#[repr(C)]
pub struct FmCaliper1DConfig {
    pub start_x: f64,
    pub start_y: f64,
    pub end_x: f64,
    pub end_y: f64,
    pub scan_width: u32,
    pub smoothing_sigma: f64,
    pub min_edge_strength: f64,
    /// 0 = DarkToBright, 1 = BrightToDark, 2 = Any
    pub polarity: u32,
    pub step: f64,
}

/// C-compatible edge result.
#[repr(C)]
pub struct FmEdge {
    pub x: f64,
    pub y: f64,
    pub strength: f64,
    pub polarity: u32,
}

/// C-compatible edge array result.
#[repr(C)]
pub struct FmEdgeArray {
    pub edges: *mut FmEdge,
    pub count: u32,
    pub status: FmStatus,
}

/// Find all edges along a caliper scan line.
///
/// # Safety
/// `image` must be a valid GrayImage pointer. The returned `FmEdgeArray.edges` must be
/// freed with `fm_edges_free`.
#[no_mangle]
pub unsafe extern "C" fn fm_caliper1d_find_edges(
    image: *const GrayImage,
    config: *const FmCaliper1DConfig,
) -> FmEdgeArray {
    let empty = FmEdgeArray {
        edges: ptr::null_mut(),
        count: 0,
        status: FmStatus::NullPointer,
    };

    if image.is_null() || config.is_null() {
        return empty;
    }

    let image = unsafe { &*image };
    let cfg = unsafe { &*config };

    let polarity = match cfg.polarity {
        0 => EdgePolarity::DarkToBright,
        1 => EdgePolarity::BrightToDark,
        _ => EdgePolarity::Any,
    };

    let rust_config = Caliper1DConfig {
        start: Point2D::new(cfg.start_x, cfg.start_y),
        end: Point2D::new(cfg.end_x, cfg.end_y),
        scan_width: cfg.scan_width,
        smoothing_sigma: cfg.smoothing_sigma,
        min_edge_strength: cfg.min_edge_strength,
        polarity,
        step: cfg.step,
    };

    let img_ref = image.as_ref();

    match Caliper1D::find_edges(&img_ref, &rust_config) {
        Ok(result) => {
            let mut c_edges: Vec<FmEdge> = result
                .edges
                .iter()
                .map(|e| FmEdge {
                    x: e.point.x,
                    y: e.point.y,
                    strength: e.strength,
                    polarity: match e.polarity {
                        EdgePolarity::DarkToBright => 0,
                        EdgePolarity::BrightToDark => 1,
                        EdgePolarity::Any => 2,
                    },
                })
                .collect();

            let count = c_edges.len() as u32;
            let ptr = c_edges.as_mut_ptr();
            std::mem::forget(c_edges);

            FmEdgeArray {
                edges: ptr,
                count,
                status: FmStatus::Ok,
            }
        }
        Err(ref e) => FmEdgeArray {
            edges: ptr::null_mut(),
            count: 0,
            status: FmStatus::from(e),
        },
    }
}

/// Free an edge array returned by `fm_caliper1d_find_edges`.
///
/// # Safety
/// `edges` must be a valid pointer from `fm_caliper1d_find_edges` or null.
#[no_mangle]
pub unsafe extern "C" fn fm_edges_free(edges: *mut FmEdge, count: u32) {
    if !edges.is_null() {
        drop(unsafe { Vec::from_raw_parts(edges, count as usize, count as usize) });
    }
}

// ── Diameter ────────────────────────────────────────────────────────────────

/// C-compatible diameter measurement config.
#[repr(C)]
pub struct FmDiameterConfig {
    pub center_x: f64,
    pub center_y: f64,
    pub nominal_radius: f64,
    pub search_margin: f64,
    pub num_calipers: u32,
    pub scan_width: u32,
    pub smoothing_sigma: f64,
    pub min_edge_strength: f64,
    pub polarity: u32,
    pub geometric_refinement: u32,
}

/// C-compatible diameter result.
#[repr(C)]
pub struct FmDiameterResult {
    pub center_x: f64,
    pub center_y: f64,
    pub diameter: f64,
    pub radius: f64,
    pub rms_error: f64,
    pub num_points: u32,
    pub status: FmStatus,
}

/// Measure diameter.
///
/// # Safety
/// `image` and `config` must be valid non-null pointers.
#[no_mangle]
pub unsafe extern "C" fn fm_diameter_measure(
    image: *const GrayImage,
    config: *const FmDiameterConfig,
) -> FmDiameterResult {
    let err_result = |status| FmDiameterResult {
        center_x: 0.0,
        center_y: 0.0,
        diameter: 0.0,
        radius: 0.0,
        rms_error: 0.0,
        num_points: 0,
        status,
    };

    if image.is_null() || config.is_null() {
        return err_result(FmStatus::NullPointer);
    }

    let image = unsafe { &*image };
    let cfg = unsafe { &*config };

    let polarity = match cfg.polarity {
        0 => EdgePolarity::DarkToBright,
        1 => EdgePolarity::BrightToDark,
        _ => EdgePolarity::Any,
    };

    let rust_config = crate::gauges::diameter::DiameterGaugeConfig {
        nominal_center: Point2D::new(cfg.center_x, cfg.center_y),
        nominal_radius: cfg.nominal_radius,
        search_margin: cfg.search_margin,
        num_calipers: cfg.num_calipers,
        scan_width: cfg.scan_width,
        smoothing_sigma: cfg.smoothing_sigma,
        min_edge_strength: cfg.min_edge_strength,
        polarity,
        geometric_refinement: cfg.geometric_refinement != 0,
        max_iterations: 50,
    };

    let img_ref = image.as_ref();

    match crate::gauges::diameter::DiameterGauge::measure(&img_ref, &rust_config) {
        Ok(r) => FmDiameterResult {
            center_x: r.circle.center.x,
            center_y: r.circle.center.y,
            diameter: r.diameter,
            radius: r.circle.radius,
            rms_error: r.rms_error,
            num_points: r.num_points as u32,
            status: FmStatus::Ok,
        },
        Err(ref e) => err_result(FmStatus::from(e)),
    }
}

// ── Thread Pitch ────────────────────────────────────────────────────────────

/// C-compatible thread pitch config.
#[repr(C)]
pub struct FmThreadPitchConfig {
    pub start_x: f64,
    pub start_y: f64,
    pub end_x: f64,
    pub end_y: f64,
    pub scan_width: u32,
    pub smoothing_sigma: f64,
    pub min_peak_prominence: f64,
    pub expected_pitch_min: f64,
    pub expected_pitch_max: f64,
    pub step: f64,
}

/// C-compatible thread pitch result.
#[repr(C)]
pub struct FmThreadPitchResult {
    pub mean_pitch_px: f64,
    pub std_dev_px: f64,
    pub thread_count: u32,
    pub status: FmStatus,
}

/// Measure thread pitch by peak detection.
///
/// # Safety
/// `image` and `config` must be valid non-null pointers.
#[no_mangle]
pub unsafe extern "C" fn fm_thread_pitch_measure(
    image: *const GrayImage,
    config: *const FmThreadPitchConfig,
) -> FmThreadPitchResult {
    let err_result = |status| FmThreadPitchResult {
        mean_pitch_px: 0.0,
        std_dev_px: 0.0,
        thread_count: 0,
        status,
    };

    if image.is_null() || config.is_null() {
        return err_result(FmStatus::NullPointer);
    }

    let image = unsafe { &*image };
    let cfg = unsafe { &*config };

    let rust_config = crate::gauges::thread_pitch::ThreadPitchGaugeConfig {
        start: Point2D::new(cfg.start_x, cfg.start_y),
        end: Point2D::new(cfg.end_x, cfg.end_y),
        scan_width: cfg.scan_width,
        smoothing_sigma: cfg.smoothing_sigma,
        min_peak_prominence: cfg.min_peak_prominence,
        expected_pitch_range: (cfg.expected_pitch_min, cfg.expected_pitch_max),
        step: cfg.step,
    };

    let img_ref = image.as_ref();

    match crate::gauges::thread_pitch::ThreadPitchGauge::measure_by_peaks(&img_ref, &rust_config) {
        Ok(r) => FmThreadPitchResult {
            mean_pitch_px: r.mean_pitch_px,
            std_dev_px: r.std_dev_px,
            thread_count: r.thread_count as u32,
            status: FmStatus::Ok,
        },
        Err(ref e) => err_result(FmStatus::from(e)),
    }
}
