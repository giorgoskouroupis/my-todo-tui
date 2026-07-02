pub struct InputBuffer {
    text: String,
    cursor: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum WordCharKind {
    Whitespace,
    Word,
    Separator,
}

fn word_char_kind(ch: char) -> WordCharKind {
    if ch.is_whitespace() {
        WordCharKind::Whitespace
    } else if ch.is_alphanumeric() || ch == '_' {
        WordCharKind::Word
    } else {
        WordCharKind::Separator
    }
}

impl InputBuffer {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            cursor: 0,
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    pub fn clear(&mut self) {
        self.text.clear();
        self.cursor = 0;
    }

    pub fn set_text(&mut self, text: &str) {
        self.text = text.to_string();
        self.cursor = self.text.len();
    }

    fn clamp_cursor(&self, pos: usize) -> usize {
        pos.min(self.text.len())
    }

    pub fn insert_char(&mut self, ch: char) {
        self.text.insert(self.cursor, ch);
        self.cursor += ch.len_utf8();
        self.cursor = self.clamp_cursor(self.cursor);
    }

    pub fn insert_str(&mut self, s: &str) {
        self.text.insert_str(self.cursor, s);
        self.cursor += s.len();
        self.cursor = self.clamp_cursor(self.cursor);
    }

    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            let prev = self.cursor;
            self.cursor = self.cursor.saturating_sub(1);
            self.cursor = self.char_boundary_left(prev);
            self.text.drain(self.cursor..prev);
        }
    }

    pub fn delete(&mut self) {
        if self.cursor < self.text.len() {
            let next = self.char_boundary_right(self.cursor);
            self.text.drain(self.cursor..next);
        }
    }

    pub fn delete_word_back(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let start = self.word_boundary_left();
        if start < self.cursor {
            self.text.drain(start..self.cursor);
            self.cursor = start;
        }
    }

    pub fn delete_word_forward(&mut self) {
        if self.cursor >= self.text.len() {
            return;
        }
        let end = self.word_boundary_right();
        if end > self.cursor {
            self.text.drain(self.cursor..end);
        }
    }

    pub fn move_left(&mut self) {
        if self.cursor > 0 {
            let prev = self.cursor;
            self.cursor = self.char_boundary_left(prev);
        }
    }

    pub fn move_right(&mut self) {
        if self.cursor < self.text.len() {
            self.cursor = self.char_boundary_right(self.cursor);
        }
    }

    pub fn move_word_left(&mut self) {
        self.cursor = self.word_boundary_left();
    }

    pub fn move_word_right(&mut self) {
        self.cursor = self.word_boundary_right();
    }

    pub fn move_home(&mut self) {
        self.cursor = 0;
    }

    pub fn move_end(&mut self) {
        self.cursor = self.text.len();
    }

    fn char_boundary_left(&self, pos: usize) -> usize {
        if pos <= 1 {
            return 0;
        }
        let mut p = pos.saturating_sub(1);
        while p > 0 && !self.text.is_char_boundary(p) {
            p -= 1;
        }
        p
    }

    fn char_boundary_right(&self, pos: usize) -> usize {
        let mut p = pos + 1;
        while p < self.text.len() && !self.text.is_char_boundary(p) {
            p += 1;
        }
        p
    }

    fn word_boundary_left(&self) -> usize {
        if self.cursor == 0 {
            return 0;
        }

        let chars: Vec<(usize, char)> = self.text[..self.cursor].char_indices().collect();
        let mut idx = chars.len();

        while idx > 0 && word_char_kind(chars[idx - 1].1) == WordCharKind::Whitespace {
            idx -= 1;
        }
        if idx == 0 {
            return 0;
        }

        let target = word_char_kind(chars[idx - 1].1);
        while idx > 0 && word_char_kind(chars[idx - 1].1) == target {
            idx -= 1;
        }

        chars.get(idx).map_or(0, |(pos, _)| *pos)
    }

    fn word_boundary_right(&self) -> usize {
        if self.cursor >= self.text.len() {
            return self.text.len();
        }

        let chars: Vec<(usize, char)> = self.text[self.cursor..]
            .char_indices()
            .map(|(pos, ch)| (self.cursor + pos, ch))
            .collect();
        let mut idx = 0;

        while idx < chars.len() && word_char_kind(chars[idx].1) == WordCharKind::Whitespace {
            idx += 1;
        }
        if idx == chars.len() {
            return self.text.len();
        }

        let target = word_char_kind(chars[idx].1);
        while idx < chars.len() && word_char_kind(chars[idx].1) == target {
            idx += 1;
        }

        chars.get(idx).map_or(self.text.len(), |(pos, _)| *pos)
    }
}

#[cfg(test)]
mod tests {
    use super::InputBuffer;

    #[test]
    fn delete_word_back_stops_at_separator() {
        let mut input = InputBuffer::new();
        input.insert_str("alpha/beta");

        input.delete_word_back();
        assert_eq!(input.text(), "alpha/");

        input.delete_word_back();
        assert_eq!(input.text(), "alpha");
    }

    #[test]
    fn delete_word_forward_stops_at_separator() {
        let mut input = InputBuffer::new();
        input.insert_str("alpha/beta");
        input.move_home();

        input.delete_word_forward();
        assert_eq!(input.text(), "/beta");

        input.delete_word_forward();
        assert_eq!(input.text(), "beta");
    }

    #[test]
    fn word_delete_skips_whitespace_before_deleting_next_class() {
        let mut input = InputBuffer::new();
        input.insert_str("alpha / beta");

        input.delete_word_back();
        assert_eq!(input.text(), "alpha / ");

        input.delete_word_back();
        assert_eq!(input.text(), "alpha ");
    }
}
