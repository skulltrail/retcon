use crate::git::commit::{CommitData, CommitId, CommitModifications, EditableField};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Type of visual selection mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualType {
    /// Line-wise selection (V) - selects entire rows
    Line,
    /// Block-wise selection (Ctrl+V) - selects rectangular region
    Block,
}

/// Current mode of the application
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppMode {
    /// Normal navigation mode
    Normal,
    /// Visual selection mode (vim-like)
    Visual {
        /// Starting position when visual mode was entered (row, column)
        anchor: (usize, usize),
        /// Type of visual selection
        visual_type: VisualType,
    },
    /// Editing a specific field on a commit
    Editing {
        commit_idx: usize,
        field: EditableField,
    },
    /// Search/filter mode
    Search,
    /// Reordering commits (move mode)
    #[allow(dead_code)]
    Reorder,
    /// Confirmation dialog
    Confirming(ConfirmAction),
    /// Help screen
    Help,
    /// Quitting (confirm if dirty)
    Quitting,
}

/// Actions that require confirmation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfirmAction {
    ApplyChanges,
    DiscardChanges,
    #[allow(dead_code)]
    QuitWithChanges,
}

/// Snapshot of state for undo/redo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UndoSnapshot {
    pub commit_order: Vec<CommitId>,
    pub modifications: HashMap<CommitId, CommitModifications>,
    pub deleted: HashSet<CommitId>,
    pub description: String,
}

/// Central application state
pub struct AppState {
    /// All loaded commits in current display order
    pub commits: Vec<CommitData>,

    /// Original order of commits (for detecting reorder changes)
    pub original_order: Vec<CommitId>,

    /// Current order of commits (may differ from original if reordered)
    pub current_order: Vec<CommitId>,

    /// Pending modifications per commit
    pub modifications: HashMap<CommitId, CommitModifications>,

    /// Selected commits (for multi-select operations)
    pub selected: HashSet<CommitId>,

    /// Commits marked for deletion
    pub deleted: HashSet<CommitId>,

    /// Index of the cursor (focused commit in visible list)
    pub cursor: usize,

    /// Current application mode
    pub mode: AppMode,

    /// Current search/filter query
    pub search_query: String,

    /// Filtered commit indices (None = show all)
    pub filtered_indices: Option<Vec<usize>>,

    /// Undo stack
    pub undo_stack: Vec<UndoSnapshot>,

    /// Redo stack
    pub redo_stack: Vec<UndoSnapshot>,

    /// Scroll offset for table (vertical)
    pub scroll_offset: usize,

    /// Horizontal scroll offset for table
    pub h_scroll_offset: usize,

    /// Current column index (for inline editing navigation)
    pub column_index: usize,

    /// Current branch name
    pub branch_name: String,

    /// Whether branch has upstream (affects force-push warning)
    pub has_upstream: bool,

    /// Error message to display (cleared on next action)
    pub error_message: Option<String>,

    /// Success message to display (cleared on next action)
    pub success_message: Option<String>,

    /// Inline edit buffer (current value being edited)
    pub edit_buffer: String,

    /// Original value before inline edit started
    pub edit_original: String,

    /// Cursor position within the edit buffer
    pub edit_cursor: usize,

    /// Commits targeted by visual selection for editing
    /// Set when pressing 'e' in visual mode, cleared after edit completes
    pub visual_edit_targets: Option<Vec<CommitId>>,

    /// Scroll offset for detail pane (vertical)
    pub detail_scroll: usize,

    /// Maximum scroll for detail pane (computed during render)
    #[allow(dead_code)]
    pub detail_max_scroll: usize,

    /// Whether to sync author field changes to committer fields (default: true)
    /// When enabled, editing author name/email/date will also update the
    /// corresponding committer field unless --separate-author-committer is used.
    pub sync_author_to_committer: bool,

    /// Scroll offset for help screen (vertical)
    pub help_scroll: usize,
}

impl AppState {
    /// Create a new AppState with loaded commits
    pub fn new(commits: Vec<CommitData>, branch_name: String, has_upstream: bool) -> Self {
        let original_order: Vec<CommitId> = commits.iter().map(|c| c.id).collect();
        let current_order = original_order.clone();

        Self {
            commits,
            original_order,
            current_order,
            modifications: HashMap::new(),
            selected: HashSet::new(),
            deleted: HashSet::new(),
            cursor: 0,
            mode: AppMode::Normal,
            search_query: String::new(),
            filtered_indices: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            scroll_offset: 0,
            h_scroll_offset: 0,
            column_index: 0,
            branch_name,
            has_upstream,
            error_message: None,
            success_message: None,
            edit_buffer: String::new(),
            edit_original: String::new(),
            edit_cursor: 0,
            visual_edit_targets: None,
            detail_scroll: 0,
            detail_max_scroll: 0,
            sync_author_to_committer: true,
            help_scroll: 0,
        }
    }

