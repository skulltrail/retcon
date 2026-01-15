use crate::state::AppState;
use crate::ui::theme::Theme;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

/// Render the title bar
pub fn render_title_bar(frame: &mut Frame<'_>, area: Rect, state: &AppState, theme: &Theme) {
    let mut spans = vec![
        Span::styled(" retcon ", theme.title),
        Span::raw("- Retroactive Continuity CLI"),
    ];

    // Add dirty indicator
    if state.is_dirty() {
        spans.push(Span::styled(" [modified]", theme.warning));
    }

    // Right-align branch name
    let left_width: usize = spans.iter().map(|s| s.content.len()).sum();
    let branch_text = format!("[{}] ", state.branch_name);
    let padding = area
        .width
        .saturating_sub(left_width as u16 + branch_text.len() as u16);

    if padding > 0 {
        spans.push(Span::raw(" ".repeat(padding as usize)));
    }
    spans.push(Span::styled(branch_text, theme.info));

    let line = Line::from(spans);
    let para = Paragraph::new(line).style(theme.title);

    frame.render_widget(para, area);
}
