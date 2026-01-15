use crate::ui::layout::HelpLayout;
use crate::ui::theme::Theme;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

/// Render the help screen
pub fn render_help_screen(frame: &mut Frame<'_>, area: Rect, theme: &Theme) {
    let layout = HelpLayout::fullscreen(area);

    // Clear background
    frame.render_widget(Clear, layout.outer);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.dialog_border)
        .title(Line::from(" Help - Keybindings ").style(theme.dialog_title))
        .style(ratatui::style::Style::default().bg(theme.dialog_bg));

    let help_text = build_help_text(theme);

    let para = Paragraph::new(help_text)
        .block(block)
        .wrap(Wrap { trim: false });

    frame.render_widget(para, layout.outer);
}

fn build_help_text(theme: &Theme) -> Vec<Line<'static>> {
    let title_style = theme.title;
    let key_style = theme.keybinding_key;

    let mut lines = Vec::new();

    // Header
    lines.push(Line::from(vec![
        Span::styled("retcon", title_style),
        Span::raw(" - Retroactive Continuity CLI"),
    ]));

    // Navigation section
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("Navigation", title_style)));
    lines.push(Line::from(""));
    lines.push(key_line("j / ↓", "Move cursor down (row)", key_style));
    lines.push(key_line("k / ↑", "Move cursor up (row)", key_style));
    lines.push(key_line("h / ←", "Move to previous column", key_style));
    lines.push(key_line("l / →", "Move to next column", key_style));
    lines.push(key_line("g / Home", "Go to first commit", key_style));
    lines.push(key_line("G / End", "Go to last commit", key_style));
    lines.push(key_line("Ctrl+d", "Page down", key_style));
    lines.push(key_line("Ctrl+u", "Page up", key_style));

    // Selection section (for batch editing)
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Selection (Batch Edit)",
        title_style,
    )));
    lines.push(Line::from(""));
    lines.push(key_line(
        "Space",
        "Toggle selection on current commit",
        key_style,
    ));
    lines.push(key_line("Ctrl+a", "Select all commits", key_style));
    lines.push(key_line("Ctrl+n", "Deselect all commits", key_style));
    lines.push(Line::from("  (Edit applies to all selected commits)"));

    // Visual Selection section
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Visual Selection (Vim-like)",
        title_style,
    )));
    lines.push(Line::from(""));
    lines.push(key_line("v", "Enter line-wise visual mode", key_style));
    lines.push(key_line(
        "Ctrl+v",
        "Enter block-wise visual mode",
        key_style,
    ));
    lines.push(Line::from("  In Visual Mode:"));
    lines.push(key_line("j/k", "Extend selection vertically", key_style));
    lines.push(key_line(
        "h/l",
        "Extend selection horizontally (block)",
        key_style,
    ));
    lines.push(key_line("g/G", "Extend to first/last commit", key_style));
    lines.push(key_line("e / Enter", "Edit selected commits", key_style));
    lines.push(key_line(
        "Space",
        "Toggle checkbox on visual range",
        key_style,
    ));
    lines.push(key_line("v / Ctrl+v", "Switch mode or exit", key_style));
    lines.push(key_line("Esc", "Cancel visual selection", key_style));

    // Editing section
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("Inline Editing", title_style)));
    lines.push(Line::from(""));
    lines.push(key_line(
        "e / Enter",
        "Start editing current cell",
        key_style,
    ));
    lines.push(key_line("Tab", "Move to next column", key_style));
    lines.push(key_line("Shift+Tab", "Move to previous column", key_style));
    lines.push(Line::from("  (Changes apply to selected commits if any)"));

    // In Edit Mode section
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("In Edit Mode", title_style)));
    lines.push(Line::from(""));
    lines.push(key_line("Enter", "Confirm and save edit", key_style));
    lines.push(key_line("Esc", "Cancel edit", key_style));
    lines.push(key_line("Tab", "Save and edit next column", key_style));
    lines.push(key_line(
        "Shift+Tab",
        "Save and edit previous column",
        key_style,
    ));
    lines.push(key_line("Backspace", "Delete character", key_style));
    lines.push(key_line("Alt+Bksp", "Delete word backward", key_style));
    lines.push(key_line("Alt+←/→", "Move by word", key_style));
    lines.push(key_line(
        "Ctrl+U/K",
        "Delete to start/end of line",
        key_style,
    ));
    lines.push(key_line("Ctrl+A/E", "Move to start/end of line", key_style));

    // Search section
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("Search/Filter", title_style)));
    lines.push(Line::from(""));
    lines.push(key_line("/", "Open search bar", key_style));
    lines.push(key_line("Enter", "Apply filter", key_style));
    lines.push(key_line("Esc", "Clear filter", key_style));

    // Undo/Redo section
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("Undo/Redo", title_style)));
    lines.push(Line::from(""));
    lines.push(key_line("u", "Undo last change", key_style));
    lines.push(key_line("Ctrl+r", "Redo", key_style));

    // Actions section
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("Actions", title_style)));
    lines.push(Line::from(""));
    lines.push(key_line(
        "w",
        "Write/apply changes (rewrite history)",
        key_style,
    ));
    lines.push(key_line("r", "Reset/discard all changes", key_style));

    // General section
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("General", title_style)));
    lines.push(Line::from(""));
    lines.push(key_line("?", "Show this help", key_style));
    lines.push(key_line(
        "q",
        "Quit (prompts if unsaved changes)",
        key_style,
    ));

    // Footer
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::raw("Press "),
        Span::styled("q", key_style),
        Span::raw(" or "),
        Span::styled("Esc", key_style),
        Span::raw(" to close help"),
    ]));

    lines
}

fn key_line(
    key: &'static str,
    desc: &'static str,
    key_style: ratatui::style::Style,
) -> Line<'static> {
    Line::from(vec![
        Span::raw("  "),
        Span::styled(format!("{:12}", key), key_style),
        Span::raw(desc),
    ])
}
