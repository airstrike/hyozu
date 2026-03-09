use hyozu::mark::pie;

pub fn data() -> hyozu::Data {
    hyozu::data(hyozu::pie([
        pie::slice(35).name("Desktop"),
        pie::slice(30).name("Mobile"),
        pie::slice(20).name("Tablet"),
        pie::slice(15).name("Other"),
    ]))
    .title("Device Usage")
}
