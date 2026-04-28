/// Pixel-to-millimeter calibration for anisotropic (non-square) pixels.
///
/// When `h_pixel_size_mm != v_pixel_size_mm`, a line at an angle in pixel space
/// has a different real-world length and angle. This module handles the correct
/// geometry for all conversions.

/// Pixel calibration parameters.
///
/// Set both to 0.0 to disable calibration (results stay in pixels only).
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct PixelCalibration {
    /// Horizontal pixel size in mm (mm per pixel along X axis). 0 = uncalibrated.
    pub h_pixel_size_mm: f64,
    /// Vertical pixel size in mm (mm per pixel along Y axis). 0 = uncalibrated.
    pub v_pixel_size_mm: f64,
}

impl PixelCalibration {
    pub const UNCALIBRATED: Self = Self {
        h_pixel_size_mm: 0.0,
        v_pixel_size_mm: 0.0,
    };

    #[inline]
    pub fn new(h_mm: f64, v_mm: f64) -> Self {
        Self {
            h_pixel_size_mm: h_mm,
            v_pixel_size_mm: v_mm,
        }
    }

    /// Whether calibration is active (both pixel sizes > 0).
    #[inline]
    pub fn is_calibrated(&self) -> bool {
        self.h_pixel_size_mm > 0.0 && self.v_pixel_size_mm > 0.0
    }

    /// Convert a distance in pixels to mm, given the direction of the measurement.
    ///
    /// For anisotropic pixels, a line segment from (x1,y1) to (x2,y2) has
    /// real-world length:
    ///   sqrt((dx * h_px_mm)^2 + (dy * v_px_mm)^2)
    ///
    /// This function takes the pixel-space dx and dy components and returns
    /// the calibrated distance.
    pub fn distance_px_to_mm(&self, dx_px: f64, dy_px: f64) -> f64 {
        if !self.is_calibrated() {
            return 0.0;
        }
        let dx_mm = dx_px * self.h_pixel_size_mm;
        let dy_mm = dy_px * self.v_pixel_size_mm;
        (dx_mm * dx_mm + dy_mm * dy_mm).sqrt()
    }

    /// Convert a distance along a known angle (in pixel space) to mm.
    ///
    /// `distance_px` is the pixel-space distance, `angle_rad` is the angle
    /// of the measurement direction in pixel space (0 = horizontal).
    pub fn distance_along_angle_to_mm(&self, distance_px: f64, angle_rad: f64) -> f64 {
        if !self.is_calibrated() {
            return 0.0;
        }
        let dx = distance_px * angle_rad.cos();
        let dy = distance_px * angle_rad.sin();
        self.distance_px_to_mm(dx, dy)
    }

    /// Convert a point from pixel coordinates to mm coordinates.
    #[inline]
    pub fn point_to_mm(&self, x_px: f64, y_px: f64) -> (f64, f64) {
        if !self.is_calibrated() {
            return (0.0, 0.0);
        }
        (x_px * self.h_pixel_size_mm, y_px * self.v_pixel_size_mm)
    }

    /// Convert a radius from a circle fit to mm.
    ///
    /// For anisotropic pixels, a circle in pixel space is an ellipse in
    /// real space. We return the mean radius: `r * (h + v) / 2`.
    /// This is the standard approximation for small anisotropy.
    pub fn radius_to_mm(&self, radius_px: f64) -> f64 {
        if !self.is_calibrated() {
            return 0.0;
        }
        radius_px * (self.h_pixel_size_mm + self.v_pixel_size_mm) / 2.0
    }

    /// Convert a pixel-space angle to real-world angle.
    ///
    /// With anisotropic pixels, a 45° line in pixel space is NOT 45° in real
    /// space. The real angle is:
    ///   atan2(dy * v_px_mm, dx * h_px_mm)
    pub fn angle_to_real(&self, angle_px_rad: f64) -> f64 {
        if !self.is_calibrated() {
            return 0.0;
        }
        let dx = angle_px_rad.cos() * self.h_pixel_size_mm;
        let dy = angle_px_rad.sin() * self.v_pixel_size_mm;
        dy.atan2(dx)
    }

    /// Convert a scan-line direction angle to a scale factor for distances.
    ///
    /// Useful for Caliper1D: the scan direction has an angle, and all
    /// distances along it scale by this factor.
    pub fn scan_direction_scale(&self, dx_dir: f64, dy_dir: f64) -> f64 {
        if !self.is_calibrated() {
            return 0.0;
        }
        let len_px = (dx_dir * dx_dir + dy_dir * dy_dir).sqrt();
        if len_px < 1e-15 {
            return 0.0;
        }
        let ux = dx_dir / len_px;
        let uy = dy_dir / len_px;
        let dx_mm = ux * self.h_pixel_size_mm;
        let dy_mm = uy * self.v_pixel_size_mm;
        (dx_mm * dx_mm + dy_mm * dy_mm).sqrt()
    }
}

impl Default for PixelCalibration {
    fn default() -> Self {
        Self::UNCALIBRATED
    }
}

// ── Calibrated result wrappers ──────────────────────────────────────────────

/// Calibrated caliper pair result (pixel + mm).
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CalibratedDistance {
    pub distance_px: f64,
    pub distance_mm: f64,
}

/// Calibrated diameter result (pixel + mm).
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CalibratedDiameter {
    pub diameter_px: f64,
    pub diameter_mm: f64,
    pub radius_px: f64,
    pub radius_mm: f64,
}

/// Calibrated radius result (pixel + mm).
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CalibratedRadius {
    pub radius_px: f64,
    pub radius_mm: f64,
}

/// Calibrated chamfer result (pixel + mm, angles in both pixel-space and real-space).
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CalibratedChamfer {
    pub chamfer_width_px: f64,
    pub chamfer_width_mm: f64,
    /// Angle between chamfer and surface A in pixel space (degrees).
    pub angle_a_px_deg: f64,
    /// Angle between chamfer and surface A in real space (degrees).
    pub angle_a_mm_deg: f64,
    pub angle_b_px_deg: f64,
    pub angle_b_mm_deg: f64,
}

/// Calibrated thread pitch result (pixel + mm).
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CalibratedPitch {
    pub pitch_px: f64,
    pub pitch_mm: f64,
    pub std_dev_px: f64,
    pub std_dev_mm: f64,
}
