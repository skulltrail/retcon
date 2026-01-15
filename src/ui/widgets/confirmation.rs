use crate::git::rewrite::generate_change_summary;
use crate::state::{AppState, ConfirmAction};
use crate::ui::layout::DialogLayout;
use crate::ui::theme::Theme;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

/// State for confirmation dialog
pub struct ConfirmDialogState {
    pub selected_button: usize, // 0 = Yes, 1 = No
}

impl Default for ConfirmDialogState {
    fn default() -> Self {
        Self { selected_button: 1 } // Default to "No" for safety
    }
}

impl ConfirmDialogState {
    #[allow(dead_code)]
    pub fn select_yes(&mut self) {
        self.selected_button = 0;
    }

    #[allow(dead_code)]
    pub fn select_no(&mut self) {
        self.selected_button = 1;
    }

    pub fn toggle(&mut self) {
        self.selected_button = (self.selected_button + 1) % 2;
    }

    pub fn is_yes_selected(&self) -> bool {
        self.selected_button == 0
    }
}

/// Render the confirmation dialog
pub fn render_confirmation_dialog(
    frame: &mut Frame<'_>,
    area: Rect,
    action: &ConfirmAction,
    state: &AppState,
    dialog_state: &ConfirmDialogState,
    theme: &Theme,
) {
    let (title, content_lines, warning) = build_dialog_content(action, state);

    // Calculate dialog size based on content
    let width = 60u16.min(area.width - 4);
    let height = (content_lines.len() as u16 + 8).min(area.height - 4);

    let layout = DialogLayout::centered(area, width, height);

    // Clear background
    frame.render_widget(Clear, layout.outer);

    // Dialog block
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.dialog_border)
        .title(Line::from(format!(" {} ", title)).style(theme.dialog_title))
        .style(ratatui::style::Style::default().bg(theme.dialog_bg));

    frame.render_widget(block, layout.outer);

    // Content
    let mut lines: Vec<Line<'_>> = content_lines
        .iter()
        .map(|s| Line::from(s.as_str()))
        .collect();

    // Add warning if present
    if let Some(warn) = warning {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("Warning: ", theme.warning),
            Span::styled(warn, theme.warning),
        ]));
    }

    let content = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(content, layout.content);

    // Buttons
    let yes_style = if dialog_state.is_yes_selected() {
        theme.dialog_button_selected
    } else {
        theme.dialog_button
    };
    let no_style = if !dialog_state.is_yes_selected() {
        theme.dialog_button_selected
    } else {
        theme.dialog_button
    };

    let buttons = Line::from(vec![
        Span::raw("        "),
        Span::styled(" [Y]es ", yes_style),
        Span::raw("   "),
        Span::styled(" [N]o ", no_style),
    ]);

    let buttons_para = Paragraph::new(buttons);
    frame.render_widget(buttons_para, layout.buttons);
}

/// Build dialog content based on action type
fn build_dialog_content(
    action: &ConfirmAction,
    state: &AppState,
) -> (String, Vec<String>, Option<String>) {
    match action {
        ConfirmAction::ApplyChanges => {
            let title = "Apply Changes".to_string();
            let summary = generate_change_summary(
                &state.commits,
                &state.modifications,
                &state.original_order,
                &state.current_order,
            );

            let mut content = vec!["This will rewrite git history.".to_string(), "".to_string()];
            content.extend(summary);

            let warning = if state.has_upstream {
                Some("Branch has upstream - will require force push!".to_string())
            } else {
                None
            };

            (title, content, warning)
        }

        ConfirmAction::DiscardChanges => {
            let title = "Discard Changes".to_string();
            let content = vec![
                format!("You have {} modified commit(s).", state.modified_count()),
                "".to_string(),
                "Are you sure you want to discard all changes?".to_string(),
            ];
            (title, content, None)
        }

        ConfirmAction::QuitWithChanges => {
            let title = "Quit with Changes".to_string();
            let content = vec![
                format!("You have {} unsaved change(s).", state.modified_count()),
                "".to_string(),
                "Are you sure you want to quit?".to_string(),
            ];
            (title, content, None)
        }
    }
}
