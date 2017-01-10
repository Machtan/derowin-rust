extern crate dero;
extern crate rsdl2;
extern crate rsdl2_font;

use rsdl2_font::rusttype;
use rusttype::{FontCollection, Scale};
use std::thread;
use std::time::Duration;
use rsdl2::{Keycode, Keysym, keymod};
use std::process::{Command, Stdio};
use std::io::{Write, Read};
use dero::deromanize_escaped;
use std::fs::File;
use std::env;

const TEXT_POS: (i32, i32) = (10, 10);
const WINDOW_SIZE: (i32, i32) = (300, 40);
const MAX_FPS: u32 = 60;
const WINDOW_TITLE: &'static str = "Dero";
//const KOREAN_FONT_PATH: &'static str = "/Library/Fonts/NanumGothic.ttc";
const KOREAN_FONT_PATH: &'static str = "/Library/Fonts/NanumMyeongjo.ttc";
static FONT_POINT_SIZE: f32 = 18.0;

#[derive(Debug, Clone, Copy)]
pub enum DeroMode {
    Default,
    Lookup,
    Input,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Keymask {
    key: Keycode,
    ctrl: bool,
    gui: bool,
    shift: bool,
    alt: bool,
}

impl Keymask {
    #[inline]
    pub fn new(key: Keycode) -> Keymask {
        Keymask {
            key: key,
            ctrl: false,
            gui: false,
            shift: false,
            alt: false,
        }
    }
    
    #[inline]
    pub fn ctrl(mut self) -> Keymask {
        self.ctrl = true;
        self
    }
    
    #[inline]
    pub fn gui(mut self) -> Keymask {
        self.gui = true;
        self
    }
    
    #[inline]
    pub fn shift(mut self) -> Keymask {
        self.shift = true;
        self
    }
    
    #[inline]
    pub fn alt(mut self) -> Keymask {
        self.alt = true;
        self
    }
    
    #[inline]
    pub fn cmd(mut self) -> Keymask {
        self.gui = true;
        self
    }
    
    #[cfg(target_os = "macos")]
    #[inline]
    pub fn shortcut(mut self) -> Keymask {
        self.gui = true;
        self
    }
    
    #[cfg(not(target_os = "macos"))]
    #[inline]
    pub fn shortcut(mut self) -> Keymask {
        self.ctrl = true;
        self
    }
    
    #[inline]
    pub fn matches(&self, keysym: Keysym) -> bool {
        keysym.keycode  == self.key
        && self.ctrl    == keysym.mods.intersects(keymod::CTRL)
        && self.gui     == keysym.mods.intersects(keymod::GUI)
        && self.shift   == keysym.mods.intersects(keymod::SHIFT)
        && self.alt     == keysym.mods.intersects(keymod::ALT)
    }
}

pub fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    
    let context = rsdl2::init().everything().finish().expect("init failed");
    let mut event_context = context.events().expect("Event subsystem not initialized");
    let video_context = context.video().expect("Video subsystem not initialized");
    let window = video_context.build_window()
        .title(WINDOW_TITLE)
        .size(WINDOW_SIZE.0, WINDOW_SIZE.1)
        .center(true, true)
        .resizable()
        .finish()
        .expect("Could not create window");
    let renderer = window.build_renderer().finish().expect("Could not build renderer");
    //renderer.set_blend_mode(BlendMode::Blend);
    
    let mut file = File::open(KOREAN_FONT_PATH).expect("Could not open font file");
    let mut fontbuf = Vec::new();
    file.read_to_end(&mut fontbuf).expect("Could not read file");
    let collection = FontCollection::from_bytes(fontbuf);
    let font = collection.font_at(0).expect("No font at index 0");
    
    let clear_color = (255, 255, 255);
    let mut input = String::new();
    let mut mode = match args.get(0).map(|s| s.as_str()) {
        Some("input") => DeroMode::Input,
        Some("lookup") => DeroMode::Lookup,
        _ => DeroMode::Default,
    };
    match mode {
        DeroMode::Input => window.set_title(&format!("{} - Input", WINDOW_TITLE)),
        DeroMode::Lookup => window.set_title(&format!("{} - Look-up", WINDOW_TITLE)),
        DeroMode::Default => {}
    }
    let mut dirty = true;
    
