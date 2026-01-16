#![allow(clippy::cast_possible_truncation)]

use crate::state::{AppMode, AppState, VisualType};
use crate::ui::theme::Theme;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

/// Render the status bar at the bottom of the screen
pub fn render_status_bar(frame: &mut Frame<'_>, area: Rect, state: &AppState, theme: &Theme) {
    let mut spans = Vec::new();

    // Mode indicator
    let mode_str = match &state.mode {
        AppMode::Normal => " NORMAL ",
        AppMode::Visual { visual_type, .. } => match visual_type {
            VisualType::Line => " V-LINE ",
            VisualType::Block => " V-BLOCK ",
        },
        AppMode::Editing { .. } => " EDIT ",
        AppMode::Search => " SEARCH ",
        AppMode::Reorder => " REORDER ",
        AppMode::Confirming(_) => " CONFIRM ",
        AppMode::Help => " HELP ",
        AppMode::Quitting => " QUIT? ",
    };
    spans.push(Span::styled(mode_str, theme.status_bar_mode));
    spans.push(Span::raw(" "));

    // Branch name
    spans.push(Span::styled(format!("[{}]", state.branch_name), theme.info));
    spans.push(Span::raw(" "));

    // Error/success message or keybindings
    if let Some(ref err) = state.error_message {
        spans.push(Span::styled(err.clone(), theme.error));
    } else if let Some(ref msg) = state.success_message {
        spans.push(Span::styled(msg.clone(), theme.success));
    } else {
        // Show context-sensitive keybindings
        let keybindings = get_keybindings(&state.mode);
        for (key, desc) in keybindings {
            spans.push(Span::styled(key, theme.keybinding_key));
            spans.push(Span::styled(format!(" {desc} "), theme.keybinding));
        }
    }

    // Right side: dirty indicator and position
    let right_info = build_right_info(state, theme);

    // Calculate padding to right-align the info
    let left_width: usize = spans.iter().map(|s| s.content.len()).sum();
    let right_width: usize = right_info.iter().map(|s| s.content.len()).sum();
    let padding = area
        .width
        .saturating_sub(left_width as u16 + right_width as u16);

    if padding > 0 {
        spans.push(Span::raw(" ".repeat(padding as usize)));
    }
    spans.extend(right_info);

    let line = Line::from(spans);
    let para = Paragraph::new(line).style(theme.status_bar);

    frame.render_widget(para, area);
}

/// Get keybindings for the current mode
fn get_keybindings(mode: &AppMode) -> Vec<(&'static str, &'static str)> {
    match mode {
        AppMode::Normal => vec![
            ("h/j/k/l", "nav"),
            ("V", "visual"),
            ("^V", "block"),
            ("Space", "sel"),
            ("Enter", "edit"),
            ("/", "search"),
            ("w", "write"),
            ("?", "help"),
        ],
        AppMode::Visual { visual_type, .. } => match visual_type {
            VisualType::Line => vec![
                ("j/k", "extend"),
                ("e", "edit"),
                ("Space", "toggle"),
                ("^V", "block"),
                ("Esc", "cancel"),
            ],
            VisualType::Block => vec![
                ("h/j/k/l", "extend"),
                ("e", "edit"),
                ("Space", "toggle"),
                ("V", "line"),
                ("Esc", "cancel"),
            ],
        },
        AppMode::Editing { .. } => vec![("Enter", "save"), ("Esc", "cancel"), ("Tab", "next")],
        AppMode::Search => vec![("Enter", "filter"), ("Esc", "cancel")],
        AppMode::Reorder => vec![("Esc", "cancel")],
        AppMode::Confirming(_) => vec![("y", "yes"), ("n", "no"), ("Esc", "cancel")],
        AppMode::Help => vec![("q/Esc", "close")],
        AppMode::Quitting => vec![("y", "quit"), ("n", "stay")],
    }
}

/// Build the right-side info (position, dirty indicator)
fn build_right_info<'a>(state: &AppState, theme: &Theme) -> Vec<Span<'a>> {
    let mut spans = Vec::new();

    // Visual selection count
    if matches!(state.mode, AppMode::Visual { .. }) {
        let count = state.visual_selection_count();
        spans.push(Span::styled(
            format!("[{} row{}] ", count, if count == 1 { "" } else { "s" }),
            theme.info,
        ));
    }

    // Dirty indicator
    if state.is_dirty() {
        spans.push(Span::styled("[*] ", theme.warning));
    }

    // Upstream warning
    if state.has_upstream && state.is_dirty() {
        spans.push(Span::styled("(force push) ", theme.warning));
    }

    // Position
    let total = state.visible_commits().len();
    let pos = if total > 0 {
        format!("{}/{}", state.cursor + 1, total)
    } else {
        "0/0".to_string()
    };
    spans.push(Span::raw(pos));
    spans.push(Span::raw(" "));

    spans
}
