//! TUI components for interactive terminal interfaces.
//!
//! Provides reusable components for:
//! - Interactive selection menus (single/multi select) via `dialoguer`
//! - Confirmation and text input prompts via `dialoguer`
//! - A basic `ratatui` terminal setup/teardown helper
//! - Styled text helpers using `crossterm`

use std::io;

use crossterm::{
    execute,
    style::{Attribute, Color, SetAttribute, SetForegroundColor, ResetColor, Print},
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
    event::{DisableMouseCapture, EnableMouseCapture},
};
use dialoguer::{Confirm, Input, MultiSelect, Select, theme::ColorfulTheme};
use ratatui::{backend::CrosstermBackend, Terminal};

use crate::errors::{BosuaError, Result};

// ---------------------------------------------------------------------------
// Dialoguer-based prompts (equivalent to Go's huh)
// ---------------------------------------------------------------------------

/// Display a single-select menu and return the chosen index.
///
/// Returns `None` if the user cancels (e.g. Ctrl-C / Esc).
pub fn select_menu(prompt: &str, items: &[&str]) -> Result<Option<usize>> {
    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .items(items)
        .default(0)
        .interact_opt()
        .map_err(|e| BosuaError::Application(format!("Select menu error: {e}")))?;
    Ok(selection)
}

/// Display a multi-select menu and return the chosen indices.
///
/// Returns an empty vec if the user cancels.
pub fn multi_select_menu(prompt: &str, items: &[&str]) -> Result<Vec<usize>> {
    let selections = MultiSelect::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .items(items)
        .interact_opt()
        .map_err(|e| BosuaError::Application(format!("Multi-select menu error: {e}")))?;
    Ok(selections.unwrap_or_default())
}

/// Display a yes/no confirmation prompt.
///
/// `default` sets the pre-selected answer.
/// Returns `None` if the user cancels.
pub fn confirm(prompt: &str, default: bool) -> Result<Option<bool>> {
    let result = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .default(default)
        .interact_opt()
        .map_err(|e| BosuaError::Application(format!("Confirm prompt error: {e}")))?;
    Ok(result)
}

/// Display a text input prompt and return the entered string.
///
/// `default_value` is shown as the pre-filled value (if `Some`).
pub fn text_input(prompt: &str, default_value: Option<&str>) -> Result<String> {
    let theme = ColorfulTheme::default();
    let mut builder = Input::<String>::with_theme(&theme).with_prompt(prompt);
    if let Some(val) = default_value {
        builder = builder.default(val.to_string());
    }
    let result = builder
        .interact_text()
        .map_err(|e| BosuaError::Application(format!("Text input error: {e}")))?;
    Ok(result)
}

// ---------------------------------------------------------------------------
// Ratatui terminal helpers (equivalent to Go's bubbletea setup/teardown)
// ---------------------------------------------------------------------------

/// A wrapper around a `ratatui::Terminal` that restores the terminal on drop.
///
/// Use [`TerminalGuard::new`] to enter the alternate screen and enable raw
/// mode, then call [`TerminalGuard::terminal`] to get a mutable reference
/// for drawing. The terminal is automatically restored when the guard is
/// dropped.
pub struct TerminalGuard {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
}

impl TerminalGuard {
    /// Enter raw mode, switch to the alternate screen, and create a ratatui
    /// terminal ready for drawing.
    pub fn new() -> Result<Self> {
        terminal::enable_raw_mode()
            .map_err(|e| BosuaError::Application(format!("Failed to enable raw mode: {e}")))?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
            .map_err(|e| BosuaError::Application(format!("Failed to enter alternate screen: {e}")))?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)
            .map_err(|e| BosuaError::Application(format!("Failed to create terminal: {e}")))?;
        Ok(Self { terminal })
    }

    /// Get a mutable reference to the underlying ratatui terminal for drawing.
    pub fn terminal(&mut self) -> &mut Terminal<CrosstermBackend<io::Stdout>> {
        &mut self.terminal
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
        let _ = execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        );
        let _ = self.terminal.show_cursor();
    }
}

/// Convenience function to restore the terminal to its normal state.
///
/// Useful as a panic hook or cleanup in error paths where you don't have
/// access to the `TerminalGuard`.
pub fn restore_terminal() {
    let _ = terminal::disable_raw_mode();
    let _ = execute!(
        io::stdout(),
        LeaveAlternateScreen,
        DisableMouseCapture
    );
}

// ---------------------------------------------------------------------------
// Crossterm styled text helpers (equivalent to Go's lipgloss)
// ---------------------------------------------------------------------------

/// Print text with a specific foreground color.
pub fn print_colored(text: &str, color: Color) {
    let _ = execute!(
        io::stdout(),
        SetForegroundColor(color),
        Print(text),
        ResetColor,
        Print("\n"),
    );
}

/// Print bold text with a specific foreground color.
pub fn print_bold_colored(text: &str, color: Color) {
    let _ = execute!(
        io::stdout(),
        SetAttribute(Attribute::Bold),
        SetForegroundColor(color),
        Print(text),
        ResetColor,
        SetAttribute(Attribute::Reset),
        Print("\n"),
    );
}

/// Print text with dim styling.
pub fn print_dim(text: &str) {
    let _ = execute!(
        io::stdout(),
        SetAttribute(Attribute::Dim),
        Print(text),
        SetAttribute(Attribute::Reset),
        Print("\n"),
    );
}

/// Print a styled header line (bold + underlined + colored).
pub fn print_header(text: &str, color: Color) {
    let _ = execute!(
        io::stdout(),
        SetAttribute(Attribute::Bold),
        SetAttribute(Attribute::Underlined),
        SetForegroundColor(color),
        Print(text),
        ResetColor,
        SetAttribute(Attribute::Reset),
        Print("\n"),
    );
}

/// Format a key-value pair with the key in one color and value in another.
pub fn print_key_value(key: &str, value: &str, key_color: Color, value_color: Color) {
    let _ = execute!(
        io::stdout(),
        SetForegroundColor(key_color),
        SetAttribute(Attribute::Bold),
        Print(key),
        SetAttribute(Attribute::Reset),
        ResetColor,
        Print(": "),
        SetForegroundColor(value_color),
        Print(value),
        ResetColor,
        Print("\n"),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    // NOTE: Dialoguer prompts and ratatui terminal setup require a real TTY,
    // so we only test the non-interactive helpers and construction logic here.

    #[test]
    fn test_restore_terminal_does_not_panic() {
        // Should be safe to call even when not in raw mode.
        restore_terminal();
    }

    #[test]
    fn test_print_colored_does_not_panic() {
        print_colored("hello", Color::Green);
    }

    #[test]
    fn test_print_bold_colored_does_not_panic() {
        print_bold_colored("bold hello", Color::Cyan);
    }

    #[test]
    fn test_print_dim_does_not_panic() {
        print_dim("dim text");
    }

    #[test]
    fn test_print_header_does_not_panic() {
        print_header("Header", Color::Magenta);
    }

    #[test]
    fn test_print_key_value_does_not_panic() {
        print_key_value("Name", "Bosua", Color::Yellow, Color::White);
    }
}
