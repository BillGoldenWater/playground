#![warn(missing_debug_implementations)]

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::Instant,
};

use anyhow::Context;
use app::viewport::renderer::param::Param;
use tracing::Level;
use tracing_subscriber::EnvFilter;
use wgpu_context::WgpuContext;
use winit::event_loop::{ControlFlow, EventLoop};

use crate::app::App;

#[tokio::main]
async fn main() {
    run().await.expect("failed to run");
}

mod app;
mod wgpu_context;

async fn run() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    tracing_subscriber::fmt::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(Level::INFO.into())
                .from_env_lossy(),
        )
        .init();

    let mut app = App {
        ctx: WgpuContext::new()
            .await
            .context("failed to initialize wgpu context")?,
        state: Arc::new(Mutex::new(Param::default())),
        command_queue: Arc::new(Mutex::new(VecDeque::new())),

        paused: false,
        paused_pending_step: 0,

        viewport: None,

        frame_count: 0,
        last_report: Instant::now(),
        tick_multiply: 1,
        perf_offset: 0,
    };

    let event_loop =
        EventLoop::new().context("failed to initialize event loop")?;
    event_loop.set_control_flow(ControlFlow::Wait);
    event_loop
        .run_app(&mut app)
        .context("failed to run event loop")?;

    Ok(())
}
