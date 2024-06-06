#![warn(missing_debug_implementations)]

use std::{io::Cursor, num::NonZeroU32, path::Path, sync::Arc, time::Instant};

use image::{DynamicImage, ImageBuffer, ImageFormat, Luma};
use rand::{rngs::SmallRng, Rng, SeedableRng};
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use softbuffer::{Context, Surface};
use tokio::task::JoinSet;
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalSize},
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

#[derive(Debug, Clone)]
struct Board {
    pub data: Vec<bool>,
    data_out: Vec<bool>,

    width: usize,
    height: usize,
}

impl Board {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            data: vec![false; width * height],
            data_out: vec![false; width * height],

            width,
            height,
        }
    }

    pub fn rand(&mut self, seed: u64, probability: f64) {
        let mut rng = SmallRng::seed_from_u64(seed);
        for ele in self.data.iter_mut() {
            *ele = rng.gen_bool(probability);
        }
    }

    pub fn get(&self, x: usize, y: usize) -> bool {
        self.data[self.coord_to_idx(x, y)]
    }

    #[allow(unused)]
    pub fn set(&mut self, x: usize, y: usize, value: bool) {
        let idx = self.coord_to_idx(x, y);
        self.data_out[idx] = value;
    }

    fn coord_to_idx(&self, x: usize, y: usize) -> usize {
        debug_assert_eq!(self.data.len(), self.width * self.height);
        let idx = self.width * y + x;
        debug_assert!(idx < self.data.len());
        idx
    }

    fn idx_to_coord(&self, idx: usize) -> (usize, usize) {
        debug_assert!(idx < self.data.len());
        (idx % self.width, idx / self.width)
    }

    pub fn count_neighbors(&self, x: usize, y: usize) -> usize {
        const OFFSETS: [(isize, isize); 8] = [
            (-1, -1),
            (0, -1),
            (1, -1),
            (-1, 0),
            (1, 0),
            (-1, 1),
            (0, 1),
            (1, 1),
        ];

        OFFSETS
            .into_iter()
            .filter(|&(x_off, y_off)| {
                match (x.checked_add_signed(x_off), y.checked_add_signed(y_off)) {
                    (Some(x), Some(y)) if x < self.width && y < self.height => self.get(x, y),
                    _ => false,
                }
            })
            .count()
    }

    pub fn update(&mut self) {
        self.data = self
            .data
            .par_iter()
            .enumerate()
            .map(|(idx, &v)| {
                let (x, y) = self.idx_to_coord(idx);
                match self.count_neighbors(x, y) {
                    2 => v,
                    3 => true,
                    _ => false,
                }
            })
            .collect();

        // for y in 0..self.height {
        //     for x in 0..self.width {
        //         let v = self.get(x, y);
        //         let start = Instant::now();
        //         let v = match self.count_neighbors(x, y) {
        //             2 => v,
        //             3 => true,
        //             _ => false,
        //         };
        //         dbg!(start.elapsed());
        //         self.set(x, y, v);
        //     }
        // }
        //
        // std::mem::swap(&mut self.data, &mut self.data_out);
    }

    pub fn to_img(&self) -> DynamicImage {
        let img = ImageBuffer::<Luma<u8>, _>::from_vec(
            self.width as u32,
            self.height as u32,
            self.data
                .iter()
                .map(|&v| if v { 255_u8 } else { 0 })
                .collect::<Vec<_>>(),
        )
        .expect("failed to create img");

        DynamicImage::ImageLuma8(img)
    }
}

struct AppState {
    pub window: Arc<Window>,
    pub surface: Surface<Arc<Window>, Arc<Window>>,

    pub board: Board,

    print_interval_secs: f64,
    last_print: Instant,
    last_count: u64,
}

