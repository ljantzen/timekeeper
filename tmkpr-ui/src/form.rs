use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub struct Form {
    pub fields: Vec<Field>,
    pub focused: usize,
}

pub struct Field {
    pub label: &'static str,
    pub value: String,
    pub cursor: usize,
    pub completions: Vec<String>,
    pub completion_colors: Vec<Option<String>>,
    pub ac_index: Option<usize>,
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
        }
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
        match code {
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