    /// Set whether author changes should sync to committer fields
    pub fn set_sync_author_to_committer(&mut self, sync: bool) {
        self.sync_author_to_committer = sync;
    }

    /// Scroll detail pane up
    #[allow(dead_code)]
    pub fn detail_scroll_up(&mut self, amount: usize) {
        self.detail_scroll = self.detail_scroll.saturating_sub(amount);
    }

    /// Scroll detail pane down
    #[allow(dead_code)]
    pub fn detail_scroll_down(&mut self, amount: usize) {
        self.detail_scroll = (self.detail_scroll + amount).min(self.detail_max_scroll);
    }

    /// Reset detail scroll when cursor changes
    pub fn reset_detail_scroll(&mut self) {
        self.detail_scroll = 0;
    }

    /// Scroll help screen up
    pub fn help_scroll_up(&mut self, amount: usize) {
        self.help_scroll = self.help_scroll.saturating_sub(amount);
    }

    /// Scroll help screen down
    pub fn help_scroll_down(&mut self, amount: usize, max_scroll: usize) {
        self.help_scroll = (self.help_scroll + amount).min(max_scroll);
    }

    /// Reset help scroll when opening help
    pub fn reset_help_scroll(&mut self) {
        self.help_scroll = 0;
    }

    /// Total number of columns (Selection, Hash, Name, Email, Date, Message)
    pub const NUM_COLUMNS: usize = 6;

    // ==================== Cursor Position Query Methods ====================
    // These methods form a complete cursor API for future features

    /// Get the current cursor row index (in visible commits)
    #[allow(dead_code)]
    pub fn cursor_row(&self) -> usize {
        self.cursor
    }

    /// Get the current cursor column index (0-5)
    #[allow(dead_code)]
    pub fn cursor_column(&self) -> usize {
        self.column_index
    }

    /// Get the current cursor position as (row, column)
    #[allow(dead_code)]
    pub fn cursor_position(&self) -> (usize, usize) {
        (self.cursor, self.column_index)
    }

    /// Check if the cursor is on a specific row
    #[allow(dead_code)]
    pub fn is_cursor_on_row(&self, row: usize) -> bool {
        self.cursor == row
    }

    /// Check if the cursor is on a specific column
    #[allow(dead_code)]
    pub fn is_cursor_on_column(&self, column: usize) -> bool {
        self.column_index == column
    }

    /// Check if the cursor is on a specific cell (row, column)
    #[allow(dead_code)]
    pub fn is_cursor_on_cell(&self, row: usize, column: usize) -> bool {
        self.cursor == row && self.column_index == column
    }

    /// Check if the cursor is on an editable column
    #[allow(dead_code)]
    pub fn is_cursor_on_editable_column(&self) -> bool {
        // Columns 0 (Selection) and 1 (Hash) are not editable
        self.column_index >= 2
    }

    // ==================== Cursor Position Setter Methods ====================

    /// Set cursor to a specific row (clamped to valid range)
    #[allow(dead_code)]
    pub fn set_cursor_row(&mut self, row: usize) {
        let max = self.visible_commits().len().saturating_sub(1);
        self.cursor = row.min(max);
        self.adjust_scroll();
    }

    /// Set cursor to a specific column (clamped to valid range)
    #[allow(dead_code)]
    pub fn set_cursor_column(&mut self, column: usize) {
        self.column_index = column.min(Self::NUM_COLUMNS - 1);
    }

    /// Set cursor to a specific cell (row, column)
    #[allow(dead_code)]
    pub fn set_cursor_position(&mut self, row: usize, column: usize) {
        self.set_cursor_row(row);
        self.set_cursor_column(column);
    }

    /// Move column focus left (wraps to last column if at first)
    pub fn column_left(&mut self) {
        if self.column_index > 0 {
            self.column_index -= 1;
        } else {
            self.column_index = Self::NUM_COLUMNS - 1;
        }
    }

    /// Move column focus right (wraps to first column if at last)
    pub fn column_right(&mut self) {
        if self.column_index < Self::NUM_COLUMNS - 1 {
            self.column_index += 1;
        } else {
            self.column_index = 0;
        }
    }

    /// Scroll table left (for horizontal scrolling)
    #[allow(dead_code)]
    pub fn scroll_left(&mut self, amount: usize) {
        self.h_scroll_offset = self.h_scroll_offset.saturating_sub(amount);
    }

    /// Scroll table right (for horizontal scrolling)
    #[allow(dead_code)]
    pub fn scroll_right(&mut self, amount: usize, max_scroll: usize) {
        self.h_scroll_offset = (self.h_scroll_offset + amount).min(max_scroll);
    }

