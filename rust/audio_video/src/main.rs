use std::{
    collections::VecDeque,
    env::args,
    f32,
    f64::consts::TAU,
    fs::File,
    io::{Read, Write},
    sync::{Arc, Mutex, mpsc::channel},
    thread::sleep,
    time::Duration,
};

use anyhow::Context;
use bevy_math::FloatExt;
use color::{Oklch, OpaqueColor, Srgb};
use cpal::{
    BufferSize, SupportedBufferSize,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use image::{ImageBuffer, Luma, Rgb};
use rustfft::{FftPlanner, num_complex::Complex};
use tracing::{debug, info};

const FREQ_BASE: f64 = 4_000.;
const FREQ_SEQ_START: f64 = 4_000.;
const FREQ_IMAGE_SYNC: f64 = 8_000.;
const FREQ_LINE_SYNC: f64 = 12_000.;
const FREQ_DATA: f64 = 16_000.;

const USE_FILE: bool = true;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let mode = args().nth(1).unwrap_or_default();
    match mode.as_str() {
        "encode" => {
            encode()?;
        }
        "decode" => {
            decode()?;
        }
        _ => {
            eprintln!("Unknown mode, available: encode, decode");
        }
    }

    Ok(())
}

fn decode() -> anyhow::Result<()> {
    let mut img = ImageBuffer::<Luma<u8>, _>::new(36, 20);
    let mut y = 0;
    let (width, height) = img.dimensions();
    let mut frame_num = 1_usize;

    let mut fft_planner = FftPlanner::<f64>::new();

    let host = cpal::default_host();
    let input = host
        .default_input_device()
        .context("no input device available")?;
    let config = input
        .default_input_config()
        .context("failed to get default input config")?;

    let sample_rate = config.sample_rate().0 as usize;
    let sample_rate_f64 = sample_rate as f64;

    let fft_len_f64 = 1. / FREQ_BASE * sample_rate_f64;
    let fft_len = fft_len_f64.ceil() as usize;

    let idx_seq_start = FREQ_SEQ_START * fft_len as f64 / sample_rate_f64;
    let idx_image_sync =
        FREQ_IMAGE_SYNC * fft_len as f64 / sample_rate_f64;
    let idx_line_sync = FREQ_LINE_SYNC * fft_len as f64 / sample_rate_f64;
    let idx_data = FREQ_DATA * fft_len as f64 / sample_rate_f64;

    let idx_seq_start = idx_seq_start.floor() as usize;
    let idx_image_sync = idx_image_sync.floor() as usize;
    let idx_line_sync = idx_line_sync.floor() as usize;
    let idx_data = idx_data.floor() as usize;

    let fft = fft_planner.plan_fft_forward(fft_len);

    let mut fft_buf = vec![Complex::new(0., 0.); fft_len];

    let mut buf = VecDeque::<f32>::new();
    let mut line_buf = Vec::<f32>::new();
    let mut lo_data = 1000.0f64;
    let mut hi_data = 0.0f64;

    let mut last_seq_start = 0_usize;
    let mut last_img_sync = 0_usize;
    let mut last_line_sync = 0_usize;

    let mut data_cb = move |data: &[f32]| {
        buf.extend(data);
        while buf.len() > fft_len {
            fft_buf.clear();
            fft_buf.extend(
                buf.iter()
                    .take(fft_len)
                    .map(|it| Complex::new(*it as f64, 0.)),
            );
            buf.rotate_left(fft_len);
            buf.resize(buf.len().saturating_sub(fft_len), 0.0);
            fft_buf.resize(fft_len, Complex::default());
            fft.process(&mut fft_buf);

            let seq_start = fft_buf[idx_seq_start].norm();
            let img_sync = fft_buf[idx_image_sync].norm();
            let line_sync = fft_buf[idx_line_sync].norm();
            let data = fft_buf[idx_data].norm();

            let seq_start = seq_start > 1.;
            let img_sync = img_sync > 1.;
            let line_sync = line_sync > 1.;
            let seq_start = if seq_start {
                let seq_start = last_seq_start == 0;
                last_seq_start = last_seq_start.saturating_add(1);
                seq_start
            } else {
                last_seq_start = last_seq_start.saturating_sub(1);
                false
            };
            let img_sync = if img_sync {
                let img_sync = last_img_sync == 0;
                last_img_sync = last_img_sync.saturating_add(1);
                img_sync
            } else {
                last_img_sync = last_img_sync.saturating_sub(1);
                false
            };
            let line_sync = if line_sync {
                let line_sync = last_line_sync == 0;
                last_line_sync = last_line_sync.saturating_add(1);
                line_sync
            } else {
                last_line_sync = last_line_sync.saturating_sub(1);
                false
            };

            if seq_start {
                debug!(
                    "new sequence ========================================"
                );
                hi_data = 0.0;
                lo_data = 1.0;
            }

            hi_data = hi_data.max(data + f64::EPSILON);
            lo_data = lo_data.min(data);

            let data = data.remap(lo_data, hi_data, 0.0, 1.0);
            // debug!("{data:0<7.5}");
            if img_sync {
                debug!("new img ==========");
                img.save(format!("./output/{frame_num:0>10}.png"))
                    .expect("failed to save output");
                y = 0;
                frame_num += 1;
            }

            if line_sync {
                debug!("new line");

                let mut out = vec![0.0; width as usize];
                resample(&mut line_buf, &mut out);
                line_buf.clear();

                let out = out.into_iter().map(|it| (it * 255.0) as u8);

                for (x, v) in out.enumerate() {
                    img.put_pixel(x as u32, y, Luma([v]));
                }

                y = (y + 1).min(height - 1);
            }

            line_buf.push(data as f32);
            if line_buf.len() > 100_000_000 {
                line_buf.clear();
            }
        }
    };

    if USE_FILE {
        info!("loading");
        let mut file = File::options()
            .read(true)
            .open("./audio.bin")
            .context("failed to open input audio file")?;
        let mut buf = vec![];
        file.read_to_end(&mut buf)
            .context("failed to read input audio file")?;
        info!("converting");
        let buf = buf
            .chunks_exact(32 / 8)
            .map(|it| {
                let it: [u8; 32 / 8] = it.try_into().unwrap();
                f32::from_be_bytes(it)
            })
            .collect::<Vec<_>>();
        info!("decoding");
        for data in buf.chunks(4096) {
            data_cb(data)
        }
    } else {
        let mut config = config.config();
        config.channels = 1;
        let stream = input
            .build_input_stream(
                &config,
                move |data, _| data_cb(data),
                |err| panic!("{err}"),
                None,
            )
            .context("failed to build input stream")?;
        stream.play().context("failed to record")?;

        loop {
            sleep(Duration::from_secs(100));
        }
    }

    Ok(())
}

