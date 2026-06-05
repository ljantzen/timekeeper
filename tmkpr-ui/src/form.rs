use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub struct Form {
    pub fields: Vec<Field>,
    pub focused: usize,
}

pub enum FieldKind {
    Text,
    Toggle,
    DateTime,
}

pub struct Field {
    pub label: &'static str,
    pub value: String,
    pub cursor: usize,
    pub completions: Vec<String>,
    pub completion_colors: Vec<Option<String>>,
    pub ac_index: Option<usize>,
    pub kind: FieldKind,
}

pub enum FormResult {
    None,
    Submit,
    Cancel,
}

impl Field {
    pub fn new(label: &'static str, value: impl Into<String>) -> Self {
        let value = value.into();
        let cursor = value.len();
        Self {
            label,
            value,
            cursor,
            completions: vec![],
            completion_colors: vec![],
            ac_index: None,
            kind: FieldKind::Text,
        }
    }

    pub fn toggle(label: &'static str, on: bool) -> Self {
        Self {
            label,
            value: if on { "true" } else { "false" }.to_string(),
            cursor: 0,
            completions: vec![],
            completion_colors: vec![],
            ac_index: None,
            kind: FieldKind::Toggle,
        }
    }

    pub fn datetime(label: &'static str, value: impl Into<String>) -> Self {
        let value = value.into();
        let cursor = value.len();
        Self {
            label,
            value,
            cursor,
            completions: vec![],
            completion_colors: vec![],
            ac_index: None,
            kind: FieldKind::DateTime,
        }
    }

    pub fn is_on(&self) -> bool {
        self.value == "true"
    }

    pub fn with_completions(mut self, completions: Vec<String>) -> Self {
        self.completions = completions;
        self
    }

    pub fn with_completion_colors(mut self, colors: Vec<Option<String>>) -> Self {
        self.completion_colors = colors;
        self
    }

    pub fn suggestions_colored(&self) -> Vec<(&str, Option<&str>)> {
        if self.completions.is_empty() {
            return vec![];
        }
        let query = self.value.to_lowercase();
        self.completions
            .iter()
            .enumerate()
            .filter(|(_, c)| query.is_empty() || c.to_lowercase().contains(&query))
            .map(|(i, s)| {
                let color = self.completion_colors.get(i).and_then(|c| c.as_deref());
                (s.as_str(), color)
            })
            .collect()
    }

    pub fn apply_selected(&mut self) -> bool {
        if let Some(idx) = self.ac_index {
            let selected = {
                let query = self.value.to_lowercase();
                self.completions
                    .iter()
                    .filter(|c| query.is_empty() || c.to_lowercase().contains(&query))
                    .nth(idx)
                    .cloned()
            };
            if let Some(s) = selected {
                self.value = s;
                self.cursor = self.value.len();
                self.ac_index = None;
                return true;
            }
        }
        false
    }

    fn insert(&mut self, ch: char) {
        self.value.insert(self.cursor, ch);
        self.cursor += ch.len_utf8();
    }

    fn overwrite(&mut self, ch: char) {
        if self.cursor < self.value.len() {
            let start_byte = self.cursor;
            let ch_len = self.value[start_byte..]
                .chars()
                .next()
                .map(|c| c.len_utf8())
                .unwrap_or(1);
            self.value.replace_range(start_byte..start_byte + ch_len, &ch.to_string());
        } else {
            self.value.push(ch);
        }
        self.cursor += ch.len_utf8();
    }

    fn delete_back(&mut self) {
        if self.cursor > 0 {
            let before = &self.value[..self.cursor];
            let ch_len = before.chars().last().map(|c| c.len_utf8()).unwrap_or(1);
            self.cursor -= ch_len;
            self.value.drain(self.cursor..self.cursor + ch_len);
        }
    }

