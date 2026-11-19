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
