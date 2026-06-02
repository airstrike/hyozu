//! Shared rounded ring-sector path builder for polar marks (pie, donut,
//! gauge). Traces a wedge or ring segment with circular fillets at its
//! corners — the polar analogue of a rounded rectangle's `push_bar_path`.

use crate::core::Point;
use crate::widget::canvas::path::Builder;

/// Line segments per full turn when flattening arcs. Matches the pie/gauge
/// arc tessellation density.
const ARC_SEGMENTS_PER_TAU: usize = 64;

/// Appends a closed ring-sector sub-path with a uniform corner radius —
/// the common case for pie and donut slices, where both angular ends
/// round equally. See [`push_sector_path_ends`] for independent ends.
#[allow(clippy::too_many_arguments)]
pub fn push_sector_path(
    builder: &mut Builder,
    cx: f32,
    cy: f32,
    inner_radius: f32,
    outer_radius: f32,
    a0: f32,
    a1: f32,
    corner: f32,
) {
    push_sector_path_ends(builder, cx, cy, inner_radius, outer_radius, a0, a1, corner, corner);
}

/// Appends a closed ring-sector sub-path centered at `(cx, cy)`, spanning
/// `inner_radius..=outer_radius` from angle `a0` to `a1` (radians), with
/// the `a0` end rounded by up to `corner_start` pixels and the `a1` end by
/// up to `corner_end`.
///
/// Independent ends express *terminal rounding*: a gauge arc built from
/// several zone segments rounds only the first segment's leading cap and
/// the last segment's trailing cap, leaving interior joints square so the
/// zones meet without gaps.
///
/// A full wedge (`inner_radius == 0`) rounds only the outer corners — the
/// apex at the center stays sharp, matching Recharts' pie `cornerRadius`.
/// A ring (`inner_radius > 0`) rounds all corners on each rounded end.
/// Each radius is clamped to half the ring thickness and to the available
/// sweep so fillets never cross; a non-positive radius leaves that end
/// sharp. The fn issues its own `move_to`, so callers should start a fresh
/// sub-path.
#[allow(clippy::too_many_arguments)]
pub fn push_sector_path_ends(
    builder: &mut Builder,
    cx: f32,
    cy: f32,
    inner_radius: f32,
    outer_radius: f32,
    a0: f32,
    a1: f32,
    corner_start: f32,
    corner_end: f32,
) {
    let ri = inner_radius.max(0.0);
    let ro = outer_radius.max(ri);
    let sweep = a1 - a0;
    let thickness = ro - ri;

    // Drop degenerate / non-finite sectors — the NaN gap convention.
    if !cx.is_finite() || !cy.is_finite() || !ri.is_finite() || !ro.is_finite() {
        return;
    }
    if thickness <= 0.0 || sweep.abs() < f32::EPSILON || ro <= 0.0 {
        return;
    }

    let polar = |r: f32, a: f32| Point::new(cx + r * a.cos(), cy + r * a.sin());
    let is_ring = ri > f32::EPSILON;
    let sd = sweep.signum();

    // Clamp a corner radius to half the thickness and to the angular room.
    // When both ends round they split the sweep; when only one end rounds
    // it may consume the whole sweep — so a single-ended cap segment can
    // host a full half-thickness fillet. The fillet inset `delta` tops out
    // at PI/2 (rc == ro/2), beyond which only the thickness limit bites.
    let rounded_ends = (corner_start > 0.0) as u32 + (corner_end > 0.0) as u32;
    let budget = if rounded_ends >= 2 {
        sweep.abs() * 0.5
    } else {
        sweep.abs()
    };
    let s = budget.min(std::f32::consts::FRAC_PI_2).sin();
    let clamp_corner = |c: f32| {
        let mut rc = c.max(0.0).min(thickness * 0.5);
        if s > 0.0 {
            rc = rc.min(ro * s / (1.0 + s));
            if is_ring && s < 1.0 {
                rc = rc.min(ri * s / (1.0 - s));
            }
        }
        rc
    };
    let cs = clamp_corner(corner_start);
    let ce = clamp_corner(corner_end);

    if cs <= 0.0 && ce <= 0.0 {
        push_sharp_sector(builder, &polar, ri, ro, a0, a1, is_ring);
        return;
    }

    // Per-end fillet geometry. Outer fillets sit at radius `ro - rc`
    // (tangent internally to the outer circle); inner fillets at `ri + rc`
    // (tangent externally to the inner circle). `delta` is the angular
    // offset from the radial edge at which the fillet meets the arc.
    let outer = |rc: f32| {
        if rc <= 0.0 {
            (0.0_f32, 0.0_f32)
        } else {
            let rho = ro - rc;
            (rho, (rc / rho).asin())
        }
    };
    let inner = |rc: f32| {
        if rc <= 0.0 || !is_ring {
            (0.0_f32, 0.0_f32)
        } else {
            let rho = ri + rc;
            (rho, (rc / rho).asin())
        }
    };

    let (rho_os, d_os) = outer(cs);
    let (rho_oe, d_oe) = outer(ce);

    // Outer-edge contact points: where each end transitions between the
    // radial edge and the outer arc. With rc == 0 the inset collapses to
    // the sharp corner at the radius.
    let os_radial = if cs > 0.0 {
        polar(rho_os * d_os.cos(), a0)
    } else {
        polar(ro, a0)
    };
    let os_arc = polar(ro, a0 + sd * d_os);
    let oe_arc = polar(ro, a1 - sd * d_oe);
    let oe_radial = if ce > 0.0 {
        polar(rho_oe * d_oe.cos(), a1)
    } else {
        polar(ro, a1)
    };

    if is_ring {
        let (rho_is, d_is) = inner(cs);
        let (rho_ie, d_ie) = inner(ce);

        let ie_radial = if ce > 0.0 {
            polar(rho_ie * d_ie.cos(), a1)
        } else {
            polar(ri, a1)
        };
        let ie_arc = polar(ri, a1 - sd * d_ie);
        let is_arc = polar(ri, a0 + sd * d_is);
        let is_radial = if cs > 0.0 {
            polar(rho_is * d_is.cos(), a0)
        } else {
            polar(ri, a0)
        };

        builder.move_to(os_radial);
        if cs > 0.0 {
            trace_fillet(builder, polar(rho_os, a0 + sd * d_os), cs, os_radial, os_arc);
        }
        trace_arc(builder, cx, cy, ro, a0 + sd * d_os, a1 - sd * d_oe);
        if ce > 0.0 {
            trace_fillet(builder, polar(rho_oe, a1 - sd * d_oe), ce, oe_arc, oe_radial);
        }
        builder.line_to(ie_radial);
        if ce > 0.0 {
            trace_fillet(builder, polar(rho_ie, a1 - sd * d_ie), ce, ie_radial, ie_arc);
        }
        trace_arc(builder, cx, cy, ri, a1 - sd * d_ie, a0 + sd * d_is);
        if cs > 0.0 {
            trace_fillet(builder, polar(rho_is, a0 + sd * d_is), cs, is_arc, is_radial);
        }
        builder.close();
    } else {
        // Full wedge: apex at the center, only the outer corners round.
        builder.move_to(Point::new(cx, cy));
        builder.line_to(os_radial);
        if cs > 0.0 {
            trace_fillet(builder, polar(rho_os, a0 + sd * d_os), cs, os_radial, os_arc);
        }
        trace_arc(builder, cx, cy, ro, a0 + sd * d_os, a1 - sd * d_oe);
        if ce > 0.0 {
            trace_fillet(builder, polar(rho_oe, a1 - sd * d_oe), ce, oe_arc, oe_radial);
        }
        builder.line_to(Point::new(cx, cy));
        builder.close();
    }
}

