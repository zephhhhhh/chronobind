use std::path::PathBuf;

#[allow(clippy::wildcard_imports)]
use crate::palette::*;
use crate::{
    backend::InstallBackupOptions,
    popups::{format_option, toggle_option},
    ui::{KeyCodeExt, messages::AppMessage},
    widgets::{
        popup::{Popup, popup_block, popup_list, popup_list_no_block},
        text_input::TextInput,
    },
};

use ratatui::{
    Frame,
    buffer::Buffer,
    crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind},
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::Line,
    widgets::{Clear, ListItem, ListState, StatefulWidget, Widget},
};

/// Different commands that can be issued from a branch popup.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ExportManagerMessage {
    /// Export `ChronoBind` backups from the currently selected branch.
    ExportCurrentBranchChronoBind,
    /// Export `ChronoBind` backups from all branches.
    ExportAllBranchesChronoBind,
    /// Export all from the currently selected branch (Addons, WTF, `ChronoBind`).
    ExportFullCurrentBranch,
    /// Export all from all branches (Addons, WTF, `ChronoBind`).
    ExportFullAllBranches,
    /// Open the import dialog.
    OpenImportDialog,
    /// Import a `ChronoBind` backup from the specified path with the given options.
    ImportChronoBindBackup(PathBuf, InstallBackupOptions),
}

/// Popup for managing import/export operations.
#[derive(Debug, Clone)]
pub struct ExportManagerPopup {
    /// The currently selected branch in `ChronoBind`.
    pub selected_branch: Option<String>,

    /// Whether the popup should close.
    pub close: bool,
    /// The state of the list within the popup.
    pub state: ListState,

    /// Commands issued by the popup.
    pub commands: Vec<AppMessage>,
}

impl ExportManagerPopup {
    #[must_use]
    pub fn new(selected_branch: Option<String>) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self {
            selected_branch,

            close: false,
            state: list_state,

            commands: vec![],
        }
    }

    /// Push a command to the popup's command list.
    #[inline]
    pub fn push_command(&mut self, command: ExportManagerMessage) {
        self.commands.push(AppMessage::ExportManager(command));
    }

    /// Push a command to the popup's command list and close the popup.
    #[inline]
    pub fn push_command_close(&mut self, command: ExportManagerMessage) {
        self.push_command(command);
        self.close = true;
    }
}

impl ExportManagerPopup {
    /// Index of Export `ChronoBind` from current branch option.
    pub const EXPORT_CURRENT_BRANCH_CHRONOBIND_IDX: usize = 0;
    /// Index of Export `ChronoBind` from all branches option.
    pub const EXPORT_ALL_BRANCHES_CHRONOBIND_IDX: usize = 1;
    /// Index of Export all from current branch option.
    pub const EXPORT_CURRENT_BRANCH_ALL_IDX: usize = 2;
    /// Index of Export all from all branches option.
    pub const EXPORT_ALL_BRANCHES_ALL_IDX: usize = 3;
    /// Index of Import backup option.
    pub const IMPORT_BACKUP_IDX: usize = 4;
}

impl Popup for ExportManagerPopup {
    fn on_key_down(&mut self, key: &KeyEvent) {
        match key.keycode_lower() {
            KeyCode::Up | KeyCode::Char('w') => {
                self.state.select_previous();
            }
            KeyCode::Down | KeyCode::Char('s') => {
                self.state.select_next();
            }
            KeyCode::Enter | KeyCode::Char(' ' | 'd') => {
                if let Some(selected) = self.state.selected() {
                    match selected {
                        Self::EXPORT_CURRENT_BRANCH_CHRONOBIND_IDX => {
                            self.push_command(ExportManagerMessage::ExportCurrentBranchChronoBind);
                        }
                        Self::EXPORT_ALL_BRANCHES_CHRONOBIND_IDX => {
                            self.push_command(ExportManagerMessage::ExportAllBranchesChronoBind);
                        }
                        Self::EXPORT_CURRENT_BRANCH_ALL_IDX => {
                            self.push_command(ExportManagerMessage::ExportFullCurrentBranch);
                        }
                        Self::EXPORT_ALL_BRANCHES_ALL_IDX => {
                            self.push_command(ExportManagerMessage::ExportFullAllBranches);
                        }
                        Self::IMPORT_BACKUP_IDX => {
                            self.push_command(ExportManagerMessage::OpenImportDialog);
                        }
                        _ => {}
                    }
                }
            }
            KeyCode::Esc | KeyCode::Char('q') => {
                self.close = true;
            }
            _ => {}
        }
    }

