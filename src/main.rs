use env_logger;
use font8x8::{UnicodeFonts, BASIC_FONTS};
use log::{error, info};
use pixels::{Error, Pixels, SurfaceTexture};
use rayon::prelude::*;
use std::time::{Duration, Instant};
use winit::dpi::{LogicalSize, PhysicalPosition};
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
    max_round: usize,
    info: bool,
    rendering_time: Duration,
    min_scale: f64,
    max_scale: f64,
}

impl Mandelbrot {
    fn new() -> Self {
        Self {
            drawn: false,
            center_x: -0.7,
            center_y: 0.0,
            scale: 0.005,
            max_round: 512,
            info: true,
            rendering_time: Duration::ZERO,
            min_scale: f64::EPSILON,
            max_scale: 0.1,
        }
    }

    fn request_redraw(&mut self) {
        self.drawn = false;
    }

    fn move_center(&mut self, x: f64, y: f64) {
        self.center_x += x * self.scale;
        self.center_y += y * self.scale;
        info!("center ({}, {})", self.center_x, self.center_y);
    }

    fn set_center(&mut self, x: f64, y: f64) {
        self.center_x += (x - (WINDOW_WIDTH as f64 / 2.0)) * self.scale;
        self.center_y += ((WINDOW_HEIGHT as f64 / 2.0) - y) * self.scale;
        info!("center ({}, {})", self.center_x, self.center_y);
    }

    fn zoom(&mut self, in_out: f64) -> bool {
        self.scale = self.scale * 1.07_f64.powf(-1.0 * in_out);
        self.max_round = if self.scale > 0.000005 { 512 } else { 1024 };
        info!("scale {}, max_round {}", self.scale, self.max_round);

        if self.scale > self.max_scale {
            self.scale = self.max_scale;
            return false;
        }
        if self.scale < self.min_scale {
            info!("scale is smaller than machine epsilon: {}", self.scale);
            self.scale = self.min_scale;
            return false;
        }
        true
    }

    fn reset(&mut self) {
        self.drawn = false;
        self.center_x = -0.7;
        self.center_y = 0.0;
        self.scale = 0.005;
        self.max_round = 512;
        self.info = true;
        self.rendering_time = Duration::ZERO;
        self.min_scale = f64::EPSILON;
        self.max_scale = 0.1;
    }

    fn check_divergence(&self, pos_x: f64, pos_y: f64, max_round: usize) -> Option<usize> {
        if pos_x >= 2.0 || pos_y >= 2.0 {
            return Some(1);
        };

        let mut xn: f64 = 0.0;
        let mut yn: f64 = 0.0;
        let mut xn_1_power: f64 = 0.0;
        let mut yn_1_power: f64 = 0.0;

        let mut round: usize = 1;
        while round < max_round {
            let xn_1 = xn;
            let yn_1 = yn;

            xn = xn_1_power - yn_1_power + pos_x;
            yn = 2.0 * xn_1 * yn_1 + pos_y;

            // faster than xn.powf(2.0) or nx.powi(2)
            xn_1_power = xn * xn;
            yn_1_power = yn * yn;

            if (xn_1_power + yn_1_power) >= 4.0 {
                return Some(round);
            }
            round += 1
        }
        return None;
    }

    fn text(&mut self, frame: &mut [u8], x: usize, y: usize, text_string: &str) {
        if y >= WINDOW_HEIGHT as usize || x >= WINDOW_WIDTH as usize {
            return;
        }
        for (i, chr) in text_string.chars().enumerate() {
            let mut frame_index = 4 * (x + (i * 9) + (y * WINDOW_WIDTH as usize));
            if chr != ' ' {
                if let Some(glyph) = BASIC_FONTS.get(chr) {
                    for bitmap in &glyph {
                        for bit in 0..8 {
                            match *bitmap & 1 << bit {
                                0 => (),
                                _ => {
                                    let font_white: [u8; 12] = [
                                        0xb0, 0xb0, 0xb0, 0xff, // white
                                        0x00, 0x00, 0x00, 0xff, // black
                                        0x00, 0x00, 0x00, 0xff, // black
                                    ];

                                    let pos = frame_index + (4 * bit);
                                    let pixel = &mut frame[pos..(pos + 12)];
                                    pixel.copy_from_slice(&font_white);

                                    let font_black: [u8; 12] = [
                                        0x00, 0x00, 0x00, 0xff, // black
                                        0x00, 0x00, 0x00, 0xff, // black
                                        0x00, 0x00, 0x00, 0xff, // black
                                    ];

                                    let pos = frame_index + (4 * (bit + WINDOW_WIDTH as usize));
                                    let pixel = &mut frame[pos..(pos + 12)];
                                    pixel.copy_from_slice(&font_black);

                                    let pos =
                                        frame_index + (4 * (bit + (2 * WINDOW_WIDTH) as usize));
                                    let pixel = &mut frame[pos..(pos + 12)];
                                    pixel.copy_from_slice(&font_black);
                                }
                            }
                        }
                        frame_index += 4 * WINDOW_WIDTH as usize;
                    }
                }
            }
        }
    }

