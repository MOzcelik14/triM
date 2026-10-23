#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShortcutProfile {
    #[default]
    TrimDefault,
    PremierePro,
    DaVinciResolve,
}

impl ShortcutProfile {
    pub const ALL: &'static [Self] = &[
        Self::TrimDefault,
        Self::PremierePro,
        Self::DaVinciResolve,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Self::TrimDefault => "triM. Varsayılan",
            Self::PremierePro => "Adobe Premiere Pro",
            Self::DaVinciResolve => "DaVinci Resolve",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::TrimDefault => "S Böl, C Jilet, V Seçim, B Dalgalanma, Q/W Kırpma, Boşluk Oynat",
            Self::PremierePro => "C Razor, V Seçim, A İleri Seç, Ctrl+K Böl, Q/W Dalgalanma Kırpma",
            Self::DaVinciResolve => "B Blade, A Normal Seçim, Ctrl+\\ Böl, Backspace Sil, Shift+Del Boşluğu Kapat",
        }
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
}
