// Copyright 2026 Yabe Kazuhiro
//
// Permission is hereby granted, free of charge, to any person obtaining a copy of this software
// and associated documentation files (the “Software”), to deal in the Software without
// restriction, including without limitation the rights to use, copy, modify, merge, publish,
// distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the
// Software is furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all copies or
// substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT
// NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
// NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM,
// DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

use font8x8::{UnicodeFonts, BASIC_FONTS};
use log::{error, info};
use pixels::{Error, Pixels, SurfaceTexture};
use rayon::prelude::*;
use std::sync::Arc;
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalPosition, LogicalSize, PhysicalPosition};
use winit::event::{DeviceEvent, DeviceId, MouseButton, StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::KeyCode;
use winit::window::{Window, WindowId};
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

struct App {
    input: WinitInputHelper,
    window: Option<Arc<Window>>,
    pixels: Option<Pixels<'static>>,
    mandelbrot: Mandelbrot,
    pressed_pos_x: f64,
    pressed_pos_y: f64,
    pressed_time: Instant,
    double_clicked: bool,
    shiftkey_pressed: bool,
    altkey_pressed: bool,
    auto_zoom_param: f64,
}

impl App {
    fn new() -> Self {
        Self {
            input: WinitInputHelper::new(),
            window: None,
            pixels: None,
            mandelbrot: Mandelbrot::new(),
            pressed_pos_x: 0.0,
            pressed_pos_y: 0.0,
            pressed_time: Instant::now(),
            double_clicked: false,
            shiftkey_pressed: false,
            altkey_pressed: false,
            auto_zoom_param: 0.0,
        }
    }

    fn create_window(event_loop: &ActiveEventLoop) -> Window {
        let size = LogicalSize::new(WINDOW_WIDTH as f64, WINDOW_HEIGHT as f64);
        event_loop
            .create_window(
                Window::default_attributes()
                    .with_title("Mandelbrot")
                    .with_inner_size(size)
                    .with_min_inner_size(size),
            )
            .unwrap()
    }

    fn render(&mut self, event_loop: &ActiveEventLoop) {
        let Some(pixels) = self.pixels.as_mut() else {
            return;
        };

        self.mandelbrot.draw(pixels.frame_mut());
        if pixels
            .render()
            .map_err(|e| error!("pixels.render() failed: {}", e))
            .is_err()
        {
            event_loop.exit();
        }
    }

    fn update(&mut self, event_loop: &ActiveEventLoop) {
        self.input.end_step();

        if self.input.key_pressed(KeyCode::KeyQ)
            || self.input.close_requested()
            || self.input.destroyed()
        {
            event_loop.exit();
            return;
        }

        if let Some(size) = self.input.window_resized() {
            if let Some(pixels) = self.pixels.as_mut() {
                if let Err(e) = pixels.resize_surface(size.width, size.height) {
                    error!("pixels.resize_surface() failed: {}", e);
                    event_loop.exit();
                    return;
                }
            }
        }

        if self.input.key_pressed(KeyCode::Space) {
            self.auto_zoom_param = 0.0;
            self.mandelbrot.reset();
            self.mandelbrot.request_redraw();
        }

        self.update_mouse();
        self.update_zoom();
        self.update_keyboard_move();
        self.update_info();

        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    fn cursor_pixel(&self) -> Option<(usize, usize)> {
        let pixels = self.pixels.as_ref()?;
        let cursor = self.input.cursor()?;
        Some(
            pixels
                .window_pos_to_pixel(cursor)
                .unwrap_or_else(|pos| pixels.clamp_pixel_pos(pos)),
        )
    }

    fn update_mouse(&mut self) {
        if self.input.mouse_pressed(MouseButton::Left) {
            if let Some((pixel_x, pixel_y)) = self.cursor_pixel() {
                let click_interval = self.pressed_time.elapsed().as_millis();
                info!("click interval {}", click_interval);
                if click_interval < 700 {
                    self.double_clicked = true;
                    info!("double clicked");
                    self.mandelbrot.set_center(pixel_x as f64, pixel_y as f64);
                    self.mandelbrot.request_redraw();
                } else {
                    self.double_clicked = false;
                    self.pressed_pos_x = pixel_x as f64;
                    self.pressed_pos_y = pixel_y as f64;
                }
                self.pressed_time = Instant::now();
            }
        }

        if self.input.mouse_released(MouseButton::Left) && !self.double_clicked {
            if let Some((released_pos_x, released_pos_y)) = self.cursor_pixel() {
                let (drag_vector_x, drag_vector_y) = (
                    self.pressed_pos_x - released_pos_x as f64,
                    -1.0 * (self.pressed_pos_y - released_pos_y as f64),
                );
                info!("drag: ({}, {})", drag_vector_x, drag_vector_y);
                self.mandelbrot.move_center(drag_vector_x, drag_vector_y);
                self.mandelbrot.request_redraw();
            }
        }
    }

    fn update_zoom(&mut self) {
        let (_scroll_x, scroll_y) = self.input.scroll_diff();
        if scroll_y != 0.0 {
            info!("scroll: {}", scroll_y);
            self.mandelbrot.zoom(scroll_y as f64);
            self.mandelbrot.request_redraw();
        }

        if self.input.key_pressed(KeyCode::ShiftLeft) {
            self.shiftkey_pressed = true;
        } else if self.input.key_released(KeyCode::ShiftLeft) {
            self.shiftkey_pressed = false;
        }

        if self.input.key_pressed(KeyCode::AltLeft) {
            self.altkey_pressed = true;
        } else if self.input.key_released(KeyCode::AltLeft) {
            self.altkey_pressed = false;
        }

        let calc_zoom_param = |direction: f64| {
            if self.altkey_pressed {
                (0.4 * direction, true)
            } else if self.auto_zoom_param != 0.0 {
                (0.0, true)
            } else if self.shiftkey_pressed {
                (0.1 * direction, false)
            } else {
                (3.0 * direction, false)
            }
        };

        let (zoom_param, auto_zoom_update) = if self.input.key_pressed(KeyCode::PageUp) {
            calc_zoom_param(1.0)
        } else if self.input.key_pressed(KeyCode::PageDown) {
            calc_zoom_param(-1.0)
        } else {
            (self.auto_zoom_param, false)
        };
        if zoom_param != 0.0 {
            let zoom_result = self.mandelbrot.zoom(zoom_param);
            if !zoom_result {
                self.auto_zoom_param = 0.0;
            }
            self.mandelbrot.request_redraw();
        }

        if self.input.key_pressed(KeyCode::Escape) {
            self.auto_zoom_param = 0.0;
        } else if auto_zoom_update {
            self.auto_zoom_param = zoom_param;
        }
    }

    fn update_keyboard_move(&mut self) {
        let (key_move, move_x, move_y) =
            if self.input.key_pressed(KeyCode::ArrowUp) || self.input.key_pressed(KeyCode::KeyK) {
                (true, 0.0, 10.0)
            } else if self.input.key_pressed(KeyCode::ArrowDown)
                || self.input.key_pressed(KeyCode::KeyJ)
            {
                (true, 0.0, -10.0)
            } else if self.input.key_pressed(KeyCode::ArrowLeft)
                || self.input.key_pressed(KeyCode::KeyH)
            {
                (true, -10.0, 0.0)
            } else if self.input.key_pressed(KeyCode::ArrowRight)
                || self.input.key_pressed(KeyCode::KeyL)
            {
                (true, 10.0, 0.0)
            } else {
                (false, 0.0, 0.0)
            };

        if key_move {
            if let Some(window) = self.window.as_ref() {
                let scale_factor = window.scale_factor();
                let center_p_pos = PhysicalPosition::new(move_x, move_y);
                let center_offset: LogicalPosition<f64> = center_p_pos.to_logical(scale_factor);
                self.mandelbrot
                    .move_center(center_offset.x, center_offset.y);
                self.mandelbrot.request_redraw();
            }
        }
    }

    fn update_info(&mut self) {
        if self.input.key_pressed(KeyCode::KeyI) {
            self.mandelbrot.info = !self.mandelbrot.info;
            self.mandelbrot.request_redraw();
        }

        if self.input.key_pressed(KeyCode::KeyD) {
            println!();
            println!("x: {}", self.mandelbrot.center_x);
            println!("y: {}", self.mandelbrot.center_y);
            println!("scale: {}", self.mandelbrot.scale);
            println!(
                "rendering time: {}.{:04}[sec]",
                self.mandelbrot.rendering_time.as_secs(),
                self.mandelbrot.rendering_time.subsec_nanos() / 1000000
            );
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let window = Arc::new(Self::create_window(event_loop));
        let window_size = window.inner_size();
        let surface_texture =
            SurfaceTexture::new(window_size.width, window_size.height, window.clone());
        let pixels = Pixels::new(WINDOW_WIDTH, WINDOW_HEIGHT, surface_texture).unwrap();

        self.window = Some(window);
        self.pixels = Some(pixels);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        if self.input.process_window_event(&event) {
            self.render(event_loop);
        }
    }

    fn device_event(&mut self, _: &ActiveEventLoop, _: DeviceId, event: DeviceEvent) {
        self.input.process_device_event(&event);
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.update(event_loop);
    }

    fn new_events(&mut self, _: &ActiveEventLoop, _: StartCause) {
        self.input.step();
    }
}

fn main() -> Result<(), Error> {
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.run_app(&mut App::new()).unwrap();
    Ok(())
}
