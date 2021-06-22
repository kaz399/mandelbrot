use env_logger;
use log::{error, info};
use pixels::{Error, Pixels, SurfaceTexture};
use std::time::Instant;
use winit::dpi::LogicalSize;
use winit::event::{Event, VirtualKeyCode};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;
use winit_input_helper::WinitInputHelper;

const WINDOW_WIDTH: u32 = 640;
const WINDOW_HEIGHT: u32 = 480;

struct Mandelbrot {
    drawn: bool,
    center_x: f64,
    center_y: f64,
    scale: f64,
}

impl Mandelbrot {
    fn new() -> Self {
        Self {
            drawn: false,
            //center_x: 0.250,
            center_x: -0.10,
            center_y: 0.0,
            scale: 0.005,
        }
    }

    fn request_redraw(&mut self) {
        self.drawn = false;
    }

    fn set_center(&mut self, x: f64, y: f64) {
        self.center_x += (x - (WINDOW_WIDTH as f64 / 2.0)) * self.scale;
        self.center_y += ((WINDOW_HEIGHT as f64 / 2.0) - y) * self.scale;
        info!("center ({}, {})", self.center_x, self.center_y);
    }

    fn zoom(&mut self, in_out: f64) {
        self.scale = self.scale * 2.0_f64.powf(-1.005 * in_out);
        info!("scale {}", self.scale);
    }

    fn reset(&mut self) {
        self.drawn = false;
        self.center_x = -0.10;
        self.center_y = -0.0;
        self.scale = 0.005;
    }

    fn check_divergence(&self, pos_x: f64, pos_y: f64, max_round: u32) -> Option<u32> {
        let mut xn: f64 = 0.0;
        let mut yn: f64 = 0.0;
        let mut xn_1_power: f64 = 0.0;
        let mut yn_1_power: f64 = 0.0;

        let mut round: u32 = 1;
        while round <= max_round {
            let xn_1 = xn;
            let yn_1 = yn;

            xn = xn_1_power - yn_1_power + pos_x;
            yn = 2.0 * xn_1 * yn_1 + pos_y;

            xn_1_power = xn.powf(2.0);
            yn_1_power = yn.powf(2.0);

            if (xn_1_power + yn_1_power) >= 4.0 {
                return Some(round);
            }
            round += 1
        }
        return None;
    }

    fn draw(&mut self, frame: &mut [u8]) {
        if self.drawn {
            return;
        }
        info!("draw start");
        let start = Instant::now();

        let min_x = self.center_x - ((self.scale * WINDOW_WIDTH as f64) / 2.0);
        let max_y = self.center_y + ((self.scale * WINDOW_HEIGHT as f64) / 2.0);

        for (i, pixel) in frame.chunks_exact_mut(4).enumerate() {
            let x = min_x + ((i % WINDOW_WIDTH as usize) as f64) * self.scale;
            let y = max_y - ((i / WINDOW_WIDTH as usize) as f64) * self.scale;

            let max_round = 128;
            let rgba = match self.check_divergence(x, y, max_round) {
                Some(round) => {
                    //info!("({},{}) {}", x, y, round);
                    if round <= u8::MAX as u32 {
                        [0x00, ((round * 2) as u8), 0x80, 0xff]
                    } else {
                        [0x00, 0xff, 0x80, 0xff]
                    }
                }
                None => [0x00, 0x00, 0x00, 0xff],
            };

            pixel.copy_from_slice(&rgba);
        }
        let end = start.elapsed();
        info!(
            "elapsed time {}.{:04}[sec]",
            end.as_secs(),
            end.subsec_nanos() / 1000000
        );

        self.drawn = true;
    }
}

fn main() -> Result<(), Error> {
    env_logger::init();
    let event_loop = EventLoop::new();
    let mut input = WinitInputHelper::new();
    let window = {
        let size = LogicalSize::new(WINDOW_WIDTH as f64, WINDOW_HEIGHT as f64);
        WindowBuilder::new()
            .with_title("Mandelbrot")
            .with_inner_size(size)
            .with_min_inner_size(size)
            .build(&event_loop)
            .unwrap()
    };

    let mut pixels = {
        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        Pixels::new(WINDOW_WIDTH, WINDOW_HEIGHT, surface_texture)?
    };

    let mut mandelbrot = Mandelbrot::new();

    event_loop.run(move |event, _, control_flow| {
        if let Event::RedrawRequested(_) = event {
            mandelbrot.draw(pixels.get_frame());
            if pixels
                .render()
                .map_err(|e| error!("pixels.render() failed: {}", e))
                .is_err()
            {
                *control_flow = ControlFlow::Exit;
                return;
            }
        }

        if input.update(&event) {
            if input.key_pressed(VirtualKeyCode::Escape) || input.quit() {
                *control_flow = ControlFlow::Exit;
                return;
            }

            if let Some(size) = input.window_resized() {
                pixels.resize_surface(size.width, size.height);
            }

            if input.key_pressed(VirtualKeyCode::Space) {
                mandelbrot.reset();
                mandelbrot.request_redraw();
            }

            if input.mouse_released(0) {
                if let Some((x, y)) = input.mouse() {
                    info!("mouse pos: ({}, {})", x, y);
                    mandelbrot.set_center(x as f64, y as f64);
                    mandelbrot.request_redraw();
                }
            }

            let scroll_diff = input.scroll_diff();
            if scroll_diff.abs() != 0.0 {
                info!("scroll: {}", scroll_diff);
                mandelbrot.zoom(scroll_diff as f64);
                mandelbrot.request_redraw();
            }

            window.request_redraw();
        }
    });
}
