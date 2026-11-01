//! Integrated terminal: a real pseudo-terminal (ConPTY on Windows) running the user's shell,
//! with `vt100` interpreting its output and egui drawing the character grid.

use crate::theme;
use eframe::egui::{self, Color32, Event, EventFilter, Key, Modifiers, Sense};
use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

pub struct Terminal {
    parser: Arc<Mutex<vt100::Parser>>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    master: Box<dyn MasterPty + Send>,
    child: Box<dyn Child + Send + Sync>,
    exited: Arc<AtomicBool>,
    size: (u16, u16),
    pub shell: String,
}

fn default_shell() -> (String, Vec<String>) {
    #[cfg(windows)]
    {
        ("powershell.exe".into(), vec!["-NoLogo".into()])
    }
    #[cfg(not(windows))]
    {
        (
            std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into()),
            Vec::new(),
        )
    }
}

impl Terminal {
    pub fn spawn(context: &egui::Context, cwd: Option<&Path>) -> Result<Self, String> {
        let size = PtySize {
            rows: 24,
            cols: 100,
            pixel_width: 0,
            pixel_height: 0,
        };
        let pair = native_pty_system()
            .openpty(size)
            .map_err(|error| format!("could not open a terminal: {error}"))?;
        let (program, arguments) = default_shell();
        let mut command = CommandBuilder::new(&program);
        command.args(&arguments);
        if let Some(cwd) = cwd {
            command.cwd(cwd);
        }
        command.env("TERM", "xterm-256color");
        let child = pair
            .slave
            .spawn_command(command)
            .map_err(|error| format!("could not start {program}: {error}"))?;
        drop(pair.slave);

        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|error| format!("could not read the terminal: {error}"))?;
        let writer: Arc<Mutex<Box<dyn Write + Send>>> =
            Arc::new(Mutex::new(pair.master.take_writer().map_err(|error| {
                format!("could not write to the terminal: {error}")
            })?));
        let parser = Arc::new(Mutex::new(vt100::Parser::new(size.rows, size.cols, 2000)));
        let exited = Arc::new(AtomicBool::new(false));
        {
            let parser = parser.clone();
            let exited = exited.clone();
            let writer = writer.clone();
            let context = context.clone();
            std::thread::spawn(move || {
                let mut buffer = [0_u8; 8192];
                loop {
                    match reader.read(&mut buffer) {
                        Ok(0) | Err(_) => break,
                        Ok(count) => {
                            let chunk = &buffer[..count];
                            let mut parser = parser.lock().expect("terminal parser");
                            parser.process(chunk);
                            // Windows' pseudo-console asks "where is the cursor?" (ESC [ 6 n) and
                            // withholds all output until it is answered.
                            if chunk.windows(4).any(|window| window == b"[6n") {
                                let (row, column) = parser.screen().cursor_position();
                                let reply = format!("[{};{}R", row + 1, column + 1);
                                if let Ok(mut writer) = writer.lock() {
                                    let _ = writer.write_all(reply.as_bytes());
                                    let _ = writer.flush();
                                }
                            }
                            drop(parser);
                            context.request_repaint();
                        }
                    }
                }
                exited.store(true, Ordering::Relaxed);
                context.request_repaint();
            });
        }
        Ok(Self {
            parser,
            writer,
            master: pair.master,
            child,
            exited,
            size: (size.rows, size.cols),
            shell: program,
        })
    }

    pub fn has_exited(&self) -> bool {
        self.exited.load(Ordering::Relaxed)
    }

    pub fn send(&mut self, bytes: &[u8]) {
        if !self.has_exited()
            && let Ok(mut writer) = self.writer.lock()
        {
            let _ = writer.write_all(bytes);
            let _ = writer.flush();
        }
    }

    fn resize(&mut self, rows: u16, cols: u16) {
        if (rows, cols) == self.size || rows < 2 || cols < 8 {
            return;
        }
        self.size = (rows, cols);
        let _ = self.master.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        });
        self.parser
            .lock()
            .expect("terminal parser")
            .set_size(rows, cols);
    }

    /// Draw the grid and forward keyboard input. Returns true while the terminal has focus.
    pub fn show(&mut self, ui: &mut egui::Ui, request_focus: bool) -> bool {
        let font = egui::FontId::monospace(13.5);
        let (cell_height, cell_width) =
            ui.fonts_mut(|fonts| (fonts.row_height(&font), fonts.glyph_width(&font, 'M')));
        let available = ui.available_size();
        let cols = (available.x / cell_width).floor().max(1.0) as u16;
        let rows = (available.y / cell_height).floor().max(1.0) as u16;
        self.resize(rows, cols);

        let (rect, response) = ui.allocate_exact_size(available, Sense::click());
        if response.clicked() || request_focus {
            response.request_focus();
        }
        let focused = response.has_focus();
        if focused {
            ui.memory_mut(|memory| {
                memory.set_focus_lock_filter(
                    response.id,
                    EventFilter {
                        tab: true,
                        horizontal_arrows: true,
                        vertical_arrows: true,
                        escape: true,
                    },
                );
            });
            let events = ui.input(|input| input.events.clone());
            for event in events {
                match event {
                    Event::Text(text) => self.send(text.as_bytes()),
                    Event::Paste(text) => self.send(text.as_bytes()),
                    Event::Key {
                        key,
                        pressed: true,
                        modifiers,
                        ..
                    } => {
                        if let Some(bytes) = key_bytes(key, modifiers) {
                            self.send(&bytes);
                        }
                    }
                    _ => {}
                }
            }
        }

        let palette = theme::current().palette;
        let background = palette.bg_editor;
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, background);

        let parser = self.parser.lock().expect("terminal parser");
        let screen = parser.screen();
        let (screen_rows, screen_cols) = screen.size();
        for row in 0..screen_rows {
            let y = rect.top() + f32::from(row) * cell_height;
            let mut column = 0;
            while column < screen_cols {
                let Some(cell) = screen.cell(row, column) else {
                    column += 1;
                    continue;
                };
                // Group a run of cells that share colors and style.
                let (foreground, cell_background, bold, underline) = (
                    cell.fgcolor(),
                    cell.bgcolor(),
                    cell.bold(),
                    cell.underline(),
                );
                let inverse = cell.inverse();
                let mut text = String::new();
                let mut end = column;
                while end < screen_cols {
                    let Some(next) = screen.cell(row, end) else {
                        break;
                    };
                    if next.fgcolor() != foreground
                        || next.bgcolor() != cell_background
                        || next.bold() != bold
                        || next.inverse() != inverse
                    {
                        break;
                    }
                    let contents = next.contents();
                    text.push_str(if contents.is_empty() { " " } else { &contents });
                    end += 1;
                }
                let (mut fg, mut bg) = (
                    convert(foreground, palette.text, bold),
                    convert(cell_background, background, false),
                );
                if inverse {
                    std::mem::swap(&mut fg, &mut bg);
                }
                let x = rect.left() + f32::from(column) * cell_width;
                let width = f32::from(end - column) * cell_width;
                if bg != background {
                    painter.rect_filled(
                        egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(width, cell_height)),
                        0.0,
                        bg,
                    );
                }
                if text.trim().is_empty() {
                    column = end;
                    continue;
                }
                painter.text(
                    egui::pos2(x, y),
                    egui::Align2::LEFT_TOP,
                    &text,
                    font.clone(),
                    fg,
                );
                if underline {
                    painter.line_segment(
                        [
                            egui::pos2(x, y + cell_height - 1.0),
                            egui::pos2(x + width, y + cell_height - 1.0),
                        ],
                        egui::Stroke::new(1.0, fg),
                    );
                }
                column = end;
            }
        }
        if !screen.hide_cursor() {
            let (cursor_row, cursor_col) = screen.cursor_position();
            let cursor = egui::Rect::from_min_size(
                egui::pos2(
                    rect.left() + f32::from(cursor_col) * cell_width,
                    rect.top() + f32::from(cursor_row) * cell_height,
                ),
                egui::vec2(cell_width, cell_height),
            );
            if focused {
                painter.rect_filled(cursor, 0.0, theme::accent());
            } else {
                painter.rect_stroke(
                    cursor,
                    0.0,
                    egui::Stroke::new(1.0, theme::text_dim()),
                    egui::epaint::StrokeKind::Inside,
                );
            }
        }
        focused
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