    fn draw(&mut self, area: Rect, frame: &mut Frame<'_>) {
        let block = popup_block(" Import/Export manager ");

        let selected_idx = self.state.selected().unwrap_or(0);
        let items = [
            ListItem::new(highlight_str(
                format!(
                    "Export ChronoBind backups from currently selected branch: {}",
                    format_option(self.selected_branch.as_ref())
                ),
                selected_idx == Self::EXPORT_CURRENT_BRANCH_CHRONOBIND_IDX,
            )),
            ListItem::new(highlight_str(
                "Export ChronoBind backups from all branches",
                selected_idx == Self::EXPORT_ALL_BRANCHES_CHRONOBIND_IDX,
            )),
            ListItem::new(highlight_str(
                format!(
                    "Export all from currently selected branch (Addons, WTF, ChronoBind): {}",
                    format_option(self.selected_branch.as_ref())
                ),
                selected_idx == Self::EXPORT_CURRENT_BRANCH_ALL_IDX,
            )),
            ListItem::new(highlight_str(
                "Export all from all branches (Addons, WTF, ChronoBind)",
                selected_idx == Self::EXPORT_ALL_BRANCHES_ALL_IDX,
            )),
            ListItem::new(highlight_str(
                "Import ChronoBind backup",
                selected_idx == Self::IMPORT_BACKUP_IDX,
            )),
        ];

        let list_view = popup_list(block, items);

        Widget::render(Clear, area, frame.buffer_mut());
        StatefulWidget::render(list_view, area, frame.buffer_mut(), &mut self.state);
    }

    fn should_close(&self) -> bool {
        self.close
    }
    fn close(&mut self) {
        self.close = true;
    }
    fn popup_identifier(&self) -> &'static str {
        "export_manager_popup"
    }
    fn bottom_bar_options(&self) -> Option<Vec<String>> {
        Some(vec![
            "↑/↓".to_string(),
            format!("{}/Space: Select", ENTER_SYMBOL),
            "Esc: Close".to_string(),
        ])
    }
    fn internal_commands_mut(&mut self) -> Option<&mut Vec<AppMessage>> {
        Some(&mut self.commands)
    }

    fn popup_width_percent(&self) -> u16 {
        80
    }
    fn popup_height_percent(&self) -> u16 {
        80
    }
}

/// Popup for managing import/export operations.
#[derive(Debug, Clone)]
pub struct ImportDialog {
    /// Options for importing backups.
    pub import_options: InstallBackupOptions,
    /// Text import state.
    pub path_input: TextInput,

    /// Whether the popup should close.
    pub close: bool,
    /// The state of the list within the popup.
    pub state: ListState,

    /// Commands issued by the popup.
    pub commands: Vec<AppMessage>,
}

impl Default for ImportDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl ImportDialog {
    /// Create a new `ImportDialog`.
    #[must_use]
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        let mut text_input = TextInput::new_with_placeholder("Enter import path here...");
        text_input.mode = crate::widgets::text_input::InputMode::Editing;

        Self {
            import_options: InstallBackupOptions::all(),
            path_input: text_input,

            close: false,
            state: list_state,

            commands: vec![],
        }
    }

    /// Push a command to the popup's command list.
    #[inline]
    pub fn push_command(&mut self, command: ExportManagerMessage) {
        self.commands.push(AppMessage::ExportManager(command));
    }

    /// Push a command to the popup's command list and close the popup.
    #[inline]
    pub fn push_command_close(&mut self, command: ExportManagerMessage) {
        self.push_command(command);
        self.close = true;
    }
}

impl ImportDialog {
    /// Index of the include WTF (User config folder) option.
    pub const INCLUDE_WTF_IDX: usize = 0;
    /// Index of the include interface (Addons folder) option.
    pub const INCLUDE_INTERFACE_IDX: usize = 1;
    /// Index of the include characters (Character backups) option.
    pub const INCLUDE_CHARACTERS_IDX: usize = 2;
    /// Index of the import button.
    pub const IMPORT_IDX: usize = 3;

    /// Check if the given index is currently hovered
    #[inline]
    #[must_use]
    pub fn is_hovered(&self, index: usize) -> bool {
        if self.path_input.mode == crate::widgets::text_input::InputMode::Editing {
            return false;
        }
        let selected_index = self.state.selected().unwrap_or(0);
        selected_index == index
    }
}

