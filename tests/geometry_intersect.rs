use hyozu::geometry::intersect;
use iced::{Point, Rectangle};

#[test]
fn line_rect_no_intersection() {
    // Line completely to the left of rect
    let p1 = Point::new(0.0, 50.0);
    let p2 = Point::new(10.0, 50.0);
    let rect = Rectangle {
        x: 20.0,
        y: 40.0,
        width: 20.0,
        height: 20.0,
    };
    assert!(!intersect::line_rect(p1, p2, rect));
}

#[test]
fn line_rect_line_crosses_rect() {
    // Line passes through rect
    let p1 = Point::new(0.0, 50.0);
    let p2 = Point::new(100.0, 50.0);
    let rect = Rectangle {
        x: 40.0,
        y: 40.0,
        width: 20.0,
        height: 20.0,
    };
    assert!(intersect::line_rect(p1, p2, rect));
}

#[test]
fn line_rect_endpoint_inside() {
    // Line endpoint is inside rect
    let p1 = Point::new(50.0, 50.0);
    let p2 = Point::new(100.0, 50.0);
    let rect = Rectangle {
        x: 40.0,
        y: 40.0,
        width: 20.0,
        height: 20.0,
    };
    assert!(intersect::line_rect(p1, p2, rect));
}

#[test]
fn line_rect_diagonal_through() {
    // Diagonal line through rect
    let p1 = Point::new(0.0, 0.0);
    let p2 = Point::new(100.0, 100.0);
    let rect = Rectangle {
        x: 40.0,
        y: 40.0,
        width: 20.0,
        height: 20.0,
    };
    assert!(intersect::line_rect(p1, p2, rect));
}

#[test]
fn line_rect_diagonal_miss() {
    // Diagonal line misses rect
    let p1 = Point::new(0.0, 0.0);
    let p2 = Point::new(30.0, 30.0);
    let rect = Rectangle {
        x: 40.0,
        y: 40.0,
        width: 20.0,
        height: 20.0,
    };
    assert!(!intersect::line_rect(p1, p2, rect));
}

#[test]
fn rects_overlap() {
    let a = Rectangle {
        x: 0.0,
        y: 0.0,
        width: 20.0,
        height: 20.0,
    };
    let b = Rectangle {
        x: 10.0,
        y: 10.0,
        width: 20.0,
        height: 20.0,
    };
    assert!(intersect::rects(a, b));
}

#[test]
fn rects_no_overlap() {
    let a = Rectangle {
        x: 0.0,
        y: 0.0,
        width: 20.0,
        height: 20.0,
    };
    let b = Rectangle {
        x: 30.0,
        y: 30.0,
        width: 20.0,
        height: 20.0,
    };
    assert!(!intersect::rects(a, b));
}

#[test]
fn rects_touching_edge() {
    // Rects share an edge but don't overlap interior
    let a = Rectangle {
        x: 0.0,
        y: 0.0,
        width: 20.0,
        height: 20.0,
    };
    let b = Rectangle {
        x: 20.0,
        y: 0.0,
        width: 20.0,
        height: 20.0,
    };
    // Touching edge is not considered overlap
    assert!(!intersect::rects(a, b));
}

#[test]
fn segments_cross() {
    let p1 = Point::new(0.0, 0.0);
    let p2 = Point::new(10.0, 10.0);
    let p3 = Point::new(0.0, 10.0);
    let p4 = Point::new(10.0, 0.0);
    assert!(intersect::segments(p1, p2, p3, p4));
}

#[test]
fn segments_parallel() {
    let p1 = Point::new(0.0, 0.0);
    let p2 = Point::new(10.0, 0.0);
    let p3 = Point::new(0.0, 5.0);
    let p4 = Point::new(10.0, 5.0);
    assert!(!intersect::segments(p1, p2, p3, p4));
}

#[test]
fn segments_share_endpoint() {
    let p1 = Point::new(0.0, 0.0);
    let p2 = Point::new(10.0, 10.0);
    let p3 = Point::new(10.0, 10.0);
    let p4 = Point::new(20.0, 0.0);
    assert!(intersect::segments(p1, p2, p3, p4));
}
