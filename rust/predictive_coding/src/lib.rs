use std::{fs, io, path::Path};

use rand::SeedableRng;
use rand_distr::{Distribution, Normal};
use rayon::iter::{
    IndexedParallelIterator as _, IntoParallelRefIterator as _,
    IntoParallelRefMutIterator as _, ParallelIterator as _,
};
use serde::{Deserialize, Serialize};

use crate::min_max_tracker::MinMaxTracker;

pub mod idx_data;
pub mod min_max_tracker;
pub mod mnist;
pub mod utils;

pub type Fp = f64;

fn activation(x: Fp) -> Fp {
    x
    // tanh(x)
}
// fn tanh(x: Fp) -> Fp {
//     let a = x.exp();
//     let b = (-x).exp();
//     (a - b) / (a + b)
// }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layer {
    activities: Box<[Fp]>,
    errors: Box<[Fp]>,
    weights_pred: Box<[Box<[Fp]>]>,
    weights_err: Box<[Box<[Fp]>]>,
}

impl Layer {
    pub fn new(size: usize, size_prev: usize, size_next: usize) -> Self {
        let a = vec![0.0; size].into_boxed_slice();

        let mut rng = rand::rngs::SmallRng::seed_from_u64(123456);
        let std_dev = |fan_io| (0.1 / (fan_io + size) as Fp).sqrt();

        let distribution = Normal::new(0.0, std_dev(size_prev))
            .expect("std dev is finite");
        let weights_pred = (0..size)
            .map(|_| {
                (0..size_prev)
                    .map(|_| distribution.sample(&mut rng))
                    .collect::<Box<_>>()
            })
            .collect::<Box<_>>();

        let distribution = Normal::new(0.0, std_dev(size_next))
            .expect("std dev is finite");
        let weights_err = (0..size)
            .map(|_| {
                (0..size_next)
                    .map(|_| distribution.sample(&mut rng))
                    .collect::<Box<_>>()
            })
            .collect::<Box<_>>();

        Self {
            activities: a.clone(),
            errors: a,
            weights_pred,
            weights_err,
        }
    }

