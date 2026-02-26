use std::{
    io::Write as _,
    path::Path,
    sync::atomic::{self, AtomicU64},
};

mod scratch_pad;

use predictive_coding::{
    Fp, Network, RunData, RunFlags, idx_data::IdxData, mnist::Mnist,
    utils::bin_init_env,
};
use rayon::iter::{ParallelBridge as _, ParallelIterator as _};

fn main() {
    bin_init_env();

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
            use_checkpoint / mnist.len()
        ))
        .unwrap();
        serde_json::from_slice(&v).unwrap()
    } else {
        Network::new(0.01, 0.0001, 1., 1., &[10, 64, 128, 784])
    };

    let mut mnist_iter = mnist.iter().cycle().skip(use_checkpoint);
    let mut cur = mnist_iter.next().unwrap();

    let mut data_count = use_checkpoint;

    let mut label = [0.; 10];
    loop {
        label.fill(0.0);
        label[cur.label() as usize] = 1.;
        network
            .run(
                0.01,
                RunData::new(cur.data_fp(), &label),
                RunFlags::TRAIN,
            )
            .ok();
        network.input().copy_from_slice(cur.data_fp());

        let e = network.error_avg();
        let epoch = data_count / mnist.len();

        writeln!(&mut log, "{epoch},{data_count},{e}").unwrap();
        tracing::info!(
            "{epoch: >5} {data_count: >16} {lr: >12.8} {e: >23.20}",
            lr = network.learn_rate()
        );

        if data_count.is_multiple_of(save_every) {
            tracing::info!("save");
            *network.learn_rate_mut() *= 0.998;
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
    let mut network: Network = serde_json::from_slice(&network).unwrap();
    network.reset();

    let correct = AtomicU64::new(0);
    let incorrect = AtomicU64::new(0);
    mnist
        .iter()
        .zip(std::iter::repeat(network))
        .enumerate()
        .par_bridge()
        .for_each(|(idx, (cur, mut network))| {
            let run = tracing::warn_span!("might explode", idx).entered();
            network
                .run(
                    0.01,
                    RunData::new_input(cur.data_fp()),
                    RunFlags::INFER,
                )
                .ok();
            drop(run);

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

            if u8::try_from(result).unwrap() == cur.label() {
                correct.fetch_add(1, atomic::Ordering::Relaxed);
            } else {
                incorrect.fetch_add(1, atomic::Ordering::Relaxed);
            }

            let correct = correct.load(atomic::Ordering::Relaxed);
            let incorrect = incorrect.load(atomic::Ordering::Relaxed);
            let total = correct + incorrect;
            if total.is_multiple_of(1000) {
                tracing::debug!("progress: {total}");
            }
        });

    let correct = correct.into_inner();
    let incorrect = incorrect.into_inner();

    tracing::info!("correct: {correct}, incorrect: {incorrect}");
    correct as f64 / mnist.len() as f64
}

fn run_reverse(
    input: &[Fp; 10],
    checkpoint: impl AsRef<Path>,
) -> Network {
    let network = std::fs::read(checkpoint).unwrap();
    let mut network: Network = serde_json::from_slice(&network).unwrap();

    network.reset();
    network
        .run(0.01, RunData::new_output(input), RunFlags::INFER_REVERSE)
        .ok();

    network
}

fn run_reverse_println(input: u8, checkpoint: impl AsRef<Path>) {
    assert!((0..=9_u8).contains(&input));

    let mut i = [0.; 10];
    i[input as usize] = 1.;

    let mut network = run_reverse(&i, checkpoint);

    tracing::info!("input: {input}");
    let o = IdxData::new([28, 28].into(), network.input().to_vec());
    o.to_u8_normalize().idx().println_as_image();
}
