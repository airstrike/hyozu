//! Frequency-domain plots for real evenly sampled signals.
//! Enable the `spectral` feature to use the RustFFT-backed transforms.
use crate::{Data, Diagram, axis, data, line};
use rustfft::FftPlanner;
use rustfft::num_complex::Complex;
use std::f64::consts::{PI, TAU};

#[derive(Clone, Copy, Debug)]
pub enum Window {
    Rectangular,
    Hann,
}

#[derive(Clone, Copy, Debug)]
pub struct Config {
    pub sample_rate: f64,
    pub segment: usize,
    pub overlap: usize,
    pub window: Window,
    /// Subtract each segment mean before windowing.
    pub detrend: bool,
}

/// Windowed Fourier coefficients from successive complete segments.
pub struct Transform {
    config: Config,
    frames: Vec<Vec<Complex<f64>>>,
    window_sum: f64,
    energy: f64,
}

impl Transform {
    pub fn new(samples: &[f64], config: Config) -> Option<Self> {
        if !config.sample_rate.is_finite()
            || config.sample_rate <= 0.
            || config.segment < 2
            || samples.len() < config.segment
            || config.overlap >= config.segment
            || samples.iter().any(|v| !v.is_finite())
        {
            return None;
        }
        let n = config.segment;
        let window: Vec<_> = (0..n)
            .map(|i| match config.window {
                Window::Rectangular => 1.,
                Window::Hann => 0.5 - 0.5 * (TAU * i as f64 / n as f64).cos(),
            })
            .collect();
        let fft = FftPlanner::new().plan_fft_forward(n);
        let mut scratch = vec![Complex::default(); fft.get_inplace_scratch_len()];
        let mut frames = Vec::new();
        for segment in samples.windows(n).step_by(n - config.overlap) {
            let mean = if config.detrend {
                segment.iter().sum::<f64>() / n as f64
            } else {
                0.
            };
            let mut frame: Vec<_> = segment
                .iter()
                .zip(&window)
                .map(|(&x, &w)| Complex::new((x - mean) * w, 0.))
                .collect();
            fft.process_with_scratch(&mut frame, &mut scratch);
            frame.truncate(n / 2 + 1);
            frames.push(frame);
        }
        Some(Self {
            config,
            frames,
            window_sum: window.iter().sum(),
            energy: window.iter().map(|x| x * x).sum(),
        })
    }

    fn factor(&self, i: usize) -> f64 {
        if i == 0 || (self.config.segment.is_multiple_of(2) && i == self.config.segment / 2) {
            1.
        } else {
            2.
        }
    }
    fn frequency(&self, i: usize) -> f64 {
        i as f64 * self.config.sample_rate / self.config.segment as f64
    }
    fn plot(&self, values: impl Iterator<Item = f64>) -> Data {
        data(line(
            values
                .enumerate()
                .map(|(i, v)| (self.frequency(i), v))
                .collect::<Vec<_>>(),
        ))
        .x_axis(|a| a.with_kind(axis::Kind::Scalar).title("Frequency (Hz)"))
    }

    /// One-sided magnitude spectrum of an individual frame.
    pub fn magnitude(&self, frame: usize) -> Option<Data> {
        Some(
            self.plot(
                self.frames
                    .get(frame)?
                    .iter()
                    .enumerate()
                    .map(|(i, c)| self.factor(i) * c.norm() / self.window_sum),
            ),
        )
    }

    /// Wrapped angle or continuous phase in radians, by frequency.
    pub fn phase(&self, frame: usize, unwrap: bool) -> Option<Data> {
        let mut previous = None;
        let mut offset = 0.;
        Some(self.plot(self.frames.get(frame)?.iter().map(|z| {
            let angle = z.arg();
            if unwrap && let Some(prev) = previous {
                let diff = angle - prev;
                if diff > PI {
                    offset -= TAU;
                } else if diff < -PI {
                    offset += TAU;
                }
            }
            previous = Some(angle);
            angle + offset
        })))
    }

    /// Arithmetic mean of the one-sided modified periodograms (power / Hz).
    pub fn power_density(&self) -> Data {
        self.plot((0..=self.config.segment / 2).map(|i| {
            let sum: f64 = self.frames.iter().map(|f| f[i].norm_sqr()).sum();
            sum * self.factor(i) / (self.frames.len() as f64 * self.config.sample_rate * self.energy)
        }))
    }

