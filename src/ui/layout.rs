use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Layout areas for the main UI
#[derive(Debug, Clone)]
pub struct AppLayout {
    pub title: Rect,
    pub search: Option<Rect>,
    pub table: Rect,
    pub detail: Rect,
    pub status: Rect,
}

/// Minimum terminal dimensions for usable display
pub const MIN_WIDTH: u16 = 80;
pub const MIN_HEIGHT: u16 = 20;

impl AppLayout {
    /// Check if terminal is too small
    #[must_use]
    pub fn is_too_small(area: Rect) -> bool {
        area.width < MIN_WIDTH || area.height < MIN_HEIGHT
    }

    /// Calculate layout areas based on terminal size and whether search is active
    #[must_use]
    pub fn new(area: Rect, search_active: bool) -> Self {
        let mut constraints = vec![
            Constraint::Length(1), // Title bar
        ];

        if search_active {
            constraints.push(Constraint::Length(3)); // Search bar
        }

        // Calculate dynamic detail pane height based on terminal height
        // Use percentage-based sizing: detail pane gets ~30% of remaining space
        let fixed_height = 1 + if search_active { 3 } else { 0 } + 1; // title + search + status
        let available = area.height.saturating_sub(fixed_height);
        let detail_height = (available * 30 / 100).clamp(8, 15); // 30% but between 8-15 lines
        let table_min = available.saturating_sub(detail_height).max(5);

        // Main content split between table and detail pane
        constraints.push(Constraint::Min(table_min)); // Table (grows)
        constraints.push(Constraint::Length(detail_height)); // Detail pane (flexible)
        constraints.push(Constraint::Length(1)); // Status bar

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(area);

        let mut idx = 0;

        let title = chunks[idx];
        idx += 1;

        let search = if search_active {
            let s = chunks[idx];
            idx += 1;
            Some(s)
        } else {
            None
        };

        let table = chunks[idx];
        idx += 1;

        let detail = chunks[idx];
        idx += 1;

        let status = chunks[idx];

        Self {
            title,
            search,
            table,
            detail,
            status,
        }
    }

    /// Get the height of the table area (for scroll calculations)
    #[must_use]
    pub fn table_height(&self) -> usize {
        // Account for borders (2) and header (1)
        self.table.height.saturating_sub(3) as usize
    }
}

/// Layout for the confirmation dialog
#[derive(Debug, Clone)]
pub struct DialogLayout {
    pub outer: Rect,
    #[allow(dead_code)]
    pub title: Rect,
    pub content: Rect,
    pub buttons: Rect,
}

impl DialogLayout {
    /// Create a centered dialog
    #[must_use]
    pub fn centered(area: Rect, width: u16, height: u16) -> Self {
        let x = area.x + (area.width.saturating_sub(width)) / 2;
        let y = area.y + (area.height.saturating_sub(height)) / 2;

        let outer = Rect::new(x, y, width.min(area.width), height.min(area.height));

        let inner = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(1), // Title
                Constraint::Min(3),    // Content
                Constraint::Length(3), // Buttons
            ])
            .split(outer);

        Self {
            outer,
            title: inner[0],
            content: inner[1],
            buttons: inner[2],
        }
    }
}

/// Layout for the editor popup
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct EditorLayout {
    pub outer: Rect,
    pub title: Rect,
    pub input: Rect,
    pub hint: Rect,
}

#[allow(dead_code)]
impl EditorLayout {
    /// Create an editor popup positioned near the cursor
    #[must_use]
    pub fn near_cursor(area: Rect, cursor_y: u16, multiline: bool) -> Self {
        let height = if multiline { 12 } else { 5 };
        let width = (area.width * 3 / 4).max(60).min(area.width - 4);

        // Position below cursor if there's room, otherwise above
        let y = if cursor_y + height + 2 < area.height {
            cursor_y + 2
        } else {
            cursor_y.saturating_sub(height + 1)
        };

        let x = (area.width.saturating_sub(width)) / 2;

        let outer = Rect::new(x, y, width, height);

        let inner = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(1), // Title/field name
                Constraint::Min(1),    // Input area
                Constraint::Length(1), // Hint line
            ])
            .split(outer);

        Self {
            outer,
            title: inner[0],
            input: inner[1],
            hint: inner[2],
        }
    }
}

/// Layout for the help screen
pub struct HelpLayout {
    pub outer: Rect,
}

impl HelpLayout {
    #[must_use]
    pub fn fullscreen(area: Rect) -> Self {
        let margin = 4;
        let outer = Rect::new(
            margin,
            margin / 2,
            area.width.saturating_sub(margin * 2),
            area.height.saturating_sub(margin),
        );
        Self { outer }
    }
}