    pub fn handle_key(&mut self, code: KeyCode) {
        match self.kind {
            FieldKind::Toggle => {
                if matches!(code, KeyCode::Char(' ') | KeyCode::Left | KeyCode::Right) {
                    self.value = if self.is_on() { "false" } else { "true" }.to_string();
                }
            }
            FieldKind::DateTime => match code {
                KeyCode::Char(c) => {
                    self.skip_separators_forward();
                    self.overwrite(c);
                    self.skip_separators_forward();
                }
                KeyCode::Backspace => {
                    self.skip_separators_back();
                    self.delete_back();
                }
                KeyCode::Delete if self.cursor < self.value.len() => {
                    self.skip_separators_forward();
                    let ch_len = self.value[self.cursor..]
                        .chars()
                        .next()
                        .map(|c| c.len_utf8())
                        .unwrap_or(1);
                    self.value.drain(self.cursor..self.cursor + ch_len);
                }
                KeyCode::Left if self.cursor > 0 => {
                    let before = &self.value[..self.cursor];
                    let ch_len = before.chars().last().map(|c| c.len_utf8()).unwrap_or(1);
                    self.cursor -= ch_len;
                    self.skip_separators_back();
                }
                KeyCode::Right if self.cursor < self.value.len() => {
                    let ch_len = self.value[self.cursor..]
                        .chars()
                        .next()
                        .map(|c| c.len_utf8())
                        .unwrap_or(1);
                    self.cursor += ch_len;
                    self.skip_separators_forward();
                }
                KeyCode::Home => self.cursor = 0,
                KeyCode::End => self.cursor = self.value.len(),
                _ => {}
            },
            FieldKind::Text => match code {
                KeyCode::Char(c) => self.insert(c),
                KeyCode::Backspace => self.delete_back(),
                KeyCode::Delete if self.cursor < self.value.len() => {
                    let ch_len = self.value[self.cursor..]
                        .chars()
                        .next()
                        .map(|c| c.len_utf8())
                        .unwrap_or(1);
                    self.value.drain(self.cursor..self.cursor + ch_len);
                }
                KeyCode::Left if self.cursor > 0 => {
                    let before = &self.value[..self.cursor];
                    let ch_len = before.chars().last().map(|c| c.len_utf8()).unwrap_or(1);
                    self.cursor -= ch_len;
                }
                KeyCode::Right if self.cursor < self.value.len() => {
                    let ch_len = self.value[self.cursor..]
                        .chars()
                        .next()
                        .map(|c| c.len_utf8())
                        .unwrap_or(1);
                    self.cursor += ch_len;
                }
                KeyCode::Home => self.cursor = 0,
                KeyCode::End => self.cursor = self.value.len(),
                _ => {}
            },
        }
    }

    fn skip_separators_forward(&mut self) {
        while self.cursor < self.value.len() {
            let ch = self.value[self.cursor..]
                .chars()
                .next()
                .unwrap_or(' ');
            if matches!(ch, '-' | ':' | ' ') {
                let ch_len = ch.len_utf8();
                self.cursor += ch_len;
            } else {
                break;
            }
        }
    }

