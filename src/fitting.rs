/// Geometric fitting algorithms: line fitting and circle fitting (Taubin method).
///
/// These are the numerical workhorses behind the gauge measurements.

use crate::error::{MetrologyError, MetrologyResult};
use crate::geometry::{Circle2D, Line2D, Point2D, Vec2D};

// ── Line fitting (total least squares) ──────────────────────────────────────

/// Result of a line fit.
#[derive(Debug, Clone, Copy)]
pub struct LineFitResult {
    pub line: Line2D,
    /// RMS residual (orthogonal distance from points to the line).
    pub rms_error: f64,
}

/// Fit a line to 2D points using total least squares (PCA / eigenvector of covariance).
///
/// Requires at least 2 points.
pub fn fit_line(points: &[Point2D]) -> MetrologyResult<LineFitResult> {
    let n = points.len();
    if n < 2 {
        return Err(MetrologyError::InsufficientData { needed: 2, got: n });
    }

    // Centroid
    let mut cx = 0.0;
    let mut cy = 0.0;
    for p in points {
        cx += p.x;
        cy += p.y;
    }
    cx /= n as f64;
    cy /= n as f64;

    // Covariance matrix elements
    let mut sxx = 0.0;
    let mut sxy = 0.0;
    let mut syy = 0.0;
    for p in points {
        let dx = p.x - cx;
        let dy = p.y - cy;
        sxx += dx * dx;
        sxy += dx * dy;
        syy += dy * dy;
    }

    // Eigenvector of largest eigenvalue of [[sxx, sxy], [sxy, syy]]
    // Using the analytic 2x2 eigenvector formula
    let diff = sxx - syy;
    let theta = 0.5 * sxy.atan2(diff * 0.5);

    let direction = Vec2D::new(theta.cos(), theta.sin());
    let origin = Point2D::new(cx, cy);
    let line = Line2D { origin, direction };

    // RMS residual
    let mut sum_sq = 0.0;
    for p in points {
        let d = line.signed_distance(p);
        sum_sq += d * d;
    }
    let rms_error = (sum_sq / n as f64).sqrt();

    Ok(LineFitResult { line, rms_error })
}

// ── Circle fitting (Taubin method) ──────────────────────────────────────────

/// Result of a circle fit.
#[derive(Debug, Clone, Copy)]
pub struct CircleFitResult {
    pub circle: Circle2D,
    /// RMS residual (distance from points to the fitted circle).
    pub rms_error: f64,
}

