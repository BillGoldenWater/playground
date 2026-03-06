use core::slice;
use std::{mem::transmute, ptr};

use ffmpeg_sys_next::*;

use crate::{error::AVResult, frame::Frame, packet::Packet};

pub mod error;
pub mod frame;
pub mod packet;
pub mod swscale;
pub mod utils;
pub use error::Result;

fn main() -> anyhow::Result<()> {
    unsafe {
        let input_path = c"file:./i.mkv";

        let mut ifmt_ctx: *mut AVFormatContext = ptr::null_mut();
        avformat_open_input(
            &mut ifmt_ctx,
            input_path.as_ptr(),
            ptr::null_mut(),
            ptr::null_mut(),
        )
        .wrap_unit()
        .unwrap();
        avformat_find_stream_info(ifmt_ctx, ptr::null_mut())
            .wrap_unit()
            .unwrap();

        av_dump_format(ifmt_ctx, 0, input_path.as_ptr(), 0);

        let mut vdec: *const AVCodec = ptr::null();
        let video_stream_id = av_find_best_stream(
            ifmt_ctx,
            AVMediaType::AVMEDIA_TYPE_VIDEO,
            -1,
            -1,
            &mut vdec,
            0,
        )
        .wrap_ret()
        .unwrap();
        let video_stream =
            *(*ifmt_ctx).streams.add(video_stream_id as usize);

        let mut vdec_ctx = avcodec_alloc_context3(vdec);
        assert!(!vdec_ctx.is_null());
        avcodec_parameters_to_context(vdec_ctx, (*video_stream).codecpar)
            .wrap_unit()
            .unwrap();
        (*vdec_ctx).pkt_timebase = (*video_stream).time_base;
        (*vdec_ctx).framerate =
            av_guess_frame_rate(ifmt_ctx, video_stream, ptr::null_mut());

        avcodec_open2(vdec_ctx, vdec, ptr::null_mut())
            .wrap_unit()
            .unwrap();

        let mut pkt = Packet::new();
        let mut frame = Frame::default();
        loop {
            let mut pkt = pkt.temp();
            match av_read_frame(ifmt_ctx, pkt.as_ptr()).wrap_unit() {
                Err(err) if *err == AVERROR_EOF => {
                    break;
                }
                r => r.unwrap(),
            };

            if pkt.stream_index != video_stream_id {
                continue;
            }

            avcodec_send_packet(vdec_ctx, pkt.as_ptr())
                .wrap_unit()
                .unwrap();

            match avcodec_receive_frame(vdec_ctx, frame.as_ptr())
                .wrap_unit()
            {
                Err(err) if *err == AVERROR(EAGAIN) => {
                    continue;
                }
                r => r.unwrap(),
            };

            let time_base = (*video_stream).time_base;
            let time_base = time_base.num as f64 / time_base.den as f64;
            assert_ne!(frame.pts, AV_NOPTS_VALUE);
            let ts = frame.pts as f64 * time_base;
            if ts < 1.0 {
                continue;
            };

            let (width, height) = (frame.width, frame.height);

            dbg!(ts, width, height);

            // let mut rgb_frame = Frame::default();
            //
            // rgb_frame.format = AVPixelFormat::AV_PIX_FMT_RGB24 as i32;
            // rgb_frame.width = width;
            // rgb_frame.height = height;
            //
            // av_frame_get_buffer(rgb_frame.as_ptr(), 32)
            //     .wrap_unit()
            //     .unwrap();
            // av_frame_copy_props(rgb_frame.as_ptr(), frame.as_ptr())
            //     .wrap_unit()
            //     .unwrap();
            //
            // let mut sws_ctx = SwsCtx::new();
            // sws_ctx.scale_frame(&mut rgb_frame, &frame).unwrap();

            let out_frame = frame;

            let venc =
                avcodec_find_encoder_by_name(c"libsvtav1".as_ptr());
            // let venc = avcodec_find_encoder(AVCodecID::AV_CODEC_ID_PNG);
            assert!(!venc.is_null());
            let mut venc_ctx = avcodec_alloc_context3(venc);
            assert!(!venc_ctx.is_null());
            (*venc_ctx).width = out_frame.width;
            (*venc_ctx).height = out_frame.height;
            (*venc_ctx).time_base = (*video_stream).time_base;
            (*venc_ctx).framerate = (*video_stream).avg_frame_rate;
            (*venc_ctx).pix_fmt =
                transmute::<i32, AVPixelFormat>(out_frame.format);
            (*venc_ctx).colorspace = out_frame.colorspace;
            (*venc_ctx).color_trc = out_frame.color_trc;
            (*venc_ctx).color_primaries = out_frame.color_primaries;
            (*venc_ctx).color_range = out_frame.color_range;

            avcodec_open2(venc_ctx, venc, ptr::null_mut())
                .wrap_unit()
                .unwrap();

            avcodec_send_frame(venc_ctx, out_frame.as_ptr_const())
                .wrap_unit()
                .unwrap();
            avcodec_send_frame(venc_ctx, ptr::null())
                .wrap_unit()
                .unwrap();

            let mut out_packet = Packet::new();

            avcodec_receive_packet(venc_ctx, out_packet.as_ptr())
                .wrap_unit()
                .unwrap();

            let data = slice::from_raw_parts(
                out_packet.data,
                out_packet.size as usize,
            );
            std::fs::write("out.av1", data).unwrap();

            avcodec_free_context(&mut venc_ctx);

            break;
        }

        avcodec_free_context(&mut vdec_ctx);
        avformat_close_input(&mut ifmt_ctx);
    };

    Ok(())
}
