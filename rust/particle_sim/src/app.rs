use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
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

    pub viewport: Option<Viewport>,
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
                    //let start = std::time::Instant::now();
                    viewport.renderer.update(&self.ctx);
                    //let update = start.elapsed();
                    viewport.render(&self.ctx).expect("failed to render");
                    //let render = start.elapsed() - update;
                    //println!(
                    //    "update: {: >8.2?}, render: {: >8.2?}",
                    //    update, render
                    //);
                    //while !self
                    //    .ctx
                    //    .device
                    //    .poll(wgpu::MaintainBase::Poll)
                    //    .is_queue_empty()
                    //{}
                    if !self.paused {
                        viewport.window.request_redraw();
                    }
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
                            if let Some(viewport) = self.viewport.as_ref()
                            {
                                viewport.window.request_redraw();
                                info!("requesting new frame");
                            }
                        }
                        NamedKey::Space => {
                            self.paused = !self.paused;
                            if !self.paused {
                                if let Some(viewport) =
                                    self.viewport.as_ref()
                                {
                                    viewport.window.request_redraw();
                                }
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
