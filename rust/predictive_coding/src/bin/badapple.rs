use std::{io::Write, path::Path};

use predictive_coding::{
    Fp, Network, RunData, RunFlags,
    idx_data::IdxData,
    utils::{bin_init_env, open_file_for_write},
};

fn main() {
    bin_init_env();
    if false {
        run_train(0);
        run_infer("");
    }

    // run_train(0);
    // run_infer("outputs/badapple/network_40.json");
}

fn run_train(use_checkpoint: u64) {
    let mut log = open_file_for_write("outputs/badapple/log.csv");

    let data =
        IdxData::<u8>::load("inputs/badapple/badapple-frames-idx3-ubyte");
    let data_fp: IdxData<Fp> = (&data).into();

    let mut network = if use_checkpoint != 0 {
        Network::load(format!(
            "outputs/badapple/network_{use_checkpoint}.json"
        ))
        .unwrap()
    } else {
        Network::new(
            0.01,
            0.001,
            1.,
            1.,
            true,
            &[data.len(), 1024, 512, 432],
        )
    };

    let mut output = vec![0.0; data.len()];
    let mut epoch = use_checkpoint;
    loop {
        epoch += 1;
        for (idx, frame) in data_fp.idx().iter().enumerate() {
            network.reset();
            output.fill(0.);
            output[idx] = 1.;

            network
                .run(
                    0.1,
                    RunData::new(frame.data(), &output),
                    RunFlags::TRAIN,
                )
                .ok();
            let e = network.error_avg();

            writeln!(&mut log, "{epoch},{idx},{e}").unwrap();
            tracing::debug!("{epoch: >4} {idx: >4} {e: >23.20}");
        }

        tracing::info!("save {epoch}");
        network
            .save(format!("outputs/badapple/network_{epoch}.json"))
            .unwrap();
        if epoch > 100 {
            std::process::exit(0);
        }
    }
}

fn run_infer(checkpoint: impl AsRef<Path>) {
    let data =
        IdxData::<u8>::load("inputs/badapple/badapple-frames-idx3-ubyte");

    let mut network = Network::load(checkpoint).unwrap();
    network.parallel = true;

    let mut input = vec![0.0; data.len()];
    let mut dims: [usize; 3] = data.dimensions().try_into().unwrap();
    dims[0] = 0;
    let mut output = IdxData::<u8>::new(dims.into(), vec![]);
    for idx in 0..data.len() {
        network.reset();
        input.fill(0.);
        input[idx] = 1.;
        network
            .run(
                0.1,
                RunData::new_output(&input),
                RunFlags::INFER_REVERSE,
            )
            .unwrap();

        let data = IdxData::new(
            data.dimensions()[1..].into(),
            network.input().into(),
        );
        let data_u8 = data.to_u8_normalize_trunc_neg();
        data_u8.idx().println_as_image();
        println!("{idx}");
        // std::thread::sleep(Duration::from_secs_f64(1. / 30.));
        output.push(data_u8.data());
    }
    output.save("outputs/badapple/out-frames-idx3-ubyte");
}
