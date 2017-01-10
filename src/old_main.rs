extern crate dero;
extern crate sdl2;
extern crate sdl2_ttf;
extern crate glorious;

use glorious::{ResourceManager, FrameLimiter, Device, Sprite, Rect, Renderer};
use sdl2::render::BlendMode;
use sdl2::pixels::Color;
use sdl2::rect::Point;
use std::rc::Rc;
use std::process::{Command, Stdio};
use std::io::Write;

#[cfg(target_os = "macos")]
fn copy_to_clipboard(text: &str) {
    // println!("Copying '{}' to the clipboard...", text);
    println!("{}", text);
    let mut child = Command::new("/usr/bin/pbcopy")
        .arg(text)
        .stdin(Stdio::piped())
        .spawn()
        .expect("Could not run pbcopy");
    if let Some(ref mut stdin) = child.stdin {
        stdin.write_all(text.as_bytes())
            .expect("Could not write to pbcopy");
    } else {
        unreachable!();
    }
    child.wait().expect("Error while running pbcopy");
}

#[cfg(target_os = "macos")]
fn look_up_word(text: &str) {
    let url = format!("dict://{}", &text);
    Command::new("open")
        .arg(&url)
        .status()
        .expect("Could not open dictionary app");
}

#[cfg(target_os = "macos")]
fn paste_from_clipboard() -> String {
    let output = Command::new("pbpaste").output().expect("failed to run pbpaste");
    String::from_utf8_lossy(&output.stdout).to_string()
}

#[cfg(not(target_os = "macos"))]
fn copy_to_clipboard(text: &str) {}

#[cfg(not(target_os = "macos"))]
fn look_up_word(text: &str) {}

#[cfg(not(target_os = "macos"))]
fn paste_from_clipboard() -> String {}

fn deromanize_input(text: &str) -> (String, String) {
    for (i, ch) in text.char_indices().rev() {
        let end = i + ch.len_utf8();
        let part = &text[.. end];
        if let Ok(deromanized) = dero::convert(part) {
            let rest = &text[end..];
            return (deromanized, rest.to_string());
        }
    }
    (String::new(), text.to_string())
}

const WINDOW_SIZE: (u32, u32) = (500, 500);
const MAX_FPS: u32 = 60;
const WINDOW_TITLE: &'static str = "Dero Input-Helper";
//const KOREAN_FONT_PATH: &'static str = "/Library/Fonts/NanumGothic.ttc";
const KOREAN_FONT_PATH: &'static str = "/Library/Fonts/NanumMyeongjo.ttc";
const FONT_POINT_SIZE: u16 = 18;

enum DeroMode {
    Default,
    LookupAndClear,
}

