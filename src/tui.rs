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

/// Entry point to run the TUI application
pub fn run_branch_tui() {
    let branches = create_fake_branches();
    let mut view_state = ViewState::new(branches);

    // For now, highlight the second branch
    view_state.selected_index = 1;

    let mut branch_elements = Vec::new();
    for (idx, branch) in view_state.branches.iter().enumerate() {
        branch_elements.push(render_branch(branch, idx == view_state.selected_index));
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
                        content: "Navigation: ↑↓/jk arrows | Quit: q/Esc",
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
    .print();

    println!("\n(Interactive navigation will be added in the next iteration)");
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

            assert_eq!(vm.branches(), &branches[..]);
        }

        #[test]
        fn returns_only_merged_branches_as_safe_to_delete() {
            let branches = create_test_branches();
            let vm = BranchViewModel::new(branches.clone());

            let expected = vec![&branches[2]]; // feature-2 is the only merged branch

            assert_eq!(vm.safe_to_delete_branches(), expected);
        }
    }
}
