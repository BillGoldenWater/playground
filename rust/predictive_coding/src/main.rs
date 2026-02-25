use std::{
    io::Write as _,
    path::Path,
    sync::atomic::{self, AtomicU64},
};

use rand::SeedableRng;
use rand_distr::{Distribution, Normal};
use rayon::iter::{ParallelBridge, ParallelIterator as _};
use serde::{Deserialize, Serialize};
use yansi::{Paint, Style};

mod scratch_pad;

type Fp = f64;

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
struct Layer {
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
struct Network {
    step_rate: Fp,
    learn_rate: Fp,
    activity_decay: Fp,
    weight_decay: Fp,
    layers: Box<[Layer]>,
}

impl Network {
    pub fn new(
        step_rate: Fp,
        learn_rate: Fp,
        activity_decay: Fp,
        weight_decay: Fp,
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

            cur.activities
                .iter_mut()
                .zip(&cur.errors)
                .zip(&cur.weights_err)
                .for_each(|((activity, &error), weights_err)| {
                    let feedback: Fp = weights_err
                        .iter()
                        .zip(&next.errors)
                        .map(|(&weight, &error)| weight * error)
                        .sum();
                    *activity += (-error + feedback) * self.step_rate;
                    *activity *= self.activity_decay;
                });
        }

        if !skip_input {
            let last = self.layers.last_mut().unwrap();
            assert!(last.is_valid_shape());

            last.activities.iter_mut().zip(&last.errors).for_each(
                |(activity, &error)| {
                    *activity += -error * self.step_rate;
                    *activity *= self.activity_decay;
                },
            );
        }
    }

    pub fn update_errors(&mut self, skip_output: bool) {
        if !skip_output {
            let first = self.layers.first_mut().unwrap();
            first.errors.iter_mut().zip(&first.activities).for_each(
                |(error, activity)| {
                    *error = *activity;
                },
            );
        }

        for idx in 1..self.layers.len() {
            let (a, b) = self.layers.split_at_mut(idx);
            let prev = &a[a.len() - 1];
            let cur = &mut b[0];
            assert!(cur.is_valid_shape());

            cur.errors
                .iter_mut()
                .zip(&cur.activities)
                .zip(&cur.weights_pred)
                .for_each(|((error, &activity), weights_pred)| {
                    let prediction: Fp = weights_pred
                        .iter()
                        .zip(&prev.activities)
                        .map(|(&weight, &activity)| {
                            weight * activation(activity)
                        })
                        .sum();
                    *error = activity - prediction;
                });
        }
    }

    pub fn update_weights(&mut self) {
        for a_len in 1..self.layers.len() {
            let (a, b) = self.layers.split_at_mut(a_len);
            let cur = &mut a[a.len() - 1];
            let next = &b[0];
            assert!(cur.is_valid_shape());

            cur.weights_err.iter_mut().zip(&cur.activities).for_each(
                |(weights, &activity)| {
                    weights.iter_mut().zip(&next.errors).for_each(
                        |(weight, &error)| {
                            *weight += activity
                                * activation(error)
                                * self.learn_rate;
                            *weight *= self.weight_decay;
                        },
                    );
                },
            );
        }

        for idx in 1..self.layers.len() {
            let (a, b) = self.layers.split_at_mut(idx);
            let prev = &a[a.len() - 1];
            let cur = &mut b[0];
            assert!(cur.is_valid_shape());

            cur.weights_pred.iter_mut().zip(&cur.errors).for_each(
                |(weights, &error)| {
                    weights.iter_mut().zip(&prev.activities).for_each(
                        |(weight, &activity)| {
                            *weight += error
                                * activation(activity)
                                * self.learn_rate;
                            *weight *= self.weight_decay;
                        },
                    );
                },
            );
        }
    }

    pub fn error_sum(&self) -> Fp {
        self.layers
            .iter()
            .map(|it| it.errors.iter().map(|it| it.powi(2)).sum::<Fp>())
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
}

#[derive(Debug)]
struct Mnist {
    labels: Box<[u8]>,
    images: Box<[u8]>,
    images_fp: Box<[Fp]>,
    len: usize,
    width: usize,
    height: usize,
}

impl Mnist {
    pub fn new(
        labels: impl AsRef<Path>,
        images: impl AsRef<Path>,
    ) -> Self {
        let labels = std::fs::read(labels).unwrap().into_boxed_slice();
        let images = std::fs::read(images).unwrap().into_boxed_slice();
        assert_eq!(&labels[..4], &0x00000801_u32.to_be_bytes());
        assert_eq!(&images[..4], &0x00000803_u32.to_be_bytes());

        let labels_len =
            u32::from_be_bytes(labels[4..8].try_into().unwrap()) as usize;
        let images_len =
            u32::from_be_bytes(images[4..8].try_into().unwrap()) as usize;
        assert_eq!(labels_len, images_len);

        assert_eq!(labels.len() - 8, labels_len);
        let width = u32::from_be_bytes(images[8..12].try_into().unwrap())
            as usize;
        let height =
            u32::from_be_bytes(images[12..16].try_into().unwrap())
                as usize;
        assert_eq!(images.len() - 16, images_len * width * height);

        let images_fp =
            images.iter().map(|&it| it as Fp / 255.).collect();

        Self {
            labels,
            images,
            images_fp,
            len: labels_len,
            width,
            height,
        }
    }

