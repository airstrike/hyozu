use hyozu::mark::waterfall::EntryKind::*;
use hyozu::mark::waterfall::{self};

pub fn data() -> hyozu::Data {
    hyozu::data(waterfall::waterfall([
        waterfall::entry(500, Total).text("$500"),
        waterfall::entry(-200, Decrease).text("-$200"),
        waterfall::entry(300, Total).text("$300"),
        waterfall::entry(-80, Decrease).text("-$80"),
        waterfall::entry(-60, Decrease).text("-$60"),
        waterfall::entry(20, Increase).text("+$20"),
        waterfall::entry(180, Total).text("$180"),
    ]))
    .title("Quarterly P&L")
    .x_axis_labels(["Revenue", "COGS", "Gross", "SG&A", "R&D", "Other", "EBITDA"])
}
