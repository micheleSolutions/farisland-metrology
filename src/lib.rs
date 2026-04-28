//! # farisland-metrology
//!
//! Language-agnostic measurement gauges for machine-vision metrology.
//!
//! This library provides five gauge types used in industrial vision-based
//! dimensional measurement:
//!
//! - **Caliper1D** — measure distance between edges along a scan line
//! - **DiameterGauge** — measure diameter of circular features via radial calipers + circle fitting
//! - **ChamferGauge** — measure chamfer angle, width, and height from three surface regions
//! - **RadiusGauge** — measure radius of partial arcs (fillets, rounded corners)
//! - **ThreadPitchGauge** — measure screw thread pitch via peak detection or FFT
//!
//! ## Design principles
//!
//! - **Zero external image dependencies** — operates on raw grayscale pixel buffers
//! - **Sub-pixel precision** — parabolic interpolation on gradient peaks
//! - **Numerically stable fitting** — Taubin circle fit + LM geometric refinement
//! - **FFI-first** — C ABI via `cbindgen`, multi-language bindings via `uniffi`
//! - **No allocator surprises** — all allocations are predictable, no hidden GC
//!
//! ## Intended consumers
//!
//! - Rust (native crate dependency)
//! - Java 22+ via Project Panama / FFM API (consuming C ABI)
//! - Python via PyO3 or uniffi
//! - Kotlin/Swift/Ruby via uniffi

pub mod calibration;
pub mod error;
pub mod fitting;
pub mod gauges;
pub mod geometry;
pub mod image;
pub mod profile;

#[cfg(feature = "ffi-c")]
pub mod ffi;

#[cfg(feature = "ffi-uniffi")]
pub mod uniffi_api;

// Always export uniffi scaffolding when building cdylib
#[cfg(feature = "ffi-uniffi")]
uniffi::setup_scaffolding!();
