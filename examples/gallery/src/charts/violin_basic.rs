use hyozu::{violin, violin_from_data};

pub fn data() -> hyozu::Data {
    let group_a: Vec<f64> = (0..50)
        .map(|i| 5.0 + (i as f64 * 0.1).sin() * 3.0 + (i as f64 * 0.07))
        .collect();
    let group_b: Vec<f64> = (0..50)
        .map(|i| 8.0 + (i as f64 * 0.15).cos() * 4.0 + (i as f64 * 0.05))
        .collect();
    let group_c: Vec<f64> = (0..50)
        .map(|i| 3.0 + (i as f64 * 0.2).sin() * 2.0 + (i as f64 * 0.03))
        .collect();

    hyozu::data(violin(vec![
        violin_from_data(&group_a).with_name("Treatment A"),
        violin_from_data(&group_b).with_name("Treatment B"),
        violin_from_data(&group_c).with_name("Control"),
    ]))
    .title("Treatment Response Distribution")
    .x_axis_labels(["Treatment A", "Treatment B", "Control"])
}
