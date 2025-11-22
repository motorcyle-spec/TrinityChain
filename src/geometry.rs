//! Core geometric primitives and triangle logic for TrinityChain.
//! Defines the Point and Triangle structs, subdivision logic, and validation.

use serde::{Serialize, Deserialize};
use sha2::{Digest, Sha256};
use crate::blockchain::Sha256Hash;

/// Coordinate type for high-precision geometric calculations.
pub type Coord = f64;
/// Tolerance for floating point comparisons to check for degeneracy/equality.
const GEOMETRIC_TOLERANCE: Coord = 1e-9; 

// ----------------------------------------------------------------------------
// 1.4 Coordinate System: Point
// ----------------------------------------------------------------------------

/// Represents a 2D point with high-precision coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: Coord,
    pub y: Coord,
}

impl Point {
    /// Maximum allowed coordinate value to prevent overflow/precision issues
    pub const MAX_COORDINATE: Coord = 1e10;

    /// Creates a new Point.
    /// Note: Does not validate bounds - use is_valid() to check if coordinates are within acceptable ranges.
    #[inline]
    pub fn new(x: Coord, y: Coord) -> Self {
        Point { x, y }
    }

    /// Validates that the point has finite coordinates within reasonable bounds
    pub fn is_valid(&self) -> bool {
        self.x.is_finite() &&
        self.y.is_finite() &&
        self.x.abs() < Self::MAX_COORDINATE &&
        self.y.abs() < Self::MAX_COORDINATE
    }

    /// Calculates the midpoint between this point and another.
    /// Optimized for inline computation.
    #[inline]
    pub fn midpoint(&self, other: &Point) -> Point {
        Point::new(
            (self.x + other.x) * 0.5,
            (self.y + other.y) * 0.5,
        )
    }

    /// Calculates a simple cryptographic hash of the point data.
    /// Optimized to use direct byte conversion instead of string formatting.
    /// Uses a pre-allocated buffer for better performance.
    #[inline]
    pub fn hash(&self) -> Sha256Hash {
        let mut hasher = Sha256::new();
        hasher.update(self.x.to_le_bytes());
        hasher.update(self.y.to_le_bytes());
        hasher.finalize().into()
    }

    pub fn hash_str(&self) -> String {
        hex::encode(self.hash())
    }

    /// Checks for equality with another point within a small tolerance
    /// to handle floating-point inaccuracies.
    pub fn equals(&self, other: &Point) -> bool {
        (self.x - other.x).abs() < GEOMETRIC_TOLERANCE &&
        (self.y - other.y).abs() < GEOMETRIC_TOLERANCE
    }
}

// ----------------------------------------------------------------------------
// 1.3 Triangle Data Structure & Core Methods
// ----------------------------------------------------------------------------

/// Represents a triangle defined by three points (vertices).
/// The `value` field allows the effective value to be less than geometric area
/// (e.g., after fee deduction). If None, value equals geometric area.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Triangle {
    pub a: Point,
    pub b: Point,
    pub c: Point,
    pub parent_hash: Option<Sha256Hash>,
    pub owner: String,
    /// Effective value of this triangle. If None, value = geometric area.
    /// If Some(v), value = v (must be <= geometric area).
    /// This enables fee deduction while preserving geometric identity.
    #[serde(default)]
    pub value: Option<Coord>,
}

impl Triangle {
    /// Creates a new Triangle from three vertices.
    pub fn new(a: Point, b: Point, c: Point, parent_hash: Option<Sha256Hash>, owner: String) -> Self {
        Triangle { a, b, c, parent_hash, owner, value: None }
    }

    /// Creates a new Triangle with an explicit value (for fee-reduced transfers).
    pub fn new_with_value(
        a: Point,
        b: Point,
        c: Point,
        parent_hash: Option<Sha256Hash>,
        owner: String,
        value: Coord,
    ) -> Self {
        Triangle { a, b, c, parent_hash, owner, value: Some(value) }
    }

    /// Returns the effective value of this triangle.
    /// If `value` is set, returns that; otherwise returns the geometric area.
    pub fn effective_value(&self) -> Coord {
        self.value.unwrap_or_else(|| self.area())
    }

    /// Calculates the center point (centroid) of the triangle.

    /// Calculates the area of the triangle using the Shoelace formula.
    pub fn area(&self) -> Coord {
        let val = (self.a.x * (self.b.y - self.c.y) 
                 + self.b.x * (self.c.y - self.a.y) 
                 + self.c.x * (self.a.y - self.b.y))
                 .abs();
        val / 2.0
    }

    /// Calculates the unique cryptographic hash of the triangle.
    /// Optimized to work with raw bytes and avoid string allocations.
    pub fn hash(&self) -> Sha256Hash {
        let mut hashes = [self.a.hash(), self.b.hash(), self.c.hash()];
        // Sort to ensure canonical ordering (same triangle regardless of vertex order)
        hashes.sort_unstable();

        let mut hasher = Sha256::new();
        for hash in &hashes {
            hasher.update(hash);
        }
        hasher.finalize().into()
    }

    pub fn hash_str(&self) -> String {
        hex::encode(self.hash())
    }

    // ------------------------------------------------------------------------
    // 1.6 Genesis Triangle Implementation
    // ------------------------------------------------------------------------