/// Fit a circle to 2D points using the **Taubin method**.
///
/// This is an algebraic fit that minimizes the Taubin approximation of the
/// geometric distance. It is numerically more stable than the Kåsa method
/// and does not exhibit the systematic bias towards larger circles.
///
/// Requires at least 3 non-collinear points.
///
/// Reference: G. Taubin, "Estimation of Planar Curves, Surfaces and Nonplanar
/// Space Curves Defined by Implicit Equations, with Applications to Edge and
/// Range Image Segmentation", IEEE TPAMI 13(11), 1991.
pub fn fit_circle_taubin(points: &[Point2D]) -> MetrologyResult<CircleFitResult> {
    let n = points.len();
    if n < 3 {
        return Err(MetrologyError::InsufficientData { needed: 3, got: n });
    }

    // Center the data for numerical stability
    let mut mx = 0.0;
    let mut my = 0.0;
    for p in points {
        mx += p.x;
        my += p.y;
    }
    mx /= n as f64;
    my /= n as f64;

    // Compute moments
    let mut mxx = 0.0;
    let mut myy = 0.0;
    let mut mxy = 0.0;
    let mut mxz = 0.0;
    let mut myz = 0.0;
    let mut mzz = 0.0;

    for p in points {
        let xi = p.x - mx;
        let yi = p.y - my;
        let zi = xi * xi + yi * yi;
        mxx += xi * xi;
        myy += yi * yi;
        mxy += xi * yi;
        mxz += xi * zi;
        myz += yi * zi;
        mzz += zi * zi;
    }
    let nf = n as f64;
    mxx /= nf;
    myy /= nf;
    mxy /= nf;
    mxz /= nf;
    myz /= nf;
    mzz /= nf;

    // Taubin's constraint matrix coefficients
    let mz = mxx + myy; // mean of z_i
    // Solve the Taubin system via the characteristic polynomial approach.
    // We need the smallest eigenvalue of the generalized eigenproblem M*a = η*B*a.
    //
    // M = | Mzz  Mxz  Myz |     B = | 4*Mz  2*Mx_bar  2*My_bar |
    //     | Mxz  Mxx  Mxy |         | 2*Mx   1        0        |
    //     | Myz  Mxy  Myy |         | 2*My   0        1        |
    //
    // For the centered data (mx_bar = my_bar = 0 after centering), B simplifies.

    // With centered data, the Taubin solution simplifies to:
    // (Mzz - Mz^2) * A + Mxz * B + Myz * C = eta * (4*Mz*A)
    // etc.
    // We use the Newton's method approach to find eta.

    // Coefficients for the 3x3 system (centered, so means are zero):
    // Scatter matrix N:
    let n11 = mzz - mz * mz;
    let n12 = mxz;
    let n13 = myz;
    let n22 = mxx;
    let n23 = mxy;
    let n33 = myy;

    // Constraint matrix (Taubin): diag(4*Mz, 1, 1)
    // Generalized eigenproblem: N * [A,B,C]^T = eta * diag(4*Mz, 1, 1) * [A,B,C]^T

    // Newton iteration to find eta (smallest positive generalized eigenvalue)
    let mut eta = 0.0;
    for _ in 0..50 {
        // Shifted matrix: N - eta * B
        let a11 = n11 - eta * 4.0 * mz;
        let a12 = n12;
        let a13 = n13;
        let a22 = n22 - eta;
        let a23 = n23;
        let a33 = n33 - eta;

        // Determinant
        let det = a11 * (a22 * a33 - a23 * a23) - a12 * (a12 * a33 - a23 * a13)
            + a13 * (a12 * a23 - a22 * a13);

        // Derivative of determinant w.r.t. eta (via adjugate diagonal)
        let adj11 = a22 * a33 - a23 * a23;
        let adj22 = a11 * a33 - a13 * a13;
        let adj33 = a11 * a22 - a12 * a12;

        let d_det2 = -(4.0 * mz) * adj11 - adj22 - adj33;

        if d_det2.abs() < 1e-30 {
            break;
        }

        let delta = det / d_det2;
        eta -= delta;

        if delta.abs() < 1e-12 {
            break;
        }
    }

    // Solve (N - eta*B) * [A,B,C]^T = 0 for the null vector
    let _a11 = n11 - eta * 4.0 * mz;
    let a12 = n12;
    let a13 = n13;
    let a22 = n22 - eta;
    let a23 = n23;
    let a33 = n33 - eta;

    // Find the null vector via the row with the largest absolute cross product
    // Use cofactors of the first row
    let c1 = a22 * a33 - a23 * a23;
    let c2 = -(a12 * a33 - a13 * a23);
    let c3 = a12 * a23 - a13 * a22;

    let norm = (c1 * c1 + c2 * c2 + c3 * c3).sqrt();
    if norm < 1e-15 {
        return Err(MetrologyError::DegenerateGeometry(
            "collinear points, cannot fit circle",
        ));
    }

    let a = c1 / norm;
    let b = c2 / norm;
    let c = c3 / norm;

    if a.abs() < 1e-15 {
        return Err(MetrologyError::DegenerateGeometry(
            "degenerate circle fit (A ≈ 0)",
        ));
    }

    // Circle parameters from algebraic coefficients:
    // A*(x^2+y^2) + B*x + C*y + D = 0
    // Center = (-B/(2A), -C/(2A)), Radius = sqrt(B^2+C^2 - 4AD) / (2|A|)
    let center_x = -b / (2.0 * a) + mx;
    let center_y = -c / (2.0 * a) + my;

    // Compute D from the constraint: sum of A*zi + B*xi + C*yi + D = 0
    // With centered data: D = -(A*Mz + 0 + 0) => D = -A*Mz  (since mean xi = mean yi = 0)
    let d = -a * mz;

    let r_sq = (b * b + c * c - 4.0 * a * d) / (4.0 * a * a);
    if r_sq < 0.0 {
        return Err(MetrologyError::DegenerateGeometry("negative radius squared"));
    }
    let radius = r_sq.sqrt();

    let center = Point2D::new(center_x, center_y);
    let circle = Circle2D { center, radius };

    // RMS residual
    let mut sum_sq = 0.0;
    for p in points {
        let d = center.distance_to(p) - radius;
        sum_sq += d * d;
    }
    let rms_error = (sum_sq / n as f64).sqrt();

    Ok(CircleFitResult { circle, rms_error })
}

