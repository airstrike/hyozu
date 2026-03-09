use hyozu::{Mark, xy};

pub fn data() -> hyozu::Data {
    let setosa = xy([
        (1.4, 0.2),
        (1.4, 0.2),
        (1.3, 0.2),
        (1.5, 0.2),
        (1.4, 0.2),
        (1.7, 0.4),
        (1.4, 0.3),
        (1.5, 0.2),
        (1.4, 0.2),
        (1.5, 0.1),
    ])
    .with_name("Setosa");

    let versicolor = xy([
        (4.7, 1.4),
        (4.5, 1.5),
        (4.9, 1.5),
        (4.0, 1.3),
        (4.6, 1.5),
        (4.5, 1.3),
        (4.7, 1.6),
        (3.3, 1.0),
        (4.6, 1.3),
        (3.9, 1.4),
    ])
    .with_name("Versicolor");

    let virginica = xy([
        (6.0, 2.5),
        (5.1, 1.9),
        (5.9, 2.1),
        (5.6, 1.8),
        (5.8, 2.2),
        (6.6, 2.1),
        (4.5, 1.7),
        (6.3, 1.8),
        (5.8, 1.8),
        (6.1, 2.5),
    ])
    .with_name("Virginica");

    let marks: Vec<Mark> = vec![Mark::from(setosa), Mark::from(versicolor), Mark::from(virginica)];
    hyozu::data(marks).title("Iris Dataset (Petal)")
}