    pub fn iter(&self) -> MnistIter<'_> {
        MnistIter {
            mnist: self,
            idx: 0,
        }
    }
}

#[derive(Debug, Clone)]
struct MnistIter<'m> {
    mnist: &'m Mnist,
    idx: usize,
}

impl<'a> Iterator for MnistIter<'a> {
    type Item = MnistEntry<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx >= self.mnist.len {
            return None;
        }

        let img_len = self.mnist.width * self.mnist.height;
        let img_start = 16 + self.idx * img_len;

        let label_idx = 8 + self.idx;
        self.idx += 1;

        Some(MnistEntry {
            label: self.mnist.labels[label_idx],
            data: &self.mnist.images[img_start..img_start + img_len],
            data_fp: &self.mnist.images_fp
                [img_start..img_start + img_len],
        })
    }
}

#[derive(Debug)]
struct MnistEntry<'data> {
    label: u8,
    data: &'data [u8],
    data_fp: &'data [Fp],
}

impl<'data> MnistEntry<'data> {
    pub fn println(&self) {
        for row in self.data.chunks(28) {
            for &it in row {
                print!(
                    "{}",
                    "\u{2588}\u{2588}"
                        .paint(Style::new().rgb(it, it, it))
                );
            }
            println!();
        }
    }
}

struct MinMaxTracker {
    epsilon: Fp,
    min: Fp,
    max: Fp,
    count: usize,
}

impl MinMaxTracker {
    pub fn new(epsilon: Fp) -> Self {
        Self {
            epsilon,
            min: Fp::MAX,
            max: Fp::MIN,
            count: 0,
        }
    }

