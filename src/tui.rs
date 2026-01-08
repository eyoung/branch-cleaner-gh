use r3bl_tui::{throws_with_return, ok, CommonResult, TuiColor, ANSIBasicColor, App, ComponentRegistryMap, EventPropagation, GlobalData, HasFocus, InputEvent, Key, KeyPress, SpecialKey, InputDevice, OutputDevice, TerminalWindow, key_press, RenderPipeline, render_pipeline, ZOrder, RenderOp, tui_styled_texts, tui_styled_text, new_style, tui_color, render_tui_styled_texts_into, col, row, RenderOps};

use crate::{BCBranch, BranchStore, PrStatus};

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

/// BranchViewModel manages branch view logic with dependency-injected store
/// Combines data fetching with pure view state transformations
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
    pub fn load_branches(&self) -> ViewState {
        ViewState::new(self.store.list_branches())
    }

    /// Returns branches that are safe to delete (merged PRs)
    pub fn safe_to_delete_branches<'a>(&self, state: &'a ViewState) -> Vec<&'a BCBranch> {
        state
            .branches
            .iter()
            .filter(|b| b.pr_status == PrStatus::MERGED)
            .collect()
    }

    /// Returns a new ViewState with the selection moved up
    pub fn move_up(&self, state: &ViewState) -> ViewState {
        let mut new_state = state.clone();
        if new_state.selected_index > 0 {
            new_state.selected_index -= 1;
        }
        new_state
    }

    /// Returns a new ViewState with the selection moved down
    pub fn move_down(&self, state: &ViewState) -> ViewState {
        let mut new_state = state.clone();
        let max_index = new_state.branches.len().saturating_sub(1);
        if new_state.selected_index < max_index {
            new_state.selected_index += 1;
        }
        new_state
    }
}

/// AppState wraps ViewState and ViewModel for r3bl_tui's GlobalData
#[derive(Debug, Clone)]
pub struct AppState<T: BranchStore> {
    pub view_state: ViewState,
    pub view_model: BranchViewModel<T>,
}

impl<T: BranchStore + Default> Default for AppState<T> {
    fn default() -> Self {
        let view_model = BranchViewModel::new(T::default());
        Self {
            view_state: view_model.load_branches(),
            view_model,
        }
    }
}

impl<T: BranchStore> std::fmt::Display for AppState<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AppState {{ branches: {} }}", self.view_state.branches.len())
    }
}

/// AppSignal for async message passing (future GitHub API integration)
#[derive(Debug, Clone, Default)]
pub enum AppSignal {
    #[default]
    Noop,
}

