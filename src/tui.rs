use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, List, ListItem, ListState, Paragraph},
    Frame,
};
use std::time::Duration;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::store::BranchStore;
use crate::view_model::{BranchViewModel, ViewState};
use crate::{BCBranch, PrStatus};

/// Configuration for animation timing
#[derive(Debug, Clone, Copy)]
pub struct AnimationConfig {
    /// Milliseconds between each render/poll cycle
    pub poll_interval_ms: u64,
}

impl Default for AnimationConfig {
    fn default() -> Self {
        Self {
            poll_interval_ms: 100,
        }
    }
}

impl AnimationConfig {
    /// Slower animation - better for readability
    pub fn slow() -> Self {
        Self {
            poll_interval_ms: 250,
        }
    }
}

/// Maps PR status to display colors (with animation frame for shimmer)
fn get_status_color(status: PrStatus, animation_frame: u8) -> Color {
    match status {
        PrStatus::MERGED => Color::Green,  // Safe to delete
        PrStatus::OPEN => Color::Yellow,   // Caution
        PrStatus::NONE => Color::White,    // Default
        PrStatus::LOADING => {
            // Shimmer effect: cycle through grays
            match animation_frame % 4 {
                0 => Color::Gray,
                1 => Color::DarkGray,
                2 => Color::Gray,
                _ => Color::White,
            }
        }
    }
}

/// Formats PR status for display in the TUI (with animation frame for loading dots)
fn format_status_for_display(status: PrStatus, animation_frame: u8) -> String {
    match status {
        PrStatus::OPEN => "OPEN".to_string(),
        PrStatus::MERGED => "MERGED ✓".to_string(),
        PrStatus::NONE => "No PR".to_string(),
        PrStatus::LOADING => {
            // Animate dots: Loading -> Loading. -> Loading.. -> Loading...
            let dots = ".".repeat((animation_frame % 4) as usize);
            format!("LOADING{}", dots)
        }
    }
}

/// App structure holds the application state
struct App<T: BranchStore> {
    view_state: ViewState,
    list_state: ListState,
    view_model: BranchViewModel<T>,
    animation_frame: u8,
    animation_config: AnimationConfig,
    update_rx: UnboundedReceiver<BCBranch>,
}

impl<T: BranchStore> App<T> {
    fn new(
        store: T,
        initial_branches: Vec<BCBranch>,
        update_rx: UnboundedReceiver<BCBranch>,
        animation_config: AnimationConfig,
    ) -> Self {
        let view_model = BranchViewModel::new(store);
        let view_state = ViewState::new(initial_branches);
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        Self {
            view_state,
            list_state,
            view_model,
            animation_frame: 0,
            animation_config,
            update_rx,
        }
    }

    /// Check for updates from background task and apply them (streaming - one branch at a time)
    fn check_for_updates(&mut self) {
        while let Ok(updated_branch) = self.update_rx.try_recv() {
            self.view_model
                .update_branch(&mut self.view_state, updated_branch);
        }
    }

    fn handle_event(&mut self, event: Event) -> bool {
        if let Event::Key(key) = event {
            if key.kind != KeyEventKind::Press {
                return false;
            }

            match key.code {
                KeyCode::Char('q') => return true,
                KeyCode::Up => {
                    self.view_model.move_up(&mut self.view_state);
                    self.list_state.select(Some(self.view_state.selected_index));
                }
                KeyCode::Down => {
                    self.view_model.move_down(&mut self.view_state);
                    self.list_state.select(Some(self.view_state.selected_index));
                }
                KeyCode::Char(' ') => {
                    // Toggle selection
                    self.view_model.toggle_selection(&mut self.view_state);
                }
                KeyCode::Char('d') => {
                    // Delete selected branches
                    if !self.view_state.selected_branches.is_empty() {
                        self.view_model
                            .delete_selected_branches(&mut self.view_state);
                        self.list_state.select(Some(self.view_state.selected_index));
                    }
                }
                _ => {}
            }
        }
        false
    }
}

