use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub struct Form {
    pub fields: Vec<Field>,
    pub focused: usize,
}

pub enum FieldKind {
    Text,
    Toggle,
    /// Fixed-format datetime field (e.g. "YYYY-MM-DD HH:MM").
    /// Separators (':', '-', ' ') cannot be deleted and are skipped by Left/Right.
    /// Digits are overwritten in-place; an empty field accepts free-form typing.
    Timestamp,
    /// Multi-line text. Enter inserts a newline; Alt+Enter submits the form.
    MultilineText,
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

    pub fn is_on(&self) -> bool {
        self.value == "true"
    }

    pub fn into_timestamp(mut self) -> Self {
        self.kind = FieldKind::Timestamp;
        self
    }

    pub fn into_multiline(mut self) -> Self {
        self.kind = FieldKind::MultilineText;
        self
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
            FieldKind::Timestamp => {
                let is_sep = |c: char| matches!(c, ':' | '-' | ' ');
                match code {
                    KeyCode::Left => {
                        if self.cursor > 0 {
                            let ch_len = self.value[..self.cursor]
                                .chars()
                                .last()
                                .map(|c| c.len_utf8())
                                .unwrap_or(1);
                            self.cursor -= ch_len;
                            // If we landed on a separator, skip one more
                            if self.cursor > 0 {
                                if let Some(c) = self.value[self.cursor..].chars().next() {
                                    if is_sep(c) {
                                        let ch_len = self.value[..self.cursor]
                                            .chars()
                                            .last()
                                            .map(|c| c.len_utf8())
                                            .unwrap_or(1);
                                        self.cursor -= ch_len;
                                    }
                                }
                            }
                        }
                    }
                    KeyCode::Right => {
                        if self.cursor < self.value.len() {
                            let ch_len = self.value[self.cursor..]
                                .chars()
                                .next()
                                .map(|c| c.len_utf8())
                                .unwrap_or(1);
                            self.cursor += ch_len;
                            // If we landed on a separator, skip one more
                            if self.cursor < self.value.len() {
                                if let Some(c) = self.value[self.cursor..].chars().next() {
                                    if is_sep(c) {
                                        self.cursor += c.len_utf8();
                                    }
                                }
                            }
                        }
                    }
                    KeyCode::Delete => {
                        if self.cursor < self.value.len() {
                            if let Some(c) = self.value[self.cursor..].chars().next() {
                                if !is_sep(c) {
                                    let len = c.len_utf8();
                                    self.value
                                        .replace_range(self.cursor..self.cursor + len, "_");
                                    // cursor stays put
                                }
                            }
                        }
                    }
                    KeyCode::Backspace => {
                        // Move left (skipping a separator), then blank the digit there.
                        if self.cursor > 0 {
                            let ch_len = self.value[..self.cursor]
                                .chars()
                                .last()
                                .map(|c| c.len_utf8())
                                .unwrap_or(1);
                            let mut target = self.cursor - ch_len;
                            if let Some(c) = self.value[target..].chars().next() {
                                if is_sep(c) && target > 0 {
                                    let ch_len2 = self.value[..target]
                                        .chars()
                                        .last()
                                        .map(|c| c.len_utf8())
                                        .unwrap_or(1);
                                    target -= ch_len2;
                                }
                            }
                            self.cursor = target;
                            if let Some(c) = self.value[self.cursor..].chars().next() {
                                if !is_sep(c) {
                                    let len = c.len_utf8();
                                    self.value
                                        .replace_range(self.cursor..self.cursor + len, "_");
                                }
                            }
                        }
                    }
                    KeyCode::Char(c) => {
                        if self.cursor >= self.value.len() {
                            // Cursor at end: insert mode, for typing a fresh timestamp.
                            self.insert(c);
                        } else if c.is_ascii_digit() {
                            // Cursor within existing value: overwrite current digit position.
                            if let Some(cur) = self.value[self.cursor..].chars().next() {
                                if !is_sep(cur) {
                                    let len = cur.len_utf8();
                                    self.value.replace_range(
                                        self.cursor..self.cursor + len,
                                        &c.to_string(),
                                    );
                                    self.cursor += 1;
                                    // Skip any separator we land on.
                                    if self.cursor < self.value.len() {
                                        if let Some(next) = self.value[self.cursor..].chars().next()
                                        {
                                            if is_sep(next) {
                                                self.cursor += next.len_utf8();
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        // Non-digit within filled field, or separator position: no-op.
                    }
                    KeyCode::Home => self.cursor = 0,
                    KeyCode::End => self.cursor = self.value.len(),
                    _ => {}
                }
            }
            FieldKind::MultilineText => match code {
                KeyCode::Char(c) => self.insert(c),
                KeyCode::Enter => self.insert('\n'),
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
                KeyCode::Home => {
                    let line_start = self.value[..self.cursor]
                        .rfind('\n')
                        .map(|i| i + 1)
                        .unwrap_or(0);
                    self.cursor = line_start;
                }
                KeyCode::End => {
                    let line_end = self.value[self.cursor..]
                        .find('\n')
                        .map(|i| self.cursor + i)
                        .unwrap_or(self.value.len());
                    self.cursor = line_end;
                }
                KeyCode::Up => {
                    let line_start = self.value[..self.cursor]
                        .rfind('\n')
                        .map(|i| i + 1)
                        .unwrap_or(0);
                    if line_start == 0 {
                        self.cursor = 0;
                    } else {
                        let col = self.cursor - line_start;
                        let prev_nl = line_start - 1;
                        let prev_line_start = self.value[..prev_nl]
                            .rfind('\n')
                            .map(|i| i + 1)
                            .unwrap_or(0);
                        let prev_line_len = prev_nl - prev_line_start;
                        self.cursor = prev_line_start + col.min(prev_line_len);
                        while self.cursor < self.value.len()
                            && !self.value.is_char_boundary(self.cursor)
                        {
                            self.cursor += 1;
                        }
                    }
                }
                KeyCode::Down => {
                    let line_start = self.value[..self.cursor]
                        .rfind('\n')
                        .map(|i| i + 1)
                        .unwrap_or(0);
                    let col = self.cursor - line_start;
                    if let Some(offset) = self.value[self.cursor..].find('\n') {
                        let next_line_start = self.cursor + offset + 1;
                        let next_line_end = self.value[next_line_start..]
                            .find('\n')
                            .map(|i| next_line_start + i)
                            .unwrap_or(self.value.len());
                        let next_line_len = next_line_end - next_line_start;
                        self.cursor = next_line_start + col.min(next_line_len);
                        while self.cursor < self.value.len()
                            && !self.value.is_char_boundary(self.cursor)
                        {
                            self.cursor += 1;
                        }
                    } else {
                        self.cursor = self.value.len();
                    }
                }
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

            KeyCode::Enter if key.modifiers.contains(KeyModifiers::ALT) => {
                FormResult::Submit
            }

            KeyCode::Down => {
                let focused_is_multiline =
                    matches!(self.fields[self.focused].kind, FieldKind::MultilineText);
                if focused_is_multiline {
                    self.fields[self.focused].handle_key(KeyCode::Down);
                } else {
                    let has_completions = !self.fields[self.focused].completions.is_empty();
                    if has_completions {
                        let count = self.filtered_count(self.focused);
                        if count > 0 {
                            let f = &mut self.fields[self.focused];
                            f.ac_index = Some(match f.ac_index {
                                None => 0,
                                Some(i) => (i + 1) % count,
                            });
                        }
                    } else {
                        // No completions: Down navigates to the next field.
                        self.fields[self.focused].ac_index = None;
                        self.focused = (self.focused + 1) % self.fields.len();
                    }
                }
                FormResult::None
            }

            KeyCode::Up => {
                let focused_is_multiline =
                    matches!(self.fields[self.focused].kind, FieldKind::MultilineText);
                if focused_is_multiline {
                    self.fields[self.focused].handle_key(KeyCode::Up);
                } else if self.fields[self.focused].ac_index.is_some() {
                    // Navigate backwards within an open autocomplete list.
                    let count = self.filtered_count(self.focused);
                    let f = &mut self.fields[self.focused];
                    f.ac_index = Some(match f.ac_index {
                        None => count.saturating_sub(1),
                        Some(0) => count.saturating_sub(1),
                        Some(i) => i - 1,
                    });
                } else {
                    // No active autocomplete: Up navigates to the previous field.
                    self.fields[self.focused].ac_index = None;
                    self.focused = if self.focused == 0 {
                        self.fields.len() - 1
                    } else {
                        self.focused - 1
                    };
                }
                FormResult::None
            }

            KeyCode::Enter => {
                let focused_is_multiline =
                    matches!(self.fields[self.focused].kind, FieldKind::MultilineText);
                if focused_is_multiline {
                    self.fields[self.focused].handle_key(KeyCode::Enter);
                    FormResult::None
                } else if self.fields[self.focused].ac_index.is_some() {
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

    fn ts(value: &str) -> Field {
        Field::new("label", value).into_timestamp()
    }

    // --- Timestamp field ---

    #[test]
    fn ts_left_skips_separator() {
        let mut f = ts("12:34");
        f.cursor = 3; // on '3'
        f.handle_key(KeyCode::Left);
        assert_eq!(f.cursor, 1); // on '2', skipped ':'
    }

    #[test]
    fn ts_right_skips_separator() {
        let mut f = ts("12:34");
        f.cursor = 1; // on '2'
        f.handle_key(KeyCode::Right);
        assert_eq!(f.cursor, 3); // on '3', skipped ':'
    }

    #[test]
    fn ts_left_at_first_char_is_noop() {
        let mut f = ts("12:34");
        f.cursor = 0;
        f.handle_key(KeyCode::Left);
        assert_eq!(f.cursor, 0);
    }

    #[test]
    fn ts_right_at_last_char_stops_at_end() {
        let mut f = ts("12:34");
        f.cursor = 4; // on '4'
        f.handle_key(KeyCode::Right);
        assert_eq!(f.cursor, 5); // past end
    }

    #[test]
    fn ts_delete_blanks_digit() {
        let mut f = ts("12:34");
        f.cursor = 1; // on '2'
        f.handle_key(KeyCode::Delete);
        assert_eq!(f.value, "1_:34");
        assert_eq!(f.cursor, 1); // stays put
    }

    #[test]
    fn ts_delete_on_separator_is_noop() {
        let mut f = ts("12:34");
        f.cursor = 2; // on ':'
        f.handle_key(KeyCode::Delete);
        assert_eq!(f.value, "12:34");
        assert_eq!(f.cursor, 2);
    }

    #[test]
    fn ts_backspace_blanks_prev_digit_and_moves_left() {
        let mut f = ts("12:34");
        f.cursor = 3; // on '3'
        f.handle_key(KeyCode::Backspace);
        assert_eq!(f.value, "1_:34");
        assert_eq!(f.cursor, 1); // moved to '2' position (now '_')
    }

    #[test]
    fn ts_backspace_skips_separator_when_backing_up() {
        let mut f = ts("12:34");
        f.cursor = 3; // on '3'; prev char is ':', skip it to reach '2'
        f.handle_key(KeyCode::Backspace);
        assert_eq!(f.cursor, 1);
    }

    #[test]
    fn ts_char_digit_overwrites_in_place_and_advances() {
        let mut f = ts("12:34");
        f.cursor = 1; // on '2'
        f.handle_key(KeyCode::Char('9'));
        assert_eq!(f.value, "19:34");
        assert_eq!(f.cursor, 3); // skipped ':' at 2, now on '3'
    }

    #[test]
    fn ts_char_digit_on_separator_is_noop() {
        let mut f = ts("12:34");
        f.cursor = 2; // on ':'
        f.handle_key(KeyCode::Char('9'));
        assert_eq!(f.value, "12:34");
        assert_eq!(f.cursor, 2);
    }

    #[test]
    fn ts_char_on_empty_field_inserts() {
        let mut f = ts("");
        f.handle_key(KeyCode::Char('0'));
        f.handle_key(KeyCode::Char('9'));
        f.handle_key(KeyCode::Char(':'));
        f.handle_key(KeyCode::Char('3'));
        f.handle_key(KeyCode::Char('0'));
        assert_eq!(f.value, "09:30");
    }

    #[test]
    fn ts_full_datetime_left_right_skips_all_seps() {
        // "2024-06-19 09:30": '-' at 4, '-' at 7, ' ' at 10, ':' at 13
        let mut f = ts("2024-06-19 09:30");
        f.cursor = 5; // on '0' of '06'
        f.handle_key(KeyCode::Left);
        assert_eq!(f.cursor, 3); // on '4' of '2024', skipped '-'
        f.cursor = 9; // on '9' of '19'
        f.handle_key(KeyCode::Right);
        assert_eq!(f.cursor, 11); // on '0' of '09', skipped ' '
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
        let f = Field::new("label", "").with_completions(vec!["alpha".into(), "beta".into()]);
        assert_eq!(f.suggestions_colored().len(), 2);
    }

    #[test]
    fn suggestions_filters_by_query() {
        let f = Field::new("label", "al").with_completions(vec!["alpha".into(), "beta".into()]);
        let s = f.suggestions_colored();
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].0, "alpha");
    }

    #[test]
    fn apply_selected_fills_value_and_clears_index() {
        let mut f = Field::new("label", "").with_completions(vec!["alpha".into(), "beta".into()]);
        f.ac_index = Some(1);
        let applied = f.apply_selected();
        assert!(applied);
        assert_eq!(f.value, "beta");
        assert_eq!(f.cursor, 4);
        assert!(f.ac_index.is_none());
    }

    #[test]
    fn apply_selected_noop_when_no_index() {
        let mut f = Field::new("label", "").with_completions(vec!["alpha".into()]);
        let applied = f.apply_selected();
        assert!(!applied);
    }

    // --- Form navigation ---

    // --- Up / Down field navigation ---

    #[test]
    fn down_navigates_field_when_no_completions() {
        let mut form = text_form(&["a", "b", "c"]);
        form.handle_key(key(KeyCode::Down));
        assert_eq!(form.focused, 1);
    }

    #[test]
    fn down_wraps_to_first_field_when_no_completions() {
        let mut form = text_form(&["a", "b"]);
        form.focused = 1;
        form.handle_key(key(KeyCode::Down));
        assert_eq!(form.focused, 0);
    }

    #[test]
    fn up_navigates_to_previous_field_when_no_ac_index() {
        let mut form = text_form(&["a", "b", "c"]);
        form.focused = 2;
        form.handle_key(key(KeyCode::Up));
        assert_eq!(form.focused, 1);
    }

    #[test]
    fn up_wraps_to_last_field_when_no_ac_index() {
        let mut form = text_form(&["a", "b"]);
        form.handle_key(key(KeyCode::Up));
        assert_eq!(form.focused, 1);
    }

    #[test]
    fn up_navigates_autocomplete_backwards_when_ac_index_set() {
        let mut form = Form {
            fields: vec![Field::new("label", "a").with_completions(vec![
                "alpha".into(),
                "beta".into(),
                "gamma".into(),
            ])],
            focused: 0,
        };
        form.fields[0].ac_index = Some(2);
        form.handle_key(key(KeyCode::Up));
        assert_eq!(form.fields[0].ac_index, Some(1));
        assert_eq!(form.focused, 0); // field did not change
    }

    #[test]
    fn up_wraps_autocomplete_to_last_from_zero() {
        let mut form = Form {
            fields: vec![Field::new("label", "").with_completions(vec![
                "alpha".into(),
                "beta".into(),
                "gamma".into(),
            ])],
            focused: 0,
        };
        form.fields[0].ac_index = Some(0);
        form.handle_key(key(KeyCode::Up));
        assert_eq!(form.fields[0].ac_index, Some(2));
    }

    #[test]
    fn down_opens_autocomplete_when_completions_present() {
        let mut form = Form {
            fields: vec![Field::new("label", "").with_completions(vec!["alpha".into()])],
            focused: 0,
        };
        form.handle_key(key(KeyCode::Down));
        assert_eq!(form.fields[0].ac_index, Some(0));
        assert_eq!(form.focused, 0); // stays on same field
    }

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