/// Maps PR status to display colors
fn get_status_color(status: PrStatus) -> TuiColor {
    match status {
        PrStatus::MERGED => TuiColor::Basic(ANSIBasicColor::Green),   // Safe to delete
        PrStatus::OPEN => TuiColor::Basic(ANSIBasicColor::Yellow),    // Caution
        PrStatus::NONE => TuiColor::Basic(ANSIBasicColor::White),  // Default
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

/// BranchCleanerApp implements the App trait for r3bl_tui
#[derive(Default)]
pub struct BranchCleanerApp<T: BranchStore>(std::marker::PhantomData<T>);

impl<T: BranchStore + Default> App for BranchCleanerApp<T> {
    type S = AppState<T>;
    type AS = AppSignal;

    fn app_init(
        &mut self,
        _component_registry_map: &mut ComponentRegistryMap<AppState<T>, AppSignal>,
        _has_focus: &mut HasFocus,
    ) {
        // Minimal initialization - no special setup needed
    }

    fn app_handle_input_event(
        &mut self,
        input_event: InputEvent,
        global_data: &mut GlobalData<AppState<T>, AppSignal>,
        _component_registry_map: &mut ComponentRegistryMap<AppState<T>, AppSignal>,
        _has_focus: &mut HasFocus,
    ) -> CommonResult<EventPropagation> {
        throws_with_return!({
            match input_event {
                InputEvent::Keyboard(KeyPress::Plain { key }) => match key {
                    Key::SpecialKey(SpecialKey::Up) => {
                        let new_state = global_data.state.view_model.move_up(&global_data.state.view_state);
                        global_data.state.view_state = new_state;
                        EventPropagation::ConsumedRender
                    }
                    Key::SpecialKey(SpecialKey::Down) => {
                        let new_state = global_data.state.view_model.move_down(&global_data.state.view_state);
                        global_data.state.view_state = new_state;
                        EventPropagation::ConsumedRender
                    }
                    Key::Character('q') => EventPropagation::ExitMainEventLoop,
                    _ => EventPropagation::Propagate,
                },
                _ => EventPropagation::Propagate,
            }
        });
    }

    fn app_handle_signal(
        &mut self,
        signal: &AppSignal,
        _global_data: &mut GlobalData<AppState<T>, AppSignal>,
        _component_registry_map: &mut ComponentRegistryMap<AppState<T>, AppSignal>,
        _has_focus: &mut HasFocus,
    ) -> CommonResult<EventPropagation> {
        throws_with_return!({
            match signal {
                AppSignal::Noop => EventPropagation::Propagate,
            }
        });
    }

    fn app_render(
        &mut self,
        global_data: &mut GlobalData<AppState<T>, AppSignal>,
        _component_registry_map: &mut ComponentRegistryMap<AppState<T>, AppSignal>,
        _has_focus: &mut HasFocus,
    ) -> CommonResult<RenderPipeline> {
        throws_with_return!({
            let state = &global_data.state.view_state;
            let mut pipeline = render_pipeline!();

            pipeline.push(ZOrder::Normal, {
                let mut render_ops = RenderOps::default();
                render_ops.push(RenderOp::ResetColor);

                // Header
                let header_color = tui_color!(hex "#00FFFF");
                let header_styled_texts = tui_styled_texts! {
                    tui_styled_text! {
                        @style: new_style!(bold color_fg: {header_color}),
                        @text: "Branch Cleaner - Git Branch Manager"
                    },
                };
                render_ops.push(RenderOp::MoveCursorPositionAbs(col(0) + row(0)));
                render_tui_styled_texts_into(&header_styled_texts, &mut render_ops);

                // Branch list
                let mut current_row = 2;
                for (idx, branch) in state.branches.iter().enumerate() {
                    let is_selected = idx == state.selected_index;
                    let prefix = if is_selected { "> " } else { "  " };

                    // Branch name with selection indicator
                    let branch_text = format!("{}{}", prefix, branch.name);
                    let branch_color = get_status_color(branch.pr_status);
                    let branch_styled_texts = tui_styled_texts! {
                        tui_styled_text! {
                            @style: new_style!(bold color_fg: {branch_color}),
                            @text: &branch_text
                        },
                    };
                    render_ops.push(RenderOp::MoveCursorPositionAbs(col(0) + row(current_row)));
                    render_tui_styled_texts_into(&branch_styled_texts, &mut render_ops);
                    current_row += 1;

                    // PR info if available
                    if let (Some(pr_number), Some(pr_title)) = (branch.pr_number, &branch.pr_title) {
                        let pr_text = format!("    └─ PR #{}: {}", pr_number, pr_title);
                        let grey_color = TuiColor::Basic(ANSIBasicColor::Gray);
                        let pr_styled_texts = tui_styled_texts! {
                            tui_styled_text! {
                                @style: new_style!(color_fg: {grey_color}),
                                @text: &pr_text
                            },
                        };
                        render_ops.push(RenderOp::MoveCursorPositionAbs(col(0) + row(current_row)));
                        render_tui_styled_texts_into(&pr_styled_texts, &mut render_ops);
                        current_row += 1;
                    }

                    // Status
                    let status_text = format!("    └─ Status: {}", format_status_for_display(branch.pr_status));
                    let status_color = get_status_color(branch.pr_status);
                    let status_styled_texts = tui_styled_texts! {
                        tui_styled_text! {
                            @style: new_style!(color_fg: {status_color}),
                            @text: &status_text
                        },
                    };
                    render_ops.push(RenderOp::MoveCursorPositionAbs(col(0) + row(current_row)));
                    render_tui_styled_texts_into(&status_styled_texts, &mut render_ops);
                    current_row += 1;
                }

                // Footer
                current_row += 1;
                let grey_color = TuiColor::Basic(ANSIBasicColor::Gray);
                let footer_styled_texts = tui_styled_texts! {
                    tui_styled_text! {
                        @style: new_style!(color_fg: {grey_color}),
                        @text: "Navigation: ↑↓ arrows | Quit: q"
                    },
                };
                render_ops.push(RenderOp::MoveCursorPositionAbs(col(0) + row(current_row)));
                render_tui_styled_texts_into(&footer_styled_texts, &mut render_ops);

                current_row += 1;
                let legend_styled_texts = tui_styled_texts! {
                    tui_styled_text! {
                        @style: new_style!(color_fg: {grey_color}),
                        @text: "Green = Safe to delete (merged) | Yellow = Active PR | White = No PR"
                    },
                };
                render_ops.push(RenderOp::MoveCursorPositionAbs(col(0) + row(current_row)));
                render_tui_styled_texts_into(&legend_styled_texts, &mut render_ops);

                render_ops
            });

            pipeline
        });
    }
}

/// Entry point to run the TUI application
pub async fn run_branch_tui() -> CommonResult<()> {
    // Initialize app state using InMemoryBranchStore with dependency injection
    let app_state = AppState::<crate::InMemoryBranchStore>::default();

    // Create app instance
    let app = Box::new(BranchCleanerApp::default());

    // Exit keys
    let exit_keys = &[InputEvent::Keyboard(key_press! { @char 'q' })];

    // Run r3bl_tui main loop
    let _unused: (GlobalData<_, _>, InputDevice, OutputDevice) =
        TerminalWindow::main_event_loop(app, exit_keys, app_state)?.await?;

    ok!()
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
            };

            assert_eq!(state, expected_state);
        }

        #[test]
        fn viewmodel_loads_branches_from_store() {
            // Arrange: Create a store with test branches
            let test_branches = create_test_branches();
            let store = InMemoryBranchStore::new(test_branches.clone());
            let view_model = BranchViewModel::new(store);

            // Act: Load branches through the viewmodel
            let view_state = view_model.load_branches();

            // Assert: ViewState contains branches from the store
            let expected_state = ViewState {
                branches: test_branches,
                selected_index: 0,
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
            let branches = create_test_branches();
            let state = ViewState::new(branches.clone());
            let store = InMemoryBranchStore::new(branches.clone());
            let view_model = BranchViewModel::new(store);

            let new_state = view_model.move_down(&state);

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
            let store = InMemoryBranchStore::new(branches.clone());
            let view_model = BranchViewModel::new(store);

            let state = view_model.move_down(&state);
            let state = view_model.move_down(&state);
            let state = view_model.move_up(&state);

            let expected_state = ViewState {
                branches: branches.clone(),
                selected_index: 1,
            };

            assert_eq!(state, expected_state);
        }
    }
}
