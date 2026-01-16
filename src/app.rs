use crate::error::Result;
use crate::git::commit::{CommitId, EditableField};
use crate::git::validation::{validate_date, validate_email};
use crate::git::{rewrite_history, Repository};
use crate::state::{AppMode, AppState, ConfirmAction, VisualType};
use crate::ui::layout::AppLayout;
use crate::ui::theme::Theme;
use crate::ui::widgets::{
    get_column_value, help_max_scroll, render_commit_table, render_confirmation_dialog,
    render_detail_pane, render_edit_popup, render_help_screen, render_search_bar,
    render_status_bar, render_title_bar, Column, ConfirmDialogState, SearchState,
};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::Stdout;
use std::time::Duration;

/// Main application struct
pub struct App {
    /// Application state
    pub state: AppState,
    /// Repository handle
    repo: Repository,
    /// Color theme
    theme: Theme,
    /// Should the app quit?
    should_quit: bool,
    /// Search state (when searching)
    search: SearchState,
    /// Confirmation dialog state
    confirm_dialog: ConfirmDialogState,
    /// Last known terminal area (for scroll calculations)
    last_area: ratatui::layout::Rect,
}

impl App {
    /// Create a new app with the given repository
    ///
    /// # Arguments
    /// * `repo` - The git repository to operate on
    /// * `commit_limit` - Maximum number of commits to load
    /// * `sync_author_to_committer` - Whether editing author fields should also update committer fields
    pub fn new(
        repo: Repository,
        commit_limit: usize,
        sync_author_to_committer: bool,
    ) -> Result<Self> {
        let branch_name = repo.current_branch_name()?;
        let has_upstream = repo.has_upstream().unwrap_or(false);
        let commits = repo.load_commits(commit_limit)?;

        let mut state = AppState::new(commits, branch_name, has_upstream);
        // Start at first editable column (Name)
        state.column_index = Column::Name as usize;
        // Configure author-to-committer sync behavior
        state.set_sync_author_to_committer(sync_author_to_committer);

        Ok(Self {
            state,
            repo,
            theme: Theme::default(),
            should_quit: false,
            search: SearchState::new(),
            confirm_dialog: ConfirmDialogState::default(),
            last_area: ratatui::layout::Rect::default(),
        })
    }

