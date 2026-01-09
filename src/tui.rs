use r3bl_tui::{throws_with_return, ok, CommonResult, TuiColor, ANSIBasicColor, App, ComponentRegistryMap, EventPropagation, GlobalData, HasFocus, InputEvent, Key, KeyPress, SpecialKey, InputDevice, OutputDevice, TerminalWindow, key_press, RenderPipeline, render_pipeline, ZOrder, RenderOp, tui_styled_texts, tui_styled_text, new_style, tui_color, render_tui_styled_texts_into, col, row, RenderOps, send_signal, TerminalWindowMainThreadSignal};

use crate::{BCBranch, BranchStore, PrStatus};

/// ViewState represents the pure data state of the TUI
/// This is a simple data structure with no business logic
#[derive(Clone, Debug, PartialEq)]
pub struct ViewState {
    pub branches: Vec<BCBranch>,
    pub selected_index: usize,
    pub selected_branches: Vec<String>, // Branch names selected for deletion
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
    ToggleSelection,
    DeleteSelected,
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
                        send_signal!(
                            global_data.main_thread_channel_sender,
                            TerminalWindowMainThreadSignal::ApplyAppSignal(AppSignal::MoveUp)
                        );
                        EventPropagation::ConsumedRender
                    }
                    Key::SpecialKey(SpecialKey::Down) => {
                        send_signal!(
                            global_data.main_thread_channel_sender,
                            TerminalWindowMainThreadSignal::ApplyAppSignal(AppSignal::MoveDown)
                        );
                        EventPropagation::ConsumedRender
                    }
                    Key::Character(' ') => {
                        // Space to toggle selection
                        send_signal!(
                            global_data.main_thread_channel_sender,
                            TerminalWindowMainThreadSignal::ApplyAppSignal(AppSignal::ToggleSelection)
                        );
                        EventPropagation::ConsumedRender
                    }
                    Key::Character('d') => {
                        // 'd' to delete selected branches
                        send_signal!(
                            global_data.main_thread_channel_sender,
                            TerminalWindowMainThreadSignal::ApplyAppSignal(AppSignal::DeleteSelected)
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
                AppSignal::ToggleSelection => {
                    self.view_model.toggle_selection(state);
                }
                AppSignal::DeleteSelected => {
                    self.view_model.delete_selected_branches(state);
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
                    let is_cursor_here = idx == state.selected_index;
                    let is_marked_for_deletion = state.selected_branches.contains(&branch.name);

                    // Cursor indicator and checkbox
                    let cursor = if is_cursor_here { ">" } else { " " };
                    let checkbox = if is_marked_for_deletion { "[x]" } else { "[ ]" };

                    // Branch name with selection indicator
                    let branch_text = format!("{} {} {}", cursor, checkbox, branch.name);
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
                        @text: "↑↓: Navigate | Space: Toggle selection | d: Delete selected | q: Quit"
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
pub async fn run_branch_tui<T: BranchStore>(store: T) -> CommonResult<()> {
    // 1. Create the ViewModel with injected store
    let view_model = BranchViewModel::new(store);

    // 2. Load initial state from the ViewModel
    let app_state = view_model.load_initial_state();

    // 3. Create app instance with ViewModel (holds business logic)
    let app = Box::new(BranchCleanerApp::new(view_model));

    // 4. Exit keys
    let exit_keys = &[InputEvent::Keyboard(key_press! { @char 'q' })];

    // 5. Run r3bl_tui main loop with pure data state
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
