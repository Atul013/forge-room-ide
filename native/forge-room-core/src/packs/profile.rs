use crate::workflows::text;
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Density {
    Compact,
    #[default]
    Comfortable,
    Spacious,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Layout {
    pub sidebar: bool,
    pub terminal: bool,
    pub dock: bool,
    pub preview: bool,
    pub sidebar_width: u16,
    pub dock_width: u16,
    pub terminal_height: u16,
}
impl Default for Layout {
    fn default() -> Self {
        Self {
            sidebar: true,
            terminal: true,
            dock: true,
            preview: false,
            sidebar_width: 260,
            dock_width: 364,
            terminal_height: 220,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub version: u32,
    pub name: String,
    #[serde(default)]
    pub theme: Option<String>,
    pub editor_scale: f32,
    /// Whole-interface physical pixels per layout point; None preserves automatic sizing.
    #[serde(default)]
    pub ui_scale: Option<f32>,
    #[serde(default)]
    pub density: Density,
    #[serde(default)]
    pub reduced_motion: bool,
    #[serde(default)]
    pub layout: Layout,
}
impl Default for Profile {
    fn default() -> Self {
        Self {
            version: 1,
            name: "Personal profile".into(),
            theme: None,
            editor_scale: 1.0,
            ui_scale: None,
            density: Density::Comfortable,
            reduced_motion: false,
            layout: Layout::default(),
        }
    }
}
impl Profile {
    pub fn parse(text: &str) -> Result<Self, String> {
        if text.len() > 64 * 1024 {
            return Err("Profile exceeds 64 KiB".into());
        }
        let profile: Self = serde_json::from_str(text).map_err(|e| e.to_string())?;
        profile.validate()?;
        Ok(profile)
    }
    pub fn validate(&self) -> Result<(), String> {
        text(&self.name, 160, true)?;
        if self.version != 1
            || !self.editor_scale.is_finite()
            || !(0.7..=2.0).contains(&self.editor_scale)
            || !(180..=600).contains(&self.layout.sidebar_width)
            || !(280..=800).contains(&self.layout.dock_width)
            || !(120..=800).contains(&self.layout.terminal_height)
        {
            return Err("Profile contains unsupported version, scale or layout bounds".into());
        }
        if self
            .ui_scale
            .is_some_and(|scale| !scale.is_finite() || !(0.8..=2.0).contains(&scale))
        {
            return Err("Profile interface size must be automatic or 80–200%".into());
        }
        if self.theme.as_ref().is_some_and(|theme| {
            theme.is_empty()
                || theme.len() > 180
                || !theme.bytes().all(|byte| {
                    byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_' | b'/')
                })
        }) {
            return Err("Profile theme reference is invalid".into());
        }
        Ok(())
    }
    pub fn changes(&self, next: &Self) -> Result<Vec<String>, String> {
        self.validate()?;
        next.validate()?;
        let mut changes = Vec::new();
        if self.theme != next.theme {
            changes.push(format!("Theme: {:?} → {:?}", self.theme, next.theme));
        }
        if self.editor_scale != next.editor_scale {
            changes.push(format!(
                "Editor text: {:.0}% → {:.0}%",
                self.editor_scale * 100.0,
                next.editor_scale * 100.0
            ));
        }
        if self.ui_scale != next.ui_scale {
            changes.push(format!(
                "Interface size: {:?} → {:?}",
                self.ui_scale, next.ui_scale
            ));
        }
        if self.density != next.density {
            changes.push(format!("Density: {:?} → {:?}", self.density, next.density));
        }
        if self.reduced_motion != next.reduced_motion {
            changes.push(format!(
                "Reduced motion: {} → {}",
                self.reduced_motion, next.reduced_motion
            ));
        }
        if self.layout != next.layout {
            changes.push(format!("Layout: {:?} → {:?}", self.layout, next.layout));
        }
        Ok(changes)
    }
}