    fn round_to_color(&self, round: usize) -> [u8; 4] {
        let section_size = 256_usize;
        let color_table: [(usize, usize, usize); 5] = [
            (0x00, 0x00, 0x80),
            (0x00, 0xff, 0x00),
            (0xff, 0xff, 0x00),
            (0x00, 0xff, 0xff),
            (0x00, 0x00, 0xff),
        ];

        let table_number = round / section_size;
        assert!(table_number + 1 < color_table.len());
        let color_index = round % section_size;

        let (r0, g0, b0) = color_table[table_number];
        let (r1, g1, b1) = color_table[table_number + 1];
        let interporation = |a, b| {
            (((a * (section_size - color_index) + b * color_index) / section_size) & 0xff) as u8
        };

        let r = interporation(r0, r1);
        let g = interporation(g0, g1);
        let b = interporation(b0, b1);

        [r, g, b, 0xff]
    }

    fn draw(&mut self, frame: &mut [u8]) {
        if self.drawn {
            return;
        }

        let start_time = Instant::now();
        let min_x = self.center_x - ((self.scale * WINDOW_WIDTH as f64) / 2.0);
        let max_y = self.center_y + ((self.scale * WINDOW_HEIGHT as f64) / 2.0);

        frame
            .par_chunks_exact_mut(4)
            .enumerate()
            .for_each(|(i, pixel)| {
                let x = min_x + ((i % WINDOW_WIDTH as usize) as f64) * self.scale;
                let y = max_y - ((i / WINDOW_WIDTH as usize) as f64) * self.scale;
                let rgba = match self.check_divergence(x, y, self.max_round) {
                    Some(round) => self.round_to_color(round),
                    None => [0x00, 0x00, 0x00, 0xff],
                };

                pixel.copy_from_slice(&rgba);
            });
        self.rendering_time = start_time.elapsed();
        let rendering_time_msg = format!(
            "rendering time: {}.{:04}[sec]",
            self.rendering_time.as_secs(),
            self.rendering_time.subsec_nanos() / 1000000
        );
        info!("{}", rendering_time_msg);
        if self.info {
            self.text(frame, 5, 5, format!("x: {}", self.center_x).as_str());
            self.text(frame, 5, 17, format!("y: {}", self.center_y).as_str());
            self.text(frame, 5, 29, format!("scale: {}", self.scale).as_str());
            self.text(frame, 5, 41, rendering_time_msg.as_str());
        }

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
    let mut pressed_pos_x = 0.0;
    let mut pressed_pos_y = 0.0;
    let mut pressed_time = Instant::now();
    let mut dobule_clicked = false;
    let mut shiftkey_pressed = false;
    let mut altkey_pressed = false;
    let mut auto_zoom_param = 0.0;

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
            if input.key_pressed(VirtualKeyCode::Q) || input.quit() {
                *control_flow = ControlFlow::Exit;
                return;
            }

            if let Some(size) = input.window_resized() {
                pixels.resize_surface(size.width, size.height);
            }

            if input.key_pressed(VirtualKeyCode::Space) {
                auto_zoom_param = 0.0;
                mandelbrot.reset();
                mandelbrot.request_redraw();
            }

            if input.mouse_pressed(0) {
                if let Some((x, y)) = input.mouse() {
                    let click_interval = pressed_time.elapsed().as_millis();
                    info!("click interval {}", click_interval);
                    let (pixel_x, pixel_y) = pixels
                        .window_pos_to_pixel((x, y))
                        .unwrap_or_else(|pos| pixels.clamp_pixel_pos(pos));
                    if pressed_time.elapsed().as_millis() < 700 {
                        dobule_clicked = true;
                        info!("double clicked");
                        mandelbrot.set_center(pixel_x as f64, pixel_y as f64);
                        mandelbrot.request_redraw();
                    } else {
                        dobule_clicked = false;
                        pressed_pos_x = pixel_x as f64;
                        pressed_pos_y = pixel_y as f64;
                    }
                    pressed_time = Instant::now();
                }
            }

            if input.mouse_released(0) {
                if dobule_clicked == false {
                    if let Some((x, y)) = input.mouse() {
                        let (released_pos_x, released_pos_y) = pixels
                            .window_pos_to_pixel((x, y))
                            .unwrap_or_else(|pos| pixels.clamp_pixel_pos(pos));
                        let (drag_vector_x, drag_vector_y) = (
                            pressed_pos_x - released_pos_x as f64,
                            -1.0 * (pressed_pos_y - released_pos_y as f64),
                        );
                        info!("drag: ({}, {})", drag_vector_x, drag_vector_y);
                        mandelbrot.move_center(drag_vector_x, drag_vector_y);
                        mandelbrot.request_redraw();
                    }
                }
            }

            let scroll_diff = input.scroll_diff();
            if scroll_diff.abs() != 0.0 {
                info!("scroll: {}", scroll_diff);
                mandelbrot.zoom(scroll_diff as f64);
                mandelbrot.request_redraw();
            }

            if input.key_pressed(VirtualKeyCode::LShift) {
                shiftkey_pressed = true;
            } else if input.key_released(VirtualKeyCode::LShift) {
                shiftkey_pressed = false;
            }

            if input.key_pressed(VirtualKeyCode::LAlt) {
                altkey_pressed = true;
            } else if input.key_released(VirtualKeyCode::LAlt) {
                altkey_pressed = false;
            }

            let calc_zoom_param = |direction: f64| {
                if altkey_pressed {
                    (0.2 * direction, true)
                } else if auto_zoom_param != 0.0 {
                    (0.0, true)
                } else if shiftkey_pressed {
                    (0.1 * direction, false)
                } else {
                    (3.0 * direction, false)
                }
            };

            let (zoom_param, auto_zoom_update) = if input.key_pressed(VirtualKeyCode::PageUp) {
                calc_zoom_param(1.0)
            } else if input.key_pressed(VirtualKeyCode::PageDown) {
                calc_zoom_param(-1.0)
            } else {
                (auto_zoom_param, false)
            };
            if zoom_param != 0.0 {
                let zoom_result = mandelbrot.zoom(zoom_param);
                if zoom_result == false {
                    auto_zoom_param = 0.0;
                }
                mandelbrot.request_redraw();
            }

            if input.key_pressed(VirtualKeyCode::Escape) {
                auto_zoom_param = 0.0;
            } else if auto_zoom_update {
                auto_zoom_param = zoom_param;
            }

            let (key_move, move_x, move_y) =
                if input.key_pressed(VirtualKeyCode::Up) || input.key_pressed(VirtualKeyCode::K) {
                    (true, 0.0, 10.0)
                } else if input.key_pressed(VirtualKeyCode::Down)
                    || input.key_pressed(VirtualKeyCode::J)
                {
                    (true, 0.0, -10.0)
                } else if input.key_pressed(VirtualKeyCode::Left)
                    || input.key_pressed(VirtualKeyCode::H)
                {
                    (true, -10.0, 0.0)
                } else if input.key_pressed(VirtualKeyCode::Right)
                    || input.key_pressed(VirtualKeyCode::L)
                {
                    (true, 10.0, 0.0)
                } else {
                    (false, 0.0, 0.0)
                };
            if key_move {
                let scale_factor = window.scale_factor();
                let center_p_pos = PhysicalPosition::new(move_x, move_y);
                let center_offset = center_p_pos.to_logical(scale_factor);
                mandelbrot.move_center(center_offset.x, center_offset.y);
                mandelbrot.request_redraw();
            }

            if input.key_pressed(VirtualKeyCode::I) {
                mandelbrot.info = !mandelbrot.info;
                mandelbrot.request_redraw();
            }

            if input.key_pressed(VirtualKeyCode::D) {
                println!();
                println!("x: {}", mandelbrot.center_x);
                println!("y: {}", mandelbrot.center_y);
                println!("scale: {}", mandelbrot.scale);
                println!(
                    "rendering time: {}.{:04}[sec]",
                    mandelbrot.rendering_time.as_secs(),
                    mandelbrot.rendering_time.subsec_nanos() / 1000000
                );
            }

            window.request_redraw();
        }
    });
}
