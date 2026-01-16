#![allow(clippy::cast_possible_truncation)]

use crate::git::commit::EditableField;
use crate::state::AppState;
use crate::ui::theme::Theme;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

/// Render the edit popup overlay showing the full value being edited
pub fn render_edit_popup(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &AppState,
    field: &EditableField,
    theme: &Theme,
) {
    // Calculate popup dimensions
    let content = &state.edit_buffer;
    let cursor_pos = state.edit_cursor;

    // Determine popup width based on content
    let content_width = content
        .len()
        .max(30)
        .min(area.width.saturating_sub(4) as usize);
    let popup_width = (content_width + 4) as u16;
    let popup_height = 5u16;

    // Center the popup horizontally, position near middle vertically
    let x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let y = area.y + (area.height.saturating_sub(popup_height)) / 2;

    let popup_area = Rect::new(x, y, popup_width, popup_height);

    // Clear background
    frame.render_widget(Clear, popup_area);

    // Build title
    let title = format!(" Edit: {} ", field.display_name());

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.dialog_border)
        .title(Line::from(title).style(theme.dialog_title))
        .style(ratatui::style::Style::default().bg(theme.dialog_bg));

    // Build content with cursor
    let spans = build_input_with_cursor(content, cursor_pos, theme);
    let input_line = Line::from(spans);

    // Hint line
    let hint = Line::from(vec![
        Span::styled("Enter", theme.keybinding_key),
        Span::raw(": save  "),
        Span::styled("Esc", theme.keybinding_key),
        Span::raw(": cancel  "),
        Span::styled("←/→", theme.keybinding_key),
        Span::raw(": move"),
    ]);

    let inner_area = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    // Render input line
    if inner_area.height > 1 {
        let input_area = Rect::new(inner_area.x, inner_area.y, inner_area.width, 1);
        let input_para = Paragraph::new(input_line);
        frame.render_widget(input_para, input_area);
    }

    // Render hint line
    if inner_area.height > 2 {
        let hint_area = Rect::new(
            inner_area.x,
            inner_area.y + inner_area.height - 1,
            inner_area.width,
            1,
        );
        let hint_para = Paragraph::new(hint);
        frame.render_widget(hint_para, hint_area);
    }
}

/// Build the input line with a visible cursor
fn build_input_with_cursor<'a>(content: &str, cursor_pos: usize, theme: &Theme) -> Vec<Span<'a>> {
    let mut spans = Vec::new();

    if content.is_empty() {
        // Show cursor on empty content
        spans.push(Span::styled(
            " ",
            theme.search_input.add_modifier(Modifier::REVERSED),
        ));
    } else {
        // Split content at cursor position
        let (before, at_and_after) = if cursor_pos < content.len() {
            (&content[..cursor_pos], &content[cursor_pos..])
        } else {
            (content, "")
        };

        // Text before cursor
        if !before.is_empty() {
            spans.push(Span::styled(before.to_string(), theme.search_input));
        }

        // Cursor character (or space if at end)
        if let Some(cursor_char) = at_and_after.chars().next() {
            spans.push(Span::styled(
                cursor_char.to_string(),
                theme.search_input.add_modifier(Modifier::REVERSED),
            ));

            // Text after cursor
            let after: String = at_and_after.chars().skip(1).collect();
            if !after.is_empty() {
                spans.push(Span::styled(after, theme.search_input));
            }
        } else {
            // Cursor at end - show a space with cursor style
            spans.push(Span::styled(
                " ",
                theme.search_input.add_modifier(Modifier::REVERSED),
            ));
        }
    }

    spans
}