    let m_input = Keymask::new(Keycode::I).shortcut().shift();
    let m_lookup = Keymask::new(Keycode::L).shortcut().shift();
    let m_enter = Keymask::new(Keycode::Return);
    let m_newline = Keymask::new(Keycode::Return).shift();
    let m_backspace = Keymask::new(Keycode::Backspace);
    let m_paste = Keymask::new(Keycode::V).shortcut();
    let m_clear = Keymask::new(Keycode::A).shortcut();

    'main: loop {
        use rsdl2::events::EventKind::*;
        for event in event_context.events() {
            match event.kind {
                Quit => {
                    break 'main;
                }
                TextInput(ref text) => {
                    input.push_str(text);
                    dirty = true;
                }
                // Drawing a little too much here, since window events
                // aren't too well-supported, but I'll want redraws on
                // resize.
                Window(ref _window) => {
                    dirty = true;
                }
                KeyDown(sym) | KeyRepeat(sym) => {
                    if m_backspace.matches(sym) {
                        if input.pop().is_some() {
                            dirty = true;
                        }
                    } 
                    else if m_enter.matches(sym) {
                        match mode {
                            DeroMode::Default => {
                                input.push('\n');
                                dirty = true;
                            }
                            DeroMode::Input => {
                                copy_to_clipboard(&deromanize_escaped(&input));
                                input.clear();
                                dirty = true;
                            }
                            DeroMode::Lookup => {
                                let converted = deromanize_escaped(&input);
                                look_up_word(&converted);
                                copy_to_clipboard(&converted);
                                input.clear();
                                dirty = true;
                            }
                        }
                    } 
                    else if m_newline.matches(sym) {
                        input.push('\n');
                        dirty = true;
                    } 
                    else if m_input.matches(sym) {
                        match mode {
                            DeroMode::Input => {
                                mode = DeroMode::Default;
                                window.set_title(WINDOW_TITLE);
                            }
                            _ => {
                                mode = DeroMode::Input;
                                window.set_title(&format!("{} - Input", WINDOW_TITLE));
                            }
                        }
                    } 
                    else if m_lookup.matches(sym) {
                        match mode {
                            DeroMode::Lookup => {
                                mode = DeroMode::Default;
                                window.set_title(WINDOW_TITLE);
                            }
                            _ => {
                                mode = DeroMode::Lookup;
                                window.set_title(&format!("{} - Look-up", WINDOW_TITLE));
                            }
                        }
                    }
                    else if m_paste.matches(sym) {
                        let clip = paste_from_clipboard();
                        if ! clip.is_empty() {
                            input.push_str(&clip);
                            dirty = true;
                        }
                    }
                    else if m_clear.matches(sym) {
                        if ! input.is_empty() {
                            copy_to_clipboard(&deromanize_escaped(&input));
                            input.clear();
                            dirty = true;
                        }
                    }
                }
                _ => {}
            }
        }
        if dirty {
            renderer.color(clear_color).clear().unwrap();
            
            let mut converted = deromanize_escaped(&input);
            converted.push('_');
            let line_gap = font.v_metrics(Scale::uniform(FONT_POINT_SIZE)).line_gap;
            let ascent = font.v_metrics(Scale::uniform(FONT_POINT_SIZE)).ascent;
            let line_skip = if line_gap != 0.0 {
                line_gap.ceil() as i32
            } else {
                (ascent * 1.25).ceil() as i32
            };
            //println!("Line gap: {}, Ascent: {}, Line skip: {}", line_gap, ascent, line_skip);
            for (i, line) in converted.lines().enumerate() {
                if line == "" {
                    continue;
                }
                let surf = rsdl2_font::render(&font, line, (0, 0, 0), FONT_POINT_SIZE).expect("Rendering failed");
                let tex = renderer.create_texture_from_surface(&surf).expect("Could not create texture");
                let y_indent = i as i32 * line_skip;
                let texpos = tex.rect_at(TEXT_POS.0, TEXT_POS.1 + y_indent);
                renderer.copy(&tex, None, Some(texpos)).unwrap();
            }
            
            renderer.present();
            dirty = false;
        }
        
        thread::sleep(Duration::from_millis(1000 / MAX_FPS as u64));
    }
}

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
fn paste_from_clipboard() -> String { String::new() }
