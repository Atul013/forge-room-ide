//! Local project actions. A command runs only after its button is pressed.
use crate::commands::Command;
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
    AppCommand(Command),
    Studio,
    ImportGrammar,
    ImportSnippets,
    InsertSnippet(String),
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
    if root.join("build.gradle").is_file() || root.join("build.gradle.kts").is_file() {
        let runner = if root.join("gradlew.bat").is_file() {
            ".\\gradlew.bat"
        } else {
            "gradle"
        };
        return match page {
            Page::Tests => vec![Action {
                command: format!("{runner} test"),
                ..action(
                    "Run Gradle tests",
                    "",
                    "Runs project code; inspect build.gradle before running",
                )
            }],
            Page::Workflows => vec![Action {
                command: format!("{runner} build"),
                ..action(
                    "Build Gradle project",
                    "",
                    "Runs project code; inspect build.gradle before running",
                )
            }],
            _ => Vec::new(),
        };
    }
    // Prefer one solution even when its projects also live at the root. Names are restricted
    // before placing them in a PowerShell command; quoted or shell-special names are skipped.
    let mut solutions = Vec::new();
    let mut projects = Vec::new();
    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if name.len() > 120
                || !name.bytes().all(|byte| {
                    byte.is_ascii_alphanumeric() || matches!(byte, b' ' | b'_' | b'-' | b'.')
                })
            {
                continue;
            }
            match path
                .extension()
                .and_then(|extension| extension.to_str())
                .map(str::to_ascii_lowercase)
                .as_deref()
            {
                Some("sln" | "slnx") => solutions.push(name.to_string()),
                Some("csproj" | "fsproj" | "vbproj") => projects.push(name.to_string()),
                _ => {}
            }
        }
    }
    let dotnet_target = if solutions.len() == 1 {
        solutions.first()
    } else if solutions.is_empty() && projects.len() == 1 {
        projects.first()
    } else {
        None
    };
    if let Some(target) = dotnet_target {
        return match page {
            Page::Tests => vec![Action {
                command: format!("dotnet test '{target}'"),
                ..action(
                    "Run .NET tests",
                    "",
                    "Runs tests for the selected root solution or project",
                )
            }],
            Page::Workflows => vec![Action {
                command: format!("dotnet build '{target}'"),
                ..action(
                    "Build .NET project",
                    "",
                    "Builds the selected root solution or project",
                )
            }],
            _ => Vec::new(),
        };
    }
    if root.join("CMakeLists.txt").is_file() {
        let configured = root.join("build").join("CMakeCache.txt").is_file();
        let tests_ready = root.join("build").join("CTestTestfile.cmake").is_file();
        return match page {
            Page::Workflows => {
                let mut actions = vec![action(
                    "Configure CMake",
                    "cmake -S . -B build",
                    "Generates build files in the project build folder",
                )];
                if configured {
                    actions.push(action(
                        "Build CMake project",
                        "cmake --build build",
                        "Builds the configured project",
                    ));
                }
                actions
            }
            Page::Tests if tests_ready => vec![action(
                "Run CTest tests",
                "ctest --test-dir build --output-on-failure",
                "Runs tests from the configured build folder",
            )],
            Page::Tests => vec![action(
                "Configure CMake for tests",
                "cmake -S . -B build",
                "Configure the project, then return here to run CTest",
            )],
            _ => Vec::new(),
        };
    }
    Vec::new()
}

/// A rail page: the same header (semibold title, 14 px gutter) as Explorer and Source Control.
pub fn show(
    ui: &mut egui::Ui,
    page: Page,
    root: Option<&Path>,
    runtimes: &[String],
    auto_save: bool,
    editor_scale: f32,
) -> Option<Selected> {
    egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(14, 10))
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(page.title())
                    .size(14.5)
                    .family(crate::fonts::semibold())
                    .color(crate::theme::text()),
            );
            ui.add_space(6.0);
            ui.separator();
            page_body(ui, page, root, runtimes, auto_save, editor_scale)
        })
        .inner
}