/// Bytes a terminal expects for a key press that did not produce text.
pub fn key_bytes(key: Key, modifiers: Modifiers) -> Option<Vec<u8>> {
    let sequence: &[u8] = match key {
        Key::Enter => b"\r",
        Key::Backspace => b"\x7f",
        Key::Tab => {
            if modifiers.shift {
                b"\x1b[Z"
            } else {
                b"\t"
            }
        }
        Key::Escape => b"\x1b",
        Key::ArrowUp => b"\x1b[A",
        Key::ArrowDown => b"\x1b[B",
        Key::ArrowRight => b"\x1b[C",
        Key::ArrowLeft => b"\x1b[D",
        Key::Home => b"\x1b[H",
        Key::End => b"\x1b[F",
        Key::Delete => b"\x1b[3~",
        Key::PageUp => b"\x1b[5~",
        Key::PageDown => b"\x1b[6~",
        Key::Insert => b"\x1b[2~",
        _ => {
            // Ctrl+letter sends the matching control character (Ctrl+C is 0x03).
            if modifiers.ctrl
                && !modifiers.alt
                && let Some(name) = key
                    .name()
                    .chars()
                    .next()
                    .filter(|c| c.is_ascii_alphabetic())
                && key.name().len() == 1
            {
                return Some(vec![(name.to_ascii_uppercase() as u8) - b'@']);
            }
            return None;
        }
    };
    Some(sequence.to_vec())
}

