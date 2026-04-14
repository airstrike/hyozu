use hyozu::{area, areas};

pub fn data() -> hyozu::Data {
    hyozu::data(areas([area([120, 180, 150, 210, 190, 240, 220, 260])]))
        .x_axis_labels(["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug"])
        .title("Website Traffic")
}
