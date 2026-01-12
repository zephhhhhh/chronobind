use std::path::PathBuf;

use clap::{Parser, ValueEnum};
use terminal_relaunch::TerminalType;

#[derive(Debug, Clone, PartialEq, Eq, Hash, ValueEnum)]
pub enum TargetTerminal {
    /// `Windows Terminal`. (terminal app from Microsoft Store `wt.exe`)
    #[cfg_attr(not(target_os = "windows"), value(hide = true))]
    WindowsTerminal,

    /// Third party `MacOS` terminal `iTerm2`.
    #[cfg_attr(not(target_os = "macos"), value(hide = true))]
    ITerm2,
    /// Third party `MacOS` terminal `Kitty`.
    #[cfg_attr(not(target_os = "macos"), value(hide = true))]
    Kitty,
    /// Third party `MacOS` terminal `Ghostty`.
    #[cfg_attr(not(target_os = "macos"), value(hide = true))]
    Ghostty,

    /// Third party terminal `WezTerm`.
    WezTerm,
    /// Third party terminal `Alacritty`.
    Alacritty,
}

impl TargetTerminal {
    /// Convert to `terminal_relaunch::TerminalType`.
    #[must_use]
    pub const fn to_terminal_type(&self) -> TerminalType {
        match self {
            Self::WindowsTerminal => TerminalType::WindowsTerminal,
            Self::ITerm2 => TerminalType::ITerm2,
            Self::Kitty => TerminalType::Kitty,
            Self::Ghostty => TerminalType::Ghostty,
            Self::WezTerm => TerminalType::WezTerm,
            Self::Alacritty => TerminalType::Alacritty,
        }
    }
}

/// CLI arguments for `ChronoBind` application.
#[derive(Parser, Debug, Default, Clone, PartialEq, Eq, Hash)]
#[command(name="ChronoBind", version, about, long_about = None)]
pub struct ChronoCLIArgs {
    /// Optional file path to open in the import window on startup.
    pub file_to_import: Option<PathBuf>,

    /// Disable relaunching in a terminal.
    #[arg(long, short, default_value_t = false)]
    pub no_relaunch: bool,
    /// Set a preferred terminal, will relaunch in this terminal if available.
    /// This flag is ignored if `--no-relaunch` is also set.
    #[arg(value_enum, long, short)]
    pub terminal: Option<TargetTerminal>,
    /// Only relaunch if the preferred terminal is available,
    /// do not relaunch in any other terminal.
    #[arg(long, short, default_value_t = false)]
    pub preferred_only: bool,

    /// Flag to signal if the terminal has been relaunched.
    #[arg(long = "relaunched-term", default_value_t = false, hide = true)]
    relaunched: bool,
}

impl ChronoCLIArgs {
    /// Check if the application has been relaunched in a new terminal.
    #[inline]
    #[must_use]
    pub const fn has_relaunched(&self) -> bool {
        self.relaunched
    }

    /// Check if the current terminal does not match the preferred terminal and should relaunch.
    #[inline]
    #[must_use]
    pub fn should_relaunch(&self) -> bool {
        self.terminal.as_ref().is_some_and(|preferred| {
            *terminal_relaunch::CURRENT_TERMINAL != preferred.to_terminal_type()
        })
    }
}
