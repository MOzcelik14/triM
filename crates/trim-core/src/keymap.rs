//! Serializable keymap configuration, preset profiles, and conflict resolver.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShortcutProfile {
    #[default]
    TrimDefault,
    PremierePro,
    DaVinciResolve,
    FinalCutPro,
}

impl ShortcutProfile {
    pub const ALL: &'static [Self] = &[
        Self::TrimDefault,
        Self::PremierePro,
        Self::DaVinciResolve,
        Self::FinalCutPro,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Self::TrimDefault => "triM. Varsayılan",
            Self::PremierePro => "Adobe Premiere Pro",
            Self::DaVinciResolve => "DaVinci Resolve",
            Self::FinalCutPro => "Final Cut Pro",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::TrimDefault => "S Böl, C Jilet, V Seçim, B Dalgalanma, Q/W Kırpma, Boşluk Oynat",
            Self::PremierePro => "C Razor, V Seçim, A İleri Seç, Ctrl+K Böl, Q/W Dalgalanma Kırpma",
            Self::DaVinciResolve => {
                "B Blade, A Normal Seçim, Ctrl+\\ Böl, Backspace Sil, Shift+Del Boşluğu Kapat"
            }
            Self::FinalCutPro => "B Blade, A Seçim, Cmd/Ctrl+B Böl, [ / ] Kırpma, Boşluk Oynat",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ShortcutAction {
    PlayPause,
    SelectTool,
    RazorTool,
    RippleTool,
    SplitAtPlayhead,
    RippleTrimHead,
    RippleTrimTail,
    ToggleSnap,
    Undo,
    Redo,
    DeleteSelected,
    RippleDelete,
}

impl ShortcutAction {
    pub const ALL: &'static [Self] = &[
        Self::PlayPause,
        Self::SelectTool,
        Self::RazorTool,
        Self::RippleTool,
        Self::SplitAtPlayhead,
        Self::RippleTrimHead,
        Self::RippleTrimTail,
        Self::ToggleSnap,
        Self::Undo,
        Self::Redo,
        Self::DeleteSelected,
        Self::RippleDelete,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Self::PlayPause => "Oynat / Duraklat (Play / Pause)",
            Self::SelectTool => "Seçim Aracı (Select Tool)",
            Self::RazorTool => "Jilet / Kesme Aracı (Razor / Blade)",
            Self::RippleTool => "Dalgalanma Aracı (Ripple / Trim)",
            Self::SplitAtPlayhead => "İmleç Konumunda Klibi Böl (Split)",
            Self::RippleTrimHead => "Baştan Dalgalanma Kırp (Ripple Trim Head)",
            Self::RippleTrimTail => "Sondan Dalgalanma Kırp (Ripple Trim Tail)",
            Self::ToggleSnap => "Mıknatıs / Yapışma (Snap)",
            Self::Undo => "Geri Al (Undo)",
            Self::Redo => "Yinele (Redo)",
            Self::DeleteSelected => "Seçili Klibi Sil (Delete)",
            Self::RippleDelete => "Boşluğu Kapatarak Sil (Ripple Delete)",
        }
    }

    pub fn i18n_key(&self) -> &'static str {
        match self {
            Self::PlayPause => "action_play_pause",
            Self::SelectTool => "action_select_tool",
            Self::RazorTool => "action_razor_tool",
            Self::RippleTool => "action_ripple_tool",
            Self::SplitAtPlayhead => "action_split_at_playhead",
            Self::RippleTrimHead => "action_ripple_trim_head",
            Self::RippleTrimTail => "action_ripple_trim_tail",
            Self::ToggleSnap => "action_toggle_snap",
            Self::Undo => "action_undo",
            Self::Redo => "action_redo",
            Self::DeleteSelected => "action_delete_selected",
            Self::RippleDelete => "action_ripple_delete",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeyBinding {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub key: String,
}

impl KeyBinding {
    pub fn plain(key: impl Into<String>) -> Self {
        Self {
            ctrl: false,
            shift: false,
            alt: false,
            key: key.into(),
        }
    }

    pub fn ctrl(key: impl Into<String>) -> Self {
        Self {
            ctrl: true,
            shift: false,
            alt: false,
            key: key.into(),
        }
    }

    pub fn shift(key: impl Into<String>) -> Self {
        Self {
            ctrl: false,
            shift: true,
            alt: false,
            key: key.into(),
        }
    }

    pub fn display(&self) -> String {
        let mut parts = Vec::new();
        if self.ctrl {
            parts.push("Ctrl");
        }
        if self.shift {
            parts.push("Shift");
        }
        if self.alt {
            parts.push("Alt");
        }
        let key_str = match self.key.as_str() {
            "OpenBracket" => "[",
            "CloseBracket" => "]",
            "Backslash" => "\\",
            "Comma" => ",",
            "Period" => ".",
            "Minus" => "-",
            "Equals" => "=",
            other => other,
        };
        parts.push(key_str);
        parts.join(" + ")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeymapConfig {
    pub bindings: HashMap<ShortcutAction, KeyBinding>,
}

impl KeymapConfig {
    pub fn for_profile(profile: ShortcutProfile) -> Self {
        let mut bindings = HashMap::new();
        bindings.insert(ShortcutAction::PlayPause, KeyBinding::plain("Space"));
        bindings.insert(ShortcutAction::ToggleSnap, KeyBinding::plain("N"));
        bindings.insert(ShortcutAction::Undo, KeyBinding::ctrl("Z"));
        bindings.insert(
            ShortcutAction::Redo,
            KeyBinding {
                ctrl: true,
                shift: true,
                alt: false,
                key: "Z".to_string(),
            },
        );
        bindings.insert(ShortcutAction::DeleteSelected, KeyBinding::plain("Delete"));
        bindings.insert(ShortcutAction::RippleDelete, KeyBinding::shift("Delete"));

        match profile {
            ShortcutProfile::TrimDefault => {
                bindings.insert(ShortcutAction::SelectTool, KeyBinding::plain("V"));
                bindings.insert(ShortcutAction::RazorTool, KeyBinding::plain("C"));
                bindings.insert(ShortcutAction::RippleTool, KeyBinding::plain("B"));
                bindings.insert(ShortcutAction::SplitAtPlayhead, KeyBinding::plain("S"));
                bindings.insert(ShortcutAction::RippleTrimHead, KeyBinding::plain("Q"));
                bindings.insert(ShortcutAction::RippleTrimTail, KeyBinding::plain("W"));
            }
            ShortcutProfile::PremierePro => {
                bindings.insert(ShortcutAction::SelectTool, KeyBinding::plain("V"));
                bindings.insert(ShortcutAction::RazorTool, KeyBinding::plain("C"));
                bindings.insert(ShortcutAction::RippleTool, KeyBinding::plain("B"));
                bindings.insert(ShortcutAction::SplitAtPlayhead, KeyBinding::ctrl("K"));
                bindings.insert(ShortcutAction::RippleTrimHead, KeyBinding::plain("Q"));
                bindings.insert(ShortcutAction::RippleTrimTail, KeyBinding::plain("W"));
            }
            ShortcutProfile::DaVinciResolve => {
                bindings.insert(ShortcutAction::SelectTool, KeyBinding::plain("A"));
                bindings.insert(ShortcutAction::RazorTool, KeyBinding::plain("B"));
                bindings.insert(ShortcutAction::RippleTool, KeyBinding::plain("T"));
                bindings.insert(
                    ShortcutAction::SplitAtPlayhead,
                    KeyBinding::ctrl("Backslash"),
                );
                bindings.insert(
                    ShortcutAction::RippleTrimHead,
                    KeyBinding::shift("OpenBracket"),
                );
                bindings.insert(
                    ShortcutAction::RippleTrimTail,
                    KeyBinding::shift("CloseBracket"),
                );
            }
            ShortcutProfile::FinalCutPro => {
                bindings.insert(ShortcutAction::SelectTool, KeyBinding::plain("A"));
                bindings.insert(ShortcutAction::RazorTool, KeyBinding::plain("B"));
                bindings.insert(ShortcutAction::RippleTool, KeyBinding::plain("T"));
                bindings.insert(ShortcutAction::SplitAtPlayhead, KeyBinding::ctrl("B"));
                bindings.insert(
                    ShortcutAction::RippleTrimHead,
                    KeyBinding::plain("OpenBracket"),
                );
                bindings.insert(
                    ShortcutAction::RippleTrimTail,
                    KeyBinding::plain("CloseBracket"),
                );
            }
        }
        Self { bindings }
    }

    pub fn config_path() -> PathBuf {
        crate::paths::config_dir().join("keybindings.json")
    }

    pub fn find_conflicts(&self) -> Vec<(KeyBinding, Vec<ShortcutAction>)> {
        let mut by_key: HashMap<KeyBinding, Vec<ShortcutAction>> = HashMap::new();
        for (&action, binding) in &self.bindings {
            by_key.entry(binding.clone()).or_default().push(action);
        }
        by_key
            .into_iter()
            .filter(|(_, actions)| actions.len() > 1)
            .collect()
    }

    pub fn rebind(
        &mut self,
        action: ShortcutAction,
        binding: KeyBinding,
        unbind_conflicting: bool,
    ) -> Vec<ShortcutAction> {
        let conflicts: Vec<ShortcutAction> = self
            .bindings
            .iter()
            .filter(|(&a, b)| a != action && **b == binding)
            .map(|(&a, _)| a)
            .collect();
        if unbind_conflicting {
            for a in &conflicts {
                self.bindings.remove(a);
            }
            self.bindings.insert(action, binding);
        } else if conflicts.is_empty() {
            self.bindings.insert(action, binding);
        }
        conflicts
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    pub fn save_to_path(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = self
            .to_json()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let tmp_name = format!(
            ".{}.tmp.{}",
            path.file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("keymap"),
            uuid::Uuid::new_v4()
        );
        let tmp_path = path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(tmp_name);
        std::fs::write(&tmp_path, json.as_bytes())?;
        if let Err(e) = std::fs::rename(&tmp_path, path) {
            let _ = std::fs::remove_file(&tmp_path);
            return Err(e);
        }
        Ok(())
    }

    pub fn load_from_path(path: &Path) -> Option<Self> {
        let raw = std::fs::read_to_string(path).ok()?;
        Self::from_json(&raw).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shortcut_profiles_all_have_labels() {
        for profile in ShortcutProfile::ALL {
            assert!(!profile.label().is_empty());
            assert!(!profile.description().is_empty());
        }
        assert_eq!(ShortcutProfile::default(), ShortcutProfile::TrimDefault);
    }

    #[test]
    fn test_keymap_all_presets_roundtrip_and_conflict_free() {
        for profile in ShortcutProfile::ALL {
            let cfg = KeymapConfig::for_profile(*profile);
            assert!(
                cfg.find_conflicts().is_empty(),
                "Preset {:?} should have no default conflicts",
                profile
            );
            let json = cfg.to_json().expect("serialize keymap");
            let parsed = KeymapConfig::from_json(&json).expect("deserialize keymap");
            assert_eq!(cfg, parsed);
        }
    }

    #[test]
    fn test_keymap_conflict_detection_and_override() {
        let mut cfg = KeymapConfig::for_profile(ShortcutProfile::TrimDefault);
        let conflicts = cfg.rebind(
            ShortcutAction::SplitAtPlayhead,
            KeyBinding::plain("C"),
            false,
        );
        assert_eq!(conflicts, vec![ShortcutAction::RazorTool]);
        assert_eq!(
            cfg.bindings.get(&ShortcutAction::RazorTool),
            Some(&KeyBinding::plain("C"))
        );

        let cleared = cfg.rebind(
            ShortcutAction::SplitAtPlayhead,
            KeyBinding::plain("C"),
            true,
        );
        assert_eq!(cleared, vec![ShortcutAction::RazorTool]);
        assert!(!cfg.bindings.contains_key(&ShortcutAction::RazorTool));
        assert_eq!(
            cfg.bindings.get(&ShortcutAction::SplitAtPlayhead),
            Some(&KeyBinding::plain("C"))
        );
        assert!(cfg.find_conflicts().is_empty());
    }

    #[test]
    fn test_keymap_load_corrupt_or_empty_falls_back_to_profile() {
        let dir = tempfile::tempdir().expect("tempdir");

        let empty_path = dir.path().join("empty_keybindings.json");
        std::fs::write(&empty_path, "").expect("write empty");
        assert!(KeymapConfig::load_from_path(&empty_path).is_none());

        let corrupt_path = dir.path().join("corrupt_keybindings.json");
        std::fs::write(&corrupt_path, "{ not valid json !!!").expect("write corrupt");
        assert!(KeymapConfig::load_from_path(&corrupt_path).is_none());

        let resolved = KeymapConfig::load_from_path(&corrupt_path)
            .unwrap_or_else(|| KeymapConfig::for_profile(ShortcutProfile::PremierePro));
        assert_eq!(
            resolved,
            KeymapConfig::for_profile(ShortcutProfile::PremierePro)
        );
    }
}
