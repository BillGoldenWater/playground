use core::f64;
use std::{collections::HashMap, num::NonZeroU32, sync::Arc};

use anyhow::Context;
use grammar::{Grammer, Rule, RuleId, RuleItem, RuleNode};
use image::RgbImage;
use node::{Node, Value};
use rand::{random, rngs::StdRng, SeedableRng};
use rayon::iter::{
    IndexedParallelIterator, IntoParallelRefMutIterator, ParallelBridge,
    ParallelIterator,
};
use softbuffer::Surface;
use tracing::{debug_span, instrument, warn};
use tracing_subscriber::{fmt::format::FmtSpan, EnvFilter};
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalSize},
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Fullscreen, Window},
};

pub mod grammar;
pub mod node;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_span_events(FmtSpan::ENTER | FmtSpan::CLOSE)
        .init();

    let event_loop =
        EventLoop::new().expect("failed to create event loop");

    let mut app = RandomArt { state: None };

    event_loop
        .run_app(&mut app)
        .expect("failed to run application");

    //let mut img = RgbImage::new(512, 512);
    //for _ in 0..100 {
    //    let seed = random::<u64>();
    //    // 17959246647187379579
    //    gen_for_seed(seed, &grammar, &mut img)
    //        .context("failed to generate")?;
    //}
    //gen_for_seed(
    //    &mut img,
    //    &grammar,
    //    4573230301832827450,
    //    (-1.0, -1.0),
    //    (2.0, 2.0),
    //)
    //.context("failed to generate")?;

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct RenderParameters {
    save: bool,
    save_scaled: bool,

    seed: u64,

    offset: (f64, f64),
    dimensions: (f64, f64),
}

impl Default for RenderParameters {
    fn default() -> Self {
        Self {
            save: false,
            save_scaled: false,

            seed: 10409678234255179372,

            offset: (-1.0, -1.0),
            dimensions: (2.0, 2.0),
        }
    }
}

const CANVAS_SIZE: usize = 512;
struct AppState {
    window: Arc<Window>,
    surface: Surface<Arc<Window>, Arc<Window>>,

    grammar: Grammer,

    render_buf: Box<[[f64; 3]; CANVAS_SIZE * CANVAS_SIZE]>,

    param: RenderParameters,
    last_param: Option<RenderParameters>,
}

impl AppState {
    fn new(
        window: Arc<Window>,
        surface: Surface<Arc<Window>, Arc<Window>>,
    ) -> Self {
        let mut rules = HashMap::new();
        let rule_ref = |id: u64| Box::new(RuleNode::Rule(RuleId(id)));

        rules.insert(
            RuleId(0),
            Rule {
                items: vec![RuleItem {
                    a: RuleNode::Rgb(
                        rule_ref(2),
                        rule_ref(2),
                        rule_ref(2),
                    ),
                    weight: 1.0,
                }],
            },
        );
        rules.insert(
            RuleId(1),
            Rule {
                items: vec![
                    RuleItem {
                        a: RuleNode::Lit(-1.0..=1.0),
                        weight: 1.0,
                    },
                    RuleItem {
                        a: RuleNode::X,
                        weight: 1.0,
                    },
                    RuleItem {
                        a: RuleNode::Y,
                        weight: 1.0,
                    },
                    RuleItem {
                        a: RuleNode::Sqrt(
                            RuleNode::Add(
                                RuleNode::Pow(
                                    RuleNode::Sub(
                                        RuleNode::Const(0.0).into(),
                                        RuleNode::Y.into(),
                                    )
                                    .into(),
                                    RuleNode::Const(2.0).into(),
                                )
                                .into(),
                                RuleNode::Pow(
                                    RuleNode::Sub(
                                        RuleNode::Const(0.0).into(),
                                        RuleNode::X.into(),
                                    )
                                    .into(),
                                    RuleNode::Const(2.0).into(),
                                )
                                .into(),
                            )
                            .into(),
                        ),
                        weight: 1.0,
                    },
                ],
            },
        );
        rules.insert(
            RuleId(2),
            Rule {
                items: vec![
                    RuleItem {
                        a: *rule_ref(1),
                        weight: 1.0 / 4.0,
                    },
                    RuleItem {
                        a: RuleNode::Add(rule_ref(2), rule_ref(2)),
                        weight: 3.0 / 8.0,
                    },
                    RuleItem {
                        a: RuleNode::Sub(rule_ref(2), rule_ref(2)),
                        weight: 3.0 / 8.0,
                    },
                    RuleItem {
                        a: RuleNode::Mul(rule_ref(2), rule_ref(2)),
                        weight: 3.0 / 8.0,
                    },
                    RuleItem {
                        a: RuleNode::Div(rule_ref(2), rule_ref(2)),
                        weight: 3.0 / 8.0,
                    },
                    RuleItem {
                        a: RuleNode::Mod(rule_ref(2), rule_ref(2)),
                        weight: 3.0 / 8.0,
                    },
                    RuleItem {
                        a: RuleNode::Sin(rule_ref(2)),
                        weight: 3.0 / 8.0,
                    },
                ],
            },
        );
        let grammar = Grammer { rules };

        let render_buf =
            Box::new([Default::default(); CANVAS_SIZE * CANVAS_SIZE]);

        Self {
            window,
            surface,
            grammar,
            render_buf,
            param: RenderParameters::default(),
            last_param: None,
        }
    }

