use r3bl_tui::{throws_with_return, ok, CommonResult, TuiColor, ANSIBasicColor, App, ComponentRegistryMap, EventPropagation, GlobalData, HasFocus, InputEvent, Key, KeyPress, SpecialKey, InputDevice, OutputDevice, TerminalWindow, key_press, RenderPipeline, render_pipeline, ZOrder, RenderOp, tui_styled_texts, tui_styled_text, new_style, tui_color, render_tui_styled_texts_into, col, row, RenderOps, send_signal, TerminalWindowMainThreadSignal};

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
}

/// AppState is pure data only, following r3bl_tui Elm architecture
/// No business logic - just the view state
pub type AppState = ViewState;

impl Default for AppState {
    fn default() -> Self {
        // Empty state - will be populated by run_branch_tui with actual data
        ViewState::new(vec![])
    }
}

impl std::fmt::Display for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AppState {{ branches: {} }}", self.branches.len())
    }
}

/// AppSignal represents user actions that trigger state changes
/// Following r3bl_tui's signal pattern for unidirectional data flow
#[derive(Debug, Clone, Default)]
pub enum AppSignal {
    #[default]
    Noop,
    MoveUp,
    MoveDown,
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
/// Holds the ViewModel for business logic operations
pub struct BranchCleanerApp<T: BranchStore> {
    view_model: BranchViewModel<T>,
}

impl<T: BranchStore> BranchCleanerApp<T> {
    pub fn new(view_model: BranchViewModel<T>) -> Self {
        Self { view_model }
    }
}

impl<T: BranchStore> App for BranchCleanerApp<T> {
    type S = AppState;
    type AS = AppSignal;

    fn app_init(
        &mut self,
        _component_registry_map: &mut ComponentRegistryMap<AppState, AppSignal>,
        _has_focus: &mut HasFocus,
    ) {
        // Minimal initialization - no special setup needed
    }

    fn app_handle_input_event(
        &mut self,
        input_event: InputEvent,
        global_data: &mut GlobalData<AppState, AppSignal>,
        _component_registry_map: &mut ComponentRegistryMap<AppState, AppSignal>,
        _has_focus: &mut HasFocus,
    ) -> CommonResult<EventPropagation> {
        throws_with_return!({
            match input_event {
                InputEvent::Keyboard(KeyPress::Plain { key }) => match key {
                    Key::SpecialKey(SpecialKey::Up) => {
                        // Send signal instead of mutating directly (r3bl pattern)
                        send_signal!(
                            global_data.main_thread_channel_sender,
                            TerminalWindowMainThreadSignal::ApplyAppSignal(AppSignal::MoveUp)
                        );
                        EventPropagation::ConsumedRender
                    }
                    Key::SpecialKey(SpecialKey::Down) => {
                        // Send signal instead of mutating directly (r3bl pattern)
                        send_signal!(
                            global_data.main_thread_channel_sender,
                            TerminalWindowMainThreadSignal::ApplyAppSignal(AppSignal::MoveDown)
                        );
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
        global_data: &mut GlobalData<AppState, AppSignal>,
        _component_registry_map: &mut ComponentRegistryMap<AppState, AppSignal>,
        _has_focus: &mut HasFocus,
    ) -> CommonResult<EventPropagation> {
        throws_with_return!({
            let state = &mut global_data.state;
            match signal {
                AppSignal::Noop => {}
                AppSignal::MoveUp => {
                    self.view_model.move_up(state);
                }
                AppSignal::MoveDown => {
                    self.view_model.move_down(state);
                }
            }
            EventPropagation::ConsumedRender
        });
    }

    fn app_render(
        &mut self,
        global_data: &mut GlobalData<AppState, AppSignal>,
        _component_registry_map: &mut ComponentRegistryMap<AppState, AppSignal>,
        _has_focus: &mut HasFocus,
    ) -> CommonResult<RenderPipeline> {
        throws_with_return!({
            let state = &global_data.state;
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
/// Following r3bl_tui architecture: create store, load state, inject dependencies
pub async fn run_branch_tui() -> CommonResult<()> {
    // 1. Create the data store (dependency injection)
    let store = crate::InMemoryBranchStore::default();

    // 2. Create the ViewModel with injected store
    let view_model = BranchViewModel::new(store);

    // 3. Load initial state from the ViewModel
    let app_state = view_model.load_initial_state();

    // 4. Create app instance with ViewModel (holds business logic)
    let app = Box::new(BranchCleanerApp::new(view_model));

    // 5. Exit keys
    let exit_keys = &[InputEvent::Keyboard(key_press! { @char 'q' })];

    // 6. Run r3bl_tui main loop with pure data state
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

            // Act: Load initial state from the viewmodel
            let view_state = view_model.load_initial_state();

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
            };

            assert_eq!(state, expected_state);
        }
    }
}