/// Creates a ListItem for a branch with multi-line content
fn create_branch_list_item(
    branch: &BCBranch,
    is_selected_for_deletion: bool,
    animation_frame: u8,
) -> ListItem<'_> {
    let color = get_status_color(branch.pr_status, animation_frame);
    let mut lines = vec![];

    // Branch name line with selection checkbox
    let checkbox = if is_selected_for_deletion {
        "[x] "
    } else {
        "[ ] "
    };
    lines.push(Line::from(vec![
        Span::styled(
            checkbox,
            Style::default().fg(if is_selected_for_deletion {
                Color::Red
            } else {
                Color::Gray
            }),
        ),
        Span::styled(
            branch.name.clone(),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
    ]));

    // PR info line if available (not shown for LOADING status)
    if branch.pr_status != PrStatus::LOADING {
        if let (Some(pr_number), Some(pr_title)) = (branch.pr_number, &branch.pr_title) {
            lines.push(Line::from(vec![Span::styled(
                format!("    └─ PR #{}: {}", pr_number, pr_title),
                Style::default().fg(Color::Gray),
            )]));
        }
    }

    // Status line
    lines.push(Line::from(vec![Span::styled(
        format!(
            "    └─ Status: {}",
            format_status_for_display(branch.pr_status, animation_frame)
        ),
        Style::default().fg(color),
    )]));

    ListItem::new(lines)
}

/// Renders the application UI
fn render<T: BranchStore>(frame: &mut Frame, app: &mut App<T>) {
    let [header_area, list_area, footer_area] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Fill(1),
        Constraint::Length(3),
    ])
    .areas(frame.area());

    // Render header
    let header = Paragraph::new("Branch Cleaner - Git Branch Manager")
        .block(Block::bordered())
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    frame.render_widget(header, header_area);

    // Render branch list
    let items: Vec<ListItem> = app
        .view_state
        .branches
        .iter()
        .map(|b| {
            let is_selected = app.view_state.selected_branches.contains(&b.name);
            create_branch_list_item(b, is_selected, app.animation_frame)
        })
        .collect();

    let list = List::new(items)
        .block(Block::bordered().title("Branches"))
        .highlight_style(
            Style::default()
                .bg(Color::Cyan)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    frame.render_stateful_widget(list, list_area, &mut app.list_state);

    // Render footer
    let selected_count = app.view_state.selected_branches.len();
    let delete_msg = if selected_count > 0 {
        format!("Selected: {} | Press 'd' to delete", selected_count)
    } else {
        "No branches selected".to_string()
    };

    let footer_lines = vec![
        Line::from(Span::styled(
            "Navigation: ↑↓ arrows | Space: select | d: delete | q: quit",
            Style::default().fg(Color::Gray),
        )),
        Line::from(Span::styled(
            delete_msg,
            Style::default().fg(if selected_count > 0 {
                Color::Yellow
            } else {
                Color::Gray
            }),
        )),
        Line::from(Span::styled(
            "Green = Safe to delete (merged) | Yellow = Active PR | White = No PR",
            Style::default().fg(Color::Gray),
        )),
    ];
    let footer = Paragraph::new(footer_lines);
    frame.render_widget(footer, footer_area);
}

/// Entry point to run the TUI application
pub fn run_branch_tui<T: BranchStore>(
    store: T,
    initial_branches: Vec<BCBranch>,
    update_rx: UnboundedReceiver<BCBranch>,
    animation_config: AnimationConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize terminal
    let mut terminal = ratatui::init();
    let mut app = App::new(store, initial_branches, update_rx, animation_config);

    // Main event loop
    loop {
        // Check for updates from background loader
        app.check_for_updates();

        // Increment animation frame for shimmer effect
        app.animation_frame = app.animation_frame.wrapping_add(1);

        terminal.draw(|frame| render(frame, &mut app))?;

        if event::poll(Duration::from_millis(app.animation_config.poll_interval_ms))? {
            if app.handle_event(event::read()?) {
                break;
            }
        }
    }

    // Restore terminal state
    ratatui::restore();
    Ok(())
}
