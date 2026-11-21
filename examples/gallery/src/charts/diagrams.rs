use hyozu::mark::diagram::{Flow, Node};
use hyozu::{Data, data};

fn tree() -> Vec<Node> {
    vec![
        Node::branch("Software", vec![Node::leaf("Desktop", 32.), Node::leaf("Mobile", 24.)]),
        Node::branch("Services", vec![
            Node::leaf("Support", 18.),
            Node::leaf("Consulting", 12.),
        ]),
    ]
}

pub fn sunburst() -> Data {
    data(hyozu::sunburst(&tree()))
        .legend(hyozu::legend::Anchor::Right)
        .title("Revenue by division and product")
}

pub fn hierarchical_treemap() -> Data {
    data(hyozu::hierarchical_treemap(&tree()))
        .legend(hyozu::legend::Anchor::Right)
        .title("Revenue by division and product")
}

pub fn radial_bars() -> Data {
    data(hyozu::radial_bars(
        &[("France", 68.), ("UK", 80.), ("Spain", 94.)],
        100.,
    ))
    .legend(hyozu::legend::Anchor::Right)
    .title("Target completion")
}

pub fn radar() -> Data {
    data(hyozu::radar(
        &["Cost", "Quality", "Speed", "Reliability", "Reach"],
        &[("North", [3., 5., 2., 4., 4.]), ("South", [4., 3., 5., 2., 3.])],
        5.,
    ))
    .legend(hyozu::legend::Anchor::Right)
    .title("Regional performance")
}

pub fn funnel() -> Data {
    data(hyozu::funnel(&[
        ("Visited", 1200.),
        ("Joined", 720.),
        ("Tried", 483.),
        ("Bought", 281.),
    ]))
    .legend(hyozu::legend::Anchor::Right)
    .title("Conversion stages")
}

pub fn sankey() -> Data {
    let edges = [(0, 2, 30.), (1, 2, 20.), (0, 3, 15.), (2, 4, 50.), (3, 4, 15.)].map(|(source, target, value)| Flow {
        source,
        target,
        value,
    });
    data(hyozu::sankey(&["Search", "Referral", "Trial", "Direct", "Paid"], &edges).expect("valid gallery flow"))
        .legend(hyozu::legend::Anchor::Right)
        .title("Customer acquisition")
}

fn split(view: hyozu::parts::Breakdown) -> Data {
    let rows = [
        ("Direct", 58.),
        ("Partner", 25.),
        ("Email", 8.),
        ("Events", 5.),
        ("Other sources", 4.),
    ];
    data(hyozu::parts::split_pie(&rows, 2, view).expect("positive main and detail groups"))
        .legend(hyozu::legend::Anchor::Right)
        .title("Acquisition channels")
}

pub fn split_pie() -> Data {
    split(hyozu::parts::Breakdown::Pie)
}
pub fn split_bar() -> Data {
    split(hyozu::parts::Breakdown::Bar)
}

pub fn rings() -> hyozu::Data {
    hyozu::data(
        hyozu::rings(
            &["Direct", "Retail", "Partners"],
            &[[3., 2., 1.], [4., 3., 2.], [6., 4., 2.]],
            0.32,
        )
        .expect("positive ring totals"),
    )
    .title("Channel mix over three years")
}

pub fn marker_radar() -> Data {
    data(hyozu::radar_with_style(
        &["Design", "Service", "Value", "Support", "Quality"],
        &[("North", [4., 3., 5., 3., 4.]), ("South", [2., 4., 3., 5., 3.])],
        5.,
        hyozu::RadarStyle::Markers,
    ))
    .legend(hyozu::legend::Anchor::Right)
    .title("Customer survey profiles")
}
pub fn exploded() -> Data {
    data(
        hyozu::exploded_pie(&[("Direct", 52., 0.), ("Retail", 31., 0.), ("Partners", 17., 0.24)])
            .expect("positive wedges"),
    )
    .legend(hyozu::legend::Anchor::Right)
    .title("Orders by channel")
}

pub fn wind_rose() -> Data {
    let axes = hyozu::polar::Axes::compass()
        .categories(["N", "NE", "E", "SE", "S", "SW", "W", "NW"])
        .ticks(3)
        .format(|v| format!("{v:.0}%"));
    let shares = [12., 7., 9., 5., 14., 21., 18., 14.];
    let bars: Vec<_> = ["N", "NE", "E", "SE", "S", "SW", "W", "NW"]
        .into_iter()
        .zip(shares)
        .enumerate()
        .map(|(i, (name, share))| {
            (name, hyozu::polar::Bar {
                angle: axes.spoke(i),
                width: std::f64::consts::TAU / 8. * 0.8,
                inner: 0.,
                outer: share,
            })
        })
        .collect();
    data(axes.bars(&bars).expect("positive shares")).title("Wind direction frequency")
}

pub fn radar_scale() -> Data {
    data(
        hyozu::polar::Axes::compass()
            .maximum(100.)
            .ticks(5)
            .format(|v| format!("{v:.0}%"))
            .radar(
                &["Coverage", "Accuracy", "Latency", "Uptime", "Adoption", "Retention"],
                &[
                    ("Current", [82., 74., 58., 96., 61., 70.]),
                    ("Target", [90., 85., 75., 99., 80., 85.]),
                ],
                hyozu::RadarStyle::Markers,
            ),
    )
    .legend(hyozu::legend::Anchor::Right)
    .title("Service scorecard")
}

pub fn polar_months() -> Data {
    let axes = hyozu::polar::Axes::compass()
        .categories([
            "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
        ])
        .format(|v| format!("{v:.0} mm"));
    // Twelve months plus a return to January close each loop.
    let cycle = |values: [f64; 12]| -> Vec<[f64; 2]> { (0..=12).map(|i| [axes.spoke(i), values[i % 12]]).collect() };
    data(axes.lines(&[
        (
            "2024",
            cycle([78., 61., 55., 47., 52., 40., 33., 38., 49., 71., 85., 90.]),
        ),
        (
            "2025",
            cycle([64., 70., 48., 39., 58., 46., 29., 31., 57., 80., 76., 95.]),
        ),
    ]))
    .legend(hyozu::legend::Anchor::Right)
    .title("Monthly rainfall")
}

/// Many narrow wedges: names that can't sit level in their wedge turn to
/// read along its radius.
pub fn sunburst_leaves() -> Data {
    let branch = |name: &str, leaves: &[(&str, f64)]| {
        Node::branch(
            name,
            leaves.iter().map(|&(leaf, value)| Node::leaf(leaf, value)).collect(),
        )
    };
    data(hyozu::sunburst(&[
        branch("Americas", &[
            ("Canada", 14.),
            ("United States", 38.),
            ("Mexico", 11.),
            ("Brazil", 16.),
            ("Argentina", 7.),
        ]),
        branch("Europe", &[
            ("Germany", 18.),
            ("France", 13.),
            ("Spain", 9.),
            ("Italy", 10.),
            ("Poland", 6.),
        ]),
        branch("Asia", &[
            ("Japan", 17.),
            ("Korea", 9.),
            ("India", 15.),
            ("Vietnam", 6.),
        ]),
    ]))
    .title("Orders by region and country")
}
