use std::{fmt::Display, sync::LazyLock};

/// The current terminal type.
pub static TERMINAL_TYPE: LazyLock<TerminalType> = LazyLock::new(|| {
    if running_in_windows_terminal() {
        TerminalType::WindowsTerminal
    } else if running_in_vscode_terminal() {
        TerminalType::VSCodeTerminal
    } else {
        TerminalType::Standard
    }
});

/// If better symbols are supported.
pub static BETTER_SYMBOLS: LazyLock<bool> =
    LazyLock::new(|| TERMINAL_TYPE.supports_better_symbols());

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TerminalType {
    #[default]
    /// Standard terminal.
    Standard,
    /// Windows Terminal.
    WindowsTerminal,
    /// VS Code Terminal.
    VSCodeTerminal,
}

impl Display for TerminalType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl TerminalType {
    /// Get a human-readable name for the terminal type.
    #[inline]
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Standard => "Standard Terminal",
            Self::WindowsTerminal => "Windows Terminal",
            Self::VSCodeTerminal => "VS Code Terminal",
        }
    }

    /// Returns `true` if the terminal supports better symbols.
    #[inline]
    #[must_use]
    pub const fn supports_better_symbols(&self) -> bool {
        matches!(self, Self::WindowsTerminal | Self::VSCodeTerminal)
    }
}

/// Check if running in Windows Terminal.
#[inline]
#[must_use]
pub fn running_in_windows_terminal() -> bool {
    std::env::var("WT_SESSION").is_ok()
}

/// Check if running in vs code terminal.
#[inline]
#[must_use]
pub fn running_in_vscode_terminal() -> bool {
    matches!(std::env::var("TERM_PROGRAM"), Ok(val) if val.to_lowercase().contains("vscode"))
}

/// Check if Windows Terminal is installed on this system.
#[inline]
#[must_use]
pub fn windows_terminal_installed() -> bool {
    #[cfg(not(windows))]
    {
        false
    }

    #[cfg(windows)]
    {
        /// Path to Windows Terminal install in registry.
        const WINDOWS_TERMINAL_INSTALL_PATH: &str =
            r"SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\wt.exe";

        use winreg::RegKey;
        use winreg::enums::HKEY_CURRENT_USER;

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        hkcu.open_subkey(WINDOWS_TERMINAL_INSTALL_PATH).is_ok()
    }
}

/// Relaunch the application in windows terminal
/// # Errors
/// Returns an error if the relaunch fails or if not on Windows.
#[inline]
pub fn relaunch_in_windows_terminal() -> color_eyre::Result<()> {
    #[cfg(not(windows))]
    {
        Err(color_eyre::eyre::eyre!(
            "Relaunching in Windows Terminal is only supported on Windows."
        ))
    }

    #[cfg(windows)]
    {
        use std::env;
        use std::process::Command;

        let current_exe = env::current_exe()?;
        let current_wd = env::current_dir()?;
        let args: Vec<String> = env::args().skip(1).collect();

        Command::new("wt")
            .arg("new-tab")
            .arg("--startingDirectory")
            .arg(current_wd)
            .arg("--")
            .arg(current_exe)
            .args(&args)
            .spawn()?;

        Ok(())
    }
}
