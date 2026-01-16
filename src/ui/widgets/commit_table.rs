#![allow(clippy::cast_possible_truncation)]

use crate::git::commit::{CommitData, CommitModifications, EditableField};
use crate::state::{AppMode, AppState, VisualType};
use crate::ui::theme::Theme;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Row, Table, TableState};
use ratatui::Frame;
use unicode_width::UnicodeWidthStr;

/// Column indices for the table
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Column {
    Selection = 0,
    Hash = 1,
    Name = 2,
    Email = 3,
    Date = 4,
    Message = 5,
}

impl Column {
    #[must_use]
    pub fn from_index(idx: usize) -> Option<Self> {
        match idx {
            0 => Some(Column::Selection),
            1 => Some(Column::Hash),
            2 => Some(Column::Name),
            3 => Some(Column::Email),
            4 => Some(Column::Date),
            5 => Some(Column::Message),
            _ => None,
        }
    }

    #[must_use]
    pub fn is_editable(&self) -> bool {
        !matches!(self, Column::Selection | Column::Hash)
    }

    #[must_use]
    pub fn to_editable_field(&self) -> Option<EditableField> {
        match self {
            Column::Name => Some(EditableField::AuthorName),
            Column::Email => Some(EditableField::AuthorEmail),
            Column::Date => Some(EditableField::AuthorDate),
            Column::Message => Some(EditableField::Message),
            _ => None,
        }
    }
}

/// Column definitions with widths
struct ColumnDef {
    header: &'static str,
    min_width: u16,
    max_width: u16,
    weight: u16,
}

const COLUMNS: &[ColumnDef] = &[
    ColumnDef {
        header: " ",
        min_width: 3,
        max_width: 3,
        weight: 0,
    },
    ColumnDef {
        header: "Hash",
        min_width: 7,
        max_width: 7,
        weight: 0,
    },
    ColumnDef {
        header: "Name",
        min_width: 15,
        max_width: 30,
        weight: 2,
    },
    ColumnDef {
        header: "Email",
        min_width: 20,
        max_width: 35,
        weight: 2,
    },
    ColumnDef {
        header: "Date",
        min_width: 16,
        max_width: 16,
        weight: 0,
    },
    ColumnDef {
        header: "Message",
        min_width: 20,
        max_width: 60,
        weight: 3,
    },
];

const MESSAGE_MAX_WIDTH: usize = 50;

/// Context for rendering a single row
struct RowContext<'a> {
    row_idx: usize,
    cursor_row: usize,
    cursor_col: usize,
    is_selected: bool,
    is_deleted: bool,
    is_editing: bool,
    visual_selection: Option<VisualSelection>,
    mods: Option<&'a CommitModifications>,
    edit_buffer: &'a str,
    theme: &'a Theme,
}

/// Visual selection info for the current render
struct VisualSelection {
    visual_type: VisualType,
    start_row: usize,
    end_row: usize,
    start_col: usize,
    end_col: usize,
}

impl VisualSelection {
    fn contains_cell(&self, row: usize, col: usize) -> bool {
        let row_in_range = row >= self.start_row && row <= self.end_row;
        match self.visual_type {
            VisualType::Line => row_in_range,
            VisualType::Block => row_in_range && col >= self.start_col && col <= self.end_col,
        }
    }
}