    pub fn on_resize(&mut self) {
        let PhysicalSize { width, height } = self.window.inner_size();
        self.surface
            .resize(
                NonZeroU32::new(width.max(1)).unwrap(),
                NonZeroU32::new(height.max(1)).unwrap(),
            )
            .expect("failed to resize surface");
    }

    #[instrument(level = "debug", skip(self))]
    pub fn update(&mut self) {
        let need_update = if let Some(ref last) = self.last_param {
            *last != self.param
        } else {
            true
        };

        if need_update {
            self.render();
        }

        let span = debug_span!("scaling").entered();
        let PhysicalSize { width, height } = self.window.inner_size();
        let mut buf = self
            .surface
            .buffer_mut()
            .expect("failed to get surface buffer");

        if (width * height) as usize != buf.len() {
            warn!("window dimention and buffer size didn't match, skipping render");
            return;
        }

        let size_f = CANVAS_SIZE as f64;
        let x_scaler = size_f / width as f64;
        let y_scaler = size_f / height as f64;
        buf.par_iter_mut().enumerate().for_each(|(idx, px)| {
            let x = idx as u32 % width;
            let y = idx as u32 / width;
            let x = (x as f64 * x_scaler) as usize;
            let y = (y as f64 * y_scaler) as usize;

            let v = Value::from(self.render_buf[y * CANVAS_SIZE + x]);
            *px = u32::from_be_bytes(v.to_argb8());
        });
        drop(span);

        let span = debug_span!("present").entered();
        buf.present().expect("failed to present buffer");
        drop(span);

        self.last_param = Some(self.param);
    }

    #[instrument(level = "debug", skip(self))]
    pub fn render(&mut self) {
        //let RenderParameters {
        //    offset, dimensions, ..
        //} = self.param;
        //if offset.0 < -1.0
        //    || offset.1 < -1.0
        //    || dimensions.0 + offset.0 > 1.0
        //    || dimensions.1 + offset.1 > 1.0
        //{
        //    println!("param {:#?}", self.param);
        //    println!("param out of bounds, restoring");
        //    if let Some(last) = self.last_param {
        //        self.param = last;
        //    } else {
        //        self.param = RenderParameters::default();
        //    }
        //}

        println!("rendering {:#?}", self.param);

        let RenderParameters {
            save,
            save_scaled,

            seed,

            offset,
            dimensions,
        } = self.param;
        if save || save_scaled {
            let mut img = RgbImage::new(1024, 1024);

            if save {
                println!("saving original");
                let result = gen_for_seed(
                    &mut img,
                    &self.grammar,
                    seed,
                    (-1.0, -1.0),
                    (2.0, 2.0),
                    "-1024",
                );
                if let Err(err) = result {
                    eprintln!("failed to save result: {err:?}");
                }
            }
            if save_scaled {
                println!("saving scaled");
                let result = gen_for_seed(
                    &mut img,
                    &self.grammar,
                    seed,
                    offset,
                    dimensions,
                    "-1024-scaled",
                );
                if let Err(err) = result {
                    eprintln!("failed to save result: {err:?}");
                }
            }

            self.param.save = false;
            self.param.save_scaled = false;
        }
        let mut rng = StdRng::seed_from_u64(seed);
        let expr = self.grammar.gen(&mut rng, RuleId(0), 12);

        let size = CANVAS_SIZE as u32;
        let size_f = size as f64;
        self.render_buf.par_iter_mut().enumerate().for_each(
            |(idx, px)| {
                let x = idx as u32 % size;
                let y = idx as u32 / size;
                let x = x as f64 / size_f;
                let y = y as f64 / size_f;

                let x = x * dimensions.0 + offset.0;
                let y = y * dimensions.0 + offset.1;
                let v = expr.eval(x, y);
                *px = v.to_rgb();
            },
        );
    }
}