impl Popup for ImportDialog {
    fn on_key_down(&mut self, key: &KeyEvent) {
        match key.keycode_lower() {
            KeyCode::Up | KeyCode::Char('w') => {
                self.state.select_previous();
            }
            KeyCode::Down | KeyCode::Char('s') => {
                self.state.select_next();
            }
            KeyCode::Char('t') => {
                self.path_input.mode = crate::widgets::text_input::InputMode::Editing;
            }
            KeyCode::Enter | KeyCode::Char(' ' | 'd' | 'e') => {
                match self.state.selected().unwrap_or_default() {
                    Self::INCLUDE_WTF_IDX => {
                        self.import_options.include_wtf = !self.import_options.include_wtf;
                    }
                    Self::INCLUDE_INTERFACE_IDX => {
                        self.import_options.include_interface =
                            !self.import_options.include_interface;
                    }
                    Self::INCLUDE_CHARACTERS_IDX => {
                        self.import_options.include_character_backups =
                            !self.import_options.include_character_backups;
                    }
                    Self::IMPORT_IDX => {
                        let import_path = parse_path(&self.path_input.input);
                        self.push_command_close(ExportManagerMessage::ImportChronoBindBackup(
                            import_path,
                            self.import_options,
                        ));
                    }
                    _ => {}
                }
            }
            KeyCode::Esc | KeyCode::Char('q') => {
                self.close = true;
            }
            _ => {}
        }
    }

    fn handle_event(&mut self, event: &Event) -> bool {
        if self.path_input.mode == crate::widgets::text_input::InputMode::Editing {
            self.path_input.handle_event(event);
            return true;
        }
        if let Event::Key(key_event) = event
            && key_event.kind == KeyEventKind::Press
        {
            self.on_key_down(key_event);
        }
        true
    }

    fn draw(&mut self, area: Rect, frame: &mut Frame<'_>) {
        let block = popup_block(" Import ChronoBind backup ")
            .border_style(Style::default().fg(PALETTE.log_info_fg));
        let inner_area = block.inner(area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Path display
                Constraint::Length(1), // Separator
                Constraint::Fill(1),   // Remaining
            ])
            .split(inner_area);

        let items = [
            toggle_option(
                "Include 'WTF' folder",
                self.import_options.include_wtf,
                self.is_hovered(Self::INCLUDE_WTF_IDX),
            ),
            toggle_option(
                "Include 'Interface' folder",
                self.import_options.include_interface,
                self.is_hovered(Self::INCLUDE_INTERFACE_IDX),
            ),
            toggle_option(
                "Include ChronoBind character backups",
                self.import_options.include_character_backups,
                self.is_hovered(Self::INCLUDE_CHARACTERS_IDX),
            ),
            Line::from(dual_highlight_str(
                "Import backup",
                self.is_hovered(Self::IMPORT_IDX),
            ))
            .centered(),
        ];

        let mut list_view = popup_list_no_block(items);
        if self.path_input.mode == crate::widgets::text_input::InputMode::Editing {
            list_view = list_view.highlight_style(Style::new());
        }

        Widget::render(block, area, frame.buffer_mut());
        self.path_input.render(chunks[0], frame);
        draw_horizontal_separator(
            chunks[1],
            frame.buffer_mut(),
            1,
            Style::new().fg(PALETTE.log_info_fg).dim(),
        );
        StatefulWidget::render(list_view, chunks[2], frame.buffer_mut(), &mut self.state);
    }

    fn should_close(&self) -> bool {
        self.close
    }
    fn close(&mut self) {
        self.close = true;
    }
    fn popup_identifier(&self) -> &'static str {
        "import_dialog"
    }
    fn bottom_bar_options(&self) -> Option<Vec<String>> {
        if self.path_input.mode == crate::widgets::text_input::InputMode::Editing {
            Some(vec![format!("{}/Esc: Finish editing", ENTER_SYMBOL)])
        } else {
            Some(vec![
                "↑/↓".to_string(),
                format!("{}/Space: Select", ENTER_SYMBOL),
                "T: Edit path".to_string(),
                "Esc: Close".to_string(),
            ])
        }
    }
    fn internal_commands_mut(&mut self) -> Option<&mut Vec<AppMessage>> {
        Some(&mut self.commands)
    }

    fn popup_width_percent(&self) -> u16 {
        90
    }
    fn popup_height_percent(&self) -> u16 {
        0
    }
    fn popup_min_height(&self) -> u16 {
        8
    }
}

/// Draw a horizontal separator line in the given area of the buffer.
pub fn draw_horizontal_separator(area: Rect, buf: &mut Buffer, inset: u16, style: Style) {
    if area.height < 1 || (area.width - (inset * 2)) < 1 {
        return;
    }

    for x in (area.left() + inset)..(area.right() - inset) {
        if let Some(cell) = buf.cell_mut((x, area.top())) {
            cell.set_style(style);
            cell.set_symbol(ratatui::symbols::line::HORIZONTAL);
        }
    }
}

/// Parse a path string, removing surrounding quotes if present.
#[inline]
#[must_use]
fn parse_path(input: &str) -> PathBuf {
    let s = input.trim();
    let unquoted = s
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .or_else(|| s.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')))
        .unwrap_or(s);

    PathBuf::from(unquoted)
}