/// Render the commit table
pub fn render_commit_table(frame: &mut Frame<'_>, area: Rect, state: &AppState, theme: &Theme) {
    let is_editing = matches!(state.mode, AppMode::Editing { .. });
    let editing_row = if let AppMode::Editing { commit_idx, .. } = &state.mode {
        Some(*commit_idx)
    } else {
        None
    };

    // Extract visual selection bounds if in visual mode
    let visual_selection = if let Some(((sr, sc), (er, ec))) = state.visual_range() {
        state.visual_type().map(|vt| VisualSelection {
            visual_type: vt,
            start_row: sr,
            end_row: er,
            start_col: sc,
            end_col: ec,
        })
    } else {
        None
    };

    // Build header
    let header_cells: Vec<Cell<'_>> = COLUMNS
        .iter()
        .enumerate()
        .map(|(idx, col)| {
            let is_active_col = is_editing && state.column_index == idx;
            let style = if is_active_col {
                theme.table_header.add_modifier(Modifier::REVERSED)
            } else {
                theme.table_header
            };
            Cell::from(col.header).style(style)
        })
        .collect();
    let header = Row::new(header_cells).height(1);

    let visible = state.visible_commits();

    // Build rows
    let rows: Vec<Row<'_>> = visible
        .iter()
        .enumerate()
        .map(|(idx, commit)| {
            let ctx = RowContext {
                row_idx: idx,
                cursor_row: state.cursor,
                cursor_col: state.column_index,
                is_selected: state.is_selected(commit.id),
                is_deleted: state.is_deleted(commit.id),
                is_editing: editing_row == Some(idx),
                visual_selection: visual_selection.as_ref().map(|v| VisualSelection {
                    visual_type: v.visual_type,
                    start_row: v.start_row,
                    end_row: v.end_row,
                    start_col: v.start_col,
                    end_col: v.end_col,
                }),
                mods: state.modifications.get(&commit.id),
                edit_buffer: &state.edit_buffer,
                theme,
            };
            create_row(commit, &ctx)
        })
        .collect();

    let widths = calculate_column_widths(area.width, state.h_scroll_offset);
    let title = build_title(state, &visible);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border)
        .title(Line::from(title).style(if state.is_dirty() {
            theme.title_dirty
        } else {
            theme.title
        }));

    // No row-level highlight - all styling is per-cell
    let table = Table::new(rows, widths).header(header).block(block);

    let mut table_state = TableState::default();
    table_state.select(Some(state.cursor));

    frame.render_stateful_widget(table, area, &mut table_state);

    // Scroll indicator
    let total_min_width: u16 = COLUMNS.iter().map(|c| c.min_width).sum();
    if total_min_width > area.width.saturating_sub(4) {
        let indicator = "← scroll →".to_string();
        let x = area.x + area.width - indicator.len() as u16 - 2;
        let y = area.y;
        if x > area.x {
            let span = Span::styled(indicator, theme.keybinding);
            frame.render_widget(
                ratatui::widgets::Paragraph::new(span),
                Rect::new(x, y, 12, 1),
            );
        }
    }
}

fn build_title(state: &AppState, visible: &[&CommitData]) -> String {
    let modified = state.modified_count();
    let deleted = state.deleted_count();

    if deleted > 0 && modified > 0 {
        format!(" Commits ({modified} modified, {deleted} deleted) ")
    } else if deleted > 0 {
        format!(" Commits ({deleted} deleted) ")
    } else if modified > 0 {
        format!(" Commits ({modified} modified) ")
    } else {
        format!(" Commits ({}) ", visible.len())
    }
}

/// Create a row with clean, non-conflicting cell styles
fn create_row<'a>(commit: &CommitData, ctx: &RowContext<'a>) -> Row<'a> {
    let is_cursor_row = ctx.row_idx == ctx.cursor_row;

    // Selection checkbox - show 'D' for deleted, 'x' for selected
    let checkbox_text = if ctx.is_deleted {
        "[D]"
    } else if ctx.is_selected {
        "[x]"
    } else {
        "[ ]"
    };
    let checkbox_base_style = if ctx.is_deleted {
        ctx.theme.deleted
    } else if ctx.is_selected {
        ctx.theme.checkbox_checked
    } else {
        ctx.theme.checkbox_unchecked
    };
    let checkbox_style = cell_style(
        ctx,
        Column::Selection as usize,
        false, // checkbox not modifiable
        checkbox_base_style,
    );
    let checkbox = Cell::from(Span::styled(checkbox_text, checkbox_style));

    // Hash
    let hash_style = cell_style(ctx, Column::Hash as usize, false, ctx.theme.hash);
    let hash = Cell::from(Span::styled(commit.short_hash.clone(), hash_style));

    // Name
    let name_modified = ctx.mods.and_then(|m| m.author_name.as_ref()).is_some();
    let name_value = if ctx.is_editing && is_cursor_row && ctx.cursor_col == Column::Name as usize {
        ctx.edit_buffer.to_string()
    } else {
        ctx.mods
            .and_then(|m| m.author_name.clone())
            .unwrap_or_else(|| commit.author.name.clone())
    };
    let name_style = cell_style(ctx, Column::Name as usize, name_modified, ctx.theme.author);
    let name = Cell::from(Span::styled(truncate_string(&name_value, 30), name_style));

    // Email
    let email_modified = ctx.mods.and_then(|m| m.author_email.as_ref()).is_some();
    let email_value = if ctx.is_editing && is_cursor_row && ctx.cursor_col == Column::Email as usize
    {
        ctx.edit_buffer.to_string()
    } else {
        ctx.mods
            .and_then(|m| m.author_email.clone())
            .unwrap_or_else(|| commit.author.email.clone())
    };
    let email_style = cell_style(
        ctx,
        Column::Email as usize,
        email_modified,
        ctx.theme.author,
    );
    let email = Cell::from(Span::styled(truncate_string(&email_value, 35), email_style));

    // Date
    let date_modified = ctx.mods.and_then(|m| m.author_date).is_some();
    let date_value = if ctx.is_editing && is_cursor_row && ctx.cursor_col == Column::Date as usize {
        ctx.edit_buffer.to_string()
    } else {
        ctx.mods.and_then(|m| m.author_date).map_or_else(
            || commit.format_author_date(),
            |d| d.format("%Y-%m-%d %H:%M").to_string(),
        )
    };
    let date_style = cell_style(ctx, Column::Date as usize, date_modified, ctx.theme.date);
    let date = Cell::from(Span::styled(date_value, date_style));

    // Message
    let message_modified = ctx.mods.and_then(|m| m.message.as_ref()).is_some();
    let message_value =
        if ctx.is_editing && is_cursor_row && ctx.cursor_col == Column::Message as usize {
            ctx.edit_buffer.to_string()
        } else {
            let summary = ctx.mods.and_then(|m| m.message.as_ref()).map_or_else(
                || commit.summary.clone(),
                |m| m.lines().next().unwrap_or("").to_string(),
            );
            truncate_string(&summary, MESSAGE_MAX_WIDTH)
        };
    let message_style = cell_style(
        ctx,
        Column::Message as usize,
        message_modified,
        ctx.theme.message,
    );
    let message = Cell::from(Span::styled(message_value, message_style));

    Row::new([checkbox, hash, name, email, date, message])
}