struct RandomArt {
    state: Option<AppState>,
}

impl RandomArt {
    pub fn close(&mut self, event_loop: &ActiveEventLoop) {
        self.state = None;
        event_loop.exit();
    }
}

const INIT_SIZE: (u32, u32) = (512, 512);

impl ApplicationHandler for RandomArt {
    fn resumed(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
    ) {
        let (width, height) = INIT_SIZE;
        let window: Arc<_> = event_loop
            .create_window(
                Window::default_attributes()
                    .with_inner_size(LogicalSize::new(width, height)),
            )
            .expect("failed to create window")
            .into();
        let context = softbuffer::Context::new(window.clone()).unwrap();
        let surface = Surface::new(&context, window.clone()).unwrap();

        let mut state = AppState::new(window, surface);
        state.on_resize();

        self.state = Some(state);
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        if let Some(state) = self.state.as_mut() {
            match event {
                WindowEvent::Resized(_) => {
                    state.on_resize();
                }
                WindowEvent::RedrawRequested => {
                    state.update();
                }
                WindowEvent::KeyboardInput { event, .. } => {
                    if event.state != ElementState::Released {
                        return;
                    }
                    fn update_off(
                        param: &mut RenderParameters,
                        dx: f64,
                        dy: f64,
                    ) {
                        param.offset.0 += dx;
                        param.offset.1 += dy;
                    }
                    fn do_move(
                        param: &mut RenderParameters,
                        dir: (f64, f64),
                        update_off: &impl Fn(&mut RenderParameters, f64, f64),
                    ) {
                        let scaler = 0.1;
                        update_off(
                            param,
                            dir.0 * (scaler * param.dimensions.0).abs(),
                            dir.1 * (scaler * param.dimensions.1).abs(),
                        );
                    }
                    fn do_zoom(
                        param: &mut RenderParameters,
                        scale_up: bool,
                        update_off: &impl Fn(&mut RenderParameters, f64, f64),
                    ) {
                        let scaler = 1.5;
                        let scaler =
                            if scale_up { scaler } else { 1.0 / scaler };

                        let (w, h) = param.dimensions;
                        let (new_w, new_h) = (w * scaler, h * scaler);
                        param.dimensions = (new_w, new_h);
                        let (dw, dh) = (new_w - w, new_h - h);
                        update_off(
                            param,
                            dw / 2.0 * -1.0,
                            dh / 2.0 * -1.0,
                        )
                    }
                    match event.physical_key {
                        PhysicalKey::Code(KeyCode::KeyR) => {
                            state.param.seed = random::<u64>();
                        }
                        PhysicalKey::Code(KeyCode::Space) => {
                            //let _ = state.window.request_inner_size(
                            //    LogicalSize::new(
                            //        INIT_SIZE.0,
                            //        INIT_SIZE.0,
                            //    ),
                            //);
                            let default = RenderParameters::default();
                            state.param.offset = default.offset;
                            state.param.dimensions = default.dimensions;
                        }
                        PhysicalKey::Code(KeyCode::KeyF) => {
                            if state.window.fullscreen().is_none() {
                                state.window.set_fullscreen(Some(
                                    Fullscreen::Borderless(None),
                                ));
                            } else {
                                state.window.set_fullscreen(None);
                            }
                        }
                        PhysicalKey::Code(KeyCode::KeyQ) => {
                            self.close(event_loop);
                            return;
                        }
                        PhysicalKey::Code(KeyCode::Escape) => {
                            if state.window.fullscreen().is_some() {
                                state.window.set_fullscreen(None);
                            } else {
                                self.close(event_loop);
                                return;
                            }
                        }
                        // zooming and moving
                        PhysicalKey::Code(KeyCode::KeyU) => {
                            do_zoom(&mut state.param, true, &update_off);
                        }
                        PhysicalKey::Code(KeyCode::KeyD) => {
                            do_zoom(&mut state.param, false, &update_off);
                        }
                        PhysicalKey::Code(KeyCode::KeyH) => {
                            do_move(
                                &mut state.param,
                                (-1.0, 0.0),
                                &update_off,
                            );
                        }
                        PhysicalKey::Code(KeyCode::KeyJ) => {
                            do_move(
                                &mut state.param,
                                (0.0, 1.0),
                                &update_off,
                            );
                        }
                        PhysicalKey::Code(KeyCode::KeyK) => {
                            do_move(
                                &mut state.param,
                                (0.0, -1.0),
                                &update_off,
                            );
                        }
                        PhysicalKey::Code(KeyCode::KeyL) => {
                            do_move(
                                &mut state.param,
                                (1.0, 0.0),
                                &update_off,
                            );
                        }
                        // saving to disk
                        PhysicalKey::Code(KeyCode::KeyS) => {
                            state.param.save_scaled = true;
                        }
                        PhysicalKey::Code(KeyCode::KeyO) => {
                            state.param.save = true;
                        }
                        _ => return,
                    }
                    state.window.request_redraw();
                }
                WindowEvent::CloseRequested => {
                    self.close(event_loop);
                }
                _ => {}
            }
        }
    }
}

