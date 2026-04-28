/// Minimal grayscale image wrapper — no external image library dependency.
///
/// The library operates on raw grayscale pixel data. Consumers are responsible
/// for decoding images into this format before passing them in.

use crate::error::{MetrologyError, MetrologyResult};

/// 8-bit grayscale image. Row-major, no padding.
#[derive(Debug, Clone)]
pub struct GrayImage {
    pub(crate) data: Vec<u8>,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

impl GrayImage {
    /// Create from raw pixel buffer. `data.len()` must equal `width * height`.
    pub fn new(data: Vec<u8>, width: u32, height: u32) -> MetrologyResult<Self> {
        let expected = width as usize * height as usize;
        if expected == 0 || data.len() != expected {
            return Err(MetrologyError::InvalidImageDimensions {
                width,
                height,
                buffer_len: data.len(),
            });
        }
        Ok(Self {
            data,
            width,
            height,
        })
    }

    /// Wrap a borrowed slice as a GrayImage (zero-copy reference).
    /// Caller must guarantee the slice outlives the returned `GrayImageRef`.
    pub fn wrap(data: &[u8], width: u32, height: u32) -> MetrologyResult<GrayImageRef<'_>> {
        let expected = width as usize * height as usize;
        if expected == 0 || data.len() != expected {
            return Err(MetrologyError::InvalidImageDimensions {
                width,
                height,
                buffer_len: data.len(),
            });
        }
        Ok(GrayImageRef {
            data,
            width,
            height,
        })
    }

    #[inline]
    pub fn width(&self) -> u32 {
        self.width
    }

    #[inline]
    pub fn height(&self) -> u32 {
        self.height
    }

    #[inline]
    pub fn pixel(&self, x: u32, y: u32) -> u8 {
        self.data[(y as usize) * (self.width as usize) + (x as usize)]
    }

    /// Bilinear interpolation at sub-pixel coordinates.
    pub fn sample(&self, x: f64, y: f64) -> f64 {
        let x0 = x.floor() as i64;
        let y0 = y.floor() as i64;
        let x1 = x0 + 1;
        let y1 = y0 + 1;

        let w = self.width as i64;
        let h = self.height as i64;

        let get = |cx: i64, cy: i64| -> f64 {
            let cx = cx.clamp(0, w - 1) as usize;
            let cy = cy.clamp(0, h - 1) as usize;
            self.data[cy * (self.width as usize) + cx] as f64
        };

        let fx = x - x0 as f64;
        let fy = y - y0 as f64;

        let v00 = get(x0, y0);
        let v10 = get(x1, y0);
        let v01 = get(x0, y1);
        let v11 = get(x1, y1);

        v00 * (1.0 - fx) * (1.0 - fy)
            + v10 * fx * (1.0 - fy)
            + v01 * (1.0 - fx) * fy
            + v11 * fx * fy
    }

    pub fn as_ref(&self) -> GrayImageRef<'_> {
        GrayImageRef {
            data: &self.data,
            width: self.width,
            height: self.height,
        }
    }
}

/// Borrowed view of a grayscale image (zero-copy).
#[derive(Debug, Clone, Copy)]
pub struct GrayImageRef<'a> {
    pub(crate) data: &'a [u8],
    pub(crate) width: u32,
    pub(crate) height: u32,
}

impl<'a> GrayImageRef<'a> {
    #[inline]
    pub fn width(&self) -> u32 {
        self.width
    }

    #[inline]
    pub fn height(&self) -> u32 {
        self.height
    }

    #[inline]
    pub fn pixel(&self, x: u32, y: u32) -> u8 {
        self.data[(y as usize) * (self.width as usize) + (x as usize)]
    }

    pub fn sample(&self, x: f64, y: f64) -> f64 {
        let x0 = x.floor() as i64;
        let y0 = y.floor() as i64;
        let x1 = x0 + 1;
        let y1 = y0 + 1;

        let w = self.width as i64;
        let h = self.height as i64;

        let get = |cx: i64, cy: i64| -> f64 {
            let cx = cx.clamp(0, w - 1) as usize;
            let cy = cy.clamp(0, h - 1) as usize;
            self.data[cy * (self.width as usize) + cx] as f64
        };

        let fx = x - x0 as f64;
        let fy = y - y0 as f64;

        let v00 = get(x0, y0);
        let v10 = get(x1, y0);
        let v01 = get(x0, y1);
        let v11 = get(x1, y1);

        v00 * (1.0 - fx) * (1.0 - fy)
            + v10 * fx * (1.0 - fy)
            + v01 * (1.0 - fx) * fy
            + v11 * fx * fy
    }
}