    /// One tile per time/frequency bin. True frame centers and endpoint
    /// frequency bins determine the extents. Decibel scale uses 10 log10.
    pub fn spectrogram(&self, decibels: bool) -> Option<Diagram> {
        let n = self.config.segment;
        let fs = self.config.sample_rate;
        let hop = (n - self.config.overlap) as f64 / fs;
        let first = n as f64 / (2. * fs) - hop / 2.;
        let times: Vec<_> = (0..=self.frames.len()).map(|i| first + i as f64 * hop).collect();
        let bins = n / 2 + 1;
        let mut frequencies = Vec::with_capacity(bins + 1);
        frequencies.push(0.);
        frequencies.extend((1..bins).map(|i| (i as f64 - 0.5) * fs / n as f64));
        frequencies.push(fs / 2.);
        let values: Vec<_> = (0..bins)
            .flat_map(|i| {
                self.frames.iter().map(move |f| {
                    let power = self.factor(i) * f[i].norm_sqr() / (fs * self.energy);
                    if decibels {
                        10. * power.max(f64::MIN_POSITIVE).log10()
                    } else {
                        power
                    }
                })
            })
            .collect();
        crate::field::tiles(&times, &frequencies, &values)
    }

    /// Cross-spectral density magnitude or magnitude-squared coherence of
    /// synchronized transforms with identical segment configuration.
    pub fn cross(&self, other: &Self, coherence: bool) -> Option<Data> {
        if self.config.segment != other.config.segment
            || self.config.sample_rate != other.config.sample_rate
            || self.config.overlap != other.config.overlap
            || self.frames.len() != other.frames.len()
        {
            return None;
        }
        Some(self.plot((0..=self.config.segment / 2).map(|i| {
            let mut cross = Complex::<f64>::default();
            let (mut xx, mut yy) = (0., 0.);
            for (a, b) in self.frames.iter().zip(&other.frames) {
                cross += a[i].conj() * b[i];
                xx += a[i].norm_sqr();
                yy += b[i].norm_sqr();
            }
            if coherence {
                if xx > 0. && yy > 0. {
                    (cross.norm_sqr() / (xx * yy)).clamp(0., 1.)
                } else {
                    0.
                }
            } else {
                self.factor(i) * cross.norm()
                    / (self.frames.len() as f64 * self.config.sample_rate * (self.energy * other.energy).sqrt())
            }
        })))
    }
}

/// Cross-correlation at positive and negative discrete lags. Passing the
/// same slice twice gives autocorrelation. Normalization uses the zero-lag
/// signal energies over the full series.
pub fn correlation(x: &[f64], y: &[f64], max_lag: usize, normalized: bool) -> Option<Data> {
    if x.is_empty() || x.len() != y.len() || x.iter().chain(y).any(|v| !v.is_finite()) {
        return None;
    }
    let n = x.len();
    let max = max_lag.min(n - 1) as isize;
    let energy = if normalized {
        (x.iter().map(|v| v * v).sum::<f64>() * y.iter().map(|v| v * v).sum::<f64>()).sqrt()
    } else {
        1.
    };
    if energy <= 0. || !energy.is_finite() {
        return None;
    }
    let points: Vec<_> = (-max..=max)
        .map(|lag| {
            let start = lag.max(0) as usize;
            let end = n - (-lag).max(0) as usize;
            let value = (start..end).map(|i| x[i] * y[(i as isize - lag) as usize]).sum::<f64>() / energy;
            [lag as f64, value]
        })
        .collect();
    Some(crate::cartesian::stem(&points, 0.))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sine_amplitude_power_and_coherence() {
        let signal: Vec<_> = (0..64).map(|i| (TAU * f64::from(i) * 8. / 64.).sin()).collect();
        let transform = Transform::new(&signal, Config {
            sample_rate: 64.,
            segment: 64,
            overlap: 0,
            window: Window::Rectangular,
            detrend: false,
        })
        .expect("valid signal");
        let amplitude = transform.magnitude(0).expect("first frame");
        assert!((amplitude.line(0).expect("spectrum").points[8].y - 1.).abs() < 1e-10);
        let power = transform.power_density();
        assert!((power.line(0).expect("spectrum").points.iter().map(|v| v.y).sum::<f64>() - 0.5).abs() < 1e-10);
        assert!(
            (transform
                .cross(&transform, true)
                .expect("common transform")
                .line(0)
                .expect("coherence")
                .points[8]
                .y
                - 1.)
                .abs()
                < 1e-10
        );
    }
}