#[allow(unused)]
fn render(
    img: &mut RgbImage,
    expr: &Node,
    offset: (f64, f64),
    dimensions: (f64, f64),
) {
    assert!(offset.0 >= -1.0);
    assert!(offset.1 >= -1.0);
    assert!(dimensions.0 + offset.0 <= 1.0);
    assert!(dimensions.1 + offset.1 <= 1.0);

    let (width, height) = img.dimensions();
    img.enumerate_pixels_mut()
        .par_bridge()
        .for_each(|(x, y, px)| {
            let x = x as f64 / width as f64;
            let y = y as f64 / width as f64;
            let x = x * dimensions.0 + offset.0;
            let y = y * dimensions.0 + offset.0;
            let v = expr.eval(x, y);
            px.0 = v.to_rgb8();
        });
}

#[allow(unused)]
fn gen_for_seed(
    img: &mut RgbImage,
    grammar: &Grammer,
    seed: u64,
    offset: (f64, f64),
    dimensions: (f64, f64),
    tag: &str,
) -> anyhow::Result<()> {
    println!("{seed}");
    std::fs::create_dir_all("output")
        .context("failed to create output dir")
        .expect("test");
    let mut rng = StdRng::seed_from_u64(seed);
    let expr = grammar.gen(&mut rng, RuleId(0), 12);
    //println!("{expr:?}");
    println!("expr generated");
    assert!(offset.0 >= -1.0);
    assert!(offset.1 >= -1.0);
    assert!(dimensions.0 + offset.0 <= 1.0);
    assert!(dimensions.1 + offset.1 <= 1.0);

    render(img, &expr, offset, dimensions);
    println!("evaluated");

    img.save(format!("output/{seed}{tag}.png"))
        .context("failed to save image")?;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(format!("output/{seed}{tag}-grammar.cbor"))
        .context("failed to open file for save grammar")?;
    ciborium::into_writer(grammar, &mut file)
        .context("failed to save grammar")?;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(format!("output/{seed}{tag}-expr.cbor"))
        .context("failed to open file for save expr")?;
    ciborium::into_writer(&expr, &mut file)
        .context("failed to save expr")?;
    println!("saved");

    Ok(())
}
