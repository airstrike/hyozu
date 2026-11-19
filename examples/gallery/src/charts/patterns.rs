use hyozu::{Data, data};

pub fn polar() -> Data {
    let observations: Vec<_> = (0..=180)
        .map(|i| {
            let angle = std::f64::consts::TAU * (f64::from(i) / 180.).powf(1.2);
            [angle, 5. + 3. * (3. * angle).cos()]
        })
        .collect();
    data(hyozu::polar::lines(&[("Amplitude", observations)])).title("Directional sensor response")
}

pub fn broken_bars() -> Data {
    let acquisition =
        hyozu::cartesian::broken_bars("Acquire", &[[1., 4.], [7., 3.], [13., 5.]], [1., 0.65]).expect("finite lane");
    let processing =
        hyozu::cartesian::broken_bars("Process", &[[4., 2.], [9., 3.], [17., 3.]], [0., 0.65]).expect("finite lane");
    data(vec![
        hyozu::Mark::Diagram(acquisition),
        hyozu::Mark::Diagram(processing),
    ])
    .x_axis(|a| a.title("Elapsed time (s)"))
    .y_axis(|a| a.none())
    .legend(hyozu::legend::Anchor::Bottom)
    .title("Instrument activity")
}

pub fn sparsity() -> Data {
    let occupied: Vec<_> = (0..48usize)
        .flat_map(|row| {
            (0..48usize)
                .filter(move |&column| row.abs_diff(column) <= 1 || row.abs_diff(column) == 12)
                .map(move |column| [row, column])
        })
        .collect();
    data(hyozu::field::sparsity(48, 48, &occupied).expect("in-bounds indices")).title("Finite-difference stencil")
}
