use crate::git::commit::{CommitData, CommitModifications};
use crate::state::AppState;
use crate::ui::theme::Theme;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
};
use ratatui::Frame;

/// Render the commit detail pane
pub fn render_detail_pane(frame: &mut Frame<'_>, area: Rect, state: &AppState, theme: &Theme) {
    let commit = match state.cursor_commit() {
        Some(c) => c,
        None => {
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(theme.border)
                .title(Line::from(" Commit Details ").style(theme.title));
            let para = Paragraph::new("No commit selected").block(block);
            frame.render_widget(para, area);
            return;
        }
    };

    let mods = state.modifications.get(&commit.id);
    let lines = build_detail_lines(commit, mods, theme);

    // Calculate content height for scrollbar
    let content_height = lines.len();
    let visible_height = area.height.saturating_sub(2) as usize; // Account for borders
    let needs_scroll = content_height > visible_height;

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border)
        .title(Line::from(" Commit Details ").style(theme.title));

    let para = Paragraph::new(lines.clone())
        .block(block)
        .scroll((state.detail_scroll as u16, 0));

    frame.render_widget(para, area);

    // Render scrollbar if content overflows
    if needs_scroll {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"));

        let mut scrollbar_state =
            ScrollbarState::new(content_height.saturating_sub(visible_height))
                .position(state.detail_scroll);

        // Render scrollbar in the right border area
        let scrollbar_area = Rect::new(
            area.x + area.width - 1,
            area.y + 1,
            1,
            area.height.saturating_sub(2),
        );
        frame.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
    }
}

/// Build the lines for the detail pane
fn build_detail_lines<'a>(
    commit: &CommitData,
    mods: Option<&CommitModifications>,
    theme: &Theme,
) -> Vec<Line<'a>> {
    let mut lines = Vec::new();

    // Hash (never modified)
    lines.push(Line::from(vec![
        Span::styled("Hash:      ", theme.info),
        Span::styled(commit.id.0.to_string(), theme.hash),
    ]));

    // Author
    let author_name_mod = mods.and_then(|m| m.author_name.as_ref()).is_some();
    let author_email_mod = mods.and_then(|m| m.author_email.as_ref()).is_some();
    let author_name = mods
        .and_then(|m| m.author_name.clone())
        .unwrap_or_else(|| commit.author.name.clone());
    let author_email = mods
        .and_then(|m| m.author_email.clone())
        .unwrap_or_else(|| commit.author.email.clone());

    lines.push(Line::from(vec![
        Span::styled("Author:    ", theme.info),
        Span::styled(
            author_name,
            theme.field_style(author_name_mod, theme.author),
        ),
        Span::raw(" <"),
        Span::styled(
            author_email,
            theme.field_style(author_email_mod, theme.author),
        ),
        Span::raw(">"),
    ]));

    // Author date
    let author_date_mod = mods.and_then(|m| m.author_date).is_some();
    let author_date = mods
        .and_then(|m| m.author_date)
        .map(|d| d.format("%Y-%m-%d %H:%M:%S %z").to_string())
        .unwrap_or_else(|| commit.format_author_date_full());

    lines.push(Line::from(vec![
        Span::styled("A. Date:   ", theme.info),
        Span::styled(author_date, theme.field_style(author_date_mod, theme.date)),
    ]));

    // Committer
    let committer_name_mod = mods.and_then(|m| m.committer_name.as_ref()).is_some();
    let committer_email_mod = mods.and_then(|m| m.committer_email.as_ref()).is_some();
    let committer_name = mods
        .and_then(|m| m.committer_name.clone())
        .unwrap_or_else(|| commit.committer.name.clone());
    let committer_email = mods
        .and_then(|m| m.committer_email.clone())
        .unwrap_or_else(|| commit.committer.email.clone());

    lines.push(Line::from(vec![
        Span::styled("Committer: ", theme.info),
        Span::styled(
            committer_name,
            theme.field_style(committer_name_mod, theme.author),
        ),
        Span::raw(" <"),
        Span::styled(
            committer_email,
            theme.field_style(committer_email_mod, theme.author),
        ),
        Span::raw(">"),
    ]));

    // Committer date
    let committer_date_mod = mods.and_then(|m| m.committer_date).is_some();
    let committer_date = mods
        .and_then(|m| m.committer_date)
        .map(|d| d.format("%Y-%m-%d %H:%M:%S %z").to_string())
        .unwrap_or_else(|| commit.format_committer_date_full());

    lines.push(Line::from(vec![
        Span::styled("C. Date:   ", theme.info),
        Span::styled(
            committer_date,
            theme.field_style(committer_date_mod, theme.date),
        ),
    ]));

    // Parent info
    if !commit.parent_ids.is_empty() {
        let parent_str = commit
            .parent_ids
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(Line::from(vec![
            Span::styled("Parents:   ", theme.info),
            Span::raw(parent_str),
        ]));
    }

    // Merge indicator
    if commit.is_merge {
        lines.push(Line::from(vec![
            Span::styled("           ", theme.info),
            Span::styled("(merge commit)", theme.warning),
        ]));
    }

    // Empty line before message
    lines.push(Line::from(""));

    // Commit message section
    let message_modified = mods.and_then(|m| m.message.as_ref()).is_some();
    let message = mods
        .and_then(|m| m.message.clone())
        .unwrap_or_else(|| commit.message.clone());

    lines.push(Line::from(vec![Span::styled("Message:", theme.info)]));

    // Add each line of the message with proper styling
    let message_style = theme.field_style(message_modified, theme.message);
    for line in message.lines() {
        lines.push(Line::from(vec![
            Span::styled("  ", theme.info), // Indent
            Span::styled(line.to_string(), message_style),
        ]));
    }

    lines
}
