use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, List, ListItem, ListState, Paragraph},
    Frame,
};
use std::time::Duration;

use crate::{BCBranch, PrStatus};

/// ViewState represents the pure data state of the TUI
/// This is a simple data structure with no business logic
#[derive(Clone, Debug, PartialEq)]
pub struct ViewState {
    pub branches: Vec<BCBranch>,
    pub selected_index: usize,
}

impl ViewState {
    /// Create a new ViewState with the given branches
    pub fn new(branches: Vec<BCBranch>) -> Self {
        Self {
            branches,
            selected_index: 0,
        }
    }
}

/// BranchViewModel provides pure functions for managing branch view logic
/// All functions are stateless and transform ViewState immutably
pub struct BranchViewModel;

impl BranchViewModel {
    /// Returns branches that are safe to delete (merged PRs)
    pub fn safe_to_delete_branches(state: &ViewState) -> Vec<&BCBranch> {
        state
            .branches
            .iter()
            .filter(|b| b.pr_status == PrStatus::MERGED)
            .collect()
    }

    /// Returns a new ViewState with the selection moved up
    pub fn move_up(state: &ViewState) -> ViewState {
        let mut new_state = state.clone();
        if new_state.selected_index > 0 {
            new_state.selected_index -= 1;
        }
        new_state
    }

    /// Returns a new ViewState with the selection moved down
    pub fn move_down(state: &ViewState) -> ViewState {
        let mut new_state = state.clone();
        let max_index = new_state.branches.len().saturating_sub(1);
        if new_state.selected_index < max_index {
            new_state.selected_index += 1;
        }
        new_state
    }
}


/// Maps PR status to display colors
fn get_status_color(status: PrStatus) -> Color {
    match status {
        PrStatus::MERGED => Color::Green,   // Safe to delete
        PrStatus::OPEN => Color::Yellow,    // Caution
        PrStatus::NONE => Color::White,     // Default
    }
}

/// Formats PR status for display in the TUI
fn format_status_for_display(status: PrStatus) -> &'static str {
    match status {
        PrStatus::OPEN => "OPEN",
        PrStatus::MERGED => "MERGED ✓",
        PrStatus::NONE => "No PR",
    }
}

/// Creates fake branch data for testing the TUI
fn create_fake_branches() -> Vec<BCBranch> {
    vec![
        BCBranch::new("main", PrStatus::NONE),
        BCBranch::with_pr(
            "feature/add-tui",
            PrStatus::OPEN,
            42,
            "Add TUI interface with iocraft",
        ),
        BCBranch::with_pr(
            "old-feature-branch",
            PrStatus::MERGED,
            23,
            "Old feature implementation",
        ),
        BCBranch::new("experimental/refactor", PrStatus::NONE),
        BCBranch::with_pr(
            "bugfix/handle-errors",
            PrStatus::MERGED,
            15,
            "Fix error handling in repository",
        ),
        BCBranch::with_pr(
            "feature/github-integration",
            PrStatus::OPEN,
            50,
            "Integrate GitHub API for PR fetching",
        ),
        BCBranch::with_pr(
            "cleanup/remove-old-code",
            PrStatus::MERGED,
            31,
            "Remove deprecated functions and cleanup",
        ),
    ]
}

/// App structure holds the application state
struct App {
    view_state: ViewState,
    list_state: ListState,
}

impl App {
    fn new(branches: Vec<BCBranch>) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        Self {
            view_state: ViewState::new(branches),
            list_state,
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
                    self.view_state = BranchViewModel::move_up(&self.view_state);
                    self.list_state.select(Some(self.view_state.selected_index));
                }
                KeyCode::Down => {
                    self.view_state = BranchViewModel::move_down(&self.view_state);
                    self.list_state.select(Some(self.view_state.selected_index));
                }
                _ => {}
            }
        }
        false
    }
}

/// Creates a ListItem for a branch with multi-line content
fn create_branch_list_item(branch: &BCBranch) -> ListItem<'_> {
    let color = get_status_color(branch.pr_status);
    let mut lines = vec![];

    // Branch name line
    lines.push(Line::from(vec![Span::styled(
        branch.name.clone(),
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    )]));

    // PR info line if available
    if let (Some(pr_number), Some(pr_title)) = (branch.pr_number, &branch.pr_title) {
        lines.push(Line::from(vec![Span::styled(
            format!("    └─ PR #{}: {}", pr_number, pr_title),
            Style::default().fg(Color::Gray),
        )]));
    }

    // Status line
    lines.push(Line::from(vec![Span::styled(
        format!("    └─ Status: {}", format_status_for_display(branch.pr_status)),
        Style::default().fg(color),
    )]));

    ListItem::new(lines)
}

/// Renders the application UI
fn render(frame: &mut Frame, app: &mut App) {
    let [header_area, list_area, footer_area] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Fill(1),
        Constraint::Length(2),
    ])
    .areas(frame.area());

    // Render header
    let header = Paragraph::new("Branch Cleaner - Git Branch Manager")
        .block(Block::bordered())
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
    frame.render_widget(header, header_area);

    // Render branch list
    let items: Vec<ListItem> = app
        .view_state
        .branches
        .iter()
        .map(create_branch_list_item)
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
    let footer_lines = vec![
        Line::from(Span::styled(
            "Navigation: ↑↓ arrows | Quit: q",
            Style::default().fg(Color::Gray),
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
pub fn run_branch_tui() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize terminal
    let mut terminal = ratatui::init();
    let mut app = App::new(create_fake_branches());

    // Main event loop
    loop {
        terminal.draw(|frame| render(frame, &mut app))?;

        if event::poll(Duration::from_millis(100))? {
            if app.handle_event(event::read()?) {
                break;
            }
        }
    }

    // Restore terminal state
    ratatui::restore();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_branches() -> Vec<BCBranch> {
        vec![
            BCBranch::new("main", PrStatus::NONE),
            BCBranch::with_pr("feature-1", PrStatus::OPEN, 1, "Feature 1"),
            BCBranch::with_pr("feature-2", PrStatus::MERGED, 2, "Feature 2"),
        ]
    }

    mod view_model {
        use super::*;

        #[test]
        fn can_create_view_state_with_branches() {
            let branches = create_test_branches();
            let state = ViewState::new(branches.clone());

            let expected_state = ViewState {
                branches: branches.clone(),
                selected_index: 0,
            };

            assert_eq!(state, expected_state);
        }

        #[test]
        fn returns_only_merged_branches_as_safe_to_delete() {
            let branches = create_test_branches();
            let state = ViewState::new(branches.clone());

            let expected = vec![&branches[2]]; // feature-2 is the only merged branch

            assert_eq!(BranchViewModel::safe_to_delete_branches(&state), expected);
        }

        #[test]
        fn move_down_increments_selected_index() {
            let branches = create_test_branches();
            let state = ViewState::new(branches.clone());

            let new_state = BranchViewModel::move_down(&state);

            let expected_state = ViewState {
                branches: branches.clone(),
                selected_index: 1,
            };

            assert_eq!(new_state, expected_state);
        }

        #[test]
        fn move_up_decrements_selected_index() {
            let branches = create_test_branches();
            let state = ViewState::new(branches.clone());

            let state = BranchViewModel::move_down(&state);
            let state = BranchViewModel::move_down(&state);
            let state = BranchViewModel::move_up(&state);

            let expected_state = ViewState {
                branches: branches.clone(),
                selected_index: 1,
            };

            assert_eq!(state, expected_state);
        }
    }
}