    pub fn update(&mut self, x: Fp) {
        let epsilon = x.abs() * self.epsilon;

        self.count += 1;
        if x < self.min - epsilon {
            self.min = x;
            self.count = 0;
        }
        if x > self.max + epsilon {
            self.max = x;
            self.count = 0;
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new(self.epsilon)
    }
}

fn main() {
    if false {
        run_train(0, 0);
        run_test("");
        run_reverse_println(0, "");
    }
    // run_train(1, 1000);
    // dbg!(run_test("./network_30000_0.json"));
    // dbg!(run_test("./network_60000_1.json"));
    // dbg!(run_test("./network_120000_2.json"));
    // dbg!(run_test("./network_868000_14.json"));

    // for x in 0..10 {
    //     run_reverse_println(x, "./network_60000_1.json");
    // }

    // let mut i = [1.; 10];
    // i[1] = 1.;
    // i[3] = 1.5;
    //
    // let mut network = run_reverse(&i, "./network_60000_1.json");
    //
    // let mut o = network.input().to_vec();
    // o.sort_unstable_by(|a, b| a.total_cmp(b));
    // let max = *o.last().unwrap();
    // let o = network
    //     .input()
    //     .iter()
    //     .map(|it| (it / max * 255.) as u8)
    //     .collect::<Vec<_>>();
    //
    // MnistEntry {
    //     label: 0,
    //     data: &o,
    //     data_fp: network.input(),
    // }
    // .println();
}

fn run_train(use_checkpoint: usize, save_every: usize) {
    let mut log = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open("./log.csv")
        .unwrap();

    let mnist = Mnist::new(
        "./mnist/train-labels-idx1-ubyte",
        "./mnist/train-images-idx3-ubyte",
    );

    let mut network = if use_checkpoint > 1 {
        let v = std::fs::read(format!(
            "./network_{use_checkpoint}_{}.json",
            use_checkpoint / mnist.len
        ))
        .unwrap();
        serde_json::from_slice(&v).unwrap()
    } else {
        Network::new(0.01, 0.0001, 1., 1., &[10, 64, 128, 784])
    };

    let mut mnist_iter = mnist.iter().cycle().skip(use_checkpoint);
    let mut cur = mnist_iter.next().unwrap();

    let mut data_count = use_checkpoint;

    let mut err_mm = MinMaxTracker::new(0.01);
    let mut err_mm2 = MinMaxTracker::new(0.01);

    loop {
        network.input().copy_from_slice(cur.data_fp);
        network.output().fill(0.0);
        network.output()[cur.label as usize] = 1.;

        network.update_activities(true, true);
        network.update_errors(true);

        let e = network.error_avg();
        err_mm.update(e);

        let epoch = data_count / mnist.len;

        if err_mm.count > network.layers.len() {
            network.update_weights();

            err_mm.reset();
            err_mm2.update(e);
        }
        if !e.is_finite() || e > u32::MAX as f64 {
            println!("explode {e:?}");
            network.reset();
        }

        if err_mm2.count > network.layers.len() {
            err_mm2.reset();
            writeln!(&mut log, "{epoch},{data_count},{e}").unwrap();
            println!(
                "{epoch: >5} {data_count: >16} {lr: >12.8} {e: >23.20}",
                lr = network.learn_rate
            );

            if data_count.is_multiple_of(save_every) {
                println!("save");
                network.learn_rate *= 0.998;
                let mut network = network.clone();
                network.reset();
                let network = serde_json::to_string(&network).unwrap();
                std::fs::write(
                    format!("./network_{data_count}_{epoch}.json"),
                    network,
                )
                .unwrap();
            }

            cur = mnist_iter.next().unwrap();
            network.reset();
            data_count += 1;
        }
    }
}

fn run_test(checkpoint: impl AsRef<Path>) -> f64 {
    let mnist = Mnist::new(
        "./mnist/t10k-labels-idx1-ubyte",
        "./mnist/t10k-images-idx3-ubyte",
    );
    // let mnist = Mnist::new(
    //     "./mnist/train-labels-idx1-ubyte",
    //     "./mnist/train-images-idx3-ubyte",
    // );

    let network = std::fs::read(checkpoint).unwrap();
    let network: Network = serde_json::from_slice(&network).unwrap();

    let correct = AtomicU64::new(0);
    let incorrect = AtomicU64::new(0);
    mnist
        .iter()
        .zip(std::iter::repeat(network))
        .enumerate()
        .par_bridge()
        .for_each(|(idx, (cur, mut network))| {
            network.reset();
            network.input().copy_from_slice(cur.data_fp);

            let mut err_mm = MinMaxTracker::new(0.01);
            'run: loop {
                network.update_activities(true, false);
                network.update_errors(true);

                let e = network.error_avg();
                err_mm.update(e);

                if !e.is_finite() || e > u32::MAX as f64 {
                    println!("explode, idx: {idx} error: {e:?}");
                    break 'run;
                }

                if err_mm.count > network.layers.len() {
                    break 'run;
                }
            }

            let o = network.output();
            let mut out = o
                .iter()
                .enumerate()
                .map(|(idx, v)| (*v, idx))
                .collect::<Box<_>>();
            out.sort_unstable_by(|a, b| a.0.total_cmp(&b.0));
            let result = out.last().unwrap().1;

            // cur.println();

            //             println!(
            //                 "error: {e: >13.10?} out: {o: >4.1?} \
            // label: {} result: {}",
            //                 cur.label, result
            //             );

            if u8::try_from(result).unwrap() == cur.label {
                correct.fetch_add(1, atomic::Ordering::Relaxed);
            } else {
                incorrect.fetch_add(1, atomic::Ordering::Relaxed);
            }

            let correct = correct.load(atomic::Ordering::Relaxed);
            let incorrect = incorrect.load(atomic::Ordering::Relaxed);
            let total = correct + incorrect;
            if total.is_multiple_of(1000) {
                println!("progress: {total}");
            }
        });

    let correct = correct.into_inner();
    let incorrect = incorrect.into_inner();

    println!("correct: {correct}, incorrect: {incorrect}");
    correct as f64 / mnist.len as f64
}

fn run_reverse(
    input: &[Fp; 10],
    checkpoint: impl AsRef<Path>,
) -> Network {
    let network = std::fs::read(checkpoint).unwrap();
    let mut network: Network = serde_json::from_slice(&network).unwrap();

    network.reset();
    network.output().copy_from_slice(input);

    let mut err_mm = MinMaxTracker::new(0.01);
    loop {
        network.update_activities(false, true);
        network.update_errors(true);

        let e = network.error_avg();
        err_mm.update(e);

        if !e.is_finite() || e > u32::MAX as f64 {
            println!("explode, error: {e:?}");
            break;
        }

        if err_mm.count > network.layers.len() {
            break;
        }
    }

    network
}

fn run_reverse_println(input: u8, checkpoint: impl AsRef<Path>) {
    assert!((0..=9_u8).contains(&input));

    let mut i = [0.; 10];
    i[input as usize] = 1.;

    let mut network = run_reverse(&i, checkpoint);

    let mut o = network.input().to_vec();
    o.sort_unstable_by(|a, b| a.total_cmp(b));
    let max = *o.last().unwrap();
    let o = network
        .input()
        .iter()
        .map(|it| (it / max * 255.) as u8)
        .collect::<Vec<_>>();

    println!("input: {input}");
    MnistEntry {
        label: 0,
        data: &o,
        data_fp: network.input(),
    }
    .println();
}