fn page_body(
    ui: &mut egui::Ui,
    page: Page,
    root: Option<&Path>,
    runtimes: &[String],
    auto_save: bool,
    editor_scale: f32,
) -> Option<Selected> {
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
            ui.separator();
            ui.label("AI agents use installed Claude Code or Codex CLI programs.");
            ui.small("Provider sign-in and local model setup happen in those programs. An open trusted folder is required.");
            if ui.button("Open AI Agent").clicked() {
                return Some(Selected::AppCommand(Command::OpenAgent));
            }
        }
        Page::Extensions => {
            ui.label("Native extension support is being developed.");
            ui.small("JavaScript VSIX extensions cannot run in this native preview.");
            ui.separator();
            ui.label("Syntax grammars");
            ui.small("Import a local .sublime-syntax file for editor coloring. Grammars do not run extension code.");
            if ui.button("Import syntax grammar…").clicked() {
                return Some(Selected::ImportGrammar);
            }
            for name in crate::syntax::grammar_names() {
                ui.label(name);
            }
            ui.separator();
            ui.label("Plain-text snippets");
            ui.small("Import a VS Code JSON snippet file. Snippets with placeholders are skipped.");
            if ui.button("Import snippets…").clicked() {
                return Some(Selected::ImportSnippets);
            }
            for snippet in crate::snippets::installed() {
                if ui
                    .button(format!("Insert {} ({})", snippet.name, snippet.prefix))
                    .clicked()
                {
                    return Some(Selected::InsertSnippet(snippet.body));
                }
            }
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
            if let Some(action) = ui
                .horizontal(|ui| {
                    ui.label(format!("Editor text: {:.0}%", editor_scale * 100.0));
                    if ui
                        .button("−")
                        .on_hover_text("Decrease editor font size (Ctrl+-)")
                        .clicked()
                    {
                        return Some(Selected::AppCommand(Command::ZoomOut));
                    }
                    if ui
                        .button("+")
                        .on_hover_text("Increase editor font size (Ctrl+=)")
                        .clicked()
                    {
                        return Some(Selected::AppCommand(Command::ZoomIn));
                    }
                    if ui
                        .button("Reset")
                        .on_hover_text("Reset editor font size (Ctrl+0)")
                        .clicked()
                    {
                        return Some(Selected::AppCommand(Command::ZoomReset));
                    }
                    None
                })
                .inner
            {
                return Some(action);
            }
            if ui
                .button(if auto_save {
                    "Auto Save: On"
                } else {
                    "Auto Save: Off"
                })
                .on_hover_text("Save changed files after typing stops")
                .clicked()
            {
                return Some(Selected::AppCommand(Command::ToggleAutoSave));
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
            if ui.button("Reset panel layout").clicked() {
                return Some(Selected::AppCommand(Command::ResetLayout));
            }
            ui.label("Project data");
            ui.small("Room state is stored locally with the project. Theme choice is stored in user settings.");
        }
        Page::Accessibility => {
            ui.label("Keyboard access");
            ui.small("Commands with assigned shortcuts in this preview:");
            ui.small("Theme contrast can be changed in Studio.");
            ui.add_space(6.0);
            egui::ScrollArea::vertical()
                .id_salt("keyboard_shortcuts")
                .show(ui, |ui| {
                    for (_, title, shortcut) in crate::commands::ALL {
                        if shortcut.is_empty() {
                            continue;
                        }
                        ui.horizontal(|ui| {
                            ui.monospace(*shortcut);
                            ui.label(*title);
                        });
                    }
                });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gradle_actions_prefer_project_wrapper() {
        let folder = std::env::temp_dir().join(format!(
            "forge-room-gradle-actions-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&folder).unwrap();
        fs::write(folder.join("build.gradle.kts"), "").unwrap();
        assert_eq!(actions(&folder, Page::Tests)[0].command, "gradle test");
        fs::write(folder.join("gradlew.bat"), "").unwrap();
        assert_eq!(
            actions(&folder, Page::Workflows)[0].command,
            ".\\gradlew.bat build"
        );
        assert!(actions(&folder, Page::RunDebug).is_empty());
        fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn dotnet_actions_require_one_unambiguous_root_file() {
        let folder = std::env::temp_dir().join(format!(
            "forge-room-dotnet-actions-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&folder).unwrap();
        fs::write(folder.join("Demo.sln"), "").unwrap();
        assert_eq!(
            actions(&folder, Page::Workflows)[0].command,
            "dotnet build 'Demo.sln'"
        );
        assert_eq!(
            actions(&folder, Page::Tests)[0].command,
            "dotnet test 'Demo.sln'"
        );
        assert!(actions(&folder, Page::RunDebug).is_empty());
        fs::write(folder.join("Other.csproj"), "").unwrap();
        assert_eq!(
            actions(&folder, Page::Tests)[0].command,
            "dotnet test 'Demo.sln'"
        );
        fs::write(folder.join("Second.sln"), "").unwrap();
        assert!(actions(&folder, Page::Tests).is_empty());
        fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn cmake_actions_follow_configure_build_test_state() {
        let folder = std::env::temp_dir().join(format!(
            "forge-room-cmake-actions-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&folder).unwrap();
        fs::write(folder.join("CMakeLists.txt"), "project(example)").unwrap();
        assert_eq!(actions(&folder, Page::Workflows).len(), 1);
        assert_eq!(
            actions(&folder, Page::Tests)[0].command,
            "cmake -S . -B build"
        );
        fs::create_dir(folder.join("build")).unwrap();
        fs::write(folder.join("build").join("CMakeCache.txt"), "").unwrap();
        assert_eq!(actions(&folder, Page::Workflows).len(), 2);
        fs::write(folder.join("build").join("CTestTestfile.cmake"), "").unwrap();
        assert_eq!(
            actions(&folder, Page::Tests)[0].command,
            "ctest --test-dir build --output-on-failure"
        );
        fs::remove_dir_all(folder).unwrap();
    }
}
