use hyozu::mark::pie;

pub fn data() -> hyozu::Data {
    hyozu::data(
        hyozu::pie([
            pie::slice(42).name("Rent"),
            pie::slice(25).name("Food"),
            pie::slice(15).name("Transport"),
            pie::slice(10).name("Utilities"),
            pie::slice(8).name("Other"),
        ])
        .hole(0.55)
        .gap(2.0)
        .labels(pie::label::Label::percent()),
    )
    .title("Monthly Expenses")
}