/// Sharp (un-rounded) sector — the `corner <= 0` fallback, matching the
/// hand-traced wedge/ring the pie and gauge renderers used previously.
fn push_sharp_sector(
    builder: &mut Builder,
    polar: &impl Fn(f32, f32) -> Point,
    ri: f32,
    ro: f32,
    a0: f32,
    a1: f32,
    is_ring: bool,
) {
    if is_ring {
        builder.move_to(polar(ri, a0));
        builder.line_to(polar(ro, a0));
        trace_arc_pts(builder, polar, ro, a0, a1);
        builder.line_to(polar(ri, a1));
        trace_arc_pts(builder, polar, ri, a1, a0);
        builder.close();
    } else {
        builder.move_to(polar(0.0, a0));
        builder.line_to(polar(ro, a0));
        trace_arc_pts(builder, polar, ro, a0, a1);
        builder.close();
    }
}

/// Traces a main arc (line segments) from `from` to `to` at `radius`
/// about `(cx, cy)`. Mirrors the pie/gauge `trace_arc`: emits only
/// `line_to`, so the caller must already sit at the arc's start point.
fn trace_arc(builder: &mut Builder, cx: f32, cy: f32, radius: f32, from: f32, to: f32) {
    let sweep = to - from;
    let segments = ((sweep.abs() / std::f32::consts::TAU) * ARC_SEGMENTS_PER_TAU as f32).ceil() as usize;
    let segments = segments.max(1);
    for i in 1..=segments {
        let t = i as f32 / segments as f32;
        let angle = from + sweep * t;
        builder.line_to(Point::new(cx + radius * angle.cos(), cy + radius * angle.sin()));
    }
}

/// Variant taking the shared `polar` closure (sharp-fallback path).
fn trace_arc_pts(builder: &mut Builder, polar: &impl Fn(f32, f32) -> Point, radius: f32, from: f32, to: f32) {
    let sweep = to - from;
    let segments = ((sweep.abs() / std::f32::consts::TAU) * ARC_SEGMENTS_PER_TAU as f32).ceil() as usize;
    let segments = segments.max(1);
    for i in 1..=segments {
        let t = i as f32 / segments as f32;
        builder.line_to(polar(radius, from + sweep * t));
    }
}

/// Traces a corner fillet: a `radius`-arc about `center` from point
/// `from` to point `to`, taking the short angular way. The caller must
/// already sit at `from`.
fn trace_fillet(builder: &mut Builder, center: Point, radius: f32, from: Point, to: Point) {
    let a_from = (from.y - center.y).atan2(from.x - center.x);
    let a_to = (to.y - center.y).atan2(to.x - center.x);
    let mut sweep = a_to - a_from;
    // Normalize to the short way, (-PI, PI].
    while sweep <= -std::f32::consts::PI {
        sweep += std::f32::consts::TAU;
    }
    while sweep > std::f32::consts::PI {
        sweep -= std::f32::consts::TAU;
    }
    let segments = ((sweep.abs() / std::f32::consts::TAU) * ARC_SEGMENTS_PER_TAU as f32).ceil() as usize;
    let segments = segments.max(1);
    for i in 1..=segments {
        let t = i as f32 / segments as f32;
        let angle = a_from + sweep * t;
        builder.line_to(Point::new(
            center.x + radius * angle.cos(),
            center.y + radius * angle.sin(),
        ));
    }
}
