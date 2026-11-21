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

/// One monthly revenue record: (region, product, month, revenue).
type Sale = (&'static str, &'static str, f64, f64);

fn sales() -> Vec<Sale> {
    let regions = [
        ("North", 1.0),
        ("South", 1.6),
        ("East", 0.8),
        ("West", 2.1),
        ("Central", 1.3),
    ];
    let products = [("Hardware", 14.0, 0.6), ("Software", 9.0, 1.4), ("Services", 6.0, 0.9)];
    let mut rows = Vec::new();
    for (region, scale) in regions {
        for (product, base, growth) in products {
            for month in 0..12 {
                let m = f64::from(month);
                let season = (m * 0.52 + base).sin() * 1.5;
                rows.push((region, product, m + 1., scale * (base + growth * m + season)));
            }
        }
    }
    rows
}

fn trend(rows: &[Sale]) -> hyozu::Data {
    hyozu::data(hyozu::line(rows.iter().map(|r| (r.2, r.3)).collect::<Vec<_>>()))
        .x_axis(|a| a.with_kind(hyozu::axis::Kind::Scalar))
        .palette(hyozu::palette::tonal(0x267e74))
}

pub fn wrap() -> Grid {
    // Software revenue by region, one panel per region in three columns.
    let software = sales().into_iter().filter(|r| r.1 == "Software");
    hyozu::facet::wrap(software, |r| r.0)
        .columns(3)
        .chart(|group| trend(group.rows()))
        .expect("three columns")
}

pub fn grid() -> Grid {
    // Region down the side, product across the top; y is free per row.
    let rows = sales().into_iter().filter(|r| r.0 != "Central" && r.0 != "East");
    hyozu::facet::grid(rows, |r| r.0, |r| r.1)
        .scales(Scales::FreeY)
        .chart(|group| trend(group.rows()))
        .expect("product columns")
}
