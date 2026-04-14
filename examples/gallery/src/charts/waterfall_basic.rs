use hyozu::mark::waterfall::EntryKind::*;
use hyozu::mark::waterfall::{self};

pub fn data() -> hyozu::Data {
    hyozu::data(waterfall::waterfall([
        waterfall::entry(500, Total).label("$500"),
        waterfall::entry(-200, Decrease).label("-$200"),
        waterfall::entry(300, Total).label("$300"),
        waterfall::entry(-80, Decrease).label("-$80"),
        waterfall::entry(-60, Decrease).label("-$60"),
        waterfall::entry(20, Increase).label("+$20"),
        waterfall::entry(180, Total).label("$180"),
    ]))
    .title("Quarterly P&L")
    .x_axis_labels(["Revenue", "COGS", "Gross", "SG&A", "R&D", "Other", "EBITDA"])
}
