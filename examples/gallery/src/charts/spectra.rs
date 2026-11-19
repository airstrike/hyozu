use hyozu::spectral::{Config, Transform, Window};
use hyozu::{Data, data, palette};

fn samples() -> Vec<f64> {
    (0..1024)
        .map(|i| {
            let t = f64::from(i) / 128.;
            (std::f64::consts::TAU * (3. * t + 0.7 * t * t)).sin() + 0.38 * (std::f64::consts::TAU * 22. * t).cos()
        })
        .collect()
}

fn spectrum() -> Transform {
    Transform::new(&samples(), Config {
        sample_rate: 128.,
        segment: 128,
        overlap: 96,
        window: Window::Hann,
        detrend: true,
    })
    .expect("valid sampling configuration")
}

pub fn amplitude() -> Data {
    spectrum().magnitude(0).expect("first frame").title("Signal amplitude")
}
pub fn phase() -> Data {
    spectrum().phase(0, true).expect("first frame").title("Unwrapped phase")
}
pub fn psd() -> Data {
    spectrum().power_density().title("Welch power spectral density")
}
pub fn spectrogram() -> Data {
    data(spectrum().spectrogram(true).expect("spectral bins"))
        .palette(palette::gradient([0x192f43, 0x437f91, 0x86bca9, 0xf5da84]))
        .title("Frequency through time")
}
pub fn autocorrelation() -> Data {
    let s = samples();
    hyozu::spectral::correlation(&s, &s, 25, true)
        .expect("nonempty signal")
        .title("Signal autocorrelation")
}
pub fn coherence() -> Data {
    let s = samples()
        .into_iter()
        .enumerate()
        .map(|(i, v)| v + 0.5 * (i as f64 * 0.42).sin())
        .collect::<Vec<_>>();
    let other = Transform::new(&s, Config {
        sample_rate: 128.,
        segment: 128,
        overlap: 96,
        window: Window::Hann,
        detrend: true,
    })
    .expect("valid sampling configuration");
    spectrum()
        .cross(&other, true)
        .expect("matching samples")
        .y_axis_bounds(0., 1.)
        .title("Magnitude-squared coherence")
}
