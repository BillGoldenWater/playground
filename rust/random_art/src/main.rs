use core::f64;
use std::{collections::HashMap, num::NonZeroU32, sync::Arc};

use anyhow::Context;
use grammar::{Grammer, Rule, RuleId, RuleItem, RuleNode};
use image::RgbImage;
use node::Node;
use rand::{random, rngs::StdRng, SeedableRng};
use rayon::iter::{
    IndexedParallelIterator, IntoParallelRefMutIterator, ParallelBridge,
    ParallelIterator,
};
use softbuffer::Surface;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{ElementState, WindowEvent},
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

pub mod grammar;
pub mod node;

fn main() -> anyhow::Result<()> {
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

    height: u32,
    width: u32,

    seed: u64,

    offset: (f64, f64),
    dimensions: (f64, f64),
}

impl Default for RenderParameters {
    fn default() -> Self {
        Self {
            save: false,
            save_scaled: false,

            height: 1,
            width: 1,

            seed: 10409678234255179372,

            offset: (-1.0, -1.0),
            dimensions: (2.0, 2.0),
        }
    }
}

struct AppState {
    window: Arc<Window>,
    surface: Surface<Arc<Window>, Arc<Window>>,

    grammar: Grammer,

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

        Self {
            window,
            surface,
            grammar,
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
        self.param.width = width;
        self.param.height = height;
    }

    pub fn update(&mut self) {
        let need_update = if let Some(ref last) = self.last_param {
            *last != self.param
        } else {
            true
        };

        if !need_update {
            println!("ignored");
            return;
        }

        let mut buf = self
            .surface
            .buffer_mut()
            .expect("failed to get surface buffer");

        let RenderParameters {
            offset, dimensions, ..
        } = self.param;
        if offset.0 < -1.0
            || offset.1 < -1.0
            || dimensions.0 + offset.0 > 1.0
            || dimensions.1 + offset.1 > 1.0
        {
            println!("param {:#?}", self.param);
            println!("param out of bounds, restoring");
            if let Some(last) = self.last_param {
                self.param = last;
            } else {
                self.param = RenderParameters::default();
            }
        }

        println!("rendering {:#?}", self.param);

        let RenderParameters {
            save,
            save_scaled,

            height,
            width,

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

        buf.par_iter_mut().enumerate().for_each(|(idx, px)| {
            // argb
            let x = idx as u32 % width;
            let y = idx as u32 / height;
            let x = x as f64 / width as f64;
            let y = y as f64 / height as f64;
            let x = offset.0 + x * dimensions.0;
            let y = offset.1 + y * dimensions.1;
            let v = expr.eval(x, y);
            *px = u32::from_be_bytes(v.to_argb8());
        });

        buf.present().expect("failed to present buffer");

        self.last_param = Some(self.param);
    }
}

struct RandomArt {
    state: Option<AppState>,
}

const INIT_SIZE: (u32, u32) = (512, 512);
const MAX_SIZE: (u32, u32) = (1024, 1024);

impl ApplicationHandler for RandomArt {
    fn resumed(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
    ) {
        let (width, height) = INIT_SIZE;
        let window: Arc<_> = event_loop
            .create_window(
                Window::default_attributes()
                    .with_inner_size(PhysicalSize::new(width, height))
                    .with_max_inner_size(PhysicalSize::new(
                        MAX_SIZE.0, MAX_SIZE.1,
                    )),
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
                            let _ = state.window.request_inner_size(
                                PhysicalSize::new(
                                    INIT_SIZE.0,
                                    INIT_SIZE.0,
                                ),
                            );
                            let default = RenderParameters::default();
                            state.param.offset = default.offset;
                            state.param.dimensions = default.dimensions;
                        }
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
                    self.state = None;
                    event_loop.exit();
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
            let y = y as f64 / height as f64;
            let x = offset.0 + x * dimensions.0;
            let y = offset.1 + y * dimensions.1;
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