/// Compute the style for a single cell
///
/// Priority (highest to lowest):
/// 1. Cursor cell (active editing or navigation target)
/// 2. Visual selection
/// 3. Modified value (yellow)
/// 4. Base field color
fn cell_style(ctx: &RowContext<'_>, col: usize, is_modified: bool, base: Style) -> Style {
    let is_cursor_cell = ctx.row_idx == ctx.cursor_row && col == ctx.cursor_col;
    let is_in_visual = ctx
        .visual_selection
        .as_ref()
        .is_some_and(|v| v.contains_cell(ctx.row_idx, col));

    // Determine the base style (modified values are yellow)
    let field_style = if is_modified {
        ctx.theme.modified_value
    } else {
        base
    };

    // Apply state-based styling
    if is_cursor_cell {
        if is_in_visual {
            // Cursor within visual selection
            ctx.theme.cell_visual_cursor
        } else if ctx.is_editing {
            // Editing mode - bold reverse
            field_style.add_modifier(Modifier::REVERSED | Modifier::BOLD)
        } else {
            // Normal cursor
            ctx.theme.cell_cursor
        }
    } else if is_in_visual {
        // In visual selection but not cursor
        ctx.theme.cell_visual.patch(field_style)
    } else {
        // Normal cell
        field_style
    }
}

fn calculate_column_widths(total_width: u16, h_scroll: usize) -> Vec<Constraint> {
    let available = total_width.saturating_sub(4);

    let fixed_width: u16 = COLUMNS
        .iter()
        .filter(|c| c.weight == 0)
        .map(|c| c.min_width)
        .sum();

    let flexible_remaining = available.saturating_sub(fixed_width);
    let total_weight: u16 = COLUMNS.iter().map(|c| c.weight).sum();

    let widths: Vec<Constraint> = COLUMNS
        .iter()
        .map(|col| {
            if col.weight == 0 {
                Constraint::Length(col.min_width)
            } else {
                let flex_width = if total_weight > 0 {
                    (flexible_remaining * col.weight / total_weight)
                        .max(col.min_width)
                        .min(col.max_width)
                } else {
                    col.min_width
                };
                Constraint::Length(flex_width)
            }
        })
        .collect();

    let _ = h_scroll; // Reserved for future horizontal scrolling
    widths
}

fn truncate_string(s: &str, max_width: usize) -> String {
    let width = s.width();
    if width <= max_width {
        s.to_string()
    } else if max_width <= 3 {
        s.chars().take(max_width).collect()
    } else {
        let mut result = String::new();
        let mut current_width = 0;
        let target_width = max_width - 3;

        for c in s.chars() {
            let char_width = unicode_width::UnicodeWidthChar::width(c).unwrap_or(1);
            if current_width + char_width > target_width {
                break;
            }
            result.push(c);
            current_width += char_width;
        }
        result.push_str("...");
        result
    }
}

/// Get the value for a column from a commit
#[must_use]
pub fn get_column_value(
    commit: &CommitData,
    mods: Option<&CommitModifications>,
    column: Column,
) -> String {
    match column {
        Column::Selection => String::new(),
        Column::Hash => commit.short_hash.clone(),
        Column::Name => mods
            .and_then(|m| m.author_name.clone())
            .unwrap_or_else(|| commit.author.name.clone()),
        Column::Email => mods
            .and_then(|m| m.author_email.clone())
            .unwrap_or_else(|| commit.author.email.clone()),
        Column::Date => mods.and_then(|m| m.author_date).map_or_else(
            || commit.format_author_date_full(),
            |d| d.format("%Y-%m-%d %H:%M:%S %z").to_string(),
        ),
        Column::Message => mods
            .and_then(|m| m.message.clone())
            .unwrap_or_else(|| commit.message.clone()),
    }
}
