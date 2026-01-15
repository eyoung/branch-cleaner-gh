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

    /// Toggles selection of the current branch (add if not selected, remove if selected)
    pub fn toggle_selection(&self, state: &mut ViewState) {
        if state.selected_index >= state.branches.len() {
            return; // Safety: invalid index
        }

        let current_branch_name = &state.branches[state.selected_index].name;

        if let Some(pos) = state
            .selected_branches
            .iter()
            .position(|name| name == current_branch_name)
        {
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

    /// Updates a single branch in the state (for streaming updates)
    /// Finds the branch by name and replaces it with the updated version
    /// Auto-selects merged branches when they transition from LOADING
    pub fn update_branch(&self, state: &mut ViewState, updated_branch: BCBranch) {
        if let Some(pos) = state
            .branches
            .iter()
            .position(|b| b.name == updated_branch.name)
        {
            let was_loading = state.branches[pos].pr_status == PrStatus::LOADING;
            let is_now_merged = updated_branch.pr_status == PrStatus::MERGED;

            // Auto-select merged branches when they transition from LOADING
            if was_loading && is_now_merged {
                if !state.selected_branches.contains(&updated_branch.name) {
                    state.selected_branches.push(updated_branch.name.clone());
                }
            }

            state.branches[pos] = updated_branch;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::InMemoryBranchStore;

    fn create_test_branches() -> Vec<BCBranch> {
        vec![
            BCBranch::new("main", PrStatus::NONE),
            BCBranch::with_pr("feature-1", PrStatus::OPEN, 1, "Feature 1"),
            BCBranch::with_pr("feature-2", PrStatus::MERGED, 2, "Feature 2"),
        ]
    }

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

    #[test]
    fn update_branch_replaces_loading_branch_with_enriched_data() {
        // Arrange: State with branches in LOADING status
        let loading_branches = vec![
            BCBranch::new("main", PrStatus::LOADING),
            BCBranch::new("feature-1", PrStatus::LOADING),
        ];
        let mut state = ViewState::new(loading_branches.clone());
        let store = InMemoryBranchStore::new(loading_branches);
        let view_model = BranchViewModel::new(store);

        // Act: Update feature-1 with enriched data
        let enriched = BCBranch::with_pr("feature-1", PrStatus::OPEN, 42, "My Feature");
        view_model.update_branch(&mut state, enriched.clone());

        // Assert: feature-1 is updated, main still loading
        assert_eq!(state.branches[0].pr_status, PrStatus::LOADING);
        assert_eq!(state.branches[1], enriched);
    }

    #[test]
    fn update_branch_auto_selects_merged_branches() {
        // Arrange: State with branches in LOADING status (no auto-selection yet)
        let loading_branches = vec![
            BCBranch::new("main", PrStatus::LOADING),
            BCBranch::new("feature-merged", PrStatus::LOADING),
        ];
        let mut state = ViewState::new(loading_branches.clone());
        // LOADING branches are not auto-selected
        assert!(state.selected_branches.is_empty());

        let store = InMemoryBranchStore::new(loading_branches);
        let view_model = BranchViewModel::new(store);

        // Act: Update feature-merged to MERGED status
        let merged = BCBranch::with_pr("feature-merged", PrStatus::MERGED, 10, "Merged PR");
        view_model.update_branch(&mut state, merged);

        // Assert: feature-merged is now auto-selected
        assert_eq!(state.selected_branches, vec!["feature-merged".to_owned()]);
    }

    #[test]
    fn update_branch_does_not_select_non_merged_branches() {
        // Arrange: State with branches in LOADING status
        let loading_branches = vec![BCBranch::new("feature-open", PrStatus::LOADING)];
        let mut state = ViewState::new(loading_branches.clone());
        let store = InMemoryBranchStore::new(loading_branches);
        let view_model = BranchViewModel::new(store);

        // Act: Update to OPEN status
        let open = BCBranch::with_pr("feature-open", PrStatus::OPEN, 5, "Open PR");
        view_model.update_branch(&mut state, open);

        // Assert: Not selected (only MERGED branches are auto-selected)
        assert!(state.selected_branches.is_empty());
    }

    #[test]
    fn update_branch_ignores_unknown_branches() {
        // Arrange: State with known branches
        let branches = vec![BCBranch::new("main", PrStatus::LOADING)];
        let mut state = ViewState::new(branches.clone());
        let store = InMemoryBranchStore::new(branches);
        let view_model = BranchViewModel::new(store);

        // Act: Try to update a branch that doesn't exist
        let unknown = BCBranch::new("unknown-branch", PrStatus::MERGED);
        view_model.update_branch(&mut state, unknown);

        // Assert: State unchanged
        assert_eq!(state.branches.len(), 1);
        assert_eq!(state.branches[0].name, "main");
    }

    #[test]
    fn closed_branches_are_not_auto_selected() {
        // Arrange: Branches including one with CLOSED status (PR closed without merging)
        let branches = vec![
            BCBranch::new("main", PrStatus::NONE),
            BCBranch::with_pr("feature-closed", PrStatus::CLOSED, 3, "Closed PR"),
            BCBranch::with_pr("feature-merged", PrStatus::MERGED, 4, "Merged PR"),
        ];

        // Act: Create ViewState
        let state = ViewState::new(branches.clone());

        // Assert: Only MERGED branch is auto-selected, CLOSED is not
        assert_eq!(state.selected_branches, vec!["feature-merged".to_owned()]);
    }
}