    /// Get the visible commits (filtered or all)
    pub fn visible_commits(&self) -> Vec<&CommitData> {
        match &self.filtered_indices {
            Some(indices) => indices
                .iter()
                .filter_map(|&i| self.commits.get(i))
                .collect(),
            None => self.commits.iter().collect(),
        }
    }

    /// Get the commit at the cursor position
    pub fn cursor_commit(&self) -> Option<&CommitData> {
        match &self.filtered_indices {
            Some(indices) => indices.get(self.cursor).and_then(|&i| self.commits.get(i)),
            None => self.commits.get(self.cursor),
        }
    }

    /// Get the commit ID at the cursor position
    pub fn cursor_commit_id(&self) -> Option<CommitId> {
        self.cursor_commit().map(|c| c.id)
    }

    /// Get mutable reference to modifications for a commit
    pub fn get_or_create_modifications(&mut self, id: CommitId) -> &mut CommitModifications {
        self.modifications.entry(id).or_default()
    }

    /// Check if a commit has modifications
    #[allow(dead_code)]
    pub fn is_modified(&self, id: CommitId) -> bool {
        self.modifications
            .get(&id)
            .map(|m| m.has_modifications())
            .unwrap_or(false)
    }

    /// Check if a commit is selected
    pub fn is_selected(&self, id: CommitId) -> bool {
        self.selected.contains(&id)
    }

    /// Check if a commit is marked for deletion
    pub fn is_deleted(&self, id: CommitId) -> bool {
        self.deleted.contains(&id)
    }

    /// Toggle deletion mark on commit at cursor
    pub fn toggle_deletion(&mut self) {
        if let Some(id) = self.cursor_commit_id() {
            if self.deleted.contains(&id) {
                self.deleted.remove(&id);
            } else {
                self.deleted.insert(id);
            }
        }
    }

    /// Mark a specific commit for deletion
    pub fn mark_deleted(&mut self, id: CommitId) {
        self.deleted.insert(id);
    }

    /// Unmark a commit from deletion
    pub fn unmark_deleted(&mut self, id: CommitId) {
        self.deleted.remove(&id);
    }

    /// Get count of deleted commits
    pub fn deleted_count(&self) -> usize {
        self.deleted.len()
    }

    /// Clear all deletion marks
    pub fn clear_deletions(&mut self) {
        self.deleted.clear();
    }

    /// Toggle selection of the commit at cursor
    pub fn toggle_selection(&mut self) {
        if let Some(id) = self.cursor_commit_id() {
            if self.selected.contains(&id) {
                self.selected.remove(&id);
            } else {
                self.selected.insert(id);
            }
        }
    }

    /// Select all visible commits
    pub fn select_all(&mut self) {
        let ids: Vec<_> = self.visible_commits().iter().map(|c| c.id).collect();
        for id in ids {
            self.selected.insert(id);
        }
    }

    /// Deselect all commits
    pub fn deselect_all(&mut self) {
        self.selected.clear();
    }

