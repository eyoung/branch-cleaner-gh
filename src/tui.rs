use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, List, ListItem, ListState, Paragraph},
    Frame,
};
use std::time::Duration;

use crate::store::BranchStore;
use crate::{BCBranch, PrStatus};

/// ViewState represents the pure data state of the TUI
/// This is a simple data structure with no business logic
#[derive(Clone, Debug, PartialEq)]
pub struct ViewState {
    pub branches: Vec<BCBranch>,
    pub selected_index: usize,
    pub selected_branches: Vec<String>, // Names of branches marked for deletion
}

impl ViewState {
    /// Create a new ViewState with the given branches
    /// By default, selects all merged branches (safe to delete)
    pub fn new(branches: Vec<BCBranch>) -> Self {
        let selected_branches = branches
            .iter()
            .filter(|b| b.pr_status == PrStatus::MERGED)
            .map(|b| b.name.clone())
            .collect();

        Self {
            branches,
            selected_index: 0,
            selected_branches,
        }
    }
}

/// BranchViewModel handles business logic and data operations
/// Kept separate from AppState for testability and clean architecture
#[derive(Debug, Clone)]
pub struct BranchViewModel<T: BranchStore> {
    store: T,
}

impl<T: BranchStore> BranchViewModel<T> {
    /// Creates a new ViewModel with dependency-injected store
    pub fn new(store: T) -> Self {
        Self { store }
    }

    /// Loads branches from the store and returns ViewState
    pub fn load_initial_state(&self) -> ViewState {
        ViewState::new(self.store.list_branches())
    }

    /// Moves selection up (mutates state in place - r3bl pattern)
    pub fn move_up(&self, state: &mut ViewState) {
        if state.selected_index > 0 {
            state.selected_index -= 1;
        }
    }

    /// Moves selection down (mutates state in place - r3bl pattern)
    pub fn move_down(&self, state: &mut ViewState) {
        let max_index = state.branches.len().saturating_sub(1);
        if state.selected_index < max_index {
            state.selected_index += 1;
        }
    }