/// Iterative geometric circle fit (Levenberg-Marquardt style).
///
/// Starts from an initial estimate (e.g. from Taubin) and refines by minimizing
/// the sum of squared geometric distances. More accurate than algebraic fits,
/// especially on short arcs.
pub fn fit_circle_geometric(
    points: &[Point2D],
    initial: Circle2D,
    max_iter: u32,
    tolerance: f64,
) -> MetrologyResult<CircleFitResult> {
    let n = points.len();
    if n < 3 {
        return Err(MetrologyError::InsufficientData { needed: 3, got: n });
    }

    let mut cx = initial.center.x;
    let mut cy = initial.center.y;
    let mut r = initial.radius;
    let mut lambda = 0.001_f64;

    for _ in 0..max_iter {
        // Compute Jacobian and residuals
        let mut jtj = [[0.0_f64; 3]; 3];
        let mut jtr = [0.0_f64; 3];
        let mut sum_sq = 0.0;

        for p in points {
            let dx = p.x - cx;
            let dy = p.y - cy;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist < 1e-15 {
                continue;
            }
            let residual = dist - r;
            sum_sq += residual * residual;

            // Jacobian row: [d(residual)/d(cx), d(residual)/d(cy), d(residual)/d(r)]
            let j = [-dx / dist, -dy / dist, -1.0];

            for row in 0..3 {
                for col in 0..3 {
                    jtj[row][col] += j[row] * j[col];
                }
                jtr[row] += j[row] * residual;
            }
        }

        // Damping (LM)
        for i in 0..3 {
            jtj[i][i] *= 1.0 + lambda;
        }

        // Solve 3x3 system via Cramer's rule
        let det = jtj[0][0] * (jtj[1][1] * jtj[2][2] - jtj[1][2] * jtj[2][1])
            - jtj[0][1] * (jtj[1][0] * jtj[2][2] - jtj[1][2] * jtj[2][0])
            + jtj[0][2] * (jtj[1][0] * jtj[2][1] - jtj[1][1] * jtj[2][0]);

        if det.abs() < 1e-30 {
            return Err(MetrologyError::FittingDidNotConverge);
        }

        let inv_det = 1.0 / det;
        let delta = [
            inv_det
                * (jtr[0] * (jtj[1][1] * jtj[2][2] - jtj[1][2] * jtj[2][1])
                    - jtr[1] * (jtj[0][1] * jtj[2][2] - jtj[0][2] * jtj[2][1])
                    + jtr[2] * (jtj[0][1] * jtj[1][2] - jtj[0][2] * jtj[1][1])),
            inv_det
                * (jtj[0][0] * (jtr[1] * jtj[2][2] - jtr[2] * jtj[2][1])
                    - jtr[0] * (jtj[1][0] * jtj[2][2] - jtj[1][2] * jtj[2][0])
                    + jtj[0][2] * (jtj[1][0] * jtr[2] - jtr[1] * jtj[2][0])),
            inv_det
                * (jtj[0][0] * (jtj[1][1] * jtr[2] - jtr[1] * jtj[2][1])
                    - jtj[0][1] * (jtj[1][0] * jtr[2] - jtr[1] * jtj[2][0])
                    + jtr[0] * (jtj[1][0] * jtj[2][1] - jtj[1][1] * jtj[2][0])),
        ];

        let new_cx = cx - delta[0];
        let new_cy = cy - delta[1];
        let new_r = (r - delta[2]).abs();

        // Check new cost
        let mut new_sum_sq = 0.0;
        for p in points {
            let d = Point2D::new(new_cx, new_cy).distance_to(p) - new_r;
            new_sum_sq += d * d;
        }

        if new_sum_sq < sum_sq {
            cx = new_cx;
            cy = new_cy;
            r = new_r;
            lambda *= 0.1;

            if (sum_sq - new_sum_sq).abs() < tolerance * tolerance * n as f64 {
                break;
            }
        } else {
            lambda *= 10.0;
        }
    }

    let center = Point2D::new(cx, cy);
    let mut sum_sq = 0.0;
    for p in points {
        let d = center.distance_to(p) - r;
        sum_sq += d * d;
    }

    Ok(CircleFitResult {
        circle: Circle2D { center, radius: r },
        rms_error: (sum_sq / n as f64).sqrt(),
    })
}