/// Standard xterm colors; the first sixteen are softened to sit well on a dark or light editor.
fn indexed(index: u8, dark: bool) -> Color32 {
    const BASE: [(u8, u8, u8); 16] = [
        (0, 0, 0),
        (205, 49, 49),
        (13, 188, 121),
        (229, 229, 16),
        (36, 114, 200),
        (188, 63, 188),
        (17, 168, 205),
        (229, 229, 229),
        (102, 102, 102),
        (241, 76, 76),
        (35, 209, 139),
        (245, 245, 67),
        (59, 142, 234),
        (214, 112, 214),
        (41, 184, 219),
        (255, 255, 255),
    ];
    match index {
        0..=15 => {
            let (r, g, b) = BASE[index as usize];
            if !dark && index == 7 {
                return Color32::from_rgb(90, 90, 90);
            }
            Color32::from_rgb(r, g, b)
        }
        16..=231 => {
            let n = index - 16;
            let level = |value: u8| if value == 0 { 0 } else { 55 + value * 40 };
            Color32::from_rgb(level(n / 36), level((n % 36) / 6), level(n % 6))
        }
        _ => {
            let gray = 8 + (index - 232) * 10;
            Color32::from_rgb(gray, gray, gray)
        }
    }
}

fn convert(color: vt100::Color, default: Color32, bold: bool) -> Color32 {
    let dark = theme::current().palette.dark;
    match color {
        vt100::Color::Default => default,
        vt100::Color::Idx(index) => {
            // Bold text conventionally uses the bright variant of the first eight colors.
            let index = if bold && index < 8 { index + 8 } else { index };
            indexed(index, dark)
        }
        vt100::Color::Rgb(r, g, b) => Color32::from_rgb(r, g, b),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn special_keys_map_to_terminal_sequences() {
        assert_eq!(key_bytes(Key::Enter, Modifiers::NONE), Some(b"\r".to_vec()));
        assert_eq!(
            key_bytes(Key::ArrowUp, Modifiers::NONE),
            Some(b"\x1b[A".to_vec())
        );
        assert_eq!(
            key_bytes(Key::Tab, Modifiers::SHIFT),
            Some(b"\x1b[Z".to_vec())
        );
        assert_eq!(
            key_bytes(Key::Delete, Modifiers::NONE),
            Some(b"\x1b[3~".to_vec())
        );
        assert_eq!(key_bytes(Key::F5, Modifiers::NONE), None);
    }

    #[test]
    fn ctrl_letters_send_control_characters() {
        assert_eq!(key_bytes(Key::C, Modifiers::CTRL), Some(vec![3]));
        assert_eq!(key_bytes(Key::D, Modifiers::CTRL), Some(vec![4]));
        assert_eq!(key_bytes(Key::L, Modifiers::CTRL), Some(vec![12]));
        // Plain letters arrive as text events, not here.
        assert_eq!(key_bytes(Key::C, Modifiers::NONE), None);
    }

    #[test]
    fn indexed_colors_cover_the_xterm_cube_and_grays() {
        assert_eq!(indexed(1, true), Color32::from_rgb(205, 49, 49));
        assert_eq!(indexed(16, true), Color32::from_rgb(0, 0, 0));
        assert_eq!(indexed(231, true), Color32::from_rgb(255, 255, 255));
        assert_eq!(indexed(232, true), Color32::from_rgb(8, 8, 8));
        assert_eq!(indexed(255, true), Color32::from_rgb(238, 238, 238));
    }

    /// Needs a console session (ConPTY), so it is run by hand: `cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn runs_a_real_command_through_the_pty() {
        let context = egui::Context::default();
        let mut terminal = Terminal::spawn(&context, None).expect("spawn shell");
        terminal.send(b"echo forge-room-pty-works\r");
        let mut found = false;
        for _ in 0..80 {
            std::thread::sleep(std::time::Duration::from_millis(100));
            let parser = terminal.parser.lock().unwrap();
            if parser
                .screen()
                .contents()
                .matches("forge-room-pty-works")
                .count()
                >= 2
            {
                found = true; // once as typed, once as output
                break;
            }
        }
        let screen_text = terminal.parser.lock().unwrap().screen().contents();
        assert!(
            found,
            "command output never appeared; screen was: {screen_text:?}; exited={}",
            terminal.has_exited()
        );
    }
}
