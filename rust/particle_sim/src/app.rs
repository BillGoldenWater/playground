use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::Instant,
};

use tracing::info;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{ElementState, MouseButton, WindowEvent},
    keyboard::{Key, NamedKey},
    window::WindowAttributes,
};

use self::viewport::{
    renderer::{command::Command, param::Param, Renderer},
    Viewport,
};
use crate::wgpu_context::WgpuContext;
pub mod viewport;

#[derive(Debug)]
pub struct App {
    pub ctx: WgpuContext,
    pub state: Arc<Mutex<Param>>,
    pub command_queue: Arc<Mutex<VecDeque<Command>>>,

    pub paused: bool,
    pub paused_pending_step: u64,

    pub viewport: Option<Viewport>,

    pub last_report: Instant,
    pub frame_count: u64,
    pub tick_multiply: u64,
    pub perf_offset: i64,
}

impl ApplicationHandler for App {
    fn resumed(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
    ) {
        let window: Arc<_> = event_loop
            .create_window(
                WindowAttributes::default()
                    .with_inner_size(PhysicalSize::new(1200, 1200)),
            )
            .expect("failed to crate window")
            .into();

        self.viewport = Some(
            Viewport::new(window.clone(), &self.ctx, |ctx, surface| {
                Renderer::new(
                    ctx,
                    surface,
                    self.state.clone(),
                    self.command_queue.clone(),
                )
            })
            .expect("failed to create viewport"),
        );
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                info!("WindowEvent::CloseRequested");
                self.viewport = None;
                event_loop.exit();
            }
            WindowEvent::Resized(new_size) => {
                if let Some(viewport) = self.viewport.as_mut() {
                    viewport.resize(&self.ctx.device, new_size);
                    viewport.window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(viewport) = self.viewport.as_mut() {
                    let should_tick =
                        !self.paused || self.paused_pending_step > 0;
                    self.paused_pending_step =
                        self.paused_pending_step.saturating_sub(1);

                    if should_tick {
                        for _ in 0..self.tick_multiply {
                            viewport.renderer.update(&self.ctx);
                        }
                    }
                    viewport.render(&self.ctx).expect("failed to render");

                    self.frame_count += 1;
                    let elapsed =
                        self.last_report.elapsed().as_secs_f64();
                    if elapsed >= 1.0 {
                        let fps = self.frame_count as f64 / elapsed;
                        let tick_multiply = should_tick
                            .then_some(self.tick_multiply)
                            .unwrap_or_default();
                        println!(
                            "fps: {:.2}, tps: {:.2}, tick_multiply: {}",
                            fps,
                            fps * tick_multiply as f64,
                            tick_multiply,
                        );
                        self.frame_count = 0;
                        self.last_report = Instant::now();

                        if !self.paused {
                            if fps > 80.0 {
                                self.perf_offset += 1;
                            } else if fps < 60.0 {
                                if self.perf_offset > 0 {
                                    self.perf_offset = 0;
                                }
                                self.perf_offset -= 1;
                            } else {
                                self.perf_offset = 0;
                            }

                            if self.perf_offset >= 2 {
                                self.tick_multiply +=
                                    (self.perf_offset - 1) as u64;
                            } else if self.perf_offset <= -1 {
                                self.tick_multiply = self
                                    .tick_multiply
                                    .saturating_add_signed(
                                        self.perf_offset,
                                    )
                                    .max(1);
                            }
                        }
                    }

                    viewport.window.request_redraw();
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                if let Some(viewport) = self.viewport.as_mut() {
                    let mut state = self.state.lock().unwrap();
                    let size = viewport.window.inner_size();
                    state.mouse_pos[0] =
                        position.x as f32 / size.width as f32;
                    state.mouse_pos[1] =
                        1.0 - position.y as f32 / size.height as f32;
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let press = if state.is_pressed() { 1 } else { 0 };
                let r#type = match button {
                    MouseButton::Left => 1,
                    MouseButton::Right => 2,
                    _ => 0,
                };
                let mut state = self.state.lock().unwrap();
                state.mouse_press = press * r#type;
            }
            WindowEvent::KeyboardInput {
                event: keyboard_event,
                ..
            } if keyboard_event.state == ElementState::Released => {
                match keyboard_event.logical_key {
                    Key::Character(key) => match key.as_str() {
                        "r" => {
                            let mut cmd_queue =
                                self.command_queue.lock().unwrap();
                            cmd_queue.push_back(Command::Reset);
                        }
                        "c" => {
                            let mut state = self.state.lock().unwrap();
                            state.global_velocity_damping -= 1;
                            info!(
                                "global_velocity_damping: {}",
                                state.global_velocity_damping
                            );
                        }
                        "h" => {
                            let mut state = self.state.lock().unwrap();
                            state.global_velocity_damping += 1;
                            info!(
                                "global_velocity_damping: {}",
                                state.global_velocity_damping
                            );
                        }
                        "C" => {
                            let mut state = self.state.lock().unwrap();
                            state.global_velocity_damping -= 10;
                            info!(
                                "global_velocity_damping: {}",
                                state.global_velocity_damping
                            );
                        }
                        "H" => {
                            let mut state = self.state.lock().unwrap();
                            state.global_velocity_damping += 10;
                            info!(
                                "global_velocity_damping: {}",
                                state.global_velocity_damping
                            );
                        }
                        _ => {}
                    },
                    Key::Named(key) => match key {
                        NamedKey::ArrowUp => {
                            let mut state = self.state.lock().unwrap();
                            state.boundary_collision_factor += 1;
                            info!(
                                "boundary_collision_factor: {}",
                                state.boundary_collision_factor
                            );
                        }
                        NamedKey::ArrowDown => {
                            let mut state = self.state.lock().unwrap();
                            state.boundary_collision_factor = state
                                .boundary_collision_factor
                                .saturating_sub(1);
                            info!(
                                "boundary_collision_factor: {}",
                                state.boundary_collision_factor
                            );
                        }
                        NamedKey::ArrowRight => {
                            if self.paused {
                                info!("adding pending step");
                                self.paused_pending_step += 1;
                            }
                        }
                        NamedKey::Space => {
                            self.paused = !self.paused;
                            if !self.paused {
                                self.paused_pending_step = 0;
                            }
                            info!("paused: {}", self.paused);
                        }
                        _ => {}
                    },
                    _ => {
                        println!("{keyboard_event:?}")
                    }
                }
            }
            _ => {}
        }
    }
}
