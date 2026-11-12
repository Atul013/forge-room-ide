//! Integrated terminal: a real pseudo-terminal (ConPTY on Windows) running the user's shell,
//! with `vt100` interpreting its output and egui drawing the character grid.

use crate::theme;
use eframe::egui::{self, Color32, Event, EventFilter, Key, Modifiers, Sense};
use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

pub struct Terminal {
    parser: Arc<Mutex<vt100::Parser>>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    master: Box<dyn MasterPty + Send>,
    /// Behind a lock so that `has_exited` (which only borrows) can ask the process.
    child: Mutex<Box<dyn Child + Send + Sync>>,
    exited: Arc<AtomicBool>,
    size: (u16, u16),
    /// Dragged selection over the visible grid: (row, column) of both ends.
    selection: Option<((u16, u16), (u16, u16))>,
    /// The selected text, captured when the drag ends so that later output scrolling the grid
    /// cannot change what Copy puts on the clipboard.
    snapshot: Option<String>,
    wheel_rows: f32,
    pub shell: String,
    /// Whether this terminal was drawn this frame. A terminal nobody looks at does not need the
    /// window redrawn for every chunk of its output.
    visible: Arc<AtomicBool>,
    /// A file reference Ctrl+clicked in the output, waiting for the app to open it.
    link: Option<crate::path_links::PathRef>,
}