    #[inline]
    pub fn is_valid_shape(&self) -> bool {
        self.activities.len() == self.errors.len()
            && self.activities.len() == self.weights_pred.len()
            && self.weights_pred.len() == self.weights_err.len()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Network {
    pub step_rate: Fp,
    pub learn_rate: Fp,
    pub activity_decay: Fp,
    pub weight_decay: Fp,
    #[serde(default)]
    pub parallel: bool,
    layers: Box<[Layer]>,
}

impl Network {
    pub fn new(
        step_rate: Fp,
        learn_rate: Fp,
        activity_decay: Fp,
        weight_decay: Fp,
        parallel: bool,
        layer_sizes: &[usize],
    ) -> Self {
        assert!(!layer_sizes.is_empty());

        let mut layers = Vec::new();
        for idx in 0..layer_sizes.len() {
            let prev = idx
                .checked_sub(1)
                .map(|prev| layer_sizes[prev])
                .unwrap_or(0);
            let next = layer_sizes.get(idx + 1).copied().unwrap_or(0);

            let size = layer_sizes[idx];
            assert!(size > 0);

            layers.push(Layer::new(size, prev, next));
        }

        Self {
            step_rate,
            learn_rate,
            activity_decay,
            weight_decay,
            parallel,
            layers: layers.into_boxed_slice(),
        }
    }

    pub fn input(&mut self) -> &mut [Fp] {
        &mut self.layers.last_mut().unwrap().activities
    }

    pub fn output(&mut self) -> &mut [Fp] {
        &mut self.layers.first_mut().unwrap().activities
    }

    pub fn update_activities(
        &mut self,
        skip_input: bool,
        skip_output: bool,
    ) {
        assert!(!self.layers.is_empty());

        let start = if skip_output { 2 } else { 1 };

        for a_len in start..self.layers.len() {
            let (a, b) = self.layers.split_at_mut(a_len);
            let cur = &mut a[a.len() - 1];
            let next = &b[0];
            assert!(cur.is_valid_shape());

            let each = |((activity, &error), weights_err): (
                (&mut f64, &f64),
                &[f64],
            )| {
                let feedback: Fp = weights_err
                    .iter()
                    .zip(&next.errors)
                    .map(|(&weight, &error)| weight * error)
                    .sum();
                *activity += (-error + feedback) * self.step_rate;
                *activity *= self.activity_decay;
            };

            if self.parallel {
                cur.activities
                    .par_iter_mut()
                    .zip_eq(&cur.errors)
                    .zip_eq(cur.weights_err.par_iter().map(|it| &**it))
                    .for_each(each);
            } else {
                cur.activities
                    .iter_mut()
                    .zip(&cur.errors)
                    .zip(cur.weights_err.iter().map(|it| &**it))
                    .for_each(each);
            }
        }

        if !skip_input {
            let last = self.layers.last_mut().unwrap();
            assert!(last.is_valid_shape());

            let each = |(activity, &error): (&mut f64, &f64)| {
                *activity += -error * self.step_rate;
                *activity *= self.activity_decay;
            };

            if self.parallel {
                last.activities
                    .par_iter_mut()
                    .zip_eq(&last.errors)
                    .for_each(each);
            } else {
                last.activities
                    .iter_mut()
                    .zip(&last.errors)
                    .for_each(each);
            }
        }
    }

    pub fn update_errors(&mut self, skip_output: bool) {
        if !skip_output {
            let first = self.layers.first_mut().unwrap();
            fn each((error, activity): (&mut f64, &f64)) {
                *error = *activity;
            }

            if self.parallel {
                first
                    .errors
                    .par_iter_mut()
                    .zip_eq(&first.activities)
                    .for_each(each);
            } else {
                first
                    .errors
                    .iter_mut()
                    .zip(&first.activities)
                    .for_each(each);
            }
        }

        for idx in 1..self.layers.len() {
            let (a, b) = self.layers.split_at_mut(idx);
            let prev = &a[a.len() - 1];
            let cur = &mut b[0];
            assert!(cur.is_valid_shape());

            let each = |((error, &activity), weights_pred): (
                (&mut f64, &f64),
                &[f64],
            )| {
                let prediction: Fp = weights_pred
                    .iter()
                    .zip(&prev.activities)
                    .map(|(&weight, &activity)| {
                        weight * activation(activity)
                    })
                    .sum();
                *error = activity - prediction;
            };

            if self.parallel {
                cur.errors
                    .par_iter_mut()
                    .zip_eq(&cur.activities)
                    .zip_eq(cur.weights_pred.par_iter().map(|it| &**it))
                    .for_each(each);
            } else {
                cur.errors
                    .iter_mut()
                    .zip(&cur.activities)
                    .zip(cur.weights_pred.iter().map(|it| &**it))
                    .for_each(each);
            }
        }
    }

    pub fn update_weights(&mut self) {
        for a_len in 1..self.layers.len() {
            let (a, b) = self.layers.split_at_mut(a_len);
            let cur = &mut a[a.len() - 1];
            let next = &b[0];
            assert!(cur.is_valid_shape());

            let each = |(weights, &activity): (&mut [f64], &f64)| {
                weights.iter_mut().zip(&next.errors).for_each(
                    |(weight, &error)| {
                        *weight += activity
                            * activation(error)
                            * self.learn_rate;
                        *weight *= self.weight_decay;
                    },
                );
            };

            if self.parallel {
                cur.weights_err
                    .par_iter_mut()
                    .map(|it| &mut **it)
                    .zip_eq(&cur.activities)
                    .for_each(each);
            } else {
                cur.weights_err
                    .iter_mut()
                    .map(|it| &mut **it)
                    .zip(&cur.activities)
                    .for_each(each);
            }
        }

        for idx in 1..self.layers.len() {
            let (a, b) = self.layers.split_at_mut(idx);
            let prev = &a[a.len() - 1];
            let cur = &mut b[0];
            assert!(cur.is_valid_shape());

            let each = |(weights, &error): (&mut [f64], &f64)| {
                weights.iter_mut().zip(&prev.activities).for_each(
                    |(weight, &activity)| {
                        *weight += error
                            * activation(activity)
                            * self.learn_rate;
                        *weight *= self.weight_decay;
                    },
                );
            };

            if self.parallel {
                cur.weights_pred
                    .par_iter_mut()
                    .map(|it| &mut **it)
                    .zip_eq(&cur.errors)
                    .for_each(each);
            } else {
                cur.weights_pred
                    .iter_mut()
                    .map(|it| &mut **it)
                    .zip(&cur.errors)
                    .for_each(each);
            }
        }
    }

    /// # Params
    /// `data` is only necessary if reset is enabled
    ///
    /// # Errors
    /// if explode, return Err with avg error
    /// might be NaN
    pub fn run(
        &mut self,
        epsilon: impl Into<RunEpsilon>,
        data: RunData,
        flags: RunFlags,
    ) -> Result<(), Fp> {
        let epsilon = epsilon.into();

        let mut mm_run = MinMaxTracker::new(epsilon.run);
        let mut mm_weights = MinMaxTracker::new(epsilon.weights);

        let act_skip_input = !flags.contains(RunFlags::UPD_ACT_INPUT);
        let act_skip_output = !flags.contains(RunFlags::UPD_ACT_OUTPUT);
        let err_skip_output = !flags.contains(RunFlags::UPD_ERR_OUTPUT);

        let upd_weights = flags.contains(RunFlags::UPD_WEIGHTS);
        let upd_weights_reset =
            flags.contains(RunFlags::UPD_WEIGHTS_RESET);
        let explode_cont = flags.contains(RunFlags::EXPLODE_CONT);

        loop {
            if let Some(input) = data.input {
                self.input().copy_from_slice(input);
            }
            if let Some(output) = data.output {
                self.output().copy_from_slice(output);
            }

            self.update_errors(err_skip_output);
            self.update_activities(act_skip_input, act_skip_output);
            self.update_errors(err_skip_output);

            let e = self.error_avg();
            mm_run.update(e);

            if mm_run.count() > self.layers.len() {
                if !upd_weights {
                    break;
                }

                self.update_weights();
                if upd_weights_reset {
                    self.reset();
                }

                mm_run.reset();
                mm_weights.update(e);
            }
            if !e.is_finite() || e > u32::MAX as Fp {
                if upd_weights && explode_cont {
                    tracing::warn!("explode {e:?}");
                    self.reset();
                } else {
                    return Err(e);
                }
            }

            if mm_weights.count() > self.layers.len() {
                break;
            }
        }

        Ok(())
    }

    pub fn error_sum(&self) -> Fp {
        self.layers
            .iter()
            .map(|it| it.errors.iter().map(|it| it.abs()).sum::<Fp>())
            .sum::<Fp>()
    }

    pub fn error_avg(&self) -> Fp {
        let count =
            self.layers.iter().map(|it| it.errors.len()).sum::<usize>();
        self.error_sum() / count as Fp
    }

    pub fn reset(&mut self) {
        for cur in &mut self.layers {
            for activity in &mut cur.activities {
                *activity = 0.;
            }

            for error in &mut cur.errors {
                *error = 0.;
            }
        }
    }

    pub fn load(path: impl AsRef<Path>) -> io::Result<Self> {
        let data = fs::read(path)?;
        Ok(serde_json::from_slice(&data).unwrap())
    }

    pub fn save(&self, path: impl AsRef<Path>) -> io::Result<()> {
        let data = serde_json::to_string(self).unwrap();
        fs::write(path, data)
    }

    pub fn layers(&self) -> &[Layer] {
        &self.layers
    }
}

#[derive(Debug)]
pub struct RunEpsilon {
    pub run: Fp,
    pub weights: Fp,
}

impl RunEpsilon {
    pub fn new(run: Fp, weights: Fp) -> Self {
        Self { run, weights }
    }
}

impl From<Fp> for RunEpsilon {
    fn from(value: Fp) -> Self {
        Self::new(value, value)
    }
}

#[derive(Debug)]
pub struct RunData<'data> {
    pub input: Option<&'data [Fp]>,
    pub output: Option<&'data [Fp]>,
}

impl<'data> RunData<'data> {
    pub fn new(input: &'data [Fp], output: &'data [Fp]) -> Self {
        Self {
            input: Some(input),
            output: Some(output),
        }
    }

    pub fn new_input(input: &'data [Fp]) -> Self {
        Self {
            input: Some(input),
            output: None,
        }
    }

    pub fn new_output(output: &'data [Fp]) -> Self {
        Self {
            input: None,
            output: Some(output),
        }
    }

    pub fn none() -> Self {
        Self {
            input: None,
            output: None,
        }
    }
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct RunFlags: u8 {
        const UPD_ACT_INPUT     = 1 << 0;
        const UPD_ACT_OUTPUT    = 1 << 1;
        const UPD_ERR_OUTPUT    = 1 << 2;
        const UPD_WEIGHTS       = 1 << 3;
        /// reset network after update weights
        const UPD_WEIGHTS_RESET = 1 << 4;
        /// continue on explosion with reset
        /// only applicable when UPD_WEIGHTS is set
        const EXPLODE_CONT      = 1 << 5;

        const TRAIN = Self::UPD_WEIGHTS.bits() | Self::EXPLODE_CONT.bits();
        const INFER = Self::UPD_ACT_OUTPUT.bits();
        const INFER_REVERSE = Self::UPD_ACT_INPUT.bits();
    }
}