    fn skip_separators_back(&mut self) {
        while self.cursor > 0 {
            let before = &self.value[..self.cursor];
            let ch = before.chars().last().unwrap_or(' ');
            if matches!(ch, '-' | ':' | ' ') {
                let ch_len = ch.len_utf8();
                self.cursor -= ch_len;
            } else {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::empty())
    }

    fn text_form(fields: &[&str]) -> Form {
        Form {
            fields: fields.iter().map(|v| Field::new("label", *v)).collect(),
            focused: 0,
        }
    }

    // --- Toggle field ---

    #[test]
    fn toggle_off_by_default() {
        let f = Field::toggle("label", false);
        assert!(!f.is_on());
    }

    #[test]
    fn toggle_on_by_default() {
        let f = Field::toggle("label", true);
        assert!(f.is_on());
    }

    #[test]
    fn toggle_space_flips_off_to_on() {
        let mut f = Field::toggle("label", false);
        f.handle_key(KeyCode::Char(' '));
        assert!(f.is_on());
    }

    #[test]
    fn toggle_space_flips_on_to_off() {
        let mut f = Field::toggle("label", true);
        f.handle_key(KeyCode::Char(' '));
        assert!(!f.is_on());
    }

    #[test]
    fn toggle_left_flips() {
        let mut f = Field::toggle("label", false);
        f.handle_key(KeyCode::Left);
        assert!(f.is_on());
    }

    #[test]
    fn toggle_right_flips() {
        let mut f = Field::toggle("label", true);
        f.handle_key(KeyCode::Right);
        assert!(!f.is_on());
    }

    #[test]
    fn toggle_double_flip_restores() {
        let mut f = Field::toggle("label", false);
        f.handle_key(KeyCode::Char(' '));
        f.handle_key(KeyCode::Char(' '));
        assert!(!f.is_on());
    }

    #[test]
    fn toggle_ignores_unrelated_keys() {
        let mut f = Field::toggle("label", false);
        f.handle_key(KeyCode::Char('x'));
        f.handle_key(KeyCode::Enter);
        f.handle_key(KeyCode::Backspace);
        assert!(!f.is_on());
    }

    // --- Text field: editing ---

    #[test]
    fn text_insert_appends_at_cursor() {
        let mut f = Field::new("label", "");
        f.handle_key(KeyCode::Char('a'));
        f.handle_key(KeyCode::Char('b'));
        assert_eq!(f.value, "ab");
        assert_eq!(f.cursor, 2);
    }

    #[test]
    fn text_insert_mid_string() {
        let mut f = Field::new("label", "ac");
        f.cursor = 1;
        f.handle_key(KeyCode::Char('b'));
        assert_eq!(f.value, "abc");
        assert_eq!(f.cursor, 2);
    }

    #[test]
    fn text_backspace_removes_char_before_cursor() {
        let mut f = Field::new("label", "abc");
        f.handle_key(KeyCode::Backspace);
        assert_eq!(f.value, "ab");
        assert_eq!(f.cursor, 2);
    }

    #[test]
    fn text_backspace_at_start_is_noop() {
        let mut f = Field::new("label", "");
        f.handle_key(KeyCode::Backspace);
        assert_eq!(f.value, "");
        assert_eq!(f.cursor, 0);
    }

    #[test]
    fn text_delete_removes_char_at_cursor() {
        let mut f = Field::new("label", "abc");
        f.cursor = 0;
        f.handle_key(KeyCode::Delete);
        assert_eq!(f.value, "bc");
        assert_eq!(f.cursor, 0);
    }

    #[test]
    fn text_delete_at_end_is_noop() {
        let mut f = Field::new("label", "abc");
        f.handle_key(KeyCode::Delete);
        assert_eq!(f.value, "abc");
    }

    #[test]
    fn text_left_moves_cursor_back() {
        let mut f = Field::new("label", "abc");
        f.handle_key(KeyCode::Left);
        assert_eq!(f.cursor, 2);
    }

    #[test]
    fn text_left_at_start_is_noop() {
        let mut f = Field::new("label", "abc");
        f.cursor = 0;
        f.handle_key(KeyCode::Left);
        assert_eq!(f.cursor, 0);
    }

    #[test]
    fn text_right_moves_cursor_forward() {
        let mut f = Field::new("label", "abc");
        f.cursor = 0;
        f.handle_key(KeyCode::Right);
        assert_eq!(f.cursor, 1);
    }

    #[test]
    fn text_right_at_end_is_noop() {
        let mut f = Field::new("label", "abc");
        f.handle_key(KeyCode::Right);
        assert_eq!(f.cursor, 3);
    }

    #[test]
    fn text_home_goes_to_start() {
        let mut f = Field::new("label", "abc");
        f.handle_key(KeyCode::Home);
        assert_eq!(f.cursor, 0);
    }

    #[test]
    fn text_end_goes_to_end() {
        let mut f = Field::new("label", "abc");
        f.cursor = 0;
        f.handle_key(KeyCode::End);
        assert_eq!(f.cursor, 3);
    }

    // --- Autocomplete ---

    #[test]
    fn suggestions_empty_when_no_completions() {
        let f = Field::new("label", "");
        assert!(f.suggestions_colored().is_empty());
    }

    #[test]
    fn suggestions_returns_all_when_query_empty() {
        let f = Field::new("label", "")
            .with_completions(vec!["alpha".into(), "beta".into()]);
        assert_eq!(f.suggestions_colored().len(), 2);
    }

    #[test]
    fn suggestions_filters_by_query() {
        let f = Field::new("label", "al")
            .with_completions(vec!["alpha".into(), "beta".into()]);
        let s = f.suggestions_colored();
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].0, "alpha");
    }

    #[test]
    fn apply_selected_fills_value_and_clears_index() {
        let mut f = Field::new("label", "")
            .with_completions(vec!["alpha".into(), "beta".into()]);
        f.ac_index = Some(1);
        let applied = f.apply_selected();
        assert!(applied);
        assert_eq!(f.value, "beta");
        assert_eq!(f.cursor, 4);
        assert!(f.ac_index.is_none());
    }

    #[test]
    fn apply_selected_noop_when_no_index() {
        let mut f = Field::new("label", "")
            .with_completions(vec!["alpha".into()]);
        let applied = f.apply_selected();
        assert!(!applied);
    }

    // --- DateTime field: overwrite mode ---

    #[test]
    fn datetime_char_overwrites_at_cursor() {
        let mut f = Field::datetime("label", "2025-01-01 12:00");
        f.cursor = 0;
        f.handle_key(KeyCode::Char('2'));
        assert_eq!(f.value, "2025-01-01 12:00");
        assert_eq!(f.cursor, 1);
    }

    #[test]
    fn datetime_char_overwrites_mid_field() {
        let mut f = Field::datetime("label", "2025-01-01 12:00");
        f.cursor = 5;
        f.handle_key(KeyCode::Char('1'));
        assert_eq!(f.value, "2025-11-01 12:00");
        assert_eq!(f.cursor, 6);
    }

    #[test]
    fn datetime_char_skips_separator() {
        let mut f = Field::datetime("label", "2025-01-01 12:00");
        f.cursor = 4;
        f.handle_key(KeyCode::Char('0'));
        assert_eq!(f.value, "2025-01-01 12:00");
        assert_eq!(f.cursor, 6);
    }

    #[test]
    fn datetime_backspace_deletes_before_cursor() {
        let mut f = Field::datetime("label", "2025-01-01 12:00");
        f.cursor = 5;
        f.handle_key(KeyCode::Backspace);
        assert_eq!(f.value, "202-01-01 12:00");
        assert_eq!(f.cursor, 3);
    }

    #[test]
    fn datetime_left_from_after_separator() {
        let mut f = Field::datetime("label", "2025-01-01 12:00");
        f.cursor = 5;
        f.handle_key(KeyCode::Left);
        assert_eq!(f.cursor, 4);
    }

    #[test]
    fn datetime_right_from_separator() {
        let mut f = Field::datetime("label", "2025-01-01 12:00");
        f.cursor = 4;
        f.handle_key(KeyCode::Right);
        assert_eq!(f.cursor, 5);
    }

    // --- Form navigation ---

    #[test]
    fn form_tab_advances_focus() {
        let mut form = text_form(&["a", "b", "c"]);
        form.handle_key(key(KeyCode::Tab));
        assert_eq!(form.focused, 1);
    }

    #[test]
    fn form_tab_wraps_around() {
        let mut form = text_form(&["a", "b"]);
        form.focused = 1;
        form.handle_key(key(KeyCode::Tab));
        assert_eq!(form.focused, 0);
    }

    #[test]
    fn form_backtab_goes_back() {
        let mut form = text_form(&["a", "b", "c"]);
        form.focused = 2;
        form.handle_key(key(KeyCode::BackTab));
        assert_eq!(form.focused, 1);
    }

    #[test]
    fn form_backtab_wraps_to_last() {
        let mut form = text_form(&["a", "b"]);
        form.handle_key(key(KeyCode::BackTab));
        assert_eq!(form.focused, 1);
    }

    #[test]
    fn form_enter_advances_if_not_last() {
        let mut form = text_form(&["a", "b"]);
        let result = form.handle_key(key(KeyCode::Enter));
        assert!(matches!(result, FormResult::None));
        assert_eq!(form.focused, 1);
    }

    #[test]
    fn form_enter_submits_on_last_field() {
        let mut form = text_form(&["a", "b"]);
        form.focused = 1;
        let result = form.handle_key(key(KeyCode::Enter));
        assert!(matches!(result, FormResult::Submit));
    }

    #[test]
    fn form_esc_cancels() {
        let mut form = text_form(&["a"]);
        let result = form.handle_key(key(KeyCode::Esc));
        assert!(matches!(result, FormResult::Cancel));
    }

    #[test]
    fn form_char_inserts_into_focused_field() {
        let mut form = text_form(&["", ""]);
        form.handle_key(key(KeyCode::Char('x')));
        assert_eq!(form.fields[0].value, "x");
        assert_eq!(form.fields[1].value, "");
    }
}

