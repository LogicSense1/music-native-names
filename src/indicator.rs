use crate::Status;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Indicator {
    Enabled,
    Connecting,
    Waiting,
    Paused,
    Error,
}

impl Indicator {
    pub fn from_status(enabled: bool, status: &Status) -> Self {
        if !enabled {
            Self::Paused
        } else if status.last_error.is_some() {
            Self::Error
        } else if status.installed {
            Self::Enabled
        } else if status.music_pid == 0 && status.phase == "等待 Music" {
            Self::Waiting
        } else {
            Self::Connecting
        }
    }

    pub fn symbol(self) -> &'static str {
        match self {
            Self::Enabled => "music.note",
            Self::Connecting => "arrow.triangle.2.circlepath",
            Self::Waiting => "clock",
            Self::Paused => "pause.circle",
            Self::Error => "exclamationmark.circle",
        }
    }

    pub fn color_selector(self) -> &'static str {
        match self {
            Self::Enabled => "systemGreenColor",
            Self::Connecting | Self::Waiting => "systemOrangeColor",
            Self::Paused => "secondaryLabelColor",
            Self::Error => "systemRedColor",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Enabled => "Enabled",
            Self::Connecting => "Connecting",
            Self::Waiting => "Waiting for Music",
            Self::Paused => "Paused",
            Self::Error => "Connection or catalog error",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pause_and_error_take_precedence_over_installed_status() {
        let mut status = Status {
            installed: true,
            ..Default::default()
        };
        assert_eq!(Indicator::from_status(true, &status), Indicator::Enabled);
        status.last_error = Some("catalog unavailable".into());
        assert_eq!(Indicator::from_status(true, &status), Indicator::Error);
        assert_eq!(Indicator::from_status(false, &status), Indicator::Paused);
    }

    #[test]
    fn waiting_and_connecting_have_distinct_accessible_symbols() {
        let mut status = Status::default();
        assert_eq!(Indicator::from_status(true, &status), Indicator::Connecting);
        status.phase = "等待 Music".into();
        assert_eq!(Indicator::from_status(true, &status), Indicator::Waiting);
        let states = [
            Indicator::Enabled,
            Indicator::Connecting,
            Indicator::Waiting,
            Indicator::Paused,
            Indicator::Error,
        ];
        let symbols: std::collections::HashSet<_> = states.iter().map(|s| s.symbol()).collect();
        assert_eq!(symbols.len(), states.len());
        assert!(states.iter().all(|s| !s.label().is_empty()));
    }
}