/// Rows scrolled per mouse-wheel notch, and the wheel distance egui reports per notch.
const WHEEL_ROWS: f32 = 3.0;
const WHEEL_UNIT: f32 = 50.0;

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
        // Coding agents and other TUIs look at these to decide whether to use 24-bit colour.
        command.env("COLORTERM", "truecolor");
        command.env("TERM_PROGRAM", "forge-room");
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
        let parser = Arc::new(Mutex::new(vt100::Parser::new(size.rows, size.cols, 6000)));
        let exited = Arc::new(AtomicBool::new(false));
        let visible = Arc::new(AtomicBool::new(true));
        {
            let parser = parser.clone();
            let exited = exited.clone();
            let visible = visible.clone();
            let writer = writer.clone();
            let context = context.clone();
            std::thread::spawn(move || {
                let mut buffer = [0_u8; 8192];
                // The last bytes of the previous read: a request may be split between two reads.
                let mut tail: Vec<u8> = Vec::new();
                loop {
                    match reader.read(&mut buffer) {
                        Ok(0) | Err(_) => break,
                        Ok(count) => {
                            let chunk = &buffer[..count];
                            let mut parser = parser.lock().expect("terminal parser");
                            parser.process(chunk);
                            // Windows' pseudo-console asks "where is the cursor?" (ESC [ 6 n) and
                            // withholds all output until it is answered.
                            let mut joined = std::mem::take(&mut tail);
                            joined.extend_from_slice(chunk);
                            let asked = joined.windows(4).any(|window| window == b"\x1b[6n");
                            tail = joined[joined.len().saturating_sub(3)..].to_vec();
                            if asked {
                                let (row, column) = parser.screen().cursor_position();
                                let reply = format!("[{};{}R", row + 1, column + 1);
                                if let Ok(mut writer) = writer.lock() {
                                    let _ = writer.write_all(reply.as_bytes());
                                    let _ = writer.flush();
                                }
                            }
                            drop(parser);
                            if visible.load(Ordering::Relaxed) {
                                context.request_repaint();
                            } else {
                                // Hidden: a slow refresh is enough, and many terminals streaming
                                // at once no longer redraw the whole window for every chunk.
                                context
                                    .request_repaint_after(std::time::Duration::from_millis(500));
                            }
                        }
                    }
                }
                exited.store(true, Ordering::Relaxed);
                context.request_repaint();
            });
        }
        Ok(Self {
            visible,
            parser,
            writer,
            master: pair.master,
            child: Mutex::new(child),
            exited,
            size: (size.rows, size.cols),
            selection: None,
            snapshot: None,
            wheel_rows: 0.0,
            shell: program,
            link: None,
        })
    }

    /// Scroll the view back (positive) or forward (negative) by `rows`.
    fn scroll(&mut self, rows: i64) {
        let mut parser = self.parser.lock().expect("terminal parser");
        let current = parser.screen().scrollback() as i64;
        parser
            .screen_mut()
            .set_scrollback((current + rows).max(0) as usize);
        drop(parser);
        self.clear_selection();
    }

    /// Jump back to the live screen, as typing in a terminal does.
    fn follow_output(&mut self) {
        self.parser
            .lock()
            .expect("terminal parser")
            .screen_mut()
            .set_scrollback(0);
        self.clear_selection();
    }

    fn clear_selection(&mut self) {
        self.selection = None;
        self.snapshot = None;
    }

    /// The text under the selection as the grid looks now, clamped to the current size.
    fn live_selected_text(&self) -> Option<String> {
        let (a, b) = self.selection?;
        let ((first_row, first_col), (last_row, last_col)) = if a <= b { (a, b) } else { (b, a) };
        let parser = self.parser.lock().expect("terminal parser");
        let screen = parser.screen();
        let (rows, cols) = screen.size();
        let (first_row, last_row) = (first_row.min(rows - 1), last_row.min(rows - 1));
        let (first_col, last_col) = (first_col.min(cols - 1), last_col.min(cols - 1));
        Some(screen.contents_between(first_row, first_col, last_row, last_col + 1))
    }

    /// What Copy should copy. `None` only when nothing is selected: a selection of blank cells is
    /// still a selection, so Ctrl+C on it must not interrupt the running program.
    fn selected_text(&self) -> Option<String> {
        self.selection?;
        self.snapshot.clone().or_else(|| self.live_selected_text())
    }

    /// Forget that this terminal was drawn; [`Terminal::show`] marks it again.
    pub fn set_hidden(&self) {
        self.visible.store(false, Ordering::Relaxed);
    }

    /// Everything the terminal still remembers, from the oldest scrollback line to the screen, for
    /// copying more than fits on one screen.
    pub fn all_text(&self) -> String {
        let mut parser = self.parser.lock().unwrap();
        let (rows, cols) = (self.size.0, self.size.1);
        all_text_of(&mut parser, rows, cols)
    }

    /// The text currently on the screen, for noticing a dev server's address.
    pub fn screen_text(&self) -> String {
        self.parser.lock().unwrap().screen().contents()
    }

    /// The file reference under a screen cell (see [`reference_in`]).
    fn reference_at(&self, row: u16, column: u16, cols: u16) -> Option<crate::path_links::PathRef> {
        let parser = self.parser.lock().unwrap();
        reference_in(parser.screen(), row, column, cols)
    }

    /// The reference the user Ctrl+clicked since the last call, once.
    pub fn take_link(&mut self) -> Option<crate::path_links::PathRef> {
        self.link.take()
    }

    pub fn has_exited(&self) -> bool {
        if self.exited.load(Ordering::Relaxed) {
            return true;
        }
        // On Windows the output pipe stays open after the shell has ended (`exit`), so the reader
        // never sees the end: the process itself is asked.
        if let Ok(mut child) = self.child.lock()
            && matches!(child.try_wait(), Ok(Some(_)))
        {
            self.exited.store(true, Ordering::Relaxed);
            return true;
        }
        false
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
        // Stored cell positions belong to the old grid.
        self.clear_selection();
        let _ = self.master.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        });
        self.parser
            .lock()
            .expect("terminal parser")
            .screen_mut()
            .set_size(rows, cols);
    }

    /// Draw the grid and forward keyboard input. Returns true while the terminal has focus.
    pub fn show(&mut self, ui: &mut egui::Ui, request_focus: bool) -> bool {
        self.visible.store(true, Ordering::Relaxed);
        let font = egui::FontId::monospace(13.5);
        let (cell_height, cell_width) =
            ui.fonts_mut(|fonts| (fonts.row_height(&font), fonts.glyph_width(&font, 'M')));
        let available = ui.available_size();
        let cols = (available.x / cell_width).floor().max(1.0) as u16;
        let rows = (available.y / cell_height).floor().max(1.0) as u16;
        self.resize(rows, cols);

        let (rect, response) = ui.allocate_exact_size(available, Sense::click_and_drag());
        if response.clicked() || request_focus {
            response.request_focus();
        }
        let cell_at = |position: egui::Pos2| -> (u16, u16) {
            let column = ((position.x - rect.left()) / cell_width).floor().max(0.0) as u16;
            let row = ((position.y - rect.top()) / cell_height).floor().max(0.0) as u16;
            (row.min(rows - 1), column.min(cols - 1))
        };
        if let Some(position) = response.interact_pointer_pos() {
            if response.drag_started() {
                // The previous selection's copy must not answer Ctrl+C during this new drag.
                self.snapshot = None;
                // egui reports the drag a few pixels after the press: anchor where the press was.
                let origin = ui
                    .input(|input| input.pointer.press_origin())
                    .unwrap_or(position);
                self.selection = Some((cell_at(origin), cell_at(position)));
            } else if response.dragged()
                && let Some((anchor, _)) = self.selection
            {
                self.selection = Some((anchor, cell_at(position)));
            }
        }
        if response.drag_stopped() {
            self.snapshot = self.live_selected_text();
        }
        if response.clicked() {
            self.clear_selection();
            // Ctrl+click on `file:line:column` in the output opens that place in the editor.
            if ui.input(|input| input.modifiers.command)
                && let Some(position) = response.interact_pointer_pos()
            {
                let (row, column) = cell_at(position);
                self.link = self.reference_at(row, column, cols);
            }
        }
        if response.hovered()
            && ui.input(|input| input.modifiers.command)
            && let Some(position) = response.hover_pos()
        {
            let (row, column) = cell_at(position);
            if self.reference_at(row, column, cols).is_some() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
        }
        if response.hovered() {
            // Smoothed wheel movement arrives in small pieces; keep the fraction between frames.
            self.wheel_rows +=
                ui.input(|input| input.smooth_scroll_delta.y) / WHEEL_UNIT * WHEEL_ROWS;
            let step = self.wheel_rows.trunc() as i64;
            if step != 0 {
                self.wheel_rows -= step as f32;
                self.scroll(step);
            }
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
                    Event::Text(text) => {
                        self.follow_output();
                        self.send(text.as_bytes());
                    }
                    Event::Paste(text) => {
                        self.follow_output();
                        let bracketed = self
                            .parser
                            .lock()
                            .map(|parser| parser.screen().bracketed_paste())
                            .unwrap_or(false);
                        let bytes = paste_bytes(&text, bracketed);
                        self.send(&bytes);
                    }
                    // egui turns Ctrl+C into Copy and Ctrl+X into Cut, so a terminal must
                    // translate them back: copy a selection, otherwise interrupt the program.
                    Event::Copy => match self.selected_text() {
                        Some(text) => {
                            ui.ctx().copy_text(text);
                            self.clear_selection();
                        }
                        None => {
                            self.follow_output();
                            self.send(&[0x03]);
                        }
                    },
                    Event::Cut => {
                        self.follow_output();
                        self.send(&[0x18]);
                    }
                    Event::Key {
                        key,
                        pressed: true,
                        modifiers,
                        ..
                    } => {
                        if modifiers.shift && matches!(key, Key::PageUp | Key::PageDown) {
                            let page = i64::from(self.size.0.saturating_sub(1));
                            self.scroll(if key == Key::PageUp { page } else { -page });
                        } else if let Some(bytes) = key_bytes(key, modifiers) {
                            self.follow_output();
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

        let selection = self
            .selection
            .map(|(a, b)| if a <= b { (a, b) } else { (b, a) });
        let parser = self.parser.lock().expect("terminal parser");
        let screen = parser.screen();
        let (screen_rows, screen_cols) = screen.size();
        if let Some(((first_row, first_col), (last_row, last_col))) = selection {
            for row in first_row..=last_row.min(screen_rows.saturating_sub(1)) {
                let from = if row == first_row { first_col } else { 0 };
                let to = if row == last_row {
                    last_col
                } else {
                    screen_cols.saturating_sub(1)
                };
                painter.rect_filled(
                    egui::Rect::from_min_size(
                        egui::pos2(
                            rect.left() + f32::from(from) * cell_width,
                            rect.top() + f32::from(row) * cell_height,
                        ),
                        egui::vec2(f32::from(to - from.min(to) + 1) * cell_width, cell_height),
                    ),
                    0.0,
                    palette.selection,
                );
            }
        }
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
                    text.push_str(if contents.is_empty() { " " } else { contents });
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
        if screen.scrollback() > 0 {
            painter.text(
                rect.right_top() + egui::vec2(-10.0, 4.0),
                egui::Align2::RIGHT_TOP,
                format!("↑ {} lines back (type to return)", screen.scrollback()),
                egui::FontId::proportional(12.0),
                theme::text_dim(),
            );
        } else if !screen.hide_cursor() {
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
        if let Ok(mut child) = self.child.lock() {
            // The shell's whole tree (a dev server it started, a ping): killing only the shell can
            // leave those running with nothing to show their output.
            #[cfg(windows)]
            if let Some(pid) = child.process_id() {
                use std::os::windows::process::CommandExt;
                let _ = std::process::Command::new(
                    std::env::var_os("SystemRoot")
                        .map(PathBuf::from)
                        .unwrap_or_else(|| PathBuf::from(r"C:\Windows"))
                        .join("System32")
                        .join("taskkill.exe"),
                )
                .args(["/PID", &pid.to_string(), "/T", "/F"])
                .creation_flags(0x0800_0000)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();
            }
            let _ = child.kill();
        }
    }
}

/// Bytes a terminal expects for a key press that did not produce text.
/// The file reference under a screen cell. The row is read cell by cell, so a double-width
/// character (two cells, one character) or a combining mark does not shift the column, and a line
/// that wrapped from the row before or onto the next row is read as one line.
fn reference_in(
    screen: &vt100::Screen,
    row: u16,
    column: u16,
    cols: u16,
) -> Option<crate::path_links::PathRef> {
    let mut text = String::new();
    // A reference may begin on the previous row when that row wrapped into this one.
    if row > 0 && screen.row_wrapped(row - 1) {
        for col in 0..cols {
            push_cell(screen, row - 1, col, &mut text, &mut Vec::new());
        }
    }
    let mut char_of_cell: Vec<usize> = Vec::new();
    for col in 0..cols {
        push_cell(screen, row, col, &mut text, &mut char_of_cell);
    }
    // The recorded indexes already count the text of the row before.
    let index = *char_of_cell.get(usize::from(column))?;
    // ... and may continue on the next row when this one wrapped.
    if screen.row_wrapped(row) && row + 1 < screen.size().0 {
        for col in 0..cols {
            push_cell(screen, row + 1, col, &mut text, &mut Vec::new());
        }
    }
    crate::path_links::ref_at(&text, index)
}

/// Append the text of one cell to `text` (a blank for an empty cell, nothing for the second half
/// of a double-width character) and record which character the cell belongs to.
fn push_cell(
    screen: &vt100::Screen,
    row: u16,
    col: u16,
    text: &mut String,
    char_of_cell: &mut Vec<usize>,
) {
    let index = text.chars().count();
    match screen.cell(row, col) {
        Some(cell) if cell.is_wide_continuation() => {
            // Belongs to the wide character just before it.
            char_of_cell.push(index.saturating_sub(1));
        }
        Some(cell) if cell.has_contents() => {
            char_of_cell.push(index);
            text.push_str(cell.contents());
        }
        _ => {
            char_of_cell.push(index);
            text.push(' ');
        }
    }
}

pub fn key_bytes(key: Key, modifiers: Modifiers) -> Option<Vec<u8>> {
    // xterm's number for held modifiers: 1, plus 1 for Shift, 2 for Alt and 4 for Ctrl.
    let param =
        1 + u8::from(modifiers.shift) + 2 * u8::from(modifiers.alt) + 4 * u8::from(modifiers.ctrl);
    let modified = param > 1;
    // Arrows, Home and End: `ESC [ A`, or `ESC [ 1 ; 5 A` with Ctrl held.
    let cursor = |letter: char| -> Vec<u8> {
        if modified {
            format!("\x1b[1;{param}{letter}").into_bytes()
        } else {
            format!("\x1b[{letter}").into_bytes()
        }
    };
    // Delete, Insert, Page keys and F5 and up: `ESC [ 3 ~`, or `ESC [ 3 ; 5 ~`.
    let tilde = |number: u8| -> Vec<u8> {
        if modified {
            format!("\x1b[{number};{param}~").into_bytes()
        } else {
            format!("\x1b[{number}~").into_bytes()
        }
    };
    // F1 to F4: `ESC O P`, or `ESC [ 1 ; 2 P` with a modifier.
    let function = |letter: char| -> Vec<u8> {
        if modified {
            format!("\x1b[1;{param}{letter}").into_bytes()
        } else {
            format!("\x1bO{letter}").into_bytes()
        }
    };
    Some(match key {
        // Shift or Alt with Enter is "a new line, not a submit" for coding agents (ESC then CR).
        Key::Enter if modifiers.shift || modifiers.alt => b"\x1b\r".to_vec(),
        Key::Enter => b"\r".to_vec(),
        Key::Backspace if modifiers.ctrl => vec![0x17],
        Key::Space if modifiers.ctrl && !modifiers.alt => vec![0x00],
        Key::OpenBracket if modifiers.ctrl && !modifiers.alt => vec![0x1b],
        Key::Backslash if modifiers.ctrl && !modifiers.alt => vec![0x1c],
        Key::CloseBracket if modifiers.ctrl && !modifiers.alt => vec![0x1d],
        Key::Backspace if modifiers.alt => b"\x1b\x7f".to_vec(),
        Key::Backspace => b"\x7f".to_vec(),
        Key::Tab if modifiers.shift => b"\x1b[Z".to_vec(),
        Key::Tab => b"\t".to_vec(),
        Key::Escape => b"\x1b".to_vec(),
        Key::ArrowUp => cursor('A'),
        Key::ArrowDown => cursor('B'),
        Key::ArrowRight => cursor('C'),
        Key::ArrowLeft => cursor('D'),
        Key::Home => cursor('H'),
        Key::End => cursor('F'),
        Key::Insert => tilde(2),
        Key::Delete => tilde(3),
        Key::PageUp => tilde(5),
        Key::PageDown => tilde(6),
        Key::F1 => function('P'),
        Key::F2 => function('Q'),
        Key::F3 => function('R'),
        Key::F4 => function('S'),
        Key::F5 => tilde(15),
        Key::F6 => tilde(17),
        Key::F7 => tilde(18),
        Key::F8 => tilde(19),
        Key::F9 => tilde(20),
        Key::F10 => tilde(21),
        Key::F11 => tilde(23),
        Key::F12 => tilde(24),
        _ => {
            let letter = (key.name().len() == 1)
                .then(|| key.name().chars().next())
                .flatten()
                .filter(|c| c.is_ascii_alphabetic())?;
            if modifiers.ctrl && !modifiers.alt {
                // Ctrl+letter sends the matching control character (Ctrl+C is 0x03).
                vec![(letter.to_ascii_uppercase() as u8) - b'@']
            } else if modifiers.alt && !modifiers.ctrl {
                // Alt+letter is ESC and then the letter (word movement, agent shortcuts).
                let letter = if modifiers.shift {
                    letter.to_ascii_uppercase()
                } else {
                    letter.to_ascii_lowercase()
                };
                vec![0x1b, letter as u8]
            } else {
                return None;
            }
        }
    })
}

/// What a paste sends. A program that asked for bracketed paste (coding agents do) gets the text
/// between `ESC [ 200 ~` and `ESC [ 201 ~`, so a pasted newline is text and not Enter; for any other
/// program the line breaks become carriage returns, which is what Enter sends.
pub fn paste_bytes(text: &str, bracketed: bool) -> Vec<u8> {
    // Control characters (escape, Ctrl+C ...) are removed from what is pasted: the end marker of a
    // bracketed paste inside it would end the paste early and run the rest as typed keys, and any
    // other escape sequence would be read by the program as a command rather than as text.
    let text: String = text
        .replace("\r\n", "\n")
        .chars()
        .filter(|c| !c.is_control() || matches!(c, '\n' | '\t' | '\r'))
        .collect();
    if bracketed {
        let mut bytes = b"\x1b[200~".to_vec();
        bytes.extend_from_slice(text.as_bytes());
        bytes.extend_from_slice(b"\x1b[201~");
        bytes
    } else {
        text.replace('\n', "\r").into_bytes()
    }
}

/// The scrollback and the screen of `parser` as plain text, one line per row with trailing blanks
/// and trailing empty lines removed. The view is put back where it was.
fn all_text_of(parser: &mut vt100::Parser, rows: u16, cols: u16) -> String {
    let rows = usize::from(rows).max(1);
    let saved = parser.screen().scrollback();
    parser.screen_mut().set_scrollback(usize::MAX);
    let total = parser.screen().scrollback();
    // Each row of text, and whether it continues on the next row (a long line that wrapped).
    let mut lines: Vec<(String, bool)> = Vec::new();
    // At scrollback `offset` the screen shows lines `total - offset ..` of the whole history.
    let mut offset = total;
    let mut next_line: usize = 0;
    loop {
        parser.screen_mut().set_scrollback(offset);
        let first = total - offset;
        let skip = next_line.saturating_sub(first);
        let chunk: Vec<(String, bool)> = parser
            .screen()
            .rows(0, cols)
            .enumerate()
            .map(|(row, text)| (text, parser.screen().row_wrapped(row as u16)))
            .collect();
        lines.extend(chunk.into_iter().skip(skip));
        next_line = first + rows;
        if offset == 0 {
            break;
        }
        offset = offset.saturating_sub(rows);
    }
    parser.screen_mut().set_scrollback(saved);
    // A row that wraps onto the next is joined to it as it was typed; the others end a line.
    let mut text = String::new();
    for (row, wrapped) in &lines {
        if *wrapped {
            text.push_str(row);
        } else {
            text.push_str(row.trim_end());
            text.push('\n');
        }
    }
    text.trim_end_matches(['\n', ' ']).to_string()
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

    fn link_at(parser: &vt100::Parser, row: u16, column: u16) -> Option<(String, Option<usize>)> {
        let (_, cols) = parser.screen().size();
        reference_in(parser.screen(), row, column, cols).map(|r| (r.path, r.line))
    }

    #[test]
    fn a_reference_after_double_width_characters_is_found_at_its_own_cells() {
        let mut parser = vt100::Parser::new(2, 30, 0);
        // Four double-width characters take cells 0-7, a space cell 8, the reference starts at 9.
        parser.process("\u{65e5}\u{672c}\u{65e5}\u{672c} a.rs:3".as_bytes());
        assert_eq!(link_at(&parser, 0, 9), Some(("a.rs".into(), Some(3))));
        assert_eq!(link_at(&parser, 0, 14), Some(("a.rs".into(), Some(3))));
        // The wide characters themselves, both of their cells, are not a reference.
        for column in 0..8 {
            assert_eq!(link_at(&parser, 0, column), None, "cell {column}");
        }
        // Past the text there is nothing.
        assert_eq!(link_at(&parser, 0, 25), None);
    }

    #[test]
    fn a_path_that_wrapped_onto_the_next_row_is_read_as_one() {
        let mut parser = vt100::Parser::new(3, 10, 0);
        // Twenty characters on a ten-column screen: the second row continues the first.
        parser.process(b"error src/main.rs:12");
        assert!(parser.screen().row_wrapped(0));
        assert_eq!(
            link_at(&parser, 1, 2),
            Some(("src/main.rs".into(), Some(12)))
        );
        assert_eq!(
            link_at(&parser, 0, 8),
            Some(("src/main.rs".into(), Some(12)))
        );
    }

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
        assert_eq!(
            key_bytes(Key::F5, Modifiers::NONE),
            Some(b"\x1b[15~".to_vec())
        );
        assert_eq!(
            key_bytes(Key::F1, Modifiers::NONE),
            Some(b"\x1bOP".to_vec())
        );
        assert_eq!(key_bytes(Key::A, Modifiers::NONE), None);
        // Modified keys: the xterm forms that shells and agents understand.
        assert_eq!(
            key_bytes(Key::ArrowRight, Modifiers::CTRL),
            Some(b"\x1b[1;5C".to_vec())
        );
        assert_eq!(
            key_bytes(Key::ArrowLeft, Modifiers::ALT),
            Some(b"\x1b[1;3D".to_vec())
        );
        assert_eq!(
            key_bytes(Key::Delete, Modifiers::SHIFT),
            Some(b"\x1b[3;2~".to_vec())
        );
        assert_eq!(
            key_bytes(Key::Enter, Modifiers::SHIFT),
            Some(b"\x1b\r".to_vec())
        );
        assert_eq!(
            key_bytes(Key::Enter, Modifiers::ALT),
            Some(b"\x1b\r".to_vec())
        );
        assert_eq!(key_bytes(Key::Backspace, Modifiers::CTRL), Some(vec![0x17]));
        assert_eq!(key_bytes(Key::P, Modifiers::ALT), Some(b"\x1bp".to_vec()));
        // AltGr (Ctrl and Alt together) is a way to type a character, not a shortcut.
        assert_eq!(key_bytes(Key::Q, Modifiers::CTRL | Modifiers::ALT), None);
        // A paste is wrapped when the program asked for that, and otherwise typed with Enter.
        assert_eq!(
            paste_bytes("a\nb", true),
            b"\x1b[200~a\nb\x1b[201~".to_vec()
        );
        assert_eq!(paste_bytes("a\r\nb\nc", false), b"a\rb\rc".to_vec());
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

    #[test]
    fn the_whole_history_is_copied_in_order_without_gaps_or_repeats() {
        let mut parser = vt100::Parser::new(5, 20, 100);
        for number in 1..=23 {
            parser.process(format!("line {number}\r\n").as_bytes());
        }
        parser.process(b"last");
        // Scroll the view away from the bottom: copying must not depend on, or disturb, it.
        parser.screen_mut().set_scrollback(3);
        let text = all_text_of(&mut parser, 5, 20);
        let expected: Vec<String> = (1..=23)
            .map(|number| format!("line {number}"))
            .chain(std::iter::once("last".to_string()))
            .collect();
        assert_eq!(text, expected.join("\n"));
        assert_eq!(parser.screen().scrollback(), 3);
        // Less than a screen of text, and an empty terminal.
        let mut small = vt100::Parser::new(5, 20, 100);
        small.process(b"a\r\nb");
        assert_eq!(all_text_of(&mut small, 5, 20), "a\nb");
        assert_eq!(all_text_of(&mut vt100::Parser::new(5, 20, 100), 5, 20), "");
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

    fn wait_for(terminal: &Terminal, needle: &str) -> bool {
        for _ in 0..80 {
            std::thread::sleep(std::time::Duration::from_millis(100));
            if terminal
                .parser
                .lock()
                .unwrap()
                .screen()
                .contents()
                .contains(needle)
            {
                return true;
            }
        }
        false
    }

    #[test]
    #[ignore]
    fn scrolling_back_reveals_older_output() {
        let context = egui::Context::default();
        let mut terminal = Terminal::spawn(&context, None).expect("spawn shell");
        terminal.send(b"1..120 | ForEach-Object { \"row-$_\" }\r");
        assert!(wait_for(&terminal, "row-120"), "output never finished");
        terminal.scroll(60);
        let scrolled = terminal.parser.lock().unwrap().screen().contents();
        assert!(!scrolled.contains("row-120"), "view did not move back");
        terminal.follow_output();
        let live = terminal.parser.lock().unwrap().screen().contents();
        assert!(live.contains("row-120"));
    }

    #[test]
    #[ignore]
    fn ctrl_c_at_the_prompt_discards_the_typed_line() {
        let context = egui::Context::default();
        let mut terminal = Terminal::spawn(&context, None).expect("spawn shell");
        assert!(wait_for(&terminal, "PS "), "shell never showed a prompt");
        terminal.send(b"echo should-not-run");
        std::thread::sleep(std::time::Duration::from_millis(800));
        terminal.send(&[0x03]);
        std::thread::sleep(std::time::Duration::from_millis(800));
        terminal.send(b"\r");
        std::thread::sleep(std::time::Duration::from_millis(1500));
        let screen = terminal.parser.lock().unwrap().screen().contents();
        assert!(
            !screen.contains("should-not-run\nshould-not-run"),
            "line ran: {screen:?}"
        );
        println!("SCREEN: {screen:?}");
    }

    /// Copy semantics that must hold however the grid changes under a selection.
    #[test]
    #[ignore]
    fn selection_survives_output_resize_and_blank_cells() {
        let context = egui::Context::default();
        let mut terminal = Terminal::spawn(&context, None).expect("spawn shell");
        assert!(wait_for(&terminal, "PS "), "shell never showed a prompt");
        {
            let mut parser = terminal.parser.lock().unwrap();
            parser.process(b"\x1b[2J \x1b[H");
            parser.process(b"SELECT-ME\r\nother line\r\n");
        }
        // Nothing selected: Copy is the interrupt key, so there is no text to copy.
        assert_eq!(terminal.selected_text(), None);

        terminal.selection = Some(((0, 0), (0, 8)));
        terminal.snapshot = terminal.live_selected_text();
        assert_eq!(terminal.selected_text().as_deref(), Some("SELECT-ME"));

        // New output scrolls the grid, but the captured text is what gets copied.
        {
            let mut parser = terminal.parser.lock().unwrap();
            for index in 0..60 {
                parser.process(format!("filler {index}\r\n").as_bytes());
            }
        }
        assert_eq!(terminal.selected_text().as_deref(), Some("SELECT-ME"));

        // A selection of blank cells is still a selection.
        terminal.clear_selection();
        terminal.selection = Some(((20, 0), (20, 3)));
        assert!(terminal.selected_text().is_some());

        // Resizing drops positions that belong to the old grid; nothing panics.
        terminal.selection = Some(((0, 80), (1, 90)));
        terminal.resize(10, 20);
        assert_eq!(terminal.selection, None);
        assert_eq!(terminal.selected_text(), None);
    }

    #[test]
    fn pasted_text_cannot_end_a_bracketed_paste_early_or_carry_escape_sequences() {
        let hostile = "ok\u{1b}[201~\ncalc\n\u{1b}[201~\u{3}";
        let bytes = paste_bytes(hostile, true);
        let ends = bytes.windows(6).filter(|w| *w == b"\x1b[201~").count();
        assert_eq!(ends, 1, "{bytes:?}");
        assert!(bytes.ends_with(b"\x1b[201~"));
        let plain = paste_bytes(hostile, false);
        assert!(
            !plain.contains(&0x1b) && !plain.contains(&0x03),
            "{plain:?}"
        );
        // Text, tabs and line breaks stay.
        assert_eq!(paste_bytes("a\tb\nc", false), b"a\tb\rc".to_vec());
    }

    #[test]
    fn copying_everything_joins_a_wrapped_line_and_ctrl_punctuation_keys_are_sent() {
        let mut parser = vt100::Parser::new(5, 20, 100);
        parser.process("x".repeat(50).as_bytes());
        parser.process(b"\r\nnext");
        assert_eq!(
            all_text_of(&mut parser, 5, 20),
            format!("{}\nnext", "x".repeat(50))
        );
        assert_eq!(key_bytes(Key::Space, Modifiers::CTRL), Some(vec![0x00]));
        assert_eq!(
            key_bytes(Key::OpenBracket, Modifiers::CTRL),
            Some(vec![0x1b])
        );
        assert_eq!(key_bytes(Key::Backslash, Modifiers::CTRL), Some(vec![0x1c]));
        assert_eq!(
            key_bytes(Key::CloseBracket, Modifiers::CTRL),
            Some(vec![0x1d])
        );
    }

    /// Needs a console session (ConPTY), so it is run by hand: `cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn a_shell_that_was_exited_is_noticed() {
        let context = egui::Context::default();
        let mut terminal = Terminal::spawn(&context, None).expect("spawn shell");
        std::thread::sleep(std::time::Duration::from_millis(1500));
        assert!(!terminal.has_exited());
        terminal.send(b"exit\r");
        let mut noticed = false;
        for _ in 0..60 {
            std::thread::sleep(std::time::Duration::from_millis(100));
            if terminal.has_exited() {
                noticed = true;
                break;
            }
        }
        assert!(noticed, "the shell ended but has_exited stayed false");
    }
}
