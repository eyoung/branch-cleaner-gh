use iocraft::prelude::*;

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

/// BranchViewModel manages the business logic for the branch viewer
pub struct BranchViewModel {
    state: ViewState,
}

impl BranchViewModel {
    pub fn new(branches: Vec<BCBranch>) -> Self {
        Self {
            state: ViewState::new(branches),
        }
    }

    pub fn state(&self) -> &ViewState {
        &self.state
    }

    pub fn branches(&self) -> &[BCBranch] {
        &self.state.branches
    }

    pub fn safe_to_delete_branches(&self) -> Vec<&BCBranch> {
        self.state
            .branches
            .iter()
            .filter(|b| b.pr_status == PrStatus::MERGED)
            .collect()
    }

    pub fn move_down(&mut self) {
        let max_index = self.state.branches.len().saturating_sub(1);
        if self.state.selected_index < max_index {
            self.state.selected_index += 1;
        }
    }

    pub fn move_up(&mut self) {
        if self.state.selected_index > 0 {
            self.state.selected_index -= 1;
        }
    }
}

/// Maps PR status to display colors
fn get_status_color(status: PrStatus) -> Color {
    match status {
        PrStatus::MERGED => Color::Green,  // Safe to delete
        PrStatus::OPEN => Color::Yellow,   // Caution
        PrStatus::NONE => Color::White,    // Default
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

/// Renders a single branch entry
fn render_branch(branch: &BCBranch, is_selected: bool) -> impl Into<AnyElement<'static>> {
    let prefix = if is_selected { "> " } else { "  " };
    let branch_text = format!("{}{}", prefix, branch.name);

    element! {
        View(flex_direction: FlexDirection::Column) {
            Text(
                content: branch_text,
                color: get_status_color(branch.pr_status),
                weight: Weight::Bold,
            )

            #(if let (Some(pr_number), Some(pr_title)) = (branch.pr_number, &branch.pr_title) {
                element! {
                    Text(content: format!("    └─ PR #{}: {}", pr_number, pr_title), color: Color::Grey)
                }
            } else {
                element! {
                    Text(content: "", color: Color::White)
                }
            })

            Text(
                content: format!("    └─ Status: {}", format_status_for_display(branch.pr_status)),
                color: get_status_color(branch.pr_status),
            )
        }
    }
}

#[component]
fn BranchListView(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let branches = create_fake_branches();
    let max_index = branches.len().saturating_sub(1);
    let mut selected_index = hooks.use_state(|| 0);

    // Handle keyboard input
    hooks.use_future(async move {
        use crossterm::event::{poll, read, Event, KeyCode};
        use std::time::Duration;

        loop {
            // Poll with a small timeout to avoid blocking the render loop
            if poll(Duration::from_millis(50)).unwrap_or(false) {
                if let Ok(event) = read() {
                    match event {
                        Event::Key(key_event) => match key_event.code {
                            KeyCode::Up => {
                                if selected_index > 0 {
                                    selected_index -= 1;
                                }
                            }
                            KeyCode::Down => {
                                if selected_index < max_index {
                                    selected_index += 1;
                                }
                            }
                            KeyCode::Char('q') | KeyCode::Esc => {
                                std::process::exit(0);
                            }
                            _ => {}
                        },
                        _ => {}
                    }
                }
            }
            // Small async sleep to yield to the executor
            smol::Timer::after(Duration::from_millis(16)).await;
        }
    });

    let mut branch_elements = Vec::new();
    for (idx, branch) in branches.iter().enumerate() {
        branch_elements.push(render_branch(branch, selected_index == idx));
    }

    element! {
        View(
            border_style: BorderStyle::Round,
            border_color: Color::Cyan,
            flex_direction: FlexDirection::Column,
        ) {
            View(flex_direction: FlexDirection::Column) {
                Text(
                    content: "Branch Cleaner - Git Branch Manager",
                    weight: Weight::Bold,
                    color: Color::Cyan,
                )

                View(flex_direction: FlexDirection::Column) {
                    #(branch_elements)
                }

                View(
                    border_style: BorderStyle::Single,
                    border_color: Color::Grey,
                    flex_direction: FlexDirection::Column,
                ) {
                    Text(
                        content: "Navigation: ↑↓ arrows | Quit: q/Esc",
                        color: Color::Grey,
                    )
                    Text(
                        content: "Green = Safe to delete (merged) | Yellow = Active PR | White = No PR",
                        color: Color::Grey,
                    )
                }
            }
        }
    }
}

/// Entry point to run the TUI application
pub fn run_branch_tui() {
    use crossterm::terminal::{enable_raw_mode, disable_raw_mode};

    // Enable raw mode to capture individual key presses
    enable_raw_mode().unwrap();

    let result = smol::block_on(element!(BranchListView).render_loop());

    // Ensure raw mode is disabled on exit
    disable_raw_mode().unwrap();

    result.unwrap();
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
        fn can_create_view_model_with_branches() {
            let branches = create_test_branches();
            let vm = BranchViewModel::new(branches.clone());

            let expected_state = ViewState {
                branches: branches.clone(),
                selected_index: 0,
            };

            assert_eq!(vm.state(), &expected_state);
        }

        #[test]
        fn returns_only_merged_branches_as_safe_to_delete() {
            let branches = create_test_branches();
            let vm = BranchViewModel::new(branches.clone());

            let expected = vec![&branches[2]]; // feature-2 is the only merged branch

            assert_eq!(vm.safe_to_delete_branches(), expected);
        }

        #[test]
        fn move_down_increments_selected_index() {
            let branches = create_test_branches();
            let mut vm = BranchViewModel::new(branches.clone());

            vm.move_down();

            let expected_state = ViewState {
                branches: branches.clone(),
                selected_index: 1,
            };

            assert_eq!(vm.state(), &expected_state);
        }

        #[test]
        fn move_up_decrements_selected_index() {
            let branches = create_test_branches();
            let mut vm = BranchViewModel::new(branches.clone());

            vm.move_down();
            vm.move_down();
            vm.move_up();

            let expected_state = ViewState {
                branches: branches.clone(),
                selected_index: 1,
            };

            assert_eq!(vm.state(), &expected_state);
        }
    }
}