impl Form {
    fn filtered_count(&self, idx: usize) -> usize {
        let f = &self.fields[idx];
        let q = f.value.to_lowercase();
        f.completions
            .iter()
            .filter(|c| q.is_empty() || c.to_lowercase().contains(&q))
            .count()
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> FormResult {
        match key.code {
            KeyCode::Esc => FormResult::Cancel,

            KeyCode::Down => {
                if !self.fields[self.focused].completions.is_empty() {
                    let count = self.filtered_count(self.focused);
                    if count > 0 {
                        let f = &mut self.fields[self.focused];
                        f.ac_index = Some(match f.ac_index {
                            None => 0,
                            Some(i) => (i + 1) % count,
                        });
                    }
                }
                FormResult::None
            }

            KeyCode::Up => {
                if self.fields[self.focused].ac_index.is_some() {
                    let count = self.filtered_count(self.focused);
                    let f = &mut self.fields[self.focused];
                    f.ac_index = Some(match f.ac_index {
                        None => 0,
                        Some(0) => count.saturating_sub(1),
                        Some(i) => i - 1,
                    });
                }
                FormResult::None
            }

            KeyCode::Enter => {
                if self.fields[self.focused].ac_index.is_some() {
                    self.fields[self.focused].apply_selected();
                    if self.focused < self.fields.len() - 1 {
                        self.focused += 1;
                    }
                    FormResult::None
                } else if self.focused == self.fields.len() - 1 {
                    FormResult::Submit
                } else {
                    self.focused += 1;
                    FormResult::None
                }
            }

            KeyCode::Tab => {
                if self.fields[self.focused].ac_index.is_some() {
                    self.fields[self.focused].apply_selected();
                }
                self.fields[self.focused].ac_index = None;
                self.focused = (self.focused + 1) % self.fields.len();
                FormResult::None
            }

            KeyCode::BackTab => {
                self.fields[self.focused].ac_index = None;
                self.focused = if self.focused == 0 {
                    self.fields.len() - 1
                } else {
                    self.focused - 1
                };
                FormResult::None
            }

            _ => {
                if !key.modifiers.contains(KeyModifiers::CONTROL)
                    && !key.modifiers.contains(KeyModifiers::ALT)
                {
                    self.fields[self.focused].handle_key(key.code);
                    self.fields[self.focused].ac_index = None;
                }
                FormResult::None
            }
        }
    }
}
