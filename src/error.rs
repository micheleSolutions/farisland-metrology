/// Error types for metrology operations.
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum MetrologyError {
    /// Not enough data points to perform fitting.
    InsufficientData { needed: usize, got: usize },
    /// Profile is too short or empty.
    EmptyProfile,
    /// No edge was found matching the criteria.
    NoEdgeFound,
    /// No edge pair was found matching the criteria.
    NoEdgePairFound,
    /// Fitting did not converge within iteration limit.
    FittingDidNotConverge,
    /// Input image dimensions are invalid (zero width/height, mismatched buffer).
    InvalidImageDimensions {
        width: u32,
        height: u32,
        buffer_len: usize,
    },
    /// Scan region extends outside image bounds.
    ScanOutOfBounds,
    /// Numeric overflow or degenerate geometry.
    DegenerateGeometry(&'static str),
}

impl fmt::Display for MetrologyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InsufficientData { needed, got } => {
                write!(f, "insufficient data: need {needed}, got {got}")
            }
            Self::EmptyProfile => write!(f, "profile is empty or too short"),
            Self::NoEdgeFound => write!(f, "no edge found matching criteria"),
            Self::NoEdgePairFound => write!(f, "no edge pair found matching criteria"),
            Self::FittingDidNotConverge => write!(f, "fitting did not converge"),
            Self::InvalidImageDimensions {
                width,
                height,
                buffer_len,
            } => write!(
                f,
                "invalid image: {width}x{height} with buffer length {buffer_len}"
            ),
            Self::ScanOutOfBounds => write!(f, "scan region out of image bounds"),
            Self::DegenerateGeometry(msg) => write!(f, "degenerate geometry: {msg}"),
        }
    }
}

impl std::error::Error for MetrologyError {}

pub type MetrologyResult<T> = Result<T, MetrologyError>;
