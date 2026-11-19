use hyozu::facet::{Grid, Scales};

fn panels(scales: Scales) -> Grid {
    let rows = [
        ("North", [2., 4., 3., 6.]),
        ("East", [5., 4., 7., 8.]),
        ("South", [14., 21., 17., 28.]),
        ("West", [27., 21., 30., 26.]),
    ];
    let panels = rows
        .into_iter()
        .map(|(name, values)| {
            hyozu::data(hyozu::line(values))
                .title(name)
                .palette(hyozu::palette::tonal(0x267e74))
        })
        .collect();
    Grid::new(panels, 2, scales).expect("two panel columns")
}
pub fn fixed() -> Grid {
    panels(Scales::Fixed)
}
pub fn free() -> Grid {
    panels(Scales::FreeY)
}
