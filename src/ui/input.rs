pub struct InputBuffer {
    text: String,
    cursor: usize,
    selection: Option<(usize, usize)>,
}

impl InputBuffer {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            cursor: 0,
            selection: None,
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
        self.selection = None;
    }

    pub fn set_text(&mut self, text: &str) {
        self.text = text.to_string();
        self.cursor = self.text.len();
        self.selection = None;
    }

    #[allow(dead_code)]
    pub fn selection_range(&self) -> Option<(usize, usize)> {
        self.selection
    }

    #[allow(dead_code)]
    pub fn selected_text(&self) -> Option<String> {
        let (start, end) = self.selection?;
        if start < end {
            Some(self.text[start..end].to_string())
        } else {
            Some(self.text[end..start].to_string())
        }
    }

    fn clamp_cursor(&self, pos: usize) -> usize {
        pos.min(self.text.len())
    }

    pub fn insert_char(&mut self, ch: char) {
        self.delete_selection();
        self.text.insert(self.cursor, ch);
        self.cursor += ch.len_utf8();
        self.cursor = self.clamp_cursor(self.cursor);
    }

    #[allow(dead_code)]
    pub fn insert_str(&mut self, s: &str) {
        self.delete_selection();
        self.text.insert_str(self.cursor, s);
        self.cursor += s.len();
        self.cursor = self.clamp_cursor(self.cursor);
    }

    pub fn backspace(&mut self) {
        if self.delete_selection() {
            return;
        }
        if self.cursor > 0 {
            let prev = self.cursor;
            self.cursor = self.cursor.saturating_sub(1);
            self.cursor = self.char_boundary_left(prev);
            self.text.drain(self.cursor..prev);
            self.selection = None;
        }
    }

    pub fn delete(&mut self) {
        if self.delete_selection() {
            return;
        }
        if self.cursor < self.text.len() {
            let next = self.char_boundary_right(self.cursor);
            self.text.drain(self.cursor..next);
            self.selection = None;
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
        self.selection = None;
    }

    pub fn delete_word_forward(&mut self) {
        if self.cursor >= self.text.len() {
            return;
        }
        let end = self.word_boundary_right();
        if end > self.cursor {
            self.text.drain(self.cursor..end);
        }
        self.selection = None;
    }

    pub fn move_left(&mut self, select: bool) {
        if self.cursor > 0 {
            let prev = self.cursor;
            self.cursor = self.char_boundary_left(prev);
        }
        self.update_selection_on_move(select);
    }

    pub fn move_right(&mut self, select: bool) {
        if self.cursor < self.text.len() {
            self.cursor = self.char_boundary_right(self.cursor);
        }
        self.update_selection_on_move(select);
    }

    pub fn move_word_left(&mut self, select: bool) {
        self.cursor = self.word_boundary_left();
        self.update_selection_on_move(select);
    }

    pub fn move_word_right(&mut self, select: bool) {
        self.cursor = self.word_boundary_right();
        self.update_selection_on_move(select);
    }

    pub fn move_home(&mut self, select: bool) {
        self.cursor = 0;
        self.update_selection_on_move(select);
    }

    pub fn move_end(&mut self, select: bool) {
        self.cursor = self.text.len();
        self.update_selection_on_move(select);
    }

    fn update_selection_on_move(&mut self, select: bool) {
        if select {
            let start = self
                .selection
                .map(|(s, _)| s)
                .unwrap_or(self.cursor);
            self.selection = Some((start, self.cursor));
        } else {
            self.selection = None;
        }
    }

    fn delete_selection(&mut self) -> bool {
        if let Some((start, end)) = self.selection {
            let (lo, hi) = if start < end { (start, end) } else { (end, start) };
            if lo < hi {
                self.text.drain(lo..hi);
                self.cursor = lo;
                self.selection = None;
                return true;
            }
        }
        false
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

        let bytes = self.text.as_bytes();
        let mut pos = self.cursor.saturating_sub(1);

        while pos > 0 && bytes[pos] == b' ' {
            pos -= 1;
        }
        while pos > 0 && bytes[pos] != b' ' {
            pos -= 1;
        }

        if pos > 0 && bytes[pos] == b' ' {
            pos += 1;
        }

        if !self.text.is_char_boundary(pos) {
            self.char_boundary_right(pos)
        } else {
            pos
        }
    }

    fn word_boundary_right(&self) -> usize {
        if self.cursor >= self.text.len() {
            return self.text.len();
        }

        let bytes = self.text.as_bytes();
        let mut pos = self.cursor;

        while pos < self.text.len() && bytes[pos] == b' ' {
            pos += 1;
        }
        while pos < self.text.len() && bytes[pos] != b' ' {
            pos += 1;
        }

        pos
    }
}
