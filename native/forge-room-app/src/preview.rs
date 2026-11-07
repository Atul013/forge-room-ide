//! The Preview pane beside the editor. There is no embedded web view: it shows the address of the
//! project's local dev server (taken from the terminal output, or typed), whether anything is
//! listening there, and opens it in the system browser on request.

use crate::theme;
use eframe::egui::{self, Color32, RichText};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Clone, Debug, PartialEq, Eq)]
enum Status {
    /// A check for the current address is still running.
    Checking,
    /// A remote address: never probed from here.
    Unknown,
    Listening,
    Silent,
}

pub struct Preview {
    /// The address being previewed (what is in the address box).
    pub url: String,
    /// The result and the address it was measured for, so a late answer is never shown for another.
    status: Arc<Mutex<(String, Status)>>,
    /// Bumped for every check; only the newest check may publish its result.
    generation: Arc<AtomicU64>,
    checked: String,
    last_check: Option<Instant>,
    last_scan: Option<Instant>,
    /// Set once the user types an address, so detection stops overwriting it.
    typed: bool,
}

impl Default for Preview {
    fn default() -> Self {
        Self {
            url: String::new(),
            status: Arc::new(Mutex::new((String::new(), Status::Unknown))),
            generation: Arc::new(AtomicU64::new(0)),
            checked: String::new(),
            last_check: None,
            last_scan: None,
            typed: false,
        }
    }
}

/// The last `http://localhost:PORT` or `http://127.0.0.1:PORT` address in `text`.
pub fn find_local_url(text: &str) -> Option<String> {
    let mut found = None;
    for (start, _) in text.match_indices("http://") {
        let rest = &text[start..];
        let end = rest
            .find(|c: char| c.is_whitespace() || matches!(c, '"' | '\'' | ')' | '>' | '\u{1b}'))
            .unwrap_or(rest.len());
        let url = rest[..end].trim_end_matches(['.', ',', ';']);
        if host_port(url).is_some_and(|(host, _)| is_local(&host)) {
            found = Some(url.to_string());
        }
    }
    found
}

fn is_local(host: &str) -> bool {
    matches!(host, "localhost" | "127.0.0.1" | "[::1]" | "0.0.0.0")
}

/// Host and port of an `http(s)://` address; the port defaults by scheme.
fn host_port(url: &str) -> Option<(String, u16)> {
    let (scheme, rest) = url.split_once("://")?;
    let default = match scheme {
        "http" => 80,
        "https" => 443,
        _ => return None,
    };
    let authority = rest.split(['/', '?', '#']).next()?;
    if authority.is_empty() || authority.contains('@') {
        return None;
    }
    if authority.starts_with('[') && !authority.contains(']') {
        return None;
    }
    if let Some(end) = authority.find(']') {
        // `[::1]` or `[::1]:5173`: the colons inside the brackets are not a port separator.
        if !authority.starts_with('[') || end < 2 {
            return None;
        }
        let host = &authority[..=end];
        return match &authority[end + 1..] {
            "" => Some((host.to_string(), default)),
            port => Some((host.to_string(), port.strip_prefix(':')?.parse().ok()?)),
        };
    }
    match authority.rsplit_once(':') {
        Some((host, port)) if !host.is_empty() && !port.is_empty() => {
            Some((host.to_string(), port.parse().ok()?))
        }
        Some(_) => None,
        None => Some((authority.to_string(), default)),
    }
}

/// Only web addresses are ever handed to the browser.
fn is_web_url(url: &str) -> bool {
    host_port(url.trim()).is_some()
}

/// The address to open for something a person typed: an `http(s)` URL as it is, or a bare host
/// with `https://` put in front. Any other explicit scheme is refused.
pub fn web_url(text: &str) -> Option<String> {
    let text = text.trim();
    if text.contains("://") {
        return is_web_url(text).then(|| text.to_string());
    }
    let candidate = format!("https://{text}");
    (!text.is_empty() && !text.contains(char::is_whitespace) && is_web_url(&candidate))
        .then_some(candidate)
}

impl Preview {
    /// Take the dev-server address from terminal output, unless the user typed one.
    pub fn detect(&mut self, terminal_text: &str) {
        if self.typed
            || self
                .last_scan
                .is_some_and(|at| at.elapsed() < Duration::from_secs(1))
        {
            return;
        }
        self.last_scan = Some(Instant::now());
        if let Some(url) = find_local_url(terminal_text)
            && url != self.url
        {
            self.url = url;
        }
    }

    pub fn forget_project(&mut self) {
        *self = Self::default();
    }

    fn refresh_status(&mut self, context: &egui::Context) {
        let due = self
            .last_check
            .is_none_or(|at| at.elapsed() >= Duration::from_secs(3));
        if self.url == self.checked && !due {
            return;
        }
        self.checked = self.url.clone();
        self.last_check = Some(Instant::now());
        let url = self.url.trim().to_string();
        let ticket = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        let Some((host, port)) = host_port(&url).filter(|(host, _)| is_local(host)) else {
            // Remote addresses are never probed from here.
            *self.status.lock().unwrap() = (url, Status::Unknown);
            return;
        };
        let status = Arc::clone(&self.status);
        let generation = Arc::clone(&self.generation);
        let context = context.clone();
        std::thread::spawn(move || {
            // Wildcard and name hosts are reduced to loopback addresses; nothing else is dialled.
            let host = match host.as_str() {
                "[::1]" => "::1".to_string(),
                "0.0.0.0" => "127.0.0.1".to_string(),
                other => other.to_string(),
            };
            let reachable = (host.as_str(), port)
                .to_socket_addrs()
                .ok()
                .and_then(|mut addrs| {
                    addrs.find_map(|addr| {
                        addr.ip().is_loopback().then(|| {
                            TcpStream::connect_timeout(&addr, Duration::from_millis(600)).ok()
                        })?
                    })
                })
                .is_some();
            // A newer check (another address) owns the display now: drop this answer.
            if generation.load(Ordering::SeqCst) == ticket {
                *status.lock().unwrap() = (
                    url,
                    if reachable {
                        Status::Listening
                    } else {
                        Status::Silent
                    },
                );
                context.request_repaint();
            }
        });
    }