    /// Move cursor up
    pub fn cursor_up(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.adjust_scroll();
            self.reset_detail_scroll();
        }
    }

    /// Move cursor down
    pub fn cursor_down(&mut self) {
        let max = self.visible_commits().len().saturating_sub(1);
        if self.cursor < max {
            self.cursor += 1;
            self.adjust_scroll();
            self.reset_detail_scroll();
        }
    }

    /// Move cursor to top
    pub fn cursor_top(&mut self) {
        self.cursor = 0;
        self.scroll_offset = 0;
        self.reset_detail_scroll();
    }

    /// Move cursor to bottom
    pub fn cursor_bottom(&mut self) {
        self.cursor = self.visible_commits().len().saturating_sub(1);
        self.adjust_scroll();
        self.reset_detail_scroll();
    }

    /// Page up
    pub fn page_up(&mut self, page_size: usize) {
        self.cursor = self.cursor.saturating_sub(page_size);
        self.adjust_scroll();
        self.reset_detail_scroll();
    }

    /// Page down
    pub fn page_down(&mut self, page_size: usize) {
        let max = self.visible_commits().len().saturating_sub(1);
        self.cursor = (self.cursor + page_size).min(max);
        self.adjust_scroll();
        self.reset_detail_scroll();
    }

    /// Adjust scroll offset to keep cursor visible
    fn adjust_scroll(&mut self) {
        // This will be called with actual table height from the UI
        // For now, use a reasonable default
        let visible_height = 20;

        if self.cursor < self.scroll_offset {
            self.scroll_offset = self.cursor;
        } else if self.cursor >= self.scroll_offset + visible_height {
            self.scroll_offset = self.cursor - visible_height + 1;
        }
    }

    /// Update scroll based on actual table height
    pub fn update_scroll_for_height(&mut self, height: usize) {
        if self.cursor < self.scroll_offset {
            self.scroll_offset = self.cursor;
        } else if self.cursor >= self.scroll_offset + height {
            self.scroll_offset = self.cursor - height + 1;
        }
    }

    /// Move commit at cursor up (for reordering)
    pub fn move_commit_up(&mut self) {
        if self.cursor > 0 && self.filtered_indices.is_none() {
            self.save_undo("Reorder commits");
            self.current_order.swap(self.cursor, self.cursor - 1);
            self.commits.swap(self.cursor, self.cursor - 1);
            self.cursor -= 1;
        }
    }

    /// Move commit at cursor down (for reordering)
    pub fn move_commit_down(&mut self) {
        if self.cursor < self.commits.len() - 1 && self.filtered_indices.is_none() {
            self.save_undo("Reorder commits");
            self.current_order.swap(self.cursor, self.cursor + 1);
            self.commits.swap(self.cursor, self.cursor + 1);
            self.cursor += 1;
        }
    }

    /// Apply search filter
    pub fn apply_filter(&mut self) {
        if self.search_query.is_empty() {
            self.filtered_indices = None;
            return;
        }

        let query = self.search_query.to_lowercase();
        let indices: Vec<usize> = self
            .commits
            .iter()
            .enumerate()
            .filter(|(_, c)| {
                c.author.name.to_lowercase().contains(&query)
                    || c.author.email.to_lowercase().contains(&query)
                    || c.message.to_lowercase().contains(&query)
                    || c.short_hash.to_lowercase().contains(&query)
            })
            .map(|(i, _)| i)
            .collect();

        self.filtered_indices = if indices.is_empty() {
            None
        } else {
            Some(indices)
        };
        self.cursor = 0;
        self.scroll_offset = 0;
    }

    /// Clear search filter
    pub fn clear_filter(&mut self) {
        self.search_query.clear();
        self.filtered_indices = None;
    }

    /// Save current state to undo stack
    pub fn save_undo(&mut self, description: &str) {
        let snapshot = UndoSnapshot {
            commit_order: self.current_order.clone(),
            modifications: self.modifications.clone(),
            deleted: self.deleted.clone(),
            description: description.to_string(),
        };
        self.undo_stack.push(snapshot);
        self.redo_stack.clear(); // Clear redo stack on new change
    }

    /// Undo last change
    pub fn undo(&mut self) -> bool {
        if let Some(snapshot) = self.undo_stack.pop() {
            // Save current state to redo stack
            let current = UndoSnapshot {
                commit_order: self.current_order.clone(),
                modifications: self.modifications.clone(),
                deleted: self.deleted.clone(),
                description: snapshot.description.clone(),
            };
            self.redo_stack.push(current);

            // Restore from snapshot
            self.current_order = snapshot.commit_order;
            self.modifications = snapshot.modifications;
            self.deleted = snapshot.deleted;

            // Rebuild commits array in new order
            self.rebuild_commits_order();

            true
        } else {
            false
        }
    }

    /// Redo last undone change
    pub fn redo(&mut self) -> bool {
        if let Some(snapshot) = self.redo_stack.pop() {
            // Save current state to undo stack
            let current = UndoSnapshot {
                commit_order: self.current_order.clone(),
                modifications: self.modifications.clone(),
                deleted: self.deleted.clone(),
                description: snapshot.description.clone(),
            };
            self.undo_stack.push(current);

            // Restore from snapshot
            self.current_order = snapshot.commit_order;
            self.modifications = snapshot.modifications;
            self.deleted = snapshot.deleted;

            // Rebuild commits array in new order
            self.rebuild_commits_order();

            true
        } else {
            false
        }
    }

    /// Rebuild commits vector in current_order
    fn rebuild_commits_order(&mut self) {
        let commit_map: HashMap<CommitId, CommitData> =
            self.commits.drain(..).map(|c| (c.id, c)).collect();

        self.commits = self
            .current_order
            .iter()
            .filter_map(|id| commit_map.get(id).cloned())
            .collect();
    }

    /// Check if there are any pending changes
    pub fn is_dirty(&self) -> bool {
        // Check for modifications
        if self.modifications.values().any(|m| m.has_modifications()) {
            return true;
        }
        // Check for deletions
        if !self.deleted.is_empty() {
            return true;
        }
        // Check for reordering
        if self.current_order != self.original_order {
            return true;
        }
        false
    }

    /// Get count of modified commits
    pub fn modified_count(&self) -> usize {
        self.modifications
            .values()
            .filter(|m| m.has_modifications())
            .count()
    }

    /// Clear all modifications
    pub fn clear_modifications(&mut self) {
        self.modifications.clear();
        self.deleted.clear();
        self.current_order = self.original_order.clone();
        self.rebuild_commits_order();
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    /// Set error message (auto-cleared on next action)
    pub fn set_error(&mut self, msg: impl Into<String>) {
        self.error_message = Some(msg.into());
        self.success_message = None;
    }

    /// Set success message (auto-cleared on next action)
    pub fn set_success(&mut self, msg: impl Into<String>) {
        self.success_message = Some(msg.into());
        self.error_message = None;
    }

    /// Clear status messages
    pub fn clear_messages(&mut self) {
        self.error_message = None;
        self.success_message = None;
    }

    // ==================== Visual Selection Methods ====================

    /// Enter visual mode at current cursor position
    pub fn enter_visual_mode(&mut self, visual_type: VisualType) {
        let anchor = (self.cursor, self.column_index);
        self.mode = AppMode::Visual {
            anchor,
            visual_type,
        };
    }

    /// Exit visual mode without applying selection
    pub fn exit_visual_mode(&mut self) {
        self.mode = AppMode::Normal;
    }

    /// Get the visual selection range as ((start_row, start_col), (end_row, end_col))
    /// Returns None if not in visual mode
    pub fn visual_range(&self) -> Option<((usize, usize), (usize, usize))> {
        match &self.mode {
            AppMode::Visual { anchor, .. } => {
                let current = (self.cursor, self.column_index);
                // Normalize so start <= end
                let start_row = anchor.0.min(current.0);
                let end_row = anchor.0.max(current.0);
                let start_col = anchor.1.min(current.1);
                let end_col = anchor.1.max(current.1);
                Some(((start_row, start_col), (end_row, end_col)))
            }
            _ => None,
        }
    }

    /// Check if a cell is within the visual selection
    #[allow(dead_code)]
    pub fn is_in_visual_selection(&self, row: usize, col: usize) -> bool {
        match &self.mode {
            AppMode::Visual {
                anchor,
                visual_type,
            } => {
                let current = (self.cursor, self.column_index);
                let start_row = anchor.0.min(current.0);
                let end_row = anchor.0.max(current.0);

                match visual_type {
                    VisualType::Line => {
                        // Line-wise: entire row is selected if row is in range
                        row >= start_row && row <= end_row
                    }
                    VisualType::Block => {
                        // Block-wise: cell must be within rectangular region
                        let start_col = anchor.1.min(current.1);
                        let end_col = anchor.1.max(current.1);
                        row >= start_row && row <= end_row && col >= start_col && col <= end_col
                    }
                }
            }
            _ => false,
        }
    }

    /// Check if a row is within the visual selection (for row-level styling)
    #[allow(dead_code)]
    pub fn is_row_in_visual_selection(&self, row: usize) -> bool {
        match &self.mode {
            AppMode::Visual { anchor, .. } => {
                let start_row = anchor.0.min(self.cursor);
                let end_row = anchor.0.max(self.cursor);
                row >= start_row && row <= end_row
            }
            _ => false,
        }
    }

    /// Get the visual selection type if in visual mode
    pub fn visual_type(&self) -> Option<VisualType> {
        match &self.mode {
            AppMode::Visual { visual_type, .. } => Some(*visual_type),
            _ => None,
        }
    }

    /// Apply visual selection to the selected set (confirm visual selection)
    #[allow(dead_code)]
    pub fn apply_visual_selection(&mut self) {
        if let Some(((start_row, _), (end_row, _))) = self.visual_range() {
            // Collect IDs first to avoid borrow issues
            let ids: Vec<CommitId> = self
                .visible_commits()
                .iter()
                .enumerate()
                .filter_map(|(idx, c)| {
                    if idx >= start_row && idx <= end_row {
                        Some(c.id)
                    } else {
                        None
                    }
                })
                .collect();
            for id in ids {
                self.selected.insert(id);
            }
        }
        self.mode = AppMode::Normal;
    }

    /// Get the count of rows in visual selection
    pub fn visual_selection_count(&self) -> usize {
        match self.visual_range() {
            Some(((start_row, _), (end_row, _))) => end_row - start_row + 1,
            None => 0,
        }
    }

    /// Capture visual selection as edit targets and exit visual mode
    /// Returns the number of commits captured
    pub fn capture_visual_edit_targets(&mut self) -> usize {
        if let Some(((start_row, _), (end_row, _))) = self.visual_range() {
            let ids: Vec<CommitId> = self
                .visible_commits()
                .iter()
                .enumerate()
                .filter_map(|(idx, c)| {
                    if idx >= start_row && idx <= end_row {
                        Some(c.id)
                    } else {
                        None
                    }
                })
                .collect();
            let count = ids.len();
            self.visual_edit_targets = Some(ids);
            self.mode = AppMode::Normal;
            count
        } else {
            0
        }
    }

    /// Clear visual edit targets (called after edit completes)
    pub fn clear_visual_edit_targets(&mut self) {
        self.visual_edit_targets = None;
    }

    /// Get the commits to edit: visual targets > checkbox selected > just cursor
    pub fn commits_to_edit(&self) -> Vec<CommitId> {
        if let Some(ref targets) = self.visual_edit_targets {
            targets.clone()
        } else if !self.selected.is_empty() {
            self.selected.iter().copied().collect()
        } else if let Some(id) = self.cursor_commit_id() {
            vec![id]
        } else {
            vec![]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{FixedOffset, TimeZone};

    fn create_test_commit(id_str: &str, summary: &str) -> CommitData {
        let oid = git2::Oid::from_str(id_str).unwrap();
        let utc = FixedOffset::east_opt(0).unwrap();
        let dt = utc.with_ymd_and_hms(2024, 1, 15, 14, 30, 0).unwrap();

        CommitData {
            id: CommitId(oid),
            short_hash: id_str[..7].to_string(),
            author: crate::git::commit::Person::new("Test Author", "test@example.com"),
            author_date: dt,
            committer: crate::git::commit::Person::new("Test Author", "test@example.com"),
            committer_date: dt,
            message: summary.to_string(),
            summary: summary.to_string(),
            parent_ids: vec![],
            tree_id: git2::Oid::from_str("abcdef1234567890abcdef1234567890abcdef12").unwrap(),
            is_merge: false,
        }
    }

    fn create_test_state() -> AppState {
        let commits = vec![
            create_test_commit("1111111111111111111111111111111111111111", "First commit"),
            create_test_commit("2222222222222222222222222222222222222222", "Second commit"),
            create_test_commit("3333333333333333333333333333333333333333", "Third commit"),
        ];
        AppState::new(commits, "main".to_string(), false)
    }

    #[test]
    fn test_app_state_creation() {
        let state = create_test_state();
        assert_eq!(state.commits.len(), 3);
        assert_eq!(state.cursor, 0);
        assert_eq!(state.branch_name, "main");
        assert!(!state.has_upstream);
        assert!(state.modifications.is_empty());
    }

    #[test]
    fn test_cursor_movement() {
        let mut state = create_test_state();

        // Test cursor down
        state.cursor_down();
        assert_eq!(state.cursor, 1);
        state.cursor_down();
        assert_eq!(state.cursor, 2);

        // Test cursor up
        state.cursor_up();
        assert_eq!(state.cursor, 1);

        // Test cursor top
        state.cursor_top();
        assert_eq!(state.cursor, 0);

        // Test cursor bottom
        state.cursor_bottom();
        assert_eq!(state.cursor, 2);
    }

    #[test]
    fn test_cursor_movement_bounds() {
        let mut state = create_test_state();

        // Try to move up from top
        state.cursor_up();
        assert_eq!(state.cursor, 0);

        // Try to move down beyond bottom
        state.cursor_bottom();
        state.cursor_down();
        assert_eq!(state.cursor, 2); // Should stay at bottom
    }

    #[test]
    fn test_column_navigation() {
        let mut state = create_test_state();

        assert_eq!(state.column_index, 0);

        state.column_right();
        assert_eq!(state.column_index, 1);

        state.column_left();
        assert_eq!(state.column_index, 0);
    }

    #[test]
    fn test_column_navigation_wraps() {
        let mut state = create_test_state();

        // Test wrapping right
        state.column_index = AppState::NUM_COLUMNS - 1;
        state.column_right();
        assert_eq!(state.column_index, 0);

        // Test wrapping left
        state.column_index = 0;
        state.column_left();
        assert_eq!(state.column_index, AppState::NUM_COLUMNS - 1);
    }

    #[test]
    fn test_selection() {
        let mut state = create_test_state();

        // Toggle selection on first commit
        state.toggle_selection();
        let first_id = state.commits[0].id;
        assert!(state.is_selected(first_id));

        // Toggle again to deselect
        state.toggle_selection();
        assert!(!state.is_selected(first_id));
    }

    #[test]
    fn test_select_all() {
        let mut state = create_test_state();

        state.select_all();
        assert_eq!(state.selected.len(), 3);

        for commit in &state.commits {
            assert!(state.is_selected(commit.id));
        }
    }

    #[test]
    fn test_deselect_all() {
        let mut state = create_test_state();

        state.select_all();
        state.deselect_all();
        assert_eq!(state.selected.len(), 0);
    }

    #[test]
    fn test_modifications() {
        let mut state = create_test_state();
        let commit_id = state.commits[0].id;

        // Initially no modifications
        assert!(!state.is_modified(commit_id));
        assert!(!state.is_dirty());

        // Add a modification
        let mods = state.get_or_create_modifications(commit_id);
        mods.author_name = Some("New Author".to_string());

        assert!(state.is_modified(commit_id));
        assert!(state.is_dirty());
        assert_eq!(state.modified_count(), 1);
    }

    #[test]
    fn test_clear_modifications() {
        let mut state = create_test_state();
        let commit_id = state.commits[0].id;

        // Add modifications
        let mods = state.get_or_create_modifications(commit_id);
        mods.author_name = Some("New Author".to_string());

        assert!(state.is_dirty());

        // Clear all modifications
        state.clear_modifications();
        assert!(!state.is_dirty());
        assert_eq!(state.modified_count(), 0);
    }

    #[test]
    fn test_undo_redo() {
        let mut state = create_test_state();
        let commit_id = state.commits[0].id;

        // Save initial state
        state.save_undo("Initial modification");

        // Make a modification
        let mods = state.get_or_create_modifications(commit_id);
        mods.author_name = Some("New Author".to_string());

        // Undo should restore
        let undone = state.undo();
        assert!(undone);
        assert!(!state.is_modified(commit_id));

        // Redo should restore the modification
        let redone = state.redo();
        assert!(redone);
        assert!(state.is_modified(commit_id));
    }

    #[test]
    fn test_undo_redo_empty() {
        let mut state = create_test_state();

        // Undo with empty stack
        let undone = state.undo();
        assert!(!undone);

        // Redo with empty stack
        let redone = state.redo();
        assert!(!redone);
    }

    #[test]
    fn test_search_filter() {
        let mut state = create_test_state();

        // Apply filter
        state.search_query = "Second".to_string();
        state.apply_filter();

        // Should only show one commit
        let visible = state.visible_commits();
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].summary, "Second commit");

        // Clear filter
        state.clear_filter();
        let visible = state.visible_commits();
        assert_eq!(visible.len(), 3);
    }

    #[test]
    fn test_search_filter_case_insensitive() {
        let mut state = create_test_state();

        state.search_query = "SECOND".to_string();
        state.apply_filter();

        let visible = state.visible_commits();
        assert_eq!(visible.len(), 1);
    }

    #[test]
    fn test_search_filter_empty_result() {
        let mut state = create_test_state();

        state.search_query = "nonexistent".to_string();
        state.apply_filter();

        assert!(state.filtered_indices.is_none());
    }

    #[test]
    fn test_visual_mode() {
        let mut state = create_test_state();

        // Enter visual mode
        state.enter_visual_mode(VisualType::Line);
        assert!(matches!(state.mode, AppMode::Visual { .. }));

        // Check visual range
        let range = state.visual_range();
        assert!(range.is_some());

        // Exit visual mode
        state.exit_visual_mode();
        assert_eq!(state.mode, AppMode::Normal);
    }

    #[test]
    fn test_visual_selection_line() {
        let mut state = create_test_state();

        state.enter_visual_mode(VisualType::Line);
        state.cursor_down();
        state.cursor_down();

        // Should select rows 0, 1, 2
        assert_eq!(state.visual_selection_count(), 3);
        assert!(state.is_row_in_visual_selection(0));
        assert!(state.is_row_in_visual_selection(1));
        assert!(state.is_row_in_visual_selection(2));
    }

    #[test]
    fn test_visual_selection_block() {
        let mut state = create_test_state();

        state.enter_visual_mode(VisualType::Block);
        state.cursor_down();
        state.column_right();

        // Check that specific cells are selected
        assert!(state.is_in_visual_selection(0, 0));
        assert!(state.is_in_visual_selection(0, 1));
        assert!(state.is_in_visual_selection(1, 0));
        assert!(state.is_in_visual_selection(1, 1));
        assert!(!state.is_in_visual_selection(2, 0));
    }

    #[test]
    fn test_apply_visual_selection() {
        let mut state = create_test_state();

        state.enter_visual_mode(VisualType::Line);
        state.cursor_down();

        state.apply_visual_selection();

        // Should have selected 2 commits
        assert_eq!(state.selected.len(), 2);
        assert_eq!(state.mode, AppMode::Normal);
    }

    #[test]
    fn test_capture_visual_edit_targets() {
        let mut state = create_test_state();

        state.enter_visual_mode(VisualType::Line);
        state.cursor_down();

        let count = state.capture_visual_edit_targets();
        assert_eq!(count, 2);
        assert!(state.visual_edit_targets.is_some());
        assert_eq!(state.mode, AppMode::Normal);
    }

    #[test]
    fn test_commits_to_edit_priority() {
        let mut state = create_test_state();

        // Test 1: Just cursor (no selection, no visual targets)
        let to_edit = state.commits_to_edit();
        assert_eq!(to_edit.len(), 1);
        assert_eq!(to_edit[0], state.commits[0].id);

        // Test 2: Checkbox selection takes priority over cursor
        state.toggle_selection();
        let to_edit = state.commits_to_edit();
        assert_eq!(to_edit.len(), 1);

        // Test 3: Visual targets take priority over checkbox
        state.visual_edit_targets = Some(vec![state.commits[1].id, state.commits[2].id]);
        let to_edit = state.commits_to_edit();
        assert_eq!(to_edit.len(), 2);
        assert_eq!(to_edit[0], state.commits[1].id);
    }

    #[test]
    fn test_cursor_position_queries() {
        let mut state = create_test_state();

        state.set_cursor_position(1, 2);
        assert_eq!(state.cursor_row(), 1);
        assert_eq!(state.cursor_column(), 2);
        assert_eq!(state.cursor_position(), (1, 2));
        assert!(state.is_cursor_on_row(1));
        assert!(state.is_cursor_on_column(2));
        assert!(state.is_cursor_on_cell(1, 2));
        assert!(!state.is_cursor_on_cell(0, 2));
    }

    #[test]
    fn test_is_cursor_on_editable_column() {
        let mut state = create_test_state();

        // Column 0 and 1 are not editable (Selection, Hash)
        state.set_cursor_column(0);
        assert!(!state.is_cursor_on_editable_column());

        state.set_cursor_column(1);
        assert!(!state.is_cursor_on_editable_column());

        // Column 2+ are editable
        state.set_cursor_column(2);
        assert!(state.is_cursor_on_editable_column());
    }

    #[test]
    fn test_page_navigation() {
        let mut state = create_test_state();

        state.page_down(2);
        assert_eq!(state.cursor, 2);

        state.page_up(1);
        assert_eq!(state.cursor, 1);

        // Page down beyond bottom
        state.page_down(10);
        assert_eq!(state.cursor, 2);

        // Page up beyond top
        state.page_up(10);
        assert_eq!(state.cursor, 0);
    }

    #[test]
    fn test_detail_scroll() {
        let mut state = create_test_state();
        state.detail_max_scroll = 10;

        state.detail_scroll_down(5);
        assert_eq!(state.detail_scroll, 5);

        state.detail_scroll_up(2);
        assert_eq!(state.detail_scroll, 3);

        // Test bounds
        state.detail_scroll_down(20);
        assert_eq!(state.detail_scroll, 10); // Clamped to max

        state.detail_scroll_up(100);
        assert_eq!(state.detail_scroll, 0); // Clamped to 0
    }

    #[test]
    fn test_reset_detail_scroll() {
        let mut state = create_test_state();
        state.detail_scroll = 5;

        state.reset_detail_scroll();
        assert_eq!(state.detail_scroll, 0);
    }

    #[test]
    fn test_cursor_commit() {
        let mut state = create_test_state();

        let commit = state.cursor_commit();
        assert!(commit.is_some());
        assert_eq!(commit.unwrap().summary, "First commit");

        state.cursor_down();
        let commit = state.cursor_commit();
        assert_eq!(commit.unwrap().summary, "Second commit");
    }

    #[test]
    fn test_cursor_commit_id() {
        let state = create_test_state();

        let id = state.cursor_commit_id();
        assert!(id.is_some());
        assert_eq!(id.unwrap(), state.commits[0].id);
    }

    #[test]
    fn test_messages() {
        let mut state = create_test_state();

        state.set_error("Test error");
        assert!(state.error_message.is_some());
        assert!(state.success_message.is_none());

        state.set_success("Test success");
        assert!(state.success_message.is_some());
        assert!(state.error_message.is_none());

        state.clear_messages();
        assert!(state.error_message.is_none());
        assert!(state.success_message.is_none());
    }

    #[test]
    fn test_move_commit_up() {
        let mut state = create_test_state();
        let first_id = state.commits[0].id;
        let second_id = state.commits[1].id;

        state.cursor = 1;
        state.move_commit_up();

        // Second commit should now be first
        assert_eq!(state.commits[0].id, second_id);
        assert_eq!(state.commits[1].id, first_id);
        assert_eq!(state.cursor, 0);
        assert!(state.is_dirty()); // Order changed
    }

    #[test]
    fn test_move_commit_down() {
        let mut state = create_test_state();
        let first_id = state.commits[0].id;
        let second_id = state.commits[1].id;

        state.cursor = 0;
        state.move_commit_down();

        // First commit should now be second
        assert_eq!(state.commits[0].id, second_id);
        assert_eq!(state.commits[1].id, first_id);
        assert_eq!(state.cursor, 1);
    }

    #[test]
    fn test_visual_type() {
        let mut state = create_test_state();

        assert!(state.visual_type().is_none());

        state.enter_visual_mode(VisualType::Line);
        assert_eq!(state.visual_type(), Some(VisualType::Line));

        state.exit_visual_mode();
        assert!(state.visual_type().is_none());
    }

    #[test]
    fn test_sync_author_to_committer_default() {
        let state = create_test_state();
        // Default should be true (sync enabled)
        assert!(state.sync_author_to_committer);
    }

    #[test]
    fn test_set_sync_author_to_committer() {
        let mut state = create_test_state();

        // Default is true
        assert!(state.sync_author_to_committer);

        // Disable sync
        state.set_sync_author_to_committer(false);
        assert!(!state.sync_author_to_committer);

        // Re-enable sync
        state.set_sync_author_to_committer(true);
        assert!(state.sync_author_to_committer);
    }
}