fn main() {
    use sdl2::event::Event::*;

    let sdl_context = sdl2::init().expect("could not initialize SDL2");
    let video_subsystem = sdl_context.video().expect("could not initialize video subsystem");
    let font_context = sdl2_ttf::init().expect("Font init");

    let (ww, wh) = WINDOW_SIZE;
    let window = video_subsystem.window(WINDOW_TITLE, ww, wh)
        .allow_highdpi()
        .position_centered()
        .opengl()
        .build()
        .unwrap();

    let (w, h) = window.size();
    let (pw, ph) = window.drawable_size();
    let mut sdl2_renderer = window.renderer().present_vsync().build().unwrap();
    sdl2_renderer.set_blend_mode(BlendMode::Blend);

    let mut device = Device::new(sdl2_renderer);
    let mut renderer = device.create_renderer();
    let resources = ResourceManager::new(&device, &font_context);
    let mut limiter = FrameLimiter::new(MAX_FPS);
    let mut event_pump = sdl_context.event_pump().unwrap();
    let clear_color = (255, 255, 255);
    renderer.set_draw_color(clear_color);
    renderer.clear();
    renderer.present();
    
    let mut font = resources.font(KOREAN_FONT_PATH, FONT_POINT_SIZE);
    let mut entered_text = String::new();
    let mut changed = false;
    let mut dero_lines = Vec::new();
    let mut rem_lines = Vec::new();
    let mut mode = DeroMode::Default;
    
    'running: loop {
        let delta_time = 1.0 / MAX_FPS as f32;
        // Handle events
        for event in event_pump.poll_iter() {
            match event {
                Quit { .. } => {
                    break 'running;
                }
                KeyDown { keycode: Some(keycode), keymod, .. } => {
                    use sdl2::keyboard::Keycode::*;
                    use sdl2::keyboard::{LGUIMOD, RGUIMOD, LSHIFTMOD, RSHIFTMOD};
                    let cmd_down = keymod.intersects(LGUIMOD | RGUIMOD);
                    let shift_down = keymod.intersects(LSHIFTMOD | RSHIFTMOD);
                    match keycode {
                        Return => {
                            if let DeroMode::LookupAndClear = mode {
                                let (mut dero, rem) = deromanize_input(&entered_text);
                                dero.push_str(&rem);
                                copy_to_clipboard(&dero);
                                look_up_word(&dero);
                                entered_text.clear();
                                changed = true;
                            } else {
                                entered_text.push('\n');
                                changed = true;
                            }
                        }
                        C if cmd_down => {
                            let (mut dero, rem) = deromanize_input(&entered_text);
                            dero.push_str(&rem);
                            copy_to_clipboard(&dero);
                        }
                        X if cmd_down => {
                            let (mut dero, rem) = deromanize_input(&entered_text);
                            dero.push_str(&rem);
                            copy_to_clipboard(&dero);
                            entered_text.clear();
                            changed = true;
                        }
                        V if cmd_down => {
                            entered_text.push_str(&paste_from_clipboard());
                            changed = true;
                        }
                        A if cmd_down => {
                            entered_text.clear();
                            changed = true;
                        }
                        L if cmd_down && shift_down => {
                            if let DeroMode::LookupAndClear = mode {
                                mode = DeroMode::Default;
                                device.borrow_window_mut().map(|mut w| w.set_title(WINDOW_TITLE));
                                println!("Lookup Mode: OFF");
                            } else {
                                mode = DeroMode::LookupAndClear;
                                device.borrow_window_mut().map(|mut w| {
                                    w.set_title(&format!("{} - Lookup mode", WINDOW_TITLE))
                                });
                                println!("Lookup Mode: ON");
                            }
                        }
                        L if cmd_down => {
                            let (mut dero, rem) = deromanize_input(&entered_text);
                            dero.push_str(&rem);
                            copy_to_clipboard(&dero);
                            look_up_word(&dero);
                        }
                        Backspace => {
                            changed = entered_text.pop().is_some();
                        }
                        _ => {}
                    }
                }
                TextInput { text, .. } => {
                    entered_text.push_str(&text);
                    changed = true;
                }
                _ => {}
            }
        }
        
        // Updates go here
        if changed {
            changed = false;
            let (dero, rem) = deromanize_input(&entered_text);
            //println!("Text: '{}' | '{}'", dero, rem);
            //println!("Dero lines: {}", dero.lines().count());
            dero_lines.clear();
            rem_lines.clear();
            if ! dero.is_empty() {
                for line in dero.lines() {
                    if ! line.is_empty() {
                        let surf = font.render(line).blended(Color::RGB(0, 0, 0)).expect("Could not render text");
                        let tex = device.create_texture_from_surface(surf).expect("Could not create font texture");
                        let sprite = Sprite::new::<Rect>(Rc::new(tex), None);
                        dero_lines.push(Some(sprite));
                    } else {
                        dero_lines.push(None);
                    }
                }
                if dero.ends_with("\n") {
                    dero_lines.push(None);
                }
            }
            if ! rem.is_empty() {
                for line in rem.lines() {
                    if ! line.is_empty() {
                        let surf = font.render(line).blended(Color::RGB(255, 0, 0)).expect("Could not render text");
                        let tex = device.create_texture_from_surface(surf).expect("Could not create font texture");
                        let sprite = Sprite::new::<Rect>(Rc::new(tex), None);
                        rem_lines.push(Some(sprite));
                    } else {
                        rem_lines.push(None);
                    }
                }
                if rem.ends_with("\n") {
                    rem_lines.push(None);
                }
            }
        }
        
        // Clear the screen
        renderer.set_draw_color(clear_color);
        renderer.clear();
        
        // Start rendering
        const X_INDENT: i32 = 50;
        const Y_INDENT: i32 = 50;
        let mut x = X_INDENT;
        let mut y = Y_INDENT;
        for (i, sprite) in dero_lines.iter().enumerate() {
            x = X_INDENT;
            if let Some(ref sprite) = *sprite {
                renderer.set_draw_color((0, 255, 0));
                let debug_rect = sprite.rect.moved_to(x, y);
                //renderer.draw_rect(debug_rect).unwrap();
                sprite.render(&mut renderer, x, y, None);
                if i != dero_lines.len() - 1 {
                    y += font.recommended_line_spacing();
                } else {
                    x = X_INDENT + sprite.rect.width as i32;
                }
            } else if i != dero_lines.len() - 1 {
                y += font.recommended_line_spacing();
            }
        }
        for (i, sprite) in rem_lines.iter().enumerate() {
            if let Some(ref sprite) = *sprite {
                sprite.render(&mut renderer, x, y, None);
                x += sprite.rect.width as i32;
            }
            if i != rem_lines.len() - 1 {
                x = X_INDENT;
                y += font.recommended_line_spacing();
            }
        }
        let ul_y = y + font.recommended_line_spacing();
        let ul_start = Point::new(x, ul_y);
        let ul_end = Point::new(x + (FONT_POINT_SIZE/2) as i32, ul_y);
        renderer.set_draw_color((255, 0, 0));
        renderer.draw_line(ul_start, ul_end).unwrap();
        
        // Finish rendering
        renderer.present();
        
        limiter.limit();
    }
}