impl AppState {
    pub fn update_board_to_size(&mut self) {
        let PhysicalSize { width, height } = self.window.inner_size();
        self.board = Board::new(width as usize, height as usize);
        self.board.rand(0, 0.5);
        self.surface
            .resize(
                NonZeroU32::new(width.max(1)).unwrap(),
                NonZeroU32::new(height.max(1)).unwrap(),
            )
            .expect("failed to resize surface");
    }

    pub fn draw(&mut self) {
        let mut buffer = self
            .surface
            .buffer_mut()
            .expect("failed to get draw buffer");

        for (idx, v) in self.board.data.iter().enumerate() {
            buffer[idx] = if *v { 0x00FFFFFF } else { 0 };
        }

        buffer.present().expect("failed to present draw result");
    }

    pub fn update_fps_counter(&mut self) {
        self.last_count += 1;
        let elapsed = self.last_print.elapsed().as_secs_f64();
        if elapsed >= self.print_interval_secs {
            let fps = self.last_count as f64 / elapsed;
            let frametime = 1000.0 / fps;
            println!("fps: {fps: >8.2}, frametime: {frametime: >6.2}ms");

            self.last_print = Instant::now();
            self.last_count = 0;
        }
    }
}

struct GameOfLife {
    state: Option<AppState>,
}

impl ApplicationHandler for GameOfLife {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window: Arc<Window> = event_loop
            .create_window(
                Window::default_attributes().with_inner_size(LogicalSize::new(1000, 1000)),
            )
            .expect("failed to create window")
            .into();
        let context = Context::new(window.clone()).unwrap();
        let surface = Surface::new(&context, window.clone()).unwrap();

        let mut state = AppState {
            window,
            surface,

            board: Board::new(1, 1),

            print_interval_secs: 0.1,
            last_print: Instant::now(),
            last_count: 0,
        };
        state.update_board_to_size();

        self.state = Some(state);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        if let Some(state) = self.state.as_mut() {
            match event {
                WindowEvent::Resized(_) => {
                    state.update_board_to_size();
                }
                WindowEvent::RedrawRequested => {
                    state.board.update();
                    state.draw();
                    state.update_fps_counter();
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

#[tokio::main]
async fn main() {
    let event_loop = EventLoop::new().expect("failed to create event loop");

    let mut app = GameOfLife { state: None };

    event_loop
        .run_app(&mut app)
        .expect("failed to run application");

    // run_gen_image().await;
}

#[allow(unused)]
async fn run_gen_image() {
    let output = Path::new("./output");

    println!("init");
    let mut board = Board::new(1920, 1080);
    board.rand(0, 0.8);

    println!("start");
    let mut join_set = JoinSet::new();
    let print_interval = 0.2;
    let mut last_print = Instant::now();
    let mut last_count = 0_u64;
    for i in 0..5000 {
        let start = Instant::now();

        board.update();
        let update = start.elapsed();

        let img = board.to_img();
        let convert = start.elapsed() - update;

        {
            let mut writer = Cursor::new(Vec::<u8>::new());
            img.write_to(&mut writer, ImageFormat::Png)
                .expect("failed to encode image");

            let path = output.join(format!("{i:0>5}.png"));
            let fut = async {
                tokio::fs::write(path, writer.into_inner())
                    .await
                    .expect("failed to write image");
            };
            join_set.spawn(fut);

            // if join_set.len() > 50 {
            //     while join_set.len() > 10 && join_set.join_next().await.is_some() {}
            // }
        }
        #[allow(unused)]
        let save = start.elapsed() - update - convert;

        // dbg!(start.elapsed());
        dbg!(update);
        dbg!(convert);
        dbg!(save);

        last_count += 1;
        let elapsed = last_print.elapsed().as_secs_f64();
        if elapsed >= print_interval {
            let fps = last_count as f64 / elapsed;
            let frametime = 1000.0 / fps;
            println!("fps: {fps: >8.2}, frametime: {frametime: >6.2}ms");

            last_print = Instant::now();
            last_count = 0;
        }
    }

    while join_set.join_next().await.is_some() {}
}
