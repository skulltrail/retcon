use crate::git::commit::EditableField;
use crate::ui::layout::EditorLayout;
use crate::ui::theme::Theme;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;
use tui_textarea::TextArea;

/// State for the field editor (used for popup editor - now deprecated in favor of inline)
#[allow(dead_code)]
pub struct EditorState<'a> {
    pub textarea: TextArea<'a>,
    pub field: EditableField,
    pub original_value: String,
    pub validation_error: Option<String>,
}

#[allow(dead_code)]
impl EditorState<'_> {
    /// Create a new editor for a field
    pub fn new(field: EditableField, initial_value: &str) -> Self {
        let lines: Vec<String> = if field.is_multiline() {
            initial_value.lines().map(String::from).collect()
        } else {
            vec![initial_value.to_string()]
        };

        let mut textarea = TextArea::new(lines);
        textarea.set_cursor_line_style(ratatui::style::Style::default());

        // Move cursor to end
        textarea.move_cursor(tui_textarea::CursorMove::End);

        Self {
            textarea,
            field,
            original_value: initial_value.to_string(),
            validation_error: None,
        }
    }

    /// Get the current value
    pub fn value(&self) -> String {
        self.textarea.lines().join("\n")
    }

    /// Check if value has changed
    #[allow(dead_code)]
    pub fn is_modified(&self) -> bool {
        self.value() != self.original_value
    }

    /// Set a validation error
    pub fn set_error(&mut self, error: impl Into<String>) {
        self.validation_error = Some(error.into());
    }

    /// Clear validation error
    pub fn clear_error(&mut self) {
        self.validation_error = None;
    }
}

/// Render the editor popup (deprecated - using inline editing now)
#[allow(dead_code)]
pub fn render_editor(
    frame: &mut Frame<'_>,
    area: Rect,
    cursor_y: u16,
    editor: &mut EditorState<'_>,
    theme: &Theme,
) {
    let layout = EditorLayout::near_cursor(area, cursor_y, editor.field.is_multiline());

    // Clear the area behind the popup
    frame.render_widget(Clear, layout.outer);

    // Outer block
    let title = format!(" Edit: {} ", editor.field.display_name());
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.dialog_border)
        .title(Line::from(title).style(theme.dialog_title))
        .style(ratatui::style::Style::default().bg(theme.dialog_bg));

    frame.render_widget(block, layout.outer);

    // Render textarea
    let textarea_area = Rect::new(
        layout.input.x,
        layout.input.y,
        layout.input.width,
        layout.input.height,
    );

    frame.render_widget(&editor.textarea, textarea_area);

    // Hint line
    let hint = if let Some(ref error) = editor.validation_error {
        Line::from(vec![Span::styled(error.clone(), theme.error)])
    } else {
        let hint_text = match editor.field {
            EditableField::AuthorDate | EditableField::CommitterDate => {
                "Format: YYYY-MM-DD HH:MM:SS [+/-]HHMM"
            }
            EditableField::AuthorEmail | EditableField::CommitterEmail => "Format: user@domain.com",
            EditableField::Message => "Enter to add line | Ctrl+Enter or Esc to finish",
            _ => "Enter to confirm | Esc to cancel",
        };
        Line::from(vec![Span::styled(hint_text, theme.keybinding)])
    };

    let hint_para = Paragraph::new(hint);
    frame.render_widget(hint_para, layout.hint);
}