    /// Run the main event loop
    pub fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
        loop {
            // Draw UI
            terminal.draw(|frame| self.draw(frame))?;

            // Handle events with a small timeout for responsiveness
            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    self.handle_key(key)?;
                }
            }

            if self.should_quit {
                break;
            }
        }

        Ok(())
    }

    /// Draw the entire UI
    fn draw(&mut self, frame: &mut ratatui::Frame<'_>) {
        use ratatui::layout::Alignment;
        use ratatui::widgets::Paragraph;

        let area = frame.area();
        self.last_area = area;

        // Check if terminal is too small
        if AppLayout::is_too_small(area) {
            let msg = format!(
                "Terminal too small\n\nMinimum size: {}x{}\nCurrent size: {}x{}\n\nPlease resize your terminal",
                crate::ui::layout::MIN_WIDTH,
                crate::ui::layout::MIN_HEIGHT,
                area.width,
                area.height
            );
            let para = Paragraph::new(msg)
                .alignment(Alignment::Center)
                .style(self.theme.warning);
            frame.render_widget(para, area);
            return;
        }

        let search_active = matches!(self.state.mode, AppMode::Search);
        let layout = AppLayout::new(area, search_active);

        // Update scroll for actual table height
        self.state.update_scroll_for_height(layout.table_height());

        // Render base UI
        render_title_bar(frame, layout.title, &self.state, &self.theme);

        if let Some(search_area) = layout.search {
            let result_count = self.state.filtered_indices.as_ref().map(|i| i.len());
            render_search_bar(
                frame,
                search_area,
                &self.search.query,
                self.search.cursor,
                result_count,
                &self.theme,
            );
        }

        render_commit_table(frame, layout.table, &self.state, &self.theme);
        render_detail_pane(frame, layout.detail, &self.state, &self.theme);
        render_status_bar(frame, layout.status, &self.state, &self.theme);

        // Render overlays based on mode
        match &self.state.mode {
            AppMode::Editing { field, .. } => {
                render_edit_popup(frame, area, &self.state, field, &self.theme);
            }
            AppMode::Confirming(action) => {
                render_confirmation_dialog(
                    frame,
                    area,
                    action,
                    &self.state,
                    &self.confirm_dialog,
                    &self.theme,
                );
            }
            AppMode::Help => {
                render_help_screen(frame, area, self.state.help_scroll, &self.theme);
            }
            _ => {}
        }
    }

    /// Handle a key press
    fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        // Clear messages on any key press
        self.state.clear_messages();

        match &self.state.mode {
            AppMode::Normal => self.handle_normal_key(key),
            AppMode::Visual { .. } => self.handle_visual_key(key),
            AppMode::Editing { .. } => self.handle_inline_editing_key(key),
            AppMode::Search => self.handle_search_key(key),
            AppMode::Confirming(action) => {
                let action = action.clone();
                self.handle_confirm_key(key, &action)
            }
            AppMode::Help => self.handle_help_key(key),
            AppMode::Quitting => self.handle_quit_confirm_key(key),
            AppMode::Reorder => self.handle_normal_key(key),
        }
    }

    /// Handle key in normal mode
    fn handle_normal_key(&mut self, key: KeyEvent) -> Result<()> {
        match (key.code, key.modifiers) {
            // Quit
            (KeyCode::Char('q'), KeyModifiers::NONE) => {
                if self.state.is_dirty() {
                    self.state.mode = AppMode::Quitting;
                } else {
                    self.should_quit = true;
                }
            }

            // Vertical navigation
            (KeyCode::Char('j') | KeyCode::Down, KeyModifiers::NONE) => {
                self.state.cursor_down();
            }
            (KeyCode::Char('k') | KeyCode::Up, KeyModifiers::NONE) => {
                self.state.cursor_up();
            }
            (KeyCode::Char('g') | KeyCode::Home, KeyModifiers::NONE) => {
                self.state.cursor_top();
            }
            (KeyCode::Char('G') | KeyCode::End, KeyModifiers::NONE) => {
                self.state.cursor_bottom();
            }
            (KeyCode::Char('d'), KeyModifiers::CONTROL) => {
                self.state.page_down(10);
            }
            (KeyCode::Char('u'), KeyModifiers::CONTROL) => {
                self.state.page_up(10);
            }
            (KeyCode::PageDown, _) => {
                self.state.page_down(10);
            }
            (KeyCode::PageUp, _) => {
                self.state.page_up(10);
            }

            // Horizontal navigation (column selection)
            (KeyCode::Char('h') | KeyCode::Left, KeyModifiers::NONE) => {
                self.move_to_prev_editable_column();
            }
            (KeyCode::Char('l') | KeyCode::Right, KeyModifiers::NONE) => {
                self.move_to_next_editable_column();
            }
            (KeyCode::Tab, KeyModifiers::NONE) => {
                self.move_to_next_editable_column();
            }
            (KeyCode::BackTab, _) => {
                self.move_to_prev_editable_column();
            }

            // Selection
            (KeyCode::Char(' '), KeyModifiers::NONE) => {
                self.state.toggle_selection();
            }
            (KeyCode::Char('a'), KeyModifiers::CONTROL) => {
                self.state.select_all();
            }
            (KeyCode::Char('n'), KeyModifiers::CONTROL) => {
                self.state.deselect_all();
            }

            // Delete commit
            (KeyCode::Char('d'), KeyModifiers::NONE) => {
                self.toggle_deletion()?;
            }
            (KeyCode::Char('x'), KeyModifiers::NONE) => {
                self.toggle_deletion()?;
            }

            // Move commit up/down (reorder)
            (KeyCode::Char('K'), KeyModifiers::SHIFT) => {
                self.move_commit_up()?;
            }
            (KeyCode::Char('J'), KeyModifiers::SHIFT) => {
                self.move_commit_down()?;
            }
            (KeyCode::Char('k'), KeyModifiers::CONTROL) => {
                self.move_commit_up()?;
            }
            (KeyCode::Char('j'), KeyModifiers::CONTROL) => {
                self.move_commit_down()?;
            }

            // Start inline editing with Enter or 'e'
            (KeyCode::Enter | KeyCode::Char('e'), KeyModifiers::NONE) => {
                self.start_inline_editing()?;
            }

            // Search
            (KeyCode::Char('/'), KeyModifiers::NONE) => {
                self.search = SearchState::from_query(&self.state.search_query);
                self.state.mode = AppMode::Search;
            }

            // Undo/Redo
            (KeyCode::Char('u'), KeyModifiers::NONE) => {
                if self.state.undo() {
                    self.state.set_success("Undone");
                } else {
                    self.state.set_error("Nothing to undo");
                }
            }
            (KeyCode::Char('r'), KeyModifiers::CONTROL) => {
                if self.state.redo() {
                    self.state.set_success("Redone");
                } else {
                    self.state.set_error("Nothing to redo");
                }
            }

            // Reset
            (KeyCode::Char('r'), KeyModifiers::NONE) => {
                if self.state.is_dirty() {
                    self.confirm_dialog = ConfirmDialogState::default();
                    self.state.mode = AppMode::Confirming(ConfirmAction::DiscardChanges);
                }
            }

            // Apply changes
            (KeyCode::Char('w'), KeyModifiers::NONE) => {
                if self.state.is_dirty() {
                    self.confirm_dialog = ConfirmDialogState::default();
                    self.state.mode = AppMode::Confirming(ConfirmAction::ApplyChanges);
                } else {
                    self.state.set_error("No changes to apply");
                }
            }

            // Help
            (KeyCode::Char('?'), KeyModifiers::NONE) => {
                self.state.reset_help_scroll();
                self.state.mode = AppMode::Help;
            }

            // Visual mode - character/line-wise (v) - in table context, this is line-wise
            (KeyCode::Char('v'), KeyModifiers::NONE) => {
                self.state.enter_visual_mode(VisualType::Line);
            }

            // Visual mode - line-wise (V) - same as v for tables
            (KeyCode::Char('V'), KeyModifiers::SHIFT) => {
                self.state.enter_visual_mode(VisualType::Line);
            }

            // Visual mode - block-wise (Ctrl+V)
            (KeyCode::Char('v'), KeyModifiers::CONTROL) => {
                self.state.enter_visual_mode(VisualType::Block);
            }

            _ => {}
        }

        Ok(())
    }

    /// Move to next editable column
    fn move_to_next_editable_column(&mut self) {
        let editable_columns = [
            Column::Name as usize,
            Column::Email as usize,
            Column::Date as usize,
            Column::Message as usize,
        ];

        if let Some(pos) = editable_columns
            .iter()
            .position(|&c| c == self.state.column_index)
        {
            let next_pos = (pos + 1) % editable_columns.len();
            self.state.column_index = editable_columns[next_pos];
        } else {
            self.state.column_index = editable_columns[0];
        }
    }

    /// Move to previous editable column
    fn move_to_prev_editable_column(&mut self) {
        let editable_columns = [
            Column::Name as usize,
            Column::Email as usize,
            Column::Date as usize,
            Column::Message as usize,
        ];

        if let Some(pos) = editable_columns
            .iter()
            .position(|&c| c == self.state.column_index)
        {
            let prev_pos = if pos == 0 {
                editable_columns.len() - 1
            } else {
                pos - 1
            };
            self.state.column_index = editable_columns[prev_pos];
        } else {
            self.state.column_index = editable_columns[editable_columns.len() - 1];
        }
    }

    /// Handle key in visual selection mode
    fn handle_visual_key(&mut self, key: KeyEvent) -> Result<()> {
        match (key.code, key.modifiers) {
            // Exit visual mode
            (KeyCode::Esc, _) => {
                self.state.exit_visual_mode();
            }

            // Toggle selection type (v/V toggles to Line, Ctrl+V toggles to Block)
            (KeyCode::Char('v'), KeyModifiers::NONE)
            | (KeyCode::Char('V'), KeyModifiers::SHIFT) => {
                if let AppMode::Visual {
                    anchor,
                    visual_type,
                } = self.state.mode.clone()
                {
                    if visual_type == VisualType::Line {
                        // Already in line mode, exit
                        self.state.exit_visual_mode();
                    } else {
                        // Switch to line mode
                        self.state.mode = AppMode::Visual {
                            anchor,
                            visual_type: VisualType::Line,
                        };
                    }
                }
            }
            (KeyCode::Char('v'), KeyModifiers::CONTROL) => {
                if let AppMode::Visual {
                    anchor,
                    visual_type,
                } = self.state.mode.clone()
                {
                    if visual_type == VisualType::Block {
                        // Already in block mode, exit
                        self.state.exit_visual_mode();
                    } else {
                        // Switch to block mode
                        self.state.mode = AppMode::Visual {
                            anchor,
                            visual_type: VisualType::Block,
                        };
                    }
                }
            }

            // Vertical navigation (extends selection)
            (KeyCode::Char('j') | KeyCode::Down, KeyModifiers::NONE) => {
                self.state.cursor_down();
            }
            (KeyCode::Char('k') | KeyCode::Up, KeyModifiers::NONE) => {
                self.state.cursor_up();
            }
            (KeyCode::Char('g') | KeyCode::Home, KeyModifiers::NONE) => {
                self.state.cursor_top();
            }
            (KeyCode::Char('G') | KeyCode::End, KeyModifiers::NONE) => {
                self.state.cursor_bottom();
            }
            (KeyCode::Char('d'), KeyModifiers::CONTROL) => {
                self.state.page_down(10);
            }
            (KeyCode::Char('u'), KeyModifiers::CONTROL) => {
                self.state.page_up(10);
            }
            (KeyCode::PageDown, _) => {
                self.state.page_down(10);
            }
            (KeyCode::PageUp, _) => {
                self.state.page_up(10);
            }

            // Horizontal navigation (for block mode - extends column selection)
            (KeyCode::Char('h') | KeyCode::Left, KeyModifiers::NONE) => {
                self.state.column_left();
            }
            (KeyCode::Char('l') | KeyCode::Right, KeyModifiers::NONE) => {
                self.state.column_right();
            }

            // Toggle checkbox selection for visual range
            (KeyCode::Char(' '), KeyModifiers::NONE) => {
                // Toggle checkbox selection without exiting visual mode
                if let Some(((start_row, _), (end_row, _))) = self.state.visual_range() {
                    let visible = self.state.visible_commits();
                    let ids: Vec<_> = (start_row..=end_row)
                        .filter_map(|row| visible.get(row).map(|c| c.id))
                        .collect();
                    for id in ids {
                        if self.state.selected.contains(&id) {
                            self.state.selected.remove(&id);
                        } else {
                            self.state.selected.insert(id);
                        }
                    }
                }
            }

            // Edit visual selection (capture targets and start editing)
            (KeyCode::Char('e') | KeyCode::Enter, KeyModifiers::NONE) => {
                let count = self.state.capture_visual_edit_targets();
                if count > 0 {
                    self.start_inline_editing()?;
                }
            }

            _ => {}
        }

        Ok(())
    }

    /// Move commit at cursor up (swap with previous)
    fn move_commit_up(&mut self) -> Result<()> {
        if self.state.filtered_indices.is_some() {
            self.state.set_error("Cannot reorder while filtering");
            return Ok(());
        }

        if self.state.cursor == 0 {
            self.state.set_error("Already at top");
            return Ok(());
        }

        // Check for merge commits - can't reorder them
        if let Some(commit) = self.state.cursor_commit() {
            if commit.is_merge {
                self.state.set_error("Cannot reorder merge commits");
                return Ok(());
            }
        }

        // AppState.move_commit_up() handles save_undo internally
        self.state.move_commit_up();
        self.state.set_success("Commit moved up");
        Ok(())
    }

    /// Move commit at cursor down (swap with next)
    fn move_commit_down(&mut self) -> Result<()> {
        if self.state.filtered_indices.is_some() {
            self.state.set_error("Cannot reorder while filtering");
            return Ok(());
        }

        if self.state.cursor >= self.state.commits.len().saturating_sub(1) {
            self.state.set_error("Already at bottom");
            return Ok(());
        }

        // Check for merge commits - can't reorder them
        if let Some(commit) = self.state.cursor_commit() {
            if commit.is_merge {
                self.state.set_error("Cannot reorder merge commits");
                return Ok(());
            }
        }

        // AppState.move_commit_down() handles save_undo internally
        self.state.move_commit_down();
        self.state.set_success("Commit moved down");
        Ok(())
    }

    /// Toggle deletion on the current commit or selected commits
    fn toggle_deletion(&mut self) -> Result<()> {
        // Get commits to potentially delete: selected > cursor
        let commit_ids: Vec<CommitId> = if !self.state.selected.is_empty() {
            self.state.selected.iter().copied().collect()
        } else if let Some(id) = self.state.cursor_commit_id() {
            vec![id]
        } else {
            return Ok(());
        };

        // Check if we're toggling on or off (based on first commit)
        let will_delete = !self.state.is_deleted(commit_ids[0]);
        let count = commit_ids.len();

        // Don't allow deleting all commits
        let remaining_after = self.state.commits.len() - self.state.deleted.len();
        if will_delete && count >= remaining_after {
            self.state.set_error("Cannot delete all commits");
            return Ok(());
        }

        // Save undo state
        let description = if will_delete {
            format!("Delete {} commit(s)", count)
        } else {
            format!("Restore {} commit(s)", count)
        };
        self.state.save_undo(&description);

        // Toggle deletion for all target commits
        for id in commit_ids {
            if will_delete {
                self.state.mark_deleted(id);
            } else {
                self.state.unmark_deleted(id);
            }
        }

        // Show feedback
        if will_delete {
            if count > 1 {
                self.state
                    .set_success(format!("{} commits marked for deletion", count));
            } else {
                self.state.set_success("Commit marked for deletion");
            }
        } else if count > 1 {
            self.state
                .set_success(format!("{} commits restored", count));
        } else {
            self.state.set_success("Commit restored");
        }

        Ok(())
    }

    /// Start inline editing at current column
    fn start_inline_editing(&mut self) -> Result<()> {
        let commit = match self.state.cursor_commit() {
            Some(c) => c,
            None => return Ok(()),
        };

        // Don't allow editing merge commits
        if commit.is_merge {
            self.state.set_error("Cannot edit merge commits");
            return Ok(());
        }

        let column = match Column::from_index(self.state.column_index) {
            Some(c) => c,
            None => return Ok(()),
        };

        if !column.is_editable() {
            self.state.set_error("This column is not editable");
            return Ok(());
        }

        let field = match column.to_editable_field() {
            Some(f) => f,
            None => return Ok(()),
        };

        // Get current value for the cell
        let mods = self.state.modifications.get(&commit.id);
        let current_value = get_column_value(commit, mods, column);

        // For commit messages (multiline), open external editor
        if field == EditableField::Message {
            return self.open_external_editor(field, &current_value);
        }

        // Store in edit buffer with cursor at end
        self.state.edit_buffer = current_value.clone();
        self.state.edit_original = current_value;
        self.state.edit_cursor = self.state.edit_buffer.len();

        self.state.mode = AppMode::Editing {
            commit_idx: self.state.cursor,
            field,
        };

        Ok(())
    }

    /// Open external editor for multiline/long content
    fn open_external_editor(&mut self, field: EditableField, current_value: &str) -> Result<()> {
        use std::io::Write;
        use std::process::Command;

        // Get editor from environment
        let editor = std::env::var("EDITOR")
            .or_else(|_| std::env::var("VISUAL"))
            .unwrap_or_else(|_| "vim".to_string());

        // Create temp file with current content
        let mut temp_file = tempfile::NamedTempFile::new()?;
        temp_file.write_all(current_value.as_bytes())?;
        temp_file.flush()?;

        let temp_path = temp_file.path().to_path_buf();

        // We need to temporarily exit the TUI to run the editor
        // This is handled by dropping the terminal restore, running editor, then re-entering

        // Disable raw mode temporarily
        crossterm::terminal::disable_raw_mode()?;
        crossterm::execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen)?;

        // Run editor
        let status = Command::new(&editor).arg(&temp_path).status();

        // Re-enable TUI
        crossterm::terminal::enable_raw_mode()?;
        crossterm::execute!(std::io::stdout(), crossterm::terminal::EnterAlternateScreen)?;

        match status {
            Ok(exit_status) if exit_status.success() => {
                // Read edited content
                let new_value = std::fs::read_to_string(&temp_path)?;
                let new_value = new_value.trim_end().to_string();

                if new_value != current_value {
                    // Get commits to edit: visual targets > checkbox selected > cursor
                    let commit_ids = self.state.commits_to_edit();
                    if commit_ids.is_empty() {
                        self.state.clear_visual_edit_targets();
                        return Ok(());
                    }

                    let count = commit_ids.len();
                    self.state.save_undo(&format!(
                        "Edit {} on {} commit(s)",
                        field.display_name(),
                        count
                    ));

                    for cid in commit_ids {
                        self.apply_field_edit(cid, &field, &new_value, current_value);
                    }

                    self.state.clear_visual_edit_targets();

                    if count > 1 {
                        self.state.set_success(format!("Updated {} commits", count));
                    } else {
                        self.state.set_success("Message updated");
                    }
                }
            }
            Ok(_) => {
                self.state.set_error("Editor exited with error");
            }
            Err(e) => {
                self.state.set_error(format!("Failed to run editor: {}", e));
            }
        }

        Ok(())
    }

    /// Handle key in inline editing mode
    fn handle_inline_editing_key(&mut self, key: KeyEvent) -> Result<()> {
        let (commit_idx, field) = match &self.state.mode {
            AppMode::Editing { commit_idx, field } => (*commit_idx, *field),
            _ => return Ok(()),
        };

        match (key.code, key.modifiers) {
            // Cancel editing
            (KeyCode::Esc, _) => {
                self.state.edit_buffer.clear();
                self.state.edit_original.clear();
                self.state.clear_visual_edit_targets();
                self.state.mode = AppMode::Normal;
            }

            // Confirm edit
            (KeyCode::Enter, KeyModifiers::NONE) => {
                self.confirm_inline_edit(commit_idx, field)?;
            }

            // Tab to next field (confirm current and move)
            (KeyCode::Tab, KeyModifiers::NONE) => {
                self.confirm_inline_edit(commit_idx, field)?;
                if matches!(self.state.mode, AppMode::Normal) {
                    self.move_to_next_editable_column();
                    self.start_inline_editing()?;
                }
            }

            // Shift+Tab to previous field
            (KeyCode::BackTab, _) => {
                self.confirm_inline_edit(commit_idx, field)?;
                if matches!(self.state.mode, AppMode::Normal) {
                    self.move_to_prev_editable_column();
                    self.start_inline_editing()?;
                }
            }

            // Text editing - insert at cursor position
            (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                let cursor = self.state.edit_cursor;
                self.state.edit_buffer.insert(cursor, c);
                self.state.edit_cursor += 1;
            }

            // Delete character
            (KeyCode::Backspace, KeyModifiers::NONE) => {
                if self.state.edit_cursor > 0 {
                    self.state.edit_cursor -= 1;
                    self.state.edit_buffer.remove(self.state.edit_cursor);
                }
            }
            (KeyCode::Delete, KeyModifiers::NONE) => {
                if self.state.edit_cursor < self.state.edit_buffer.len() {
                    self.state.edit_buffer.remove(self.state.edit_cursor);
                }
            }

            // Delete word backward (Alt+Backspace, Ctrl+W, Ctrl+Backspace)
            (KeyCode::Backspace, KeyModifiers::ALT)
            | (KeyCode::Char('w'), KeyModifiers::CONTROL)
            | (KeyCode::Backspace, KeyModifiers::CONTROL) => {
                self.edit_delete_word_backward();
            }

            // Delete to start of line (Ctrl+U)
            (KeyCode::Char('u'), KeyModifiers::CONTROL) => {
                if self.state.edit_cursor > 0 {
                    self.state.edit_buffer.drain(0..self.state.edit_cursor);
                    self.state.edit_cursor = 0;
                }
            }

            // Delete to end of line (Ctrl+K)
            (KeyCode::Char('k'), KeyModifiers::CONTROL) => {
                self.state.edit_buffer.truncate(self.state.edit_cursor);
            }

            // Move by character
            (KeyCode::Left, KeyModifiers::NONE) => {
                if self.state.edit_cursor > 0 {
                    self.state.edit_cursor -= 1;
                }
            }
            (KeyCode::Right, KeyModifiers::NONE) => {
                if self.state.edit_cursor < self.state.edit_buffer.len() {
                    self.state.edit_cursor += 1;
                }
            }

            // Move by word (Alt+Arrow, Ctrl+Arrow)
            (KeyCode::Left, KeyModifiers::ALT) | (KeyCode::Left, KeyModifiers::CONTROL) => {
                self.edit_move_word_left();
            }
            (KeyCode::Right, KeyModifiers::ALT) | (KeyCode::Right, KeyModifiers::CONTROL) => {
                self.edit_move_word_right();
            }

            // Move to start/end
            (KeyCode::Home, _) => {
                self.state.edit_cursor = 0;
            }
            (KeyCode::End, _) => {
                self.state.edit_cursor = self.state.edit_buffer.len();
            }

            // Emacs-style start/end (Ctrl+A/E)
            (KeyCode::Char('a'), KeyModifiers::CONTROL) => {
                self.state.edit_cursor = 0;
            }
            (KeyCode::Char('e'), KeyModifiers::CONTROL) => {
                self.state.edit_cursor = self.state.edit_buffer.len();
            }

            _ => {}
        }

        Ok(())
    }

    /// Move edit cursor to previous word boundary
    fn edit_move_word_left(&mut self) {
        if self.state.edit_cursor == 0 {
            return;
        }
        let chars: Vec<char> = self.state.edit_buffer.chars().collect();
        let mut pos = self.state.edit_cursor;
        // Skip whitespace before cursor
        while pos > 0 && chars[pos - 1].is_whitespace() {
            pos -= 1;
        }
        // Skip word characters
        while pos > 0 && !chars[pos - 1].is_whitespace() {
            pos -= 1;
        }
        self.state.edit_cursor = pos;
    }

    /// Move edit cursor to next word boundary
    fn edit_move_word_right(&mut self) {
        let len = self.state.edit_buffer.len();
        if self.state.edit_cursor >= len {
            return;
        }
        let chars: Vec<char> = self.state.edit_buffer.chars().collect();
        let mut pos = self.state.edit_cursor;
        // Skip current word
        while pos < len && !chars[pos].is_whitespace() {
            pos += 1;
        }
        // Skip whitespace
        while pos < len && chars[pos].is_whitespace() {
            pos += 1;
        }
        self.state.edit_cursor = pos;
    }

    /// Delete word backward in edit buffer
    fn edit_delete_word_backward(&mut self) {
        if self.state.edit_cursor == 0 {
            return;
        }
        let start = self.state.edit_cursor;
        self.edit_move_word_left();
        self.state.edit_buffer.drain(self.state.edit_cursor..start);
    }

    /// Confirm inline edit and apply changes
    fn confirm_inline_edit(&mut self, _commit_idx: usize, field: EditableField) -> Result<()> {
        let new_value = self.state.edit_buffer.clone();
        let original_value = self.state.edit_original.clone();

        // Validate based on field type
        if field.is_email() {
            if let Err(e) = validate_email(&new_value) {
                self.state.set_error(e.to_string());
                return Ok(());
            }
        }

        if field.is_date() {
            if let Err(e) = validate_date(&new_value) {
                self.state.set_error(e.to_string());
                return Ok(());
            }
        }

        // Only save if value changed
        if new_value != original_value {
            // Get commits to edit: visual targets > checkbox selected > cursor
            let commit_ids = self.state.commits_to_edit();
            if commit_ids.is_empty() {
                self.state.mode = AppMode::Normal;
                self.state.clear_visual_edit_targets();
                return Ok(());
            }

            // Save undo state before modification
            let count = commit_ids.len();
            self.state.save_undo(&format!(
                "Edit {} on {} commit(s)",
                field.display_name(),
                count
            ));

            // Apply the modification to all target commits
            for cid in commit_ids {
                self.apply_field_edit(cid, &field, &new_value, &original_value);
            }

            if count > 1 {
                self.state.set_success(format!("Updated {} commits", count));
            }
        }

        // Clear edit state
        self.state.edit_buffer.clear();
        self.state.edit_original.clear();
        self.state.edit_cursor = 0;
        self.state.clear_visual_edit_targets();
        self.state.mode = AppMode::Normal;

        Ok(())
    }

    /// Apply a field edit to a single commit
    ///
    /// When `sync_author_to_committer` is enabled in the app state, editing
    /// author fields (name, email, date) will also update the corresponding
    /// committer fields. This is the default behavior since most workflows
    /// keep author and committer identical.
    fn apply_field_edit(
        &mut self,
        commit_id: CommitId,
        field: &EditableField,
        new_value: &str,
        original_value: &str,
    ) {
        let sync = self.state.sync_author_to_committer;
        let mods = self.state.get_or_create_modifications(commit_id);

        match field {
            EditableField::AuthorName => {
                mods.author_name = Some(new_value.to_string());
                // Sync to committer if enabled
                if sync {
                    mods.committer_name = Some(new_value.to_string());
                }
            }
            EditableField::AuthorEmail => {
                mods.author_email = Some(new_value.to_string());
                // Sync to committer if enabled
                if sync {
                    mods.committer_email = Some(new_value.to_string());
                }
            }
            EditableField::AuthorDate => {
                if new_value != original_value {
                    if let Ok(dt) = validate_date(new_value) {
                        mods.author_date = Some(dt);
                        // Sync to committer if enabled
                        if sync {
                            mods.committer_date = Some(dt);
                        }
                    }
                }
            }
            EditableField::CommitterName => {
                mods.committer_name = Some(new_value.to_string());
            }
            EditableField::CommitterEmail => {
                mods.committer_email = Some(new_value.to_string());
            }
            EditableField::CommitterDate => {
                if new_value != original_value {
                    if let Ok(dt) = validate_date(new_value) {
                        mods.committer_date = Some(dt);
                    }
                }
            }
            EditableField::Message => {
                mods.message = Some(new_value.to_string());
            }
        }
    }

    /// Handle key in search mode
    fn handle_search_key(&mut self, key: KeyEvent) -> Result<()> {
        match (key.code, key.modifiers) {
            (KeyCode::Esc, _) => {
                self.state.clear_filter();
                self.state.mode = AppMode::Normal;
            }
            (KeyCode::Enter, _) => {
                self.state.search_query = self.search.query.clone();
                self.state.apply_filter();
                self.state.mode = AppMode::Normal;
            }
            // Delete character
            (KeyCode::Backspace, KeyModifiers::NONE) => {
                self.search.backspace();
            }
            (KeyCode::Delete, KeyModifiers::NONE) => {
                self.search.delete();
            }
            // Delete word (Alt+Backspace on Mac, Ctrl+W or Ctrl+Backspace)
            (KeyCode::Backspace, KeyModifiers::ALT) => {
                self.search.delete_word_backward();
            }
            (KeyCode::Char('w'), KeyModifiers::CONTROL) => {
                self.search.delete_word_backward();
            }
            (KeyCode::Backspace, KeyModifiers::CONTROL) => {
                self.search.delete_word_backward();
            }
            // Delete to start of line (Ctrl+U)
            (KeyCode::Char('u'), KeyModifiers::CONTROL) => {
                self.search.delete_to_start();
            }
            // Delete to end of line (Ctrl+K)
            (KeyCode::Char('k'), KeyModifiers::CONTROL) => {
                self.search.delete_to_end();
            }
            // Move by character
            (KeyCode::Left, KeyModifiers::NONE) => {
                self.search.move_left();
            }
            (KeyCode::Right, KeyModifiers::NONE) => {
                self.search.move_right();
            }
            // Move by word (Alt+Arrow on Mac, Ctrl+Arrow)
            (KeyCode::Left, KeyModifiers::ALT) | (KeyCode::Left, KeyModifiers::CONTROL) => {
                self.search.move_word_left();
            }
            (KeyCode::Right, KeyModifiers::ALT) | (KeyCode::Right, KeyModifiers::CONTROL) => {
                self.search.move_word_right();
            }
            // Move to start/end
            (KeyCode::Home, _) => {
                self.search.move_start();
            }
            (KeyCode::End, _) => {
                self.search.move_end();
            }
            // Also support Ctrl+A/E (Emacs-style)
            (KeyCode::Char('a'), KeyModifiers::CONTROL) => {
                self.search.move_start();
            }
            (KeyCode::Char('e'), KeyModifiers::CONTROL) => {
                self.search.move_end();
            }
            (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                self.search.insert(c);
            }
            _ => {}
        }

        Ok(())
    }

    /// Handle key in confirmation dialog
    fn handle_confirm_key(&mut self, key: KeyEvent, action: &ConfirmAction) -> Result<()> {
        match (key.code, key.modifiers) {
            (KeyCode::Esc, _) | (KeyCode::Char('n'), KeyModifiers::NONE) => {
                self.state.mode = AppMode::Normal;
            }
            (KeyCode::Char('y'), KeyModifiers::NONE) | (KeyCode::Enter, _)
                if self.confirm_dialog.is_yes_selected() =>
            {
                self.execute_confirmed_action(action)?;
            }
            (KeyCode::Char('y'), KeyModifiers::NONE) => {
                self.execute_confirmed_action(action)?;
            }
            (KeyCode::Tab, _) | (KeyCode::Left, _) | (KeyCode::Right, _) => {
                self.confirm_dialog.toggle();
            }
            (KeyCode::Enter, _) => {
                if self.confirm_dialog.is_yes_selected() {
                    self.execute_confirmed_action(action)?;
                } else {
                    self.state.mode = AppMode::Normal;
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Execute a confirmed action
    fn execute_confirmed_action(&mut self, action: &ConfirmAction) -> Result<()> {
        match action {
            ConfirmAction::ApplyChanges => {
                self.apply_changes()?;
            }
            ConfirmAction::DiscardChanges => {
                self.state.clear_modifications();
                self.state.set_success("All changes discarded");
            }
            ConfirmAction::QuitWithChanges => {
                self.should_quit = true;
            }
        }

        self.state.mode = AppMode::Normal;
        Ok(())
    }

    /// Apply all pending changes to the git history
    fn apply_changes(&mut self) -> Result<()> {
        // Auto-stash any uncommitted changes before rewriting
        let stashed = self.repo.stash_changes()?;

        // Perform the rewrite (with auto-restore on failure)
        let result = self.apply_changes_inner();

        // Restore stashed changes if we stashed them
        if stashed {
            // Try to restore even if rewrite failed
            if let Err(e) = self.repo.unstash_changes() {
                // If unstash fails after successful rewrite, warn but don't fail
                if result.is_ok() {
                    self.state.set_error(&format!(
                        "Warning: Could not restore stashed changes: {}. Use 'git stash pop' manually.",
                        e
                    ));
                    return Ok(());
                }
                // If both failed, return the original error
            }
        }

        result
    }

    /// Inner implementation of apply_changes (separated for stash handling)
    fn apply_changes_inner(&mut self) -> Result<()> {
        // Create backup reference
        self.repo.create_backup_ref(&self.state.branch_name)?;

        // Perform the rewrite
        rewrite_history(
            self.repo.inner(),
            &self.state.commits,
            &self.state.modifications,
            &self.state.deleted,
            &self.state.current_order,
            &self.state.branch_name,
        )?;

        // Reload commits
        let commits = self.repo.load_commits(self.state.commits.len())?;
        let original_order: Vec<_> = commits.iter().map(|c| c.id).collect();

        self.state.commits = commits;
        self.state.original_order = original_order.clone();
        self.state.current_order = original_order;
        self.state.modifications.clear();
        self.state.undo_stack.clear();
        self.state.redo_stack.clear();

        self.state.set_success("History rewritten successfully!");

        Ok(())
    }

    /// Handle key in help screen
    fn handle_help_key(&mut self, key: KeyEvent) -> Result<()> {
        let max_scroll = help_max_scroll(self.last_area);

        match (key.code, key.modifiers) {
            // Close help
            (KeyCode::Esc, _) | (KeyCode::Char('q'), _) | (KeyCode::Char('?'), _) => {
                self.state.mode = AppMode::Normal;
            }

            // Scroll down
            (KeyCode::Char('j') | KeyCode::Down, KeyModifiers::NONE) => {
                self.state.help_scroll_down(1, max_scroll);
            }

            // Scroll up
            (KeyCode::Char('k') | KeyCode::Up, KeyModifiers::NONE) => {
                self.state.help_scroll_up(1);
            }

            // Page down
            (KeyCode::Char('d'), KeyModifiers::CONTROL)
            | (KeyCode::PageDown, _)
            | (KeyCode::Char(' '), KeyModifiers::NONE) => {
                self.state.help_scroll_down(10, max_scroll);
            }

            // Page up
            (KeyCode::Char('u'), KeyModifiers::CONTROL) | (KeyCode::PageUp, _) => {
                self.state.help_scroll_up(10);
            }

            // Go to top
            (KeyCode::Char('g'), KeyModifiers::NONE) | (KeyCode::Home, _) => {
                self.state.help_scroll = 0;
            }

            // Go to bottom
            (KeyCode::Char('G'), KeyModifiers::NONE) | (KeyCode::End, _) => {
                self.state.help_scroll = max_scroll;
            }

            _ => {}
        }

        Ok(())
    }

    /// Handle quit confirmation
    fn handle_quit_confirm_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                self.should_quit = true;
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.state.mode = AppMode::Normal;
            }
            _ => {}
        }

        Ok(())
    }
}
