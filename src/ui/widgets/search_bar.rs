use crate::ui::theme::Theme;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

/// Render the search bar
pub fn render_search_bar(
    frame: &mut Frame<'_>,
    area: Rect,
    query: &str,
    cursor_pos: usize,
    result_count: Option<usize>,
    theme: &Theme,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border_focused)
        .title(Line::from(" Search ").style(theme.title));

    // Build the search line with cursor
    let mut spans = vec![Span::styled("/", theme.search_prompt), Span::raw(" ")];

    // Add query text with cursor indication
    if query.is_empty() {
        spans.push(Span::styled("_", theme.search_input));
    } else {
        // Show query with cursor position
        let before = &query[..cursor_pos.min(query.len())];
        let cursor_char = query.chars().nth(cursor_pos).map(|c| c.to_string());
        let after = if cursor_pos < query.len() {
            &query[cursor_pos + 1..]
        } else {
            ""
        };

        spans.push(Span::styled(before.to_string(), theme.search_input));

        if let Some(c) = cursor_char {
            spans.push(Span::styled(
                c,
                theme
                    .search_input
                    .bg(ratatui::style::Color::White)
                    .fg(ratatui::style::Color::Black),
            ));
        } else {
            spans.push(Span::styled(
                "_",
                theme
                    .search_input
                    .bg(ratatui::style::Color::White)
                    .fg(ratatui::style::Color::Black),
            ));
        }

        spans.push(Span::styled(after.to_string(), theme.search_input));
    }

    // Show result count
    if let Some(count) = result_count {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(format!("({} matches)", count), theme.info));
    }

    let line = Line::from(spans);
    let para = Paragraph::new(line).block(block);

    frame.render_widget(para, area);
}

/// State for search input
pub struct SearchState {
    pub query: String,
    pub cursor: usize,
}

impl SearchState {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            cursor: 0,
        }
    }

    pub fn from_query(query: &str) -> Self {
        Self {
            query: query.to_string(),
            cursor: query.len(),
        }
    }

    pub fn insert(&mut self, c: char) {
        self.query.insert(self.cursor, c);
        self.cursor += 1;
    }

    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.query.remove(self.cursor);
        }
    }

    pub fn delete(&mut self) {
        if self.cursor < self.query.len() {
            self.query.remove(self.cursor);
        }
    }

    pub fn move_left(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    pub fn move_right(&mut self) {
        if self.cursor < self.query.len() {
            self.cursor += 1;
        }
    }

    pub fn move_start(&mut self) {
        self.cursor = 0;
    }

    pub fn move_end(&mut self) {
        self.cursor = self.query.len();
    }

    /// Move cursor to previous word boundary
    pub fn move_word_left(&mut self) {
        if self.cursor == 0 {
            return;
        }
        // Skip any whitespace immediately before cursor
        let chars: Vec<char> = self.query.chars().collect();
        let mut pos = self.cursor;
        while pos > 0 && chars[pos - 1].is_whitespace() {
            pos -= 1;
        }
        // Skip word characters
        while pos > 0 && !chars[pos - 1].is_whitespace() {
            pos -= 1;
        }
        self.cursor = pos;
    }

    /// Move cursor to next word boundary
    pub fn move_word_right(&mut self) {
        let len = self.query.len();
        if self.cursor >= len {
            return;
        }
        let chars: Vec<char> = self.query.chars().collect();
        let mut pos = self.cursor;
        // Skip current word
        while pos < len && !chars[pos].is_whitespace() {
            pos += 1;
        }
        // Skip whitespace
        while pos < len && chars[pos].is_whitespace() {
            pos += 1;
        }
        self.cursor = pos;
    }

    /// Delete word backward (Alt+Backspace / Ctrl+W)
    pub fn delete_word_backward(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let start = self.cursor;
        self.move_word_left();
        // Remove characters from new cursor position to old position
        self.query.drain(self.cursor..start);
    }

    /// Delete word forward (Alt+Delete / Ctrl+D is often delete char, so we use Alt+D)
    #[allow(dead_code)]
    pub fn delete_word_forward(&mut self) {
        let len = self.query.len();
        if self.cursor >= len {
            return;
        }
        let start = self.cursor;
        self.move_word_right();
        let end = self.cursor;
        self.cursor = start;
        self.query.drain(start..end);
    }

    /// Delete to start of line (Cmd+Backspace on Mac, Ctrl+U in terminals)
    pub fn delete_to_start(&mut self) {
        if self.cursor > 0 {
            self.query.drain(0..self.cursor);
            self.cursor = 0;
        }
    }

    /// Delete to end of line (Ctrl+K)
    pub fn delete_to_end(&mut self) {
        self.query.truncate(self.cursor);
    }

    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.query.clear();
        self.cursor = 0;
    }
}

impl Default for SearchState {
    fn default() -> Self {
        Self::new()
    }
}
