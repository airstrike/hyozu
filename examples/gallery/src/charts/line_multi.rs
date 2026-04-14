use hyozu::{Mark, line};

pub fn data() -> hyozu::Data {
    let marks: Vec<Mark> = vec![
        line([2, 5, 3, 8, 4, 7]).with_name("Tokyo").into(),
        line([1, 3, 5, 4, 6, 8]).with_name("London").into(),
        line([4, 2, 6, 3, 7, 5]).with_name("Sydney").into(),
    ];

    hyozu::data(marks)
        .x_axis_labels(["Jan", "Feb", "Mar", "Apr", "May", "Jun"])
        .title("Monthly Visitors")
}
