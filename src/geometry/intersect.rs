//! Intersection and hit-testing utilities for pixel-space geometry.
//! Uses f32 coordinates (iced's Point/Rectangle) since this is for rendering.

use crate::core::{Point, Rectangle};

/// Check if a line segment intersects a rectangle.
pub fn line_rect(p1: Point, p2: Point, rect: Rectangle) -> bool {
    // Quick bounding box check
    let line_min_x = p1.x.min(p2.x);
    let line_max_x = p1.x.max(p2.x);
    let line_min_y = p1.y.min(p2.y);
    let line_max_y = p1.y.max(p2.y);

    if line_max_x < rect.x
        || line_min_x > rect.x + rect.width
        || line_max_y < rect.y
        || line_min_y > rect.y + rect.height
    {
        return false;
    }

    // Check if either endpoint is inside the rect
    if rect.contains(p1) || rect.contains(p2) {
        return true;
    }

    // Check if line crosses any edge of the rectangle
    let corners = [
        Point::new(rect.x, rect.y),
        Point::new(rect.x + rect.width, rect.y),
        Point::new(rect.x + rect.width, rect.y + rect.height),
        Point::new(rect.x, rect.y + rect.height),
    ];

    let edges = [
        (corners[0], corners[1]),
        (corners[1], corners[2]),
        (corners[2], corners[3]),
        (corners[3], corners[0]),
    ];

    edges.iter().any(|(e1, e2)| segments(p1, p2, *e1, *e2))
}

/// Check if two rectangles intersect.
pub fn rects(a: Rectangle, b: Rectangle) -> bool {
    a.x < b.x + b.width && a.x + a.width > b.x && a.y < b.y + b.height && a.y + a.height > b.y
}

/// Check if two line segments intersect.
pub fn segments(p1: Point, p2: Point, p3: Point, p4: Point) -> bool {
    let d1 = direction(p3, p4, p1);
    let d2 = direction(p3, p4, p2);
    let d3 = direction(p1, p2, p3);
    let d4 = direction(p1, p2, p4);

    if ((d1 > 0.0 && d2 < 0.0) || (d1 < 0.0 && d2 > 0.0)) && ((d3 > 0.0 && d4 < 0.0) || (d3 < 0.0 && d4 > 0.0)) {
        return true;
    }

    if d1 == 0.0 && on_segment(p3, p4, p1) {
        return true;
    }
    if d2 == 0.0 && on_segment(p3, p4, p2) {
        return true;
    }
    if d3 == 0.0 && on_segment(p1, p2, p3) {
        return true;
    }
    if d4 == 0.0 && on_segment(p1, p2, p4) {
        return true;
    }

    false
}

/// Cross product direction for intersection testing.
fn direction(p1: Point, p2: Point, p3: Point) -> f32 {
    (p3.x - p1.x) * (p2.y - p1.y) - (p2.x - p1.x) * (p3.y - p1.y)
}

/// Check if point is on segment (assuming collinear).
fn on_segment(p1: Point, p2: Point, p: Point) -> bool {
    p.x >= p1.x.min(p2.x) && p.x <= p1.x.max(p2.x) && p.y >= p1.y.min(p2.y) && p.y <= p1.y.max(p2.y)
}