fn encode() -> anyhow::Result<()> {
    let frame_time: f64 = 1. / 10.;

    let host = cpal::default_host();
    let out = host
        .default_output_device()
        .context("no output device available")?;
    let config = out
        .default_output_config()
        .context("failed to get default output config")?;

    let sample_rate = config.sample_rate().0 as f64;
    info!("sample rate: {sample_rate}");

    let mut frame_num = 1;
    fn load_frame(
        frame_num: usize,
    ) -> anyhow::Result<ImageBuffer<Rgb<u8>, Vec<u8>>> {
        Ok(image::open(format!("./input/{frame_num:0>10}.jpg"))
            .context("failed to open input")?
            .to_rgb8())
    }

    let mut img = load_frame(frame_num)?;
    let (width, height) = img.dimensions();
    frame_num += 1;
    let (load_finish_tx, load_finish_rx) = channel::<()>();
    let (load_new_tx, load_new_rx) = channel::<usize>();
    // TODO: graceful exit
    let img_tmp = Arc::new(Mutex::new(load_frame(frame_num)?));
    load_finish_tx.send(()).unwrap();
    let img_tmp_2 = Arc::clone(&img_tmp);
    std::thread::spawn(move || {
        while let Ok(frame_num) = load_new_rx.recv() {
            let img = load_frame(frame_num);
            match img {
                Ok(img) => {
                    *img_tmp_2.lock().unwrap() = img;
                    if load_finish_tx.send(()).is_err() {
                        info!("failed to send load finish");
                        break;
                    }
                }
                Err(err) => {
                    info!("{err:?}");
                    break;
                }
            }
        }
    });

    // DEBUG
    let line_dur = frame_time / height as f64;
    info!(
        "frame_time: {}ms, line_dur: {}ms",
        frame_time * 1000.,
        line_dur * 1000.,
    );

    let mut line_ts = 0_f64;
    let mut y = 0_u32;

    let sync_dur = (sample_rate as f64 * line_dur / 2.0).floor() as usize;
    let sync_dur =
        sync_dur.min((sample_rate as f64 * 0.005).floor() as usize);
    assert_ne!(sync_dur, 0);
    let mut dur_seq_start = sync_dur;
    let mut dur_image_sync = sync_dur;
    let mut dur_line_sync = sync_dur;

    let mut sample_ts = 0_f64;
    let mut data_cb = move |data: &mut [f32]| {
        for v in data {
            let line_idx =
                line_ts / sample_rate as f64 / line_dur * width as f64;
            let x = line_idx as u32;
            let x = if x >= width {
                line_ts = 0.0;
                y += 1;
                dur_line_sync = sync_dur;
                0
            } else {
                x
            };
            if y >= height {
                dur_image_sync = sync_dur;
                y = 0;

                if load_finish_rx.recv().is_err() {
                    std::process::exit(0);
                }
                img = img_tmp.lock().unwrap().clone();

                frame_num += 1;
                if load_new_tx.send(frame_num).is_err() {
                    std::process::exit(0);
                };
            }
            let px = img.get_pixel(x, y);
            let px = OpaqueColor::<Srgb>::new(
                px.0.map(|it| it as f32 / 255.0),
            )
            .convert::<Oklch>()
            .components[0];

            let sin_wave = |freq: f64| {
                (TAU * freq * sample_ts / sample_rate).sin() as f32
            };

            let mut sample = sin_wave(FREQ_DATA);
            sample *= px as f32 * 0.8 + 0.2;

            if dur_seq_start > 0 {
                dur_seq_start -= 1;
                sample += sin_wave(FREQ_SEQ_START) * 0.2;
            }
            if dur_line_sync > 0 {
                dur_line_sync -= 1;
                sample += sin_wave(FREQ_LINE_SYNC) * 0.2;
            }
            if dur_image_sync > 0 {
                dur_image_sync -= 1;
                sample += sin_wave(FREQ_IMAGE_SYNC) * 0.2;
            }

            *v = sample * 0.5;
            sample_ts += 1.0;
            line_ts += 1.0;
        }
    };

    if USE_FILE {
        let mut file = File::options()
            .create(true)
            .write(true)
            .truncate(true)
            .open("./audio.bin")
            .context("failed to open audio output")?;
        let mut buf = vec![0.0_f32; 4096];
        loop {
            data_cb(&mut buf);
            let out = buf
                .iter()
                .copied()
                .flat_map(f32::to_be_bytes)
                .collect::<Vec<u8>>();
            file.write_all(&out)
                .context("failed to write audio output")?;
        }
    } else {
        let target_buf_size = 4096_u32;
        let buf_size = match config.buffer_size() {
            SupportedBufferSize::Range { max, .. } => {
                target_buf_size.min(*max)
            }
            SupportedBufferSize::Unknown => target_buf_size,
        };
        let mut config = config.config();
        config.buffer_size = BufferSize::Fixed(buf_size);
        config.channels = 1;
        let stream = out
            .build_output_stream::<f32, _, _>(
                &config,
                move |data, _| data_cb(data),
                |err| panic!("{err}"),
                None,
            )
            .context("failed to build output stream")?;
        stream.play().context("failed to play the stream")?;

        loop {
            sleep(Duration::from_secs(100));
        }
    }
}

fn resample(input: &mut [f32], output: &mut [f32]) {
    if input.is_empty() || output.is_empty() {
        return;
    }

    let mut out_count = vec![0_usize; output.len()];

    let len_f_in = input.len() as f64;
    let len_f_out = output.len() as f64;
    if len_f_out < len_f_in {
        for (idx, v) in input.iter().enumerate() {
            let idx = (idx as f64)
                .remap(0.0, len_f_in, 0.0, len_f_out)
                .floor() as usize;
            output[idx] += v;
            out_count[idx] += 1;
        }
    } else {
        for (idx, v) in
            output.iter_mut().zip(out_count.iter_mut()).enumerate()
        {
            let idx = (idx as f64)
                .remap(0.0, len_f_out, 0.0, len_f_in)
                .floor() as usize;
            *v.0 = input[idx];
            *v.1 += 1;
        }
    }
    for (v, c) in output.iter_mut().zip(out_count.iter_mut()) {
        *v /= *c as f32;
    }
}
