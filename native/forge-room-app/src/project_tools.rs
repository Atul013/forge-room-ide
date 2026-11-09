//! Local project actions. A command runs only after its button is pressed.
use eframe::egui;
use std::{fs, path::Path};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Workflows,
    Models,
    RunDebug,
    Tests,
    Extensions,
    Settings,
    Accessibility,
}

impl Page {
    pub fn title(self) -> &'static str {
        match self {
            Self::Workflows => "Workflows",
            Self::Models => "Models",
            Self::RunDebug => "Run & Debug",
            Self::Tests => "Tests",
            Self::Extensions => "Extensions",
            Self::Settings => "Settings",
            Self::Accessibility => "Accessibility",
        }
    }
}

struct Action {
    label: String,
    command: String,
    detail: &'static str,
}

pub enum Selected {
    Command(String),
    Studio,
    ToggleTerminal,
    ToggleDock,
    TogglePreview,
}

fn action(label: &str, command: &str, detail: &'static str) -> Action {
    Action {
        label: label.into(),
        command: command.into(),
        detail,
    }
}

fn package_scripts(root: &Path, page: Page) -> Vec<Action> {
    let Ok(text) = fs::read_to_string(root.join("package.json")) else {
        return Vec::new();
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        return Vec::new();
    };
    let Some(scripts) = value.get("scripts").and_then(|value| value.as_object()) else {
        return Vec::new();
    };
    let mut names: Vec<_> = scripts
        .keys()
        .filter(|name| {
            !name.is_empty()
                && name
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b':'))
        })
        .collect();
    names.sort();
    names
        .into_iter()
        .filter(|name| match page {
            Page::Tests => name.contains("test") || name.contains("lint") || name.contains("check"),
            Page::RunDebug => {
                name.as_str() == "start" || name.contains("dev") || name.contains("run")
            }
            _ => true,
        })
        .take(20)
        .map(|name| Action {
            label: (*name).clone(),
            command: format!("npm run {name}"),
            detail: "Project-defined npm script; inspect package.json before running",
        })
        .collect()
}

fn actions(root: &Path, page: Page) -> Vec<Action> {
    let rust_manifest = if root.join("Cargo.toml").exists() {
        Some("")
    } else if root.join("native").join("Cargo.toml").exists() {
        Some(" --manifest-path native/Cargo.toml")
    } else {
        None
    };
    if let Some(manifest) = rust_manifest {
        return match page {
            Page::Tests => vec![Action {
                command: format!("cargo test{manifest}"),
                ..action("Run Rust tests", "", "Runs the workspace test suite")
            }],
            Page::Workflows => vec![
                Action {
                    command: format!("cargo check{manifest}"),
                    ..action("Check Rust project", "", "Checks the workspace")
                },
                action(
                    "Check Rust formatting",
                    if manifest.is_empty() {
                        "cargo fmt --check"
                    } else {
                        "cargo fmt --manifest-path native/Cargo.toml --check"
                    },
                    "Reports formatting differences",
                ),
            ],
            Page::RunDebug => vec![Action {
                command: format!("cargo run{manifest}"),
                ..action("Run Rust project", "", "Runs the default binary")
            }],
            _ => Vec::new(),
        };
    }
    if root.join("package.json").exists() {
        return package_scripts(root, page);
    }
    if root.join("pyproject.toml").exists() || root.join("requirements.txt").exists() {
        return match page {
            Page::Tests => vec![action(
                "Run Python tests",
                "python -m pytest",
                "Requires pytest in the active environment",
            )],
            Page::Workflows => vec![action(
                "Compile Python files",
                "python -m compileall .",
                "Checks syntax in this folder",
            )],
            _ => Vec::new(),
        };
    }
    if root.join("go.mod").exists() {
        return match page {
            Page::Tests => vec![action(
                "Run Go tests",
                "go test ./...",
                "Runs tests in all packages",
            )],
            Page::Workflows => vec![action(
                "Check Go project",
                "go vet ./...",
                "Checks packages for suspicious code",
            )],
            Page::RunDebug => vec![action(
                "Run Go project",
                "go run .",
                "Runs the root package",
            )],
            _ => Vec::new(),
        };
    }
    if root.join("pom.xml").exists() {
        return match page {
            Page::Tests => vec![action(
                "Run Maven tests",
                "mvn test",
                "Runs the project's test phase",
            )],
            Page::Workflows => vec![action(
                "Package Maven project",
                "mvn package",
                "Builds the project package",
            )],
            _ => Vec::new(),
        };
    }
    Vec::new()
}

pub fn show(
    ui: &mut egui::Ui,
    page: Page,
    root: Option<&Path>,
    runtimes: &[String],
) -> Option<Selected> {
    ui.heading(page.title());
    ui.separator();
    match page {
        Page::Tests | Page::Workflows | Page::RunDebug => {
            let Some(root) = root else {
                ui.label("Open a project folder to discover local commands.");
                return None;
            };
            ui.label(format!("Project: {}", root.display()));
            ui.add_space(8.0);
            let actions = actions(root, page);
            if actions.is_empty() {
                ui.label("No supported local command detected for this project.");
            }
            for action in actions {
                ui.group(|ui| {
                    ui.label(action.detail);
                    ui.monospace(action.command.as_str());
                });
                if ui.button(action.label.as_str()).clicked() {
                    return Some(Selected::Command(action.command));
                }
                ui.add_space(6.0);
            }
            if page == Page::RunDebug {
                ui.small("Debugger adapters and breakpoints are not connected yet.");
            }
        }
        Page::Models => {
            ui.label("Detected project runtimes:");
            if runtimes.is_empty() {
                ui.label("Open a project to inspect its runtimes.");
            }
            for runtime in runtimes {
                ui.label(runtime.as_str());
            }
            ui.small("AI model providers are not connected in this preview.");
        }
        Page::Extensions => {
            ui.label("Native extension support is being developed.");
            ui.small("JavaScript VSIX extensions cannot run in this native preview.");
            if let Some(root) = root {
                let path = root.join(".vscode").join("extensions.json");
                if let Ok(text) = fs::read_to_string(&path) {
                    ui.separator();
                    ui.label("Project recommendations");
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
                        if let Some(items) = value.get("recommendations").and_then(|v| v.as_array())
                        {
                            for item in items.iter().take(30).filter_map(|v| v.as_str()) {
                                ui.monospace(item);
                            }
                        }
                    } else {
                        ui.small("Could not parse .vscode/extensions.json as JSON.");
                    }
                    ui.small("Recommendations are displayed only; they are not installed.");
                }
            }
        }
        Page::Settings => {
            ui.label("Appearance");
            if ui.button("Open Studio themes").clicked() {
                return Some(Selected::Studio);
            }
            ui.separator();
            ui.label("Layout");
            if ui.button("Toggle terminal").clicked() {
                return Some(Selected::ToggleTerminal);
            }
            if ui.button("Toggle Room and AI dock").clicked() {
                return Some(Selected::ToggleDock);
            }
            if ui.button("Toggle preview").clicked() {
                return Some(Selected::TogglePreview);
            }
            ui.label("Project data");
            ui.small("Room state is stored locally with the project. Theme choice is stored in user settings.");
        }
        Page::Accessibility => {
            ui.label("Keyboard access");
            ui.label("Ctrl+Shift+E  Explorer");
            ui.label("Ctrl+Shift+F  Search");
            ui.label("Ctrl+Shift+G  Source Control");
            ui.label("Ctrl+`  Terminal");
            ui.small("Theme contrast can be changed in Studio.");
        }
    }
    None
}