    /// Draw the pane. Returns true when the user closes it.
    pub fn show(&mut self, ui: &mut egui::Ui) -> bool {
        let context = ui.ctx().clone();
        self.refresh_status(&context);
        context.request_repaint_after(Duration::from_secs(3));
        let mut close = false;
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.add_space(10.0);
            ui.label(
                RichText::new("Preview")
                    .size(14.0)
                    .family(crate::fonts::semibold())
                    .color(theme::text()),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(6.0);
                if crate::icons::icon_button(ui, "Hide preview", crate::icons::close_cross)
                    .clicked()
                {
                    close = true;
                }
            });
        });
        ui.add_space(6.0);
        egui::Frame::new()
            .inner_margin(egui::Margin::symmetric(10, 0))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                let edit = ui.add(
                    egui::TextEdit::singleline(&mut self.url)
                        .hint_text("http://localhost:5173")
                        .desired_width(f32::INFINITY),
                );
                if edit.changed() {
                    self.typed = true;
                    self.last_check = None;
                }
                ui.add_space(8.0);

                let status = {
                    let measured = self.status.lock().unwrap();
                    if measured.0 == self.url.trim() {
                        measured.1.clone()
                    } else {
                        Status::Checking
                    }
                };
                let (dot, text) = if self.url.trim().is_empty() {
                    (
                        theme::text_dim(),
                        "No dev server detected. Start one in the terminal and its local address \
                         appears here, or type an address.",
                    )
                } else if !is_web_url(&self.url) {
                    (
                        Color32::from_rgb(240, 113, 120),
                        "Not a web address. Use http:// or https://.",
                    )
                } else {
                    match status {
                        Status::Listening => (
                            Color32::from_rgb(110, 200, 120),
                            "Server is running. Open it in your browser.",
                        ),
                        Status::Silent => (
                            Color32::from_rgb(226, 160, 60),
                            "Nothing is listening at this address yet.",
                        ),
                        Status::Checking => (theme::text_dim(), "Checking this address..."),
                        Status::Unknown => {
                            (theme::text_dim(), "Remote address: not checked from here.")
                        }
                    }
                };
                ui.horizontal_top(|ui| {
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(8.0, 16.0), egui::Sense::hover());
                    ui.painter().circle_filled(rect.center(), 3.5, dot);
                    ui.add(egui::Label::new(RichText::new(text).color(theme::text_dim())).wrap());
                });
                ui.add_space(10.0);

                let ready = is_web_url(&self.url);
                let open = egui::Button::new(
                    RichText::new("Open in Browser")
                        .strong()
                        .color(theme::button_text()),
                )
                .fill(theme::button())
                .corner_radius(6.0)
                .min_size(egui::vec2(ui.available_width(), 30.0));
                if ui
                    .add_enabled(ready, open)
                    .on_hover_text("Opens this address in your default browser")
                    .clicked()
                {
                    context.open_url(egui::OpenUrl::new_tab(self.url.trim()));
                }
                ui.add_space(6.0);
                if ui
                    .add_enabled(
                        ready,
                        egui::Button::new("Check again")
                            .min_size(egui::vec2(ui.available_width(), 26.0)),
                    )
                    .clicked()
                {
                    self.last_check = None;
                }
                ui.add_space(10.0);
                ui.label(
                    RichText::new(
                        "Forge Room shows no web page itself; the preview opens in your browser.",
                    )
                    .size(11.5)
                    .color(theme::text_dim()),
                );
            });
        close
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_the_last_local_address_in_terminal_output() {
        let text = "  Local:   http://localhost:5173/\n  Network: http://192.168.1.34:5173/\n\
                    later http://127.0.0.1:3000.\n";
        assert_eq!(
            find_local_url(text).as_deref(),
            Some("http://127.0.0.1:3000")
        );
        assert_eq!(find_local_url("see http://example.com:80/ now"), None);
        assert_eq!(find_local_url("nothing here"), None);
    }

    #[test]
    fn ipv6_loopback_and_bare_hosts_are_handled() {
        assert_eq!(host_port("http://[::1]/"), Some(("[::1]".into(), 80)));
        assert_eq!(host_port("https://[::1]/"), Some(("[::1]".into(), 443)));
        assert_eq!(
            host_port("http://[::1]:5173/"),
            Some(("[::1]".into(), 5173))
        );
        assert_eq!(host_port("http://[::1"), None);
        assert_eq!(
            web_url("github.com/o/r").as_deref(),
            Some("https://github.com/o/r")
        );
        assert_eq!(web_url("httpx://example.test/payload"), None);
        assert_eq!(web_url("javascript:alert(1)"), None);
        assert_eq!(web_url("ftp://example.test"), None);
        assert_eq!(web_url(""), None);
    }

    #[test]
    fn only_plain_web_addresses_are_accepted() {
        assert!(is_web_url("http://localhost:5173/app"));
        assert!(is_web_url("https://example.com"));
        assert!(!is_web_url("file:///C:/secret.txt"));
        assert!(!is_web_url("javascript:alert(1)"));
        assert!(!is_web_url("http://user@host/"));
        assert!(!is_web_url("http://"));
        assert!(!is_web_url("localhost:3000"));
    }
}
