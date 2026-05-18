# tmkpr-ui Architecture & Maintainability Guide

## Module Organization

The codebase follows a clean separation of concerns across three primary modules:

### app.rs — Application State & Logic
- **Responsibility**: Manage app state, business logic, data transformations
- **Key types**: `App`, `AppMode`, `ModeKind`, `TaskSort`, `ProjectSort`, etc.
- **Examples**: 
  - `refresh()` — fetch and display data
  - `apply_task_sort_filter()` — transform task list with current sort/filter
  - `submit_edit_project()` — persist edits to storage

### ui.rs — Rendering
- **Responsibility**: Convert app state to visual output
- **Key functions**: `render()`, `render_manage_tasks()`, `render_entries()`, etc.
- **Pattern**: Match on `ModeKind`, render appropriate modal/view
- **Helpers**: `render_form_modal()`, `parse_hex_color()`

### input.rs — Keyboard Handling
- **Responsibility**: Convert key events to app state mutations
- **Key functions**: `handle_key()` (dispatcher), `handle_normal()`, `handle_manage_tasks()`, etc.
- **Pattern**: Match on `ModeKind`, route to specialized handler

---

## Known Maintainability Issues

### High Priority

**1. TaskSort::Project now fixed (ISSUE-001)**
- ~~Sorted by UUID instead of project name~~
- ✅ Fixed: now sorts by `project_name(&project_id)`

### Medium Priority

**2. Duplication in Sort/Filter Logic (ISSUE-002)**
- `apply_project_sort_filter` and `apply_task_sort_filter` are nearly identical
- `render_manage_projects` and `render_manage_tasks` share layout and keybinding patterns
- `handle_manage_projects` and `handle_manage_tasks` are structurally identical
- **Mitigation**: Next refactor should extract generic list-panel renderer and common sort/filter helper

**3. Magic Field Indices in Forms (ISSUE-003)**
- Form handlers access fields by hardcoded index: `form.fields[0]`, `form.fields[1]`, etc.
- Risk: Adding a field to the form without updating handlers causes silent data misreads
- **Mitigation**: Consider named struct per form type or const-indexed accessors

**4. Reset Selected Index on Edit/Delete (ISSUE-006)**
- After edit or delete, list resets to `selected: 0`, jumping user to top
- After edit, sort/filter not reapplied (uses unfiltered list)
- **Mitigation**: Preserve selected position after non-destructive edits; always reapply sort/filter

**5. Scattered Entry Filter Fields (ISSUE-004)**
- `filter_project_name`, `filter_date_str`, `filter_project_id`, `filter_from`, `filter_until` on App struct
- Should be grouped as single `EntryFilter` struct like `TaskFilter` and `ProjectFilter`
- **Mitigation**: Refactor into `EntryFilter` struct, unify `has_filter()` logic

### Low Priority

**6. Duplicated Color Lookup (ISSUE-005)**
- Color resolution logic repeated in `render_entries`, `render_manage_tasks`, `render_manage_projects`
- **Mitigation**: Extract helper `fn project_color(app: &App, pid: &str) -> Option<Color>`

**7. No Unit Tests (ISSUE-007)**
- Pure functions like `parse_date_filter()`, `apply_task_sort_filter()` have no test coverage
- **Mitigation**: Add tests for date parsing and sort/filter logic

**8. Hardcoded Layout Constants (ISSUE-008)**
- Modal dimensions like `65`, `75`, `60` scattered without names
- Tick duration `250ms` in main.rs
- **Mitigation**: Define constants at module level (e.g., `const MODAL_WIDTH_PERCENT: u16 = 65;`)

**9. Confusing Filter UX (ISSUE-009)**
- "Include archived? (y/n)" with double-negation logic
- No validation — anything other than "y"/"yes" activates archive filter silently
- **Mitigation**: Clearer prompt ("Show archived: y/n?"), explicit validation with error message

---

## Design Patterns

### AppMode + ModeKind
- **AppMode**: Concrete enum with associated data (e.g., `ManageTasks { tasks: Vec<Task>, selected: usize }`)
- **ModeKind**: Simplified routing enum for pattern matching without data
- **Why**: Allows handlers to route by `mode.kind()` without exposing/destructuring payload each time

### Form Handlers Pattern
All modal forms follow the same pattern:
1. Call `form.handle_key(key)` → get `FormResult::None | Cancel | Submit`
2. On `Submit`: extract form data, call app method, transition to new mode
3. On `Cancel`: return to previous mode (usually `ManageProjects` or `ManageTasks`)

### Sort/Filter Pipeline
```
open_manage_*() 
  → apply_*_sort_filter(raw_list) 
  → populate AppMode with filtered list
  → render displays pre-filtered list
```

Filter/sort state lives on `App` (`project_sort`, `project_filter`, `task_sort`, `task_filter`) and persists across navigations.

---

## Common Pitfalls

1. **Forgetting to reapply sort/filter after edit** — edited items may appear out of order
2. **Resetting `selected: 0`** — list jumps to top, poor UX for large lists
3. **Adding form fields without updating handlers** — data reads from wrong fields silently
4. **Sorting by ID instead of display name** — produces unintuitive sort orders
5. **No validation on y/n prompts** — typos silently activate filters

---

## Future Refactoring Roadmap

### Phase 1: Extract Common List Panel Renderer
```rust
fn render_list_panel<T: Display>(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    items: Vec<ListItem>,
    selected: usize,
    hint_text: &str,
) { ... }
```
This would replace `render_manage_projects` and `render_manage_tasks` bodies.

### Phase 2: Generic Sort/Filter Helper
```rust
trait Filterable {
    fn matches_filter(&self, filter: &dyn Filter) -> bool;
}
impl Filterable for Task { ... }
impl Filterable for Project { ... }
```

### Phase 3: Form Schema Definitions
```rust
const EDIT_PROJECT_FIELDS: &[&str] = &["Name", "Description", "Color"];
fn get_form_field(form: &Form, schema: &[&str], label: &str) -> &str { ... }
```
Replaces magic indices with named lookups.

### Phase 4: Consolidate Entry Filter State
```rust
pub struct EntryFilter {
    project_id: Option<String>,
    from: Option<DateTime<Utc>>,
    until: Option<DateTime<Utc>>,
}
impl EntryFilter {
    pub fn is_active(&self) -> bool { ... }
}
```

---

## Contributing Guidelines

- **Keep modules narrow**: app.rs = logic, ui.rs = rendering, input.rs = keyboard
- **Extract duplication early**: If you write similar code twice, abstract it before writing a third copy
- **Name magic numbers**: `const MODAL_WIDTH = 65;` instead of bare `65`
- **Test pure functions**: `parse_date_filter`, sort/filter logic, transformations
- **Preserve user position**: Don't reset `selected` unless essential
- **Reapply sort/filter**: After mutations, call `apply_*_sort_filter()` before displaying list
