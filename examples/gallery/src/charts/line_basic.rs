use hyozu::line;

pub fn data() -> hyozu::Data {
    hyozu::data(line([2, 5, 3, 8, 4, 7, 6, 9, 5, 10])).title("Temperature Over Time")
}
