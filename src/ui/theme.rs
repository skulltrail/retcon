use ratatui::style::{Color, Modifier, Style};

/// Color theme for the application using terminal colors
/// These colors adapt to the user's terminal theme (dark or light)
///
/// ## Visual Hierarchy (priority order):
/// 1. **Active cell** (cursor position): REVERSED - always clearly visible
/// 2. **Visual selection**: DIM background - stable, doesn't flicker with cursor
/// 3. **Modified values**: Yellow foreground - shows pending changes
/// 4. **Base field colors**: Semantic colors (hash=magenta, author=cyan, etc.)
///
/// Key principle: Cursor highlights ONE cell only, not entire rows.
#[derive(Debug, Clone)]
pub struct Theme {
    // Base colors - using Reset to inherit terminal defaults
    #[allow(dead_code)]
    pub bg: Color,
    #[allow(dead_code)]
    pub fg: Color,
    pub border: Color,
    pub border_focused: Color,

    // Table styles
    pub table_header: Style,
    #[allow(dead_code)]
    pub table_row: Style,
    #[allow(dead_code)]
    pub table_row_alt: Style,

    // Specific field styles
    pub hash: Style,
    pub author: Style,
    pub date: Style,
    pub message: Style,
    pub modified_value: Style,

    // Cell state styles
    pub cell_cursor: Style,        // Active cell (cursor position)
    pub cell_visual: Style,        // Cell in visual selection
    pub cell_visual_cursor: Style, // Cursor cell within visual selection

    // UI elements
    pub title: Style,
    pub title_dirty: Style,
    pub status_bar: Style,
    pub status_bar_mode: Style,
    pub keybinding: Style,
    pub keybinding_key: Style,

    // Feedback
    pub error: Style,
    pub warning: Style,
    pub success: Style,
    pub info: Style,

    // Dialog
    pub dialog_bg: Color,
    pub dialog_border: Style,
    pub dialog_title: Style,
    pub dialog_button: Style,
    pub dialog_button_selected: Style,

    // Search
    pub search_prompt: Style,
    pub search_input: Style,
    #[allow(dead_code)]
    pub search_match: Style,

    // Selection checkbox
    pub checkbox_checked: Style,
    pub checkbox_unchecked: Style,

    // Deletion marker
    pub deleted: Style,
}

impl Default for Theme {
    fn default() -> Self {
        // Use terminal's native colors - these adapt to dark/light terminal themes
        // Color::Reset inherits the terminal's default foreground/background
        // Standard ANSI colors are remapped by terminal themes for visibility

        Self {
            // Base - inherit terminal defaults
            bg: Color::Reset,
            fg: Color::Reset,
            border: Color::DarkGray,
            border_focused: Color::Cyan,

            // Table
            table_header: Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            table_row: Style::default(), // Inherit terminal default
            table_row_alt: Style::default().fg(Color::DarkGray),

            // Fields - using distinct ANSI colors
            hash: Style::default().fg(Color::Magenta),
            author: Style::default().fg(Color::Cyan),
            date: Style::default().fg(Color::Blue),
            message: Style::default(), // Inherit terminal default
            modified_value: Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),

            // Cell states - clean, non-conflicting
            cell_cursor: Style::default().add_modifier(Modifier::REVERSED),
            // Visual selection: use DIM modifier which inverts/dims text - more visible
            cell_visual: Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
            cell_visual_cursor: Style::default()
                .bg(Color::Cyan)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),

            // UI
            title: Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            title_dirty: Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
            status_bar: Style::default().add_modifier(Modifier::REVERSED),
            status_bar_mode: Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD | Modifier::REVERSED),
            keybinding: Style::default().fg(Color::DarkGray),
            keybinding_key: Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),

            // Feedback
            error: Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            warning: Style::default().fg(Color::Yellow),
            success: Style::default().fg(Color::Green),
            info: Style::default().fg(Color::Cyan),

            // Dialog - use reversed for visibility
            dialog_bg: Color::Reset,
            dialog_border: Style::default().fg(Color::Cyan),
            dialog_title: Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
            dialog_button: Style::default().add_modifier(Modifier::DIM),
            dialog_button_selected: Style::default()
                .add_modifier(Modifier::BOLD | Modifier::REVERSED),

            // Search
            search_prompt: Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            search_input: Style::default(),
            search_match: Style::default().add_modifier(Modifier::REVERSED),

            // Checkbox - only the checkbox shows selection state
            checkbox_checked: Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
            checkbox_unchecked: Style::default().fg(Color::DarkGray),

            // Deletion marker - red and crossed out
            deleted: Style::default()
                .fg(Color::Red)
                .add_modifier(Modifier::BOLD | Modifier::CROSSED_OUT),
        }
    }
}

impl Theme {
    /// Get style for a field value that may be modified
    #[must_use]
    pub fn field_style(&self, is_modified: bool, base: Style) -> Style {
        if is_modified {
            self.modified_value
        } else {
            base
        }
    }
}
