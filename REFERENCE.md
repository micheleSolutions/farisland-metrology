# farisland-metrology — API Reference

Language-agnostic measurement gauges for machine-vision metrology.  
Version 0.1.0 — Far Island ecosystem shared library.

---

## Table of Contents

- [Overview](#overview)
- [Architecture](#architecture)
- [Getting Started](#getting-started)
- [Modules](#modules)
  - [geometry — 2D Primitives](#geometry)
  - [image — Grayscale Image](#image)
  - [profile — 1D Profile Extraction & Edge Detection](#profile)
  - [fitting — Geometric Fitting Algorithms](#fitting)
  - [gauges::caliper1d — Caliper1D Gauge](#caliper1d)
  - [gauges::diameter — DiameterGauge](#diametergauge)
  - [gauges::chamfer — ChamferGauge](#chamfergauge)
  - [gauges::radius — RadiusGauge](#radiusgauge)
  - [gauges::thread_pitch — ThreadPitchGauge](#threadpitchgauge)
- [C ABI (FFI)](#c-abi)
- [UniFFI Bindings (Java/Python/Kotlin)](#uniffi-bindings)
- [Algorithms](#algorithms)
- [Performance](#performance)
- [Error Handling](#error-handling)
- [Platform Support](#platform-support)

---

## Overview

`farisland-metrology` provides five gauge types used in industrial vision-based dimensional measurement, designed to replace the inline measurement logic in the legacy `rotatingTable3` plugin:

| Gauge | Measures | Algorithm |
|-------|----------|-----------|
| **Caliper1D** | Edge positions and distances along a scan line | 1D gradient peak detection + parabolic sub-pixel |
| **DiameterGauge** | Diameter of circular features | Radial calipers → Taubin circle fit + LM refinement |
| **ChamferGauge** | Chamfer angle, width, and intersection points | Multi-region edge scan → TLS line fitting |
| **RadiusGauge** | Radius of partial arcs (fillets, corners) | Arc calipers → Taubin + geometric circle fit |
| **ThreadPitchGauge** | Screw thread pitch | Peak detection (crest-to-crest) + DFT frequency analysis |

### Design Principles

- **Zero external image dependencies** — operates on raw `&[u8]` grayscale buffers
- **Sub-pixel precision** — parabolic interpolation on gradient peaks (typical ±0.1 px)
- **Numerically stable fitting** — Taubin circle fit (no Kåsa bias), LM geometric refinement for short arcs
- **FFI-first** — `repr(C)` types, C ABI via cbindgen, multi-language bindings via UniFFI
- **No hidden allocations** — all memory is predictable, no GC interactions

---

## Architecture

```
farisland-metrology/
├── src/
│   ├── lib.rs              # Crate root, module declarations
│   ├── geometry.rs          # Point2D, Vec2D, Line2D, Circle2D, Segment2D, Angle
│   ├── image.rs             # GrayImage (owned), GrayImageRef (borrowed/zero-copy)
│   ├── error.rs             # MetrologyError enum
│   ├── profile.rs           # 1D profile extraction, smoothing, gradient, edge detection
│   ├── fitting.rs           # Line fit (TLS/PCA), circle fit (Taubin + LM)
│   ├── ffi.rs               # C ABI exports (feature: ffi-c)
│   ├── uniffi_api.rs        # UniFFI bindings (feature: ffi-uniffi)
│   └── gauges/
│       ├── mod.rs
│       ├── caliper1d.rs     # Caliper1D gauge
│       ├── diameter.rs      # DiameterGauge
│       ├── chamfer.rs       # ChamferGauge
│       ├── radius.rs        # RadiusGauge
│       └── thread_pitch.rs  # ThreadPitchGauge
├── tests/
│   ├── test_geometry.rs     # 9 tests
│   ├── test_profile.rs      # 8 tests
│   ├── test_fitting.rs      # 8 tests
│   └── test_gauges.rs       # 9 tests (34 total)
├── Cargo.toml
└── build.rs                 # cbindgen header generation + uniffi scaffolding
```

---

## Getting Started

### As a Rust crate dependency

```toml
[dependencies]
farisland-metrology = { path = "../farisland-metrology" }
# or from crates.io when published:
# farisland-metrology = "0.1"
```

```rust
use farisland_metrology::image::GrayImage;
use farisland_metrology::gauges::caliper1d::{Caliper1D, Caliper1DConfig};
use farisland_metrology::geometry::Point2D;

let pixels: Vec<u8> = load_my_image(); // your image loading
let image = GrayImage::new(pixels, 640, 480).unwrap();
let img_ref = image.as_ref();

let config = Caliper1DConfig {
    start: Point2D::new(100.0, 240.0),
    end: Point2D::new(540.0, 240.0),
    scan_width: 5,
    smoothing_sigma: 1.0,
    min_edge_strength: 15.0,
    ..Default::default()
};

let result = Caliper1D::find_edges(&img_ref, &config).unwrap();
for edge in &result.edges {
    println!("Edge at ({:.2}, {:.2}), strength={:.1}", edge.point.x, edge.point.y, edge.strength);
}
```

### Via C ABI (Java Panama / C / C++)

```c
#include "farisland_metrology.h"

// Load your image pixels into a buffer
uint8_t *pixels = ...;
GrayImage *img = fm_image_create(pixels, 640, 480);

FmCaliper1DConfig cfg = {
    .start_x = 100.0, .start_y = 240.0,
    .end_x = 540.0,   .end_y = 240.0,
    .scan_width = 5,
    .smoothing_sigma = 1.0,
    .min_edge_strength = 15.0,
    .polarity = 2,  // Any
    .step = 1.0
};

FmEdgeArray result = fm_caliper1d_find_edges(img, &cfg);
if (result.status == Ok) {
    for (uint32_t i = 0; i < result.count; i++) {
        printf("Edge at (%.2f, %.2f)\n", result.edges[i].x, result.edges[i].y);
    }
    fm_edges_free(result.edges, result.count);
}
fm_image_free(img);
```

### Via Java 22+ (Project Panama / FFM API)

```java
// Generate Java bindings from farisland_metrology.h using jextract:
// jextract --output src -t com.farisland.metrology farisland_metrology.h

import com.farisland.metrology.*;
import java.lang.foreign.*;

try (var arena = Arena.ofConfined()) {
    var img = fm_image_create(pixelSegment, 640, 480);
    var cfg = FmCaliper1DConfig.allocate(arena);
    FmCaliper1DConfig.start_x(cfg, 100.0);
    FmCaliper1DConfig.end_x(cfg, 540.0);
    // ... set other fields
    var result = fm_caliper1d_find_edges(img, cfg);
    // ... read result
    fm_image_free(img);
}
```

---

## Modules

### geometry

2D geometry primitives. All types are `Copy`, `repr(C)`, and `Send + Sync`.

| Type | Fields | Description |
|------|--------|-------------|
| `Point2D` | `x: f64, y: f64` | 2D point (sub-pixel precision) |
| `Vec2D` | `dx: f64, dy: f64` | 2D vector with `normalized()`, `perpendicular()`, `dot()`, `length()` |
| `Line2D` | `origin: Point2D, direction: Vec2D` | Line defined by point + unit direction |
| `Circle2D` | `center: Point2D, radius: f64` | Circle |
| `Segment2D` | `start: Point2D, end: Point2D` | Line segment with `length()`, `midpoint()`, `direction()` |
| `Angle` | `radians: f64` | Angle with `from_degrees()`, `from_radians()`, `degrees()` |

**Key methods:**

- `Point2D::distance_to(&self, other) → f64`
- `Line2D::from_two_points(a, b) → Line2D`
- `Line2D::signed_distance(&self, point) → f64` — positive = left of direction
- `Line2D::intersect(&self, other) → Option<Point2D>` — `None` if parallel

---

### image

Minimal grayscale image wrapper with zero external dependencies.

| Type | Ownership | Description |
|------|-----------|-------------|
| `GrayImage` | Owned (`Vec<u8>`) | Owns its pixel data |
| `GrayImageRef<'a>` | Borrowed (`&[u8]`) | Zero-copy view |

**Construction:**

```rust
// Owned
let img = GrayImage::new(pixels_vec, width, height)?;

// Borrowed (zero-copy)
let img_ref = GrayImage::wrap(&pixel_slice, width, height)?;

// Convert owned → borrowed
let img_ref = img.as_ref();
```

**Pixel access:**

- `pixel(x, y) → u8` — integer coordinates
- `sample(x, y) → f64` — bilinear interpolation at sub-pixel coordinates

---

### profile

1D brightness profile extraction and processing — the signal processing backbone.

#### `extract_profile(image, start, end, scan_width, step) → Profile1D`

Extracts a 1D brightness profile along a line segment.

| Parameter | Type | Description |
|-----------|------|-------------|
| `image` | `&GrayImageRef` | Source image |
| `start` | `Point2D` | Start of scan line |
| `end` | `Point2D` | End of scan line |
| `scan_width` | `u32` | Number of parallel lines to average (1 = single line) |
| `step` | `f64` | Sampling step in pixels (1.0 = every pixel) |

When `scan_width > 1`, multiple parallel lines are sampled orthogonally and averaged to reduce noise. The profile uses bilinear interpolation for sub-pixel sampling.

#### `smooth_gaussian(profile, sigma) → Vec<f64>`

Applies Gaussian smoothing with kernel radius `3σ`. Sigma is in sample units.

#### `gradient(profile) → Vec<f64>`

Central finite differences. Returns first derivative of the profile.

#### `detect_edges(gradient, min_magnitude, polarity) → Vec<Edge1D>`

Detects edges as local maxima in the absolute gradient, with **parabolic sub-pixel refinement**.

| Parameter | Type | Description |
|-----------|------|-------------|
| `min_magnitude` | `f64` | Minimum gradient magnitude to accept |
| `polarity` | `EdgePolarity` | `DarkToBright`, `BrightToDark`, or `Any` |

Each `Edge1D` contains:
- `position: f64` — sub-pixel position in sample units
- `strength: f64` — interpolated gradient magnitude
- `polarity: EdgePolarity` — transition direction

#### `find_edge_pairs(edges, min_width, max_width) → Vec<EdgePair1D>`

Groups edges into opposite-polarity pairs. Greedy: each leading edge matches the first valid trailing edge.

---

### fitting

#### `fit_line(points) → LineFitResult`

Total Least Squares line fit via PCA (eigenvector of 2×2 covariance matrix).
Requires ≥ 2 points. Returns `line` + `rms_error`.

#### `fit_circle_taubin(points) → CircleFitResult`

**Taubin algebraic circle fit.** Numerically more stable than Kåsa, no systematic bias toward larger circles. Uses Newton iteration on the generalized eigenvalue problem.
Requires ≥ 3 non-collinear points.

Reference: G. Taubin, "Estimation of Planar Curves, Surfaces and Nonplanar Space Curves", IEEE TPAMI 13(11), 1991.

#### `fit_circle_geometric(points, initial, max_iter, tolerance) → CircleFitResult`

Iterative geometric circle fit (Levenberg-Marquardt). Minimizes sum of squared geometric distances (point-to-circle). More accurate than algebraic fits on short arcs (< 90°).

Starts from an initial estimate (typically from `fit_circle_taubin`).

---

### caliper1d

The fundamental measurement building block.

#### `Caliper1D::find_edges(image, config) → Caliper1DEdgeResult`

Detects all edges along the scan line.

#### `Caliper1D::find_strongest_edge(image, config) → Caliper1DEdge`

Returns the single edge with the highest gradient magnitude.

#### `Caliper1D::find_pairs(image, config, min_width_px, max_width_px) → Caliper1DPairResult`

Finds edge pairs (stripe/gap measurements).

#### `Caliper1D::measure_width(image, config, min_width_px, max_width_px) → f64`

Convenience: returns the distance of the first edge pair found.

**Caliper1DConfig:**

| Field | Default | Description |
|-------|---------|-------------|
| `start` | (0,0) | Scan line start point |
| `end` | (100,0) | Scan line end point |
| `scan_width` | 5 | Parallel lines to average |
| `smoothing_sigma` | 1.0 | Gaussian sigma (sample units) |
| `min_edge_strength` | 10.0 | Minimum gradient magnitude |
| `polarity` | `Any` | Edge direction filter |
| `step` | 1.0 | Sampling step (pixels) |

---

### DiameterGauge

Measures diameter of circular features using radial caliper scans + circle fitting.

#### `DiameterGauge::measure(image, config) → DiameterResult`

**Algorithm:**
1. Places `num_calipers` scan lines radially from the nominal center
2. Each scan line searches for the strongest edge between `nominal_radius ± search_margin`
3. Collects all detected edge points
4. Fits circle via Taubin + optional geometric refinement
5. Returns `diameter = 2 × fitted_radius`

**DiameterGaugeConfig:**

| Field | Default | Description |
|-------|---------|-------------|
| `nominal_center` | (0,0) | Expected circle center |
| `nominal_radius` | 100.0 | Expected radius |
| `search_margin` | 20.0 | Extra search distance beyond nominal |
| `num_calipers` | 36 | Number of radial scans (evenly spaced) |
| `scan_width` | 5 | Averaging width per caliper |
| `smoothing_sigma` | 1.0 | Gaussian sigma |
| `min_edge_strength` | 10.0 | Minimum edge strength |
| `polarity` | `Any` | Edge polarity filter |
| `geometric_refinement` | true | Use LM after Taubin |
| `max_iterations` | 50 | LM iteration limit |

**DiameterResult:**

| Field | Type | Description |
|-------|------|-------------|
| `circle` | `Circle2D` | Fitted circle (center + radius) |
| `diameter` | `f64` | 2 × radius |
| `rms_error` | `f64` | RMS distance of points to fitted circle |
| `num_points` | `usize` | Number of edge points used |
| `edge_points` | `Vec<Point2D>` | All detected contour points |

---

### ChamferGauge

Measures chamfer geometry from three scan regions (surface A, chamfer, surface B).

#### `ChamferGauge::measure(image, config) → ChamferResult`

**Algorithm:**
1. For each of the three regions, runs parallel caliper scans to detect edge points
2. Fits a line (TLS) to each set of points
3. Computes intersection points and angles

**ChamferResult:**

| Field | Type | Description |
|-------|------|-------------|
| `line_a`, `line_chamfer`, `line_b` | `Line2D` | Fitted lines |
| `angle_a` | `Angle` | Acute angle between chamfer and surface A |
| `angle_b` | `Angle` | Acute angle between chamfer and surface B |
| `chamfer_width` | `f64` | Distance between the two intersection points |
| `intersection_a`, `intersection_b` | `Point2D` | Intersection points |
| `max_rms_error` | `f64` | Worst RMS among the three fits |
| `points_per_surface` | `[usize; 3]` | Point counts |

---

### RadiusGauge

Measures radius of partial circular arcs (fillets, rounded corners).

#### `RadiusGauge::measure(image, config) → RadiusResult`

Same approach as DiameterGauge but scans only the arc between `start_angle` and `end_angle`. Geometric refinement is **strongly recommended** for arcs < 90°.

**RadiusGaugeConfig extras vs DiameterGaugeConfig:**

| Field | Default | Description |
|-------|---------|-------------|
| `start_angle` | 0.0 | Arc start (radians, 0 = +X axis) |
| `end_angle` | π/2 | Arc end (radians, counterclockwise) |

**RadiusResult includes:**
- `radius: f64` — fitted radius
- `arc_span: Angle` — actual angular coverage from detected points

---

### ThreadPitchGauge

Measures screw thread pitch from silhouette profile images. Two methods available.

#### `ThreadPitchGauge::measure_by_peaks(image, config) → ThreadPitchResult`

**Peak detection method:**
1. Extracts brightness profile along the thread axis
2. Gaussian smoothing → find local maxima (crests) with prominence filter
3. Sub-pixel refinement on each crest
4. Computes consecutive crest-to-crest distances
5. Returns mean pitch ± standard deviation

Best for: measuring individual pitch variations, detecting single-pitch errors.

**ThreadPitchResult:**

| Field | Type | Description |
|-------|------|-------------|
| `mean_pitch_px` | `f64` | Mean pitch in pixels |
| `std_dev_px` | `f64` | Standard deviation |
| `pitches` | `Vec<f64>` | Individual pitch values |
| `crest_positions` | `Vec<f64>` | Crest positions along profile |
| `thread_count` | `usize` | Number of complete threads |

#### `ThreadPitchGauge::measure_by_fft(image, config) → ThreadPitchFftResult`

**FFT method:**
1. Extracts and centers the profile (removes DC)
2. Computes DFT magnitude spectrum
3. Finds dominant frequency in the expected pitch range
4. Parabolic interpolation on the spectrum for sub-bin precision
5. Converts `pitch = N × step / frequency_bin`

Best for: robust measurement under noise, partial occlusion, non-uniform illumination.

**ThreadPitchFftResult:**

| Field | Type | Description |
|-------|------|-------------|
| `pitch_px` | `f64` | Dominant pitch in pixels |
| `dominant_magnitude` | `f64` | Confidence indicator |
| `secondary_pitch_px` | `Option<f64>` | Second candidate (quality check) |

**ThreadPitchGaugeConfig:**

| Field | Default | Description |
|-------|---------|-------------|
| `start`, `end` | (0,0)→(500,0) | Scan line along thread axis |
| `scan_width` | 10 | Averaging width |
| `smoothing_sigma` | 2.0 | Gaussian sigma |
| `min_peak_prominence` | 5.0 | Minimum crest prominence |
| `expected_pitch_range` | (5, 100) | Valid pitch range in pixels |
| `step` | 1.0 | Sampling step |

---

## C ABI

Enable with `--features ffi-c`. The build generates `farisland_metrology.h` via cbindgen.

### Naming convention

All functions: `fm_<module>_<action>`. All types: `Fm<TypeName>`.

### Memory ownership

| Pattern | Rule |
|---------|------|
| Returns `*mut T` | Caller owns. Free via corresponding `fm_*_free()`. |
| Takes `*const T` | Borrows. Caller retains ownership. |

### Available C functions

```c
// Image lifecycle
GrayImage* fm_image_create(const uint8_t* data, uint32_t width, uint32_t height);
void       fm_image_free(GrayImage* img);

// Caliper1D
FmEdgeArray       fm_caliper1d_find_edges(const GrayImage*, const FmCaliper1DConfig*);
void              fm_edges_free(FmEdge* edges, uint32_t count);

// Diameter
FmDiameterResult  fm_diameter_measure(const GrayImage*, const FmDiameterConfig*);

// Thread pitch
FmThreadPitchResult fm_thread_pitch_measure(const GrayImage*, const FmThreadPitchConfig*);
```

### Status codes

| Code | Value | Meaning |
|------|-------|---------|
| `Ok` | 0 | Success |
| `InsufficientData` | 1 | Not enough points for fitting |
| `EmptyProfile` | 2 | Profile too short |
| `NoEdgeFound` | 3 | No edge matched criteria |
| `NoEdgePairFound` | 4 | No valid edge pair found |
| `FittingDidNotConverge` | 5 | Iterative fit did not converge |
| `InvalidImageDimensions` | 6 | Buffer size mismatch |
| `ScanOutOfBounds` | 7 | Scan region outside image |
| `DegenerateGeometry` | 8 | Collinear points, parallel lines, etc. |
| `NullPointer` | 9 | Null argument passed |

---

## UniFFI Bindings

Enable with `--features ffi-uniffi`. Provides a high-level `Metrology` object usable from Java, Kotlin, Python, Swift, and Ruby.

```python
# Python example (after generating bindings with uniffi-bindgen)
from farisland_metrology import Metrology

m = Metrology(pixel_bytes, 640, 480)
edges = m.find_edges(100.0, 240.0, 540.0, 240.0, 5, 1.0, 15.0)
result = m.measure_diameter(320.0, 240.0, 100.0, 20.0, 36, 10.0)
print(f"Diameter: {result.diameter:.2f} px")
```

---

## Algorithms

### Sub-pixel edge detection

1. Extract brightness profile via bilinear interpolation along scan path
2. Average across `scan_width` parallel lines (noise reduction)
3. Gaussian smoothing (σ configurable)
4. Central-difference gradient
5. Local maxima in |gradient| above threshold
6. **Parabolic interpolation** on the 3-point neighborhood of each peak:
   ```
   offset = 0.5 × (g[i-1] - g[i+1]) / (g[i-1] - 2×g[i] + g[i+1])
   ```
   Typical precision: ±0.1 pixel.

### Taubin circle fit

Algebraic fit minimizing the Taubin approximation of geometric distance. Solved via Newton iteration on the generalized eigenvalue problem with the constraint matrix `diag(4Mz, 1, 1)`. Numerically centered for stability. No systematic bias toward larger circles (unlike Kåsa).

### Levenberg-Marquardt geometric circle fit

Iterative refinement minimizing `Σ(distance_i - r)²`. Uses 3×3 Cramer solve with adaptive damping. Converges in 5-20 iterations for typical input. Essential for accurate fitting on short arcs (< 90°).

### DFT thread pitch

Real-input DFT restricted to the frequency band corresponding to the expected pitch range. Parabolic sub-bin interpolation on the magnitude spectrum. O(N × B) where B is the number of frequency bins in range (typically B << N/2).

---

## Performance

All measurements are designed for the hot path (< 1 µs FFI overhead target).

| Operation | Typical time | Notes |
|-----------|-------------|-------|
| Profile extraction (500 samples) | ~5 µs | Dominated by bilinear sampling |
| Gaussian smoothing (500 samples, σ=1) | ~2 µs | Kernel radius 3 |
| Edge detection (500 samples) | ~1 µs | Single pass |
| Taubin circle fit (36 points) | ~1 µs | Analytic, no iteration |
| LM geometric fit (36 points, 20 iter) | ~5 µs | 3×3 solve per iteration |
| Full diameter measurement (36 calipers) | ~200 µs | End-to-end |
| Thread pitch FFT (500 samples, 50 bins) | ~50 µs | Partial DFT |

Release build with LTO. Measured on x86_64. No SIMD specialization yet.

---

## Error Handling

All public functions return `MetrologyResult<T>` (Rust) or `FmStatus` (C ABI).

The library never panics on valid input. Invalid input (null pointers, mismatched buffers) returns error status codes rather than UB.

```rust
use farisland_metrology::error::{MetrologyError, MetrologyResult};

match Caliper1D::find_edges(&img, &config) {
    Ok(result) => { /* use result */ }
    Err(MetrologyError::NoEdgeFound) => { /* handle gracefully */ }
    Err(e) => { eprintln!("Measurement failed: {e}"); }
}
```

---

## Platform Support

| Platform | Status | Artifact |
|----------|--------|----------|
| Linux x86_64 | ✅ Built + tested | `.so` + `.a` + `.h` |
| Windows x86_64 | ✅ Built + tested | `.dll` + `.lib` + `.h` |
| Linux ARM64 (Jetson Orin) | 🔜 Planned | `.so` + `.a` + `.h` |
| macOS (Apple Silicon) | 🔜 Planned | `.dylib` + `.a` + `.h` |

### Build from source

```bash
# Library only (Rust consumers)
cargo build --release

# With C header generation
cargo build --release --features ffi-c

# With UniFFI bindings
cargo build --release --features ffi-uniffi

# Run tests
cargo test
```

### Installed layout (Windows)

```
C:\DEV\MEDUSA\lib\farisland-metrology\
├── include\
│   └── farisland_metrology.h      # C header (cbindgen)
├── bin\
│   └── farisland_metrology.dll    # Dynamic library (138 KB)
└── lib\
    ├── farisland_metrology.dll.lib # Import library
    └── farisland_metrology.lib     # Static library
```

Source code at `C:\DEV\MEDUSA\farisland-metrology\`.
