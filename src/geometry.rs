/// Basic 2D geometry primitives used throughout the metrology library.
///
/// All types are `Copy` + `Send` + `Sync` — safe for FFI and parallel use.

/// A 2D point in image coordinates (sub-pixel precision).
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct Point2D {
    pub x: f64,
    pub y: f64,
}

impl Point2D {
    #[inline]
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    #[inline]
    pub fn distance_to(&self, other: &Point2D) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }
}

/// A 2D vector (direction + magnitude).
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct Vec2D {
    pub dx: f64,
    pub dy: f64,
}

impl Vec2D {
    #[inline]
    pub fn new(dx: f64, dy: f64) -> Self {
        Self { dx, dy }
    }

    #[inline]
    pub fn length(&self) -> f64 {
        (self.dx * self.dx + self.dy * self.dy).sqrt()
    }

    #[inline]
    pub fn normalized(&self) -> Self {
        let len = self.length();
        if len < 1e-15 {
            return Self { dx: 0.0, dy: 0.0 };
        }
        Self {
            dx: self.dx / len,
            dy: self.dy / len,
        }
    }

    #[inline]
    pub fn perpendicular(&self) -> Self {
        Self {
            dx: -self.dy,
            dy: self.dx,
        }
    }

    #[inline]
    pub fn dot(&self, other: &Vec2D) -> f64 {
        self.dx * other.dx + self.dy * other.dy
    }
}

/// A line defined by a point and direction.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct Line2D {
    pub origin: Point2D,
    pub direction: Vec2D,
}

impl Line2D {
    pub fn from_two_points(a: Point2D, b: Point2D) -> Self {
        let dir = Vec2D::new(b.x - a.x, b.y - a.y).normalized();
        Self {
            origin: a,
            direction: dir,
        }
    }

    /// Signed distance from a point to this line.
    /// Positive = left side of direction, negative = right side.
    pub fn signed_distance(&self, p: &Point2D) -> f64 {
        let to_p = Vec2D::new(p.x - self.origin.x, p.y - self.origin.y);
        let n = self.direction.perpendicular();
        n.dot(&to_p)
    }

    /// Intersection with another line. Returns `None` if parallel.
    pub fn intersect(&self, other: &Line2D) -> Option<Point2D> {
        let d = self.direction.dx * other.direction.dy - self.direction.dy * other.direction.dx;
        if d.abs() < 1e-12 {
            return None;
        }
        let dx = other.origin.x - self.origin.x;
        let dy = other.origin.y - self.origin.y;
        let t = (dx * other.direction.dy - dy * other.direction.dx) / d;
        Some(Point2D::new(
            self.origin.x + t * self.direction.dx,
            self.origin.y + t * self.direction.dy,
        ))
    }
}

/// A circle in 2D (center + radius).
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct Circle2D {
    pub center: Point2D,
    pub radius: f64,
}

/// A line segment between two points.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct Segment2D {
    pub start: Point2D,
    pub end: Point2D,
}

impl Segment2D {
    #[inline]
    pub fn length(&self) -> f64 {
        self.start.distance_to(&self.end)
    }

    #[inline]
    pub fn midpoint(&self) -> Point2D {
        Point2D::new(
            (self.start.x + self.end.x) * 0.5,
            (self.start.y + self.end.y) * 0.5,
        )
    }

    pub fn direction(&self) -> Vec2D {
        Vec2D::new(self.end.x - self.start.x, self.end.y - self.start.y).normalized()
    }
}

/// Angle measurement result, always stored in radians internally.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct Angle {
    pub radians: f64,
}

impl Angle {
    #[inline]
    pub fn from_radians(r: f64) -> Self {
        Self { radians: r }
    }

    #[inline]
    pub fn from_degrees(d: f64) -> Self {
        Self {
            radians: d.to_radians(),
        }
    }

    #[inline]
    pub fn degrees(&self) -> f64 {
        self.radians.to_degrees()
    }
}