    /// Returns branches that are safe to delete (merged PRs)
    pub fn safe_to_delete_branches<'a>(&self, state: &'a ViewState) -> Vec<&'a BCBranch> {
        state
            .branches
            .iter()
            .filter(|b| b.pr_status == PrStatus::MERGED)
            .collect()
    }

    /// Toggles selection of the current branch (add if not selected, remove if selected)
    pub fn toggle_selection(&self, state: &mut ViewState) {
        if state.selected_index >= state.branches.len() {
            return; // Safety: invalid index
        }

        let current_branch_name = &state.branches[state.selected_index].name;

        if let Some(pos) = state.selected_branches.iter().position(|name| name == current_branch_name) {
            // Already selected - remove it
            state.selected_branches.remove(pos);
        } else {
            // Not selected - add it
            state.selected_branches.push(current_branch_name.clone());
        }
    }

    /// Deletes selected branches from the store and updates the state
    pub fn delete_selected_branches(&mut self, state: &mut ViewState) {
        // 1. Delete branches from the store
        self.store.delete_branches(&state.selected_branches);

        // 2. Get updated branches from store
        let new_branches = self.store.list_branches();

        // 3. Select all merged branches in the new list (default selection)
        let new_selected = new_branches
            .iter()
            .filter(|b| b.pr_status == PrStatus::MERGED)
            .map(|b| b.name.clone())
            .collect();

        // 4. Update state with new branches and selection
        state.branches = new_branches;
        state.selected_branches = new_selected;
        state.selected_index = 0; // Reset to beginning after deletion
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

/// App structure holds the application state
struct App<T: BranchStore> {
    view_state: ViewState,
    list_state: ListState,
    view_model: BranchViewModel<T>,
}

impl<T: BranchStore> App<T> {
    fn new(store: T) -> Self {
        let view_model = BranchViewModel::new(store);
        let view_state = view_model.load_initial_state();
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        Self {
            view_state,
            list_state,
            view_model,
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
                        self.view_model.delete_selected_branches(&mut self.view_state);
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
fn create_branch_list_item(branch: &BCBranch, is_selected_for_deletion: bool) -> ListItem<'_> {
    let color = get_status_color(branch.pr_status);
    let mut lines = vec![];

    // Branch name line with selection checkbox
    let checkbox = if is_selected_for_deletion { "[x] " } else { "[ ] " };
    lines.push(Line::from(vec![
        Span::styled(
            checkbox,
            Style::default().fg(if is_selected_for_deletion { Color::Red } else { Color::Gray }),
        ),
        Span::styled(
            branch.name.clone(),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
    ]));

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
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
    frame.render_widget(header, header_area);

    // Render branch list
    let items: Vec<ListItem> = app
        .view_state
        .branches
        .iter()
        .map(|b| {
            let is_selected = app.view_state.selected_branches.contains(&b.name);
            create_branch_list_item(b, is_selected)
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
            Style::default().fg(if selected_count > 0 { Color::Yellow } else { Color::Gray }),
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
pub fn run_branch_tui<T: BranchStore>(store: T) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize terminal
    let mut terminal = ratatui::init();
    let mut app = App::new(store);

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
        use crate::InMemoryBranchStore;

        #[test]
        fn can_create_view_state_with_branches() {
            let branches = create_test_branches();
            let state = ViewState::new(branches.clone());

            let expected_state = ViewState {
                branches: branches.clone(),
                selected_index: 0,
                selected_branches: vec!["feature-2".to_owned()], // Only merged branch
            };

            assert_eq!(state, expected_state);
        }

        #[test]
        fn viewmodel_loads_branches_from_store() {
            // Arrange: Create a store with test branches
            let test_branches = create_test_branches();
            let store = InMemoryBranchStore::new(test_branches.clone());
            let view_model = BranchViewModel::new(store);

            // Act: Load initial state from the viewmodel
            let view_state = view_model.load_initial_state();

            // Assert: ViewState contains branches from the store
            let expected_state = ViewState {
                branches: test_branches,
                selected_index: 0,
                selected_branches: vec!["feature-2".to_owned()], // Only merged branch
            };

            assert_eq!(view_state, expected_state);
        }

        #[test]
        fn returns_only_merged_branches_as_safe_to_delete() {
            let branches = create_test_branches();
            let state = ViewState::new(branches.clone());
            let store = InMemoryBranchStore::new(branches.clone());
            let view_model = BranchViewModel::new(store);

            let expected = vec![&branches[2]]; // feature-2 is the only merged branch

            assert_eq!(view_model.safe_to_delete_branches(&state), expected);
        }

        #[test]
        fn move_down_increments_selected_index() {
            // Arrange
            let branches = create_test_branches();
            let mut state = ViewState::new(branches.clone());
            let store = InMemoryBranchStore::new(branches.clone());
            let view_model = BranchViewModel::new(store);

            // Act: Mutate state in place (r3bl pattern)
            view_model.move_down(&mut state);

            // Assert: Check entire state
            let expected_state = ViewState {
                branches: branches.clone(),
                selected_index: 1,
                selected_branches: vec!["feature-2".to_owned()], // Selection unchanged
            };

            assert_eq!(state, expected_state);
        }

        #[test]
        fn move_up_decrements_selected_index() {
            // Arrange
            let branches = create_test_branches();
            let mut state = ViewState::new(branches.clone());
            let store = InMemoryBranchStore::new(branches.clone());
            let view_model = BranchViewModel::new(store);

            // Act: Move down twice, then up once (mutating state)
            view_model.move_down(&mut state);
            view_model.move_down(&mut state);
            view_model.move_up(&mut state);

            // Assert: Check entire state
            let expected_state = ViewState {
                branches: branches.clone(),
                selected_index: 1,
                selected_branches: vec!["feature-2".to_owned()], // Selection unchanged
            };

            assert_eq!(state, expected_state);
        }

        #[test]
        fn toggle_selection_adds_unselected_branch() {
            // Arrange: State at index 0 (main branch, not pre-selected)
            let branches = create_test_branches();
            let mut state = ViewState::new(branches.clone());
            let store = InMemoryBranchStore::new(branches.clone());
            let view_model = BranchViewModel::new(store);

            // Act: Toggle selection of current branch (main at index 0)
            view_model.toggle_selection(&mut state);

            // Assert: main is now selected
            let expected_state = ViewState {
                branches: branches.clone(),
                selected_index: 0,
                selected_branches: vec!["feature-2".to_owned(), "main".to_owned()],
            };

            assert_eq!(state, expected_state);
        }

        #[test]
        fn toggle_selection_removes_selected_branch() {
            // Arrange: Move to feature-2 (index 2, already selected)
            let branches = create_test_branches();
            let mut state = ViewState::new(branches.clone());
            let store = InMemoryBranchStore::new(branches.clone());
            let view_model = BranchViewModel::new(store);
            view_model.move_down(&mut state);
            view_model.move_down(&mut state);

            // Act: Toggle selection of current branch (feature-2)
            view_model.toggle_selection(&mut state);

            // Assert: feature-2 is now unselected
            let expected_state = ViewState {
                branches: branches.clone(),
                selected_index: 2,
                selected_branches: vec![], // Empty - feature-2 removed
            };

            assert_eq!(state, expected_state);
        }

        #[test]
        fn delete_selected_branches_removes_them_and_reloads_state() {
            // Arrange: State with feature-2 selected (merged)
            let branches = create_test_branches();
            let mut state = ViewState::new(branches.clone());
            let store = InMemoryBranchStore::new(branches.clone());
            let mut view_model = BranchViewModel::new(store);

            // Act: Delete selected branches
            view_model.delete_selected_branches(&mut state);

            // Assert: feature-2 is deleted, state reloaded with remaining branches
            let expected_branches = vec![
                BCBranch::new("main", PrStatus::NONE),
                BCBranch::with_pr("feature-1", PrStatus::OPEN, 1, "Feature 1"),
            ];

            let expected_state = ViewState {
                branches: expected_branches,
                selected_index: 0, // Reset to 0
                selected_branches: vec![], // No merged branches remain
            };

            assert_eq!(state, expected_state);
        }
    }
}