    /// Defines the canonical Genesis Triangle for the TrinityChain.
    pub fn genesis() -> Self {
        const SQRT3: Coord = 1.7320508075688772;
        const HALF_SQRT3: Coord = 0.8660254037844386;
        const ONE_POINT_FIVE: Coord = 1.5;

        Triangle::new(
            Point::new(0.0, 0.0),
            Point::new(SQRT3, 0.0),
            Point::new(HALF_SQRT3, ONE_POINT_FIVE),
            None,
            "genesis_owner".to_string(),
        )
    }
    
    // ------------------------------------------------------------------------
    // 1.7 Subdivision Algorithm
    // ------------------------------------------------------------------------

    /// Subdivides the current triangle into three smaller, valid triangles.
    /// Optimized to minimize allocations and reuse computed values.
    /// Note: Children inherit geometric area (value = None). If parent had
    /// a reduced value, children's values are proportionally scaled.
    #[inline]
    pub fn subdivide(&self) -> [Triangle; 3] {
        // Compute midpoints inline to reduce function call overhead
        let mid_ab = Point::new(
            (self.a.x + self.b.x) * 0.5,
            (self.a.y + self.b.y) * 0.5,
        );
        let mid_bc = Point::new(
            (self.b.x + self.c.x) * 0.5,
            (self.b.y + self.c.y) * 0.5,
        );
        let mid_ca = Point::new(
            (self.c.x + self.a.x) * 0.5,
            (self.c.y + self.a.y) * 0.5,
        );

        let parent_hash = Some(self.hash());

        // If parent has a reduced value, scale children proportionally
        // Each child gets 25% of parent's geometric area (75% total for 3 children)
        // So each child's value = parent_value * 0.25 / 0.25 = parent_value / 3
        let child_value = self.value.map(|v| v / 3.0);

        // Child 1 (A-mid_ab-mid_ca)
        let mut t1 = Triangle::new(self.a, mid_ab, mid_ca, parent_hash, self.owner.clone());
        t1.value = child_value;

        // Child 2 (mid_ab-B-mid_bc)
        let mut t2 = Triangle::new(mid_ab, self.b, mid_bc, parent_hash, self.owner.clone());
        t2.value = child_value;

        // Child 3 (mid_ca-mid_bc-C)
        let mut t3 = Triangle::new(mid_ca, mid_bc, self.c, parent_hash, self.owner.clone());
        t3.value = child_value;

        [t1, t2, t3]
    }

    // ------------------------------------------------------------------------
    // 1.8 Geometric Validation
    // ------------------------------------------------------------------------

    /// Checks if the triangle is geometrically valid.
    /// This checks:
    /// 1. All points have valid, finite coordinates within bounds
    /// 2. The triangle is non-degenerate (Area > Tolerance)
    pub fn is_valid(&self) -> bool {
        // Check all points are valid
        if !self.a.is_valid() || !self.b.is_valid() || !self.c.is_valid() {
            return false;
        }

        // A valid triangle must have a non-zero area (i.e., not collinear points).
        self.area() > GEOMETRIC_TOLERANCE
    }
}


// ----------------------------------------------------------------------------
// Testing
// ----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_triangle() -> Triangle {
        Triangle::new(
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(0.0, 10.0),
            None,
            "test_owner".to_string(),
        )
    }

    #[test]
    fn test_point_midpoint() {
        let p1 = Point::new(1.0, 1.0);
        let p2 = Point::new(5.0, 5.0);
        let midpoint = p1.midpoint(&p2);
        assert_eq!(midpoint, Point::new(3.0, 3.0));
    }

    #[test]
    fn test_triangle_area() {
        let t = setup_test_triangle();
        assert_eq!(t.area(), 50.0);
    }

    #[test]
    fn test_triangle_hash_is_canonical() {
        let p1 = Point::new(1.0, 2.0);
        let p2 = Point::new(3.0, 4.0);
        let p3 = Point::new(5.0, 6.0);

        let t1 = Triangle::new(p1, p2, p3, None, "owner1".to_string());
        let t2 = Triangle::new(p3, p1, p2, None, "owner1".to_string());

        assert_eq!(t1.hash(), t2.hash());
    }

    #[test]
    fn test_genesis_triangle_is_canonical() {
        let g1 = Triangle::genesis();
        let expected_area = 1.299038105676658;
        assert!((g1.area() - expected_area).abs() < 1e-15, "Genesis triangle area is incorrect.");
    }

    #[test]
    fn test_subdivision_correctness() {
        let parent = setup_test_triangle();
        let parent_area = parent.area(); 
        let children = parent.subdivide();
        let total_child_area: Coord = children.iter().map(|t| t.area()).sum();
        
        assert!((total_child_area - parent_area * 0.75).abs() < 1e-9);
    }
    
    #[test]
    fn test_geometric_validation_valid() {
        let t = setup_test_triangle();
        assert!(t.is_valid(), "A normal triangle should be valid.");
        
        let g = Triangle::genesis();
        assert!(g.is_valid(), "The genesis triangle must be valid.");
    }

    #[test]
    fn test_geometric_validation_invalid_degenerate() {
        // Degenerate triangle: all points are collinear (on a straight line)
        let t_degenerate = Triangle::new(
            Point::new(1.0, 1.0),
            Point::new(2.0, 2.0),
            Point::new(3.0, 3.0),
            None,
            "owner".to_string()
        );
        assert!(!t_degenerate.is_valid(), "A degenerate (collinear) triangle should be invalid.");
    }
}
