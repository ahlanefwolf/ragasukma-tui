use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::theme::Theme;

/// Mode perilaku input teks untuk menangani baris baru (newline) dan tombol Enter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextInputMode {
    /// Field satu baris (modal pencarian, nama project, API key, base URL, dll).
    /// Karakter '\n' difilter/dibersihkan saat paste; Enter selalu memicu `InputResult::Submit`.
    #[default]
    SingleLine,
    /// Field multi baris dengan perilaku Submit on Enter (khusus Prompt Agent).
    /// Bare Enter memicu `InputResult::Submit`; Alt+J (atau Shift+Enter jika didukung) memasukkan '\n'.
    MultiLinePrompt,
    /// Field multi baris editor kode/konfigurasi (JSON/Markdown di MCP Server).
    /// Enter memasukkan karakter '\n'; Submit dilakukan via tombol Save atau shortcut form terpisah.
    MultiLineEditor,
}

/// Hasil dari pemrosesan suatu event keyboard oleh `TextInput`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputResult {
    /// Karakter diketik, dihapus, atau kursor dipindahkan
    Consumed,
    /// Pengguna menekan Enter (submit)
    Submit,
    /// Pengguna menekan Alt+J / Shift+Enter untuk newline di mode multiline
    NewlineInserted,
    /// Pengguna menekan Tab atau Down (pindah ke field berikutnya)
    NextField,
    /// Pengguna menekan BackTab (Shift+Tab) atau Up (pindah ke field sebelumnya)
    PrevField,
    /// Pengguna menekan Esc (batalkan / tutup form)
    Cancelled,
    /// Tombol tidak dikenali / diabaikan oleh input field
    Ignored,
}

/// Komponen terpadu pengelola teks reaktif di terminal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextInput {
    pub value: String,
    pub cursor: usize,          // Byte offset kursor pada string (0 <= cursor <= value.len())
    pub scroll: usize,          // Horizontal character offset untuk teks panjang
    pub placeholder: String,    // Placeholder saat value kosong
    pub is_focused: bool,       // Status apakah sedang menerima keyboard input
    pub is_password: bool,      // Masking karakter (misal API key: "••••••")
    pub max_length: Option<usize>,
    pub mode: TextInputMode,    // Mode penanganan newline dan Enter
}

impl Default for TextInput {
    fn default() -> Self {
        Self {
            value: String::new(),
            cursor: 0,
            scroll: 0,
            placeholder: String::new(),
            is_focused: false,
            is_password: false,
            max_length: None,
            mode: TextInputMode::SingleLine,
        }
    }
}

impl TextInput {
    /// Membuat instance baru TextInput dengan mode default `SingleLine`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Membuat instance baru TextInput khusus field single line.
    #[must_use]
    pub fn new_single_line() -> Self {
        Self::default()
    }

    /// Membuat instance baru TextInput khusus multiline prompt (Agent prompt).
    #[must_use]
    pub fn new_multiline_prompt() -> Self {
        Self {
            mode: TextInputMode::MultiLinePrompt,
            ..Self::default()
        }
    }

    /// Membuat instance baru TextInput khusus multiline editor (JSON/Markdown config).
    #[must_use]
    pub fn new_multiline_editor() -> Self {
        Self {
            mode: TextInputMode::MultiLineEditor,
            ..Self::default()
        }
    }

    /// Alias untuk `new_multiline_editor`.
    #[must_use]
    pub fn new_multiline() -> Self {
        Self::new_multiline_editor()
    }

    #[must_use]
    pub fn with_placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    #[must_use]
    pub fn with_value(mut self, value: impl Into<String>) -> Self {
        self.set_value(value);
        self
    }

    #[must_use]
    pub fn with_password(mut self, is_password: bool) -> Self {
        self.is_password = is_password;
        self
    }

    #[must_use]
    pub fn with_max_length(mut self, max_length: usize) -> Self {
        self.max_length = Some(max_length);
        self
    }

    #[must_use]
    pub fn with_mode(mut self, mode: TextInputMode) -> Self {
        self.mode = mode;
        self
    }

    #[must_use]
    pub fn with_focus(mut self, is_focused: bool) -> Self {
        self.is_focused = is_focused;
        self
    }

    pub fn focus(&mut self) {
        self.is_focused = true;
    }

    pub fn unfocus(&mut self) {
        self.is_focused = false;
    }

    /// Memastikan kursor berada di batas byte UTF-8 (`char_boundary`) yang sah dan tidak melampaui string.
    pub fn ensure_char_boundary(&mut self) {
        if self.cursor > self.value.len() {
            self.cursor = self.value.len();
        }
        while !self.value.is_char_boundary(self.cursor) {
            self.cursor = self.cursor.saturating_sub(1);
        }
    }

    /// Mengisi nilai baru ke buffer teks dan memposisikan kursor di akhir buffer.
    pub fn set_value(&mut self, val: impl Into<String>) {
        self.value = val.into();
        self.cursor = self.value.len();
        self.scroll = 0;
        self.ensure_char_boundary();
    }

    /// Mengembalikan representasi teks tampilan (dimask jika `is_password = true`).
    #[must_use]
    pub fn display_text(&self) -> String {
        if self.is_password {
            "•".repeat(self.value.chars().count())
        } else {
            self.value.clone()
        }
    }

    /// Mengosongkan buffer teks dan mereset kursor.
    pub fn clear(&mut self) {
        self.value.clear();
        self.cursor = 0;
        self.scroll = 0;
    }

    /// Menambahkan satu karakter UTF-8 pada posisi kursor saat ini.
    pub fn insert_char(&mut self, c: char) {
        if c == '\r' {
            return;
        }
        if c == '\n' && self.mode == TextInputMode::SingleLine {
            return;
        }
        if let Some(max) = self.max_length
            && self.value.chars().count() >= max
        {
            return;
        }
        self.ensure_char_boundary();
        self.value.insert(self.cursor, c);
        self.cursor += c.len_utf8();
    }

    /// Memasukkan teks paste dengan sanitasi newline sesuai mode.
    pub fn paste(&mut self, text: &str) {
        let sanitized: String = match self.mode {
            TextInputMode::SingleLine => text.chars().filter(|&c| c != '\r' && c != '\n').collect(),
            TextInputMode::MultiLinePrompt | TextInputMode::MultiLineEditor => {
                text.replace("\r\n", "\n").replace('\r', "")
            }
        };

        if sanitized.is_empty() {
            return;
        }

        self.ensure_char_boundary();

        let to_insert: String = if let Some(max) = self.max_length {
            let current_chars = self.value.chars().count();
            if current_chars >= max {
                return;
            }
            let available = max - current_chars;
            sanitized.chars().take(available).collect()
        } else {
            sanitized
        };

        let insert_len = to_insert.len();
        self.value.insert_str(self.cursor, &to_insert);
        self.cursor += insert_len;
        self.ensure_char_boundary();
    }

    /// Menghapus satu karakter UTF-8 di belakang kursor (Backspace).
    pub fn backspace(&mut self) {
        self.ensure_char_boundary();
        if self.cursor > 0 {
            let prev_len = self.value[..self.cursor]
                .chars()
                .next_back()
                .map_or(0, |c| c.len_utf8());
            if prev_len > 0 {
                let start = self.cursor - prev_len;
                self.value.drain(start..self.cursor);
                self.cursor = start;
            }
        }
    }

    /// Menghapus satu karakter UTF-8 di depan kursor (Delete).
    pub fn delete_forward(&mut self) {
        self.ensure_char_boundary();
        if self.cursor < self.value.len() {
            let next_len = self.value[self.cursor..]
                .chars()
                .next()
                .map_or(0, |c| c.len_utf8());
            if next_len > 0 {
                self.value.drain(self.cursor..self.cursor + next_len);
            }
        }
    }

    /// Menghapus satu kata ke belakang (Ctrl+W / Readline unix-word-rubout).
    pub fn delete_word_backward(&mut self) {
        self.ensure_char_boundary();
        if self.cursor == 0 {
            return;
        }
        let s = &self.value[..self.cursor];
        let mut chars = s.char_indices().rev().peekable();

        // 1. Lewati spasi di sebelah kiri kursor jika ada
        while let Some(&(_, c)) = chars.peek() {
            if c.is_whitespace() {
                chars.next();
            } else {
                break;
            }
        }

        // 2. Lewati karakter non-spasi hingga menemukan batas kata atau awal string
        let mut new_cursor = 0;
        for (idx, c) in chars {
            if c.is_whitespace() {
                new_cursor = idx + c.len_utf8();
                break;
            }
            new_cursor = idx;
        }

        self.value.drain(new_cursor..self.cursor);
        self.cursor = new_cursor;
        self.ensure_char_boundary();
    }

    /// Menghapus satu kata ke depan (Alt+D / Delete word forward).
    pub fn delete_word_forward(&mut self) {
        self.ensure_char_boundary();
        if self.cursor >= self.value.len() {
            return;
        }
        let s = &self.value[self.cursor..];
        let mut chars = s.char_indices().peekable();

        // 1. Lewati spasi di sebelah kanan kursor jika ada
        while let Some(&(_, c)) = chars.peek() {
            if c.is_whitespace() {
                chars.next();
            } else {
                break;
            }
        }

        // 2. Lewati karakter non-spasi hingga menemukan batas kata atau akhir string
        let mut target_offset = s.len();
        for (idx, c) in chars {
            if c.is_whitespace() {
                target_offset = idx;
                break;
            }
        }

        let end_idx = self.cursor + target_offset;
        self.value.drain(self.cursor..end_idx);
        self.ensure_char_boundary();
    }

    /// Menghapus teks dari awal baris ke posisi kursor saat ini (Ctrl+U).
    pub fn clear_to_start(&mut self) {
        self.ensure_char_boundary();
        if self.cursor == 0 {
            return;
        }
        let line_start = self.value[..self.cursor]
            .rfind('\n')
            .map_or(0, |idx| idx + 1);
        self.value.drain(line_start..self.cursor);
        self.cursor = line_start;
        self.ensure_char_boundary();
    }

    /// Menghapus teks dari posisi kursor ke akhir baris (Ctrl+K).
    pub fn clear_to_end(&mut self) {
        self.ensure_char_boundary();
        if self.cursor >= self.value.len() {
            return;
        }
        let rest = &self.value[self.cursor..];
        let end_idx = if rest.starts_with('\n') {
            self.cursor + 1
        } else {
            self.cursor + rest.find('\n').unwrap_or(rest.len())
        };
        self.value.drain(self.cursor..end_idx);
        self.ensure_char_boundary();
    }

    /// Memindahkan kursor satu karakter ke kiri.
    pub fn cursor_left(&mut self) {
        self.ensure_char_boundary();
        if self.cursor > 0 {
            let prev_len = self.value[..self.cursor]
                .chars()
                .next_back()
                .map_or(0, |c| c.len_utf8());
            self.cursor = self.cursor.saturating_sub(prev_len);
        }
    }

    /// Memindahkan kursor satu karakter ke kanan.
    pub fn cursor_right(&mut self) {
        self.ensure_char_boundary();
        if self.cursor < self.value.len() {
            let next_len = self.value[self.cursor..]
                .chars()
                .next()
                .map_or(0, |c| c.len_utf8());
            self.cursor = (self.cursor + next_len).min(self.value.len());
        }
    }

    /// Memindahkan kursor ke awal baris (Home / Ctrl+A).
    pub fn cursor_home(&mut self) {
        self.ensure_char_boundary();
        self.cursor = self.value[..self.cursor]
            .rfind('\n')
            .map_or(0, |idx| idx + 1);
    }

    /// Memindahkan kursor ke akhir baris (End / Ctrl+E).
    pub fn cursor_end(&mut self) {
        self.ensure_char_boundary();
        self.cursor =
            self.cursor + self.value[self.cursor..].find('\n').unwrap_or(self.value[self.cursor..].len());
    }

    /// Memindahkan kursor satu kata ke kiri (Alt+Left / Alt+B).
    pub fn word_left(&mut self) {
        self.ensure_char_boundary();
        if self.cursor == 0 {
            return;
        }
        let s = &self.value[..self.cursor];
        let mut chars = s.char_indices().rev().peekable();

        while let Some(&(_, c)) = chars.peek() {
            if c.is_whitespace() {
                chars.next();
            } else {
                break;
            }
        }

        let mut target = 0;
        for (idx, c) in chars {
            if c.is_whitespace() {
                target = idx + c.len_utf8();
                break;
            }
            target = idx;
        }
        self.cursor = target;
        self.ensure_char_boundary();
    }

    /// Memindahkan kursor satu kata ke kanan (Alt+Right / Alt+F).
    pub fn word_right(&mut self) {
        self.ensure_char_boundary();
        if self.cursor >= self.value.len() {
            return;
        }
        let s = &self.value[self.cursor..];
        let mut chars = s.char_indices().peekable();

        while let Some(&(_, c)) = chars.peek() {
            if c.is_whitespace() {
                chars.next();
            } else {
                break;
            }
        }

        let mut target_offset = s.len();
        for (idx, c) in chars {
            if c.is_whitespace() {
                target_offset = idx;
                break;
            }
        }
        self.cursor += target_offset;
        self.ensure_char_boundary();
    }

    /// Memindahkan kursor ke baris sebelumnya pada teks multiline.
    /// Mengembalikan `true` bila kursor berhasil berpindah baris, atau `false` bila sudah di baris pertama.
    pub fn cursor_up(&mut self) -> bool {
        self.ensure_char_boundary();
        let cur_line_start = self.value[..self.cursor]
            .rfind('\n')
            .map_or(0, |idx| idx + 1);
        if cur_line_start == 0 {
            return false;
        }
        let col = self.value[cur_line_start..self.cursor].chars().count();
        let prev_newline_idx = cur_line_start - 1;
        let prev_line_start = self.value[..prev_newline_idx]
            .rfind('\n')
            .map_or(0, |idx| idx + 1);
        let prev_line_str = &self.value[prev_line_start..prev_newline_idx];
        let prev_col_offset: usize = prev_line_str
            .chars()
            .take(col)
            .map(|c| c.len_utf8())
            .sum();
        self.cursor = prev_line_start + prev_col_offset;
        self.ensure_char_boundary();
        true
    }

    /// Memindahkan kursor ke baris berikutnya pada teks multiline.
    /// Mengembalikan `true` bila kursor berhasil berpindah baris, atau `false` bila sudah di baris terakhir.
    pub fn cursor_down(&mut self) -> bool {
        self.ensure_char_boundary();
        let cur_line_start = self.value[..self.cursor]
            .rfind('\n')
            .map_or(0, |idx| idx + 1);
        let col = self.value[cur_line_start..self.cursor].chars().count();
        let Some(newline_offset) = self.value[self.cursor..].find('\n') else {
            return false;
        };
        let next_line_start = self.cursor + newline_offset + 1;
        let next_line_end = self.value[next_line_start..]
            .find('\n')
            .map_or(self.value.len(), |idx| next_line_start + idx);
        let next_line_str = &self.value[next_line_start..next_line_end];
        let next_col_offset: usize = next_line_str
            .chars()
            .take(col)
            .map(|c| c.len_utf8())
            .sum();
        self.cursor = next_line_start + next_col_offset;
        self.ensure_char_boundary();
        true
    }

    /// Memproses event keyboard dan memperbarui internal state teks serta posisi kursor.
    pub fn handle_key(&mut self, key: KeyEvent) -> InputResult {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let alt = key.modifiers.contains(KeyModifiers::ALT);
        let shift = key.modifiers.contains(KeyModifiers::SHIFT);

        // 1. Esc: Cancel
        if key.code == KeyCode::Esc {
            return InputResult::Cancelled;
        }

        // 2. Tab / BackTab navigation
        if key.code == KeyCode::Tab {
            return InputResult::NextField;
        }
        if key.code == KeyCode::BackTab {
            return InputResult::PrevField;
        }

        // 3. Alt+J: Deterministik Newline untuk Multiline
        if alt && !ctrl && matches!(key.code, KeyCode::Char('j') | KeyCode::Char('J')) {
            return match self.mode {
                TextInputMode::MultiLinePrompt | TextInputMode::MultiLineEditor => {
                    self.insert_char('\n');
                    InputResult::NewlineInserted
                }
                TextInputMode::SingleLine => InputResult::Ignored,
            };
        }

        // 4. Enter handling
        if key.code == KeyCode::Enter {
            // Shift+Enter / Ctrl+Enter:
            if shift || ctrl {
                return match self.mode {
                    TextInputMode::MultiLinePrompt => {
                        // Swallowed dead key di raw mode terminal untuk mencegah pengiriman pesan prematur
                        InputResult::Consumed
                    }
                    TextInputMode::MultiLineEditor => {
                        self.insert_char('\n');
                        InputResult::NewlineInserted
                    }
                    TextInputMode::SingleLine => InputResult::Ignored,
                };
            }

            // Bare Enter:
            return match self.mode {
                TextInputMode::SingleLine => InputResult::Submit,
                TextInputMode::MultiLinePrompt => InputResult::Submit,
                TextInputMode::MultiLineEditor => {
                    self.insert_char('\n');
                    InputResult::NewlineInserted
                }
            };
        }

        // 5. Readline & Control Shortcuts
        if ctrl && !alt {
            match key.code {
                KeyCode::Char('w') | KeyCode::Char('W') => {
                    self.delete_word_backward();
                    return InputResult::Consumed;
                }
                KeyCode::Char('u') | KeyCode::Char('U') => {
                    self.clear_to_start();
                    return InputResult::Consumed;
                }
                KeyCode::Char('k') | KeyCode::Char('K') => {
                    self.clear_to_end();
                    return InputResult::Consumed;
                }
                KeyCode::Char('a') | KeyCode::Char('A') => {
                    self.cursor_home();
                    return InputResult::Consumed;
                }
                KeyCode::Char('e') | KeyCode::Char('E') => {
                    self.cursor_end();
                    return InputResult::Consumed;
                }
                KeyCode::Char('h') | KeyCode::Char('H') => {
                    self.backspace();
                    return InputResult::Consumed;
                }
                KeyCode::Backspace => {
                    self.delete_word_backward();
                    return InputResult::Consumed;
                }
                KeyCode::Left => {
                    self.word_left();
                    return InputResult::Consumed;
                }
                KeyCode::Right => {
                    self.word_right();
                    return InputResult::Consumed;
                }
                _ => return InputResult::Ignored,
            }
        }

        // 6. Alt Navigation & Editing Shortcuts
        if alt && !ctrl {
            match key.code {
                KeyCode::Left | KeyCode::Char('b') | KeyCode::Char('B') => {
                    self.word_left();
                    return InputResult::Consumed;
                }
                KeyCode::Right | KeyCode::Char('f') | KeyCode::Char('F') => {
                    self.word_right();
                    return InputResult::Consumed;
                }
                KeyCode::Char('d') | KeyCode::Char('D') => {
                    self.delete_word_forward();
                    return InputResult::Consumed;
                }
                _ => return InputResult::Ignored,
            }
        }

        // 7. General navigation and character editing keys
        match key.code {
            KeyCode::Left => {
                self.cursor_left();
                InputResult::Consumed
            }
            KeyCode::Right => {
                self.cursor_right();
                InputResult::Consumed
            }
            KeyCode::Up => {
                if self.cursor_up() {
                    InputResult::Consumed
                } else {
                    InputResult::PrevField
                }
            }
            KeyCode::Down => {
                if self.cursor_down() {
                    InputResult::Consumed
                } else {
                    InputResult::NextField
                }
            }
            KeyCode::Home => {
                self.cursor_home();
                InputResult::Consumed
            }
            KeyCode::End => {
                self.cursor_end();
                InputResult::Consumed
            }
            KeyCode::Backspace => {
                self.backspace();
                InputResult::Consumed
            }
            KeyCode::Delete => {
                self.delete_forward();
                InputResult::Consumed
            }
            KeyCode::Char(c) if !ctrl && !alt => {
                self.insert_char(c);
                InputResult::Consumed
            }
            _ => InputResult::Ignored,
        }
    }

    /// Mengembalikan indeks karakter (0-indexed char count) dari kursor saat ini.
    #[must_use]
    pub fn cursor_char_offset(&self) -> usize {
        let valid_cursor = self.cursor.min(self.value.len());
        self.value[..valid_cursor].chars().count()
    }

    /// Mengembalikan posisi baris dan kolom kursor (0-indexed line, 0-indexed col dalam karakter).
    #[must_use]
    pub fn cursor_line_and_col(&self) -> (usize, usize) {
        let valid_cursor = self.cursor.min(self.value.len());
        let prefix = &self.value[..valid_cursor];
        let line = prefix.chars().filter(|&c| c == '\n').count();
        let last_nl = prefix.rfind('\n').map_or(0, |idx| idx + 1);
        let col = prefix[last_nl..].chars().count();
        (line, col)
    }

    /// Mengembalikan representasi teks tampilan (memperhitungkan masking password jika aktif).
    #[must_use]
    pub fn display_content(&self) -> String {
        if self.is_password {
            "•".repeat(self.value.chars().count())
        } else {
            self.value.clone()
        }
    }

    /// Menghitung horizontal scroll offset agar indeks kursor tetap terlihat di dalam viewport.
    #[must_use]
    pub fn calculate_scroll(
        cursor_char_idx: usize,
        available_width: usize,
        current_scroll: usize,
    ) -> usize {
        if available_width == 0 {
            return 0;
        }
        let mut scroll = current_scroll;
        if cursor_char_idx < scroll {
            scroll = cursor_char_idx;
        } else if cursor_char_idx >= scroll + available_width {
            scroll = cursor_char_idx + 1 - available_width;
        }
        scroll
    }

    /// Memperbarui nilai `scroll` pada `self` agar kursor saat ini berada di dalam lebar area yang tersedia.
    pub fn update_scroll(&mut self, available_width: usize) {
        let cursor_char_idx = self.cursor_char_offset();
        self.scroll = Self::calculate_scroll(cursor_char_idx, available_width, self.scroll);
    }

    /// Helper untuk merender input teks ke frame.
    pub fn render(&self, frame: &mut Frame, area: Rect, title: &str, theme: &Theme) {
        render_text_input(frame, area, self, title, theme);
    }
}

/// Merender widget `TextInput` ke dalam frame terminal dengan block, border responsif,
/// styling placeholder, masking password, dan pengaturan kursor terminal presisi.
pub fn render_text_input(
    frame: &mut Frame,
    area: Rect,
    input: &TextInput,
    title: &str,
    _theme: &Theme,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let border_style = if input.is_focused {
        Theme::border_focused()
    } else {
        Theme::border_subtle()
    };

    let mut block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style);

    if !title.is_empty() {
        let title_style = if input.is_focused {
            Theme::badge_gold()
        } else {
            Style::default().fg(Theme::TEXT_SECONDARY)
        };
        block = block.title(Line::from(vec![
            Span::raw(" "),
            Span::styled(title, title_style),
            Span::raw(" "),
        ]));
    }

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    if inner_area.width == 0 || inner_area.height == 0 {
        return;
    }

    let available_width = inner_area.width as usize;
    let available_height = inner_area.height as usize;

    if input.value.is_empty() {
        if !input.placeholder.is_empty() {
            let ph_style = if input.is_focused {
                Style::default()
                    .fg(Theme::TEXT_MUTED)
                    .add_modifier(Modifier::ITALIC)
            } else {
                Style::default()
                    .fg(Theme::TEXT_SECONDARY)
                    .add_modifier(Modifier::ITALIC)
            };
            let ph_slice: String = input.placeholder.chars().take(available_width).collect();
            let para = Paragraph::new(Line::from(Span::styled(ph_slice, ph_style)));
            frame.render_widget(para, inner_area);
        }

        if input.is_focused {
            frame.set_cursor_position((inner_area.x, inner_area.y));
        }
        return;
    }

    if input.mode == TextInputMode::SingleLine || available_height == 1 {
        let display_str = input.display_content();
        let cursor_char_idx = input.cursor_char_offset();
        let scroll = TextInput::calculate_scroll(cursor_char_idx, available_width, input.scroll);
        let visible_slice: String = display_str
            .chars()
            .skip(scroll)
            .take(available_width)
            .collect();

        let text_style = Style::default().fg(Theme::TEXT_PRIMARY);
        let para = Paragraph::new(Line::from(Span::styled(visible_slice, text_style)));
        frame.render_widget(para, inner_area);

        if input.is_focused {
            let cursor_offset = (cursor_char_idx.saturating_sub(scroll)) as u16;
            let max_col = inner_area.width.saturating_sub(1);
            let cursor_x = inner_area.x + cursor_offset.min(max_col);
            let cursor_y = inner_area.y;
            frame.set_cursor_position((cursor_x, cursor_y));
        }
    } else {
        let display_str = input.display_content();
        let (cursor_line, cursor_col) = input.cursor_line_and_col();

        let v_scroll = if cursor_line < input.scroll {
            cursor_line
        } else if cursor_line >= input.scroll + available_height {
            cursor_line + 1 - available_height
        } else {
            input.scroll
        };

        let all_lines: Vec<&str> = display_str.split('\n').collect();
        let mut rendered_lines = Vec::with_capacity(available_height);

        for (line_idx, line) in all_lines
            .into_iter()
            .enumerate()
            .skip(v_scroll)
            .take(available_height)
        {
            let h_scroll = if line_idx == cursor_line {
                TextInput::calculate_scroll(cursor_col, available_width, 0)
            } else {
                0
            };
            let visible_slice: String = line.chars().skip(h_scroll).take(available_width).collect();
            rendered_lines.push(Line::from(Span::styled(
                visible_slice,
                Style::default().fg(Theme::TEXT_PRIMARY),
            )));
        }

        let para = Paragraph::new(rendered_lines);
        frame.render_widget(para, inner_area);

        if input.is_focused {
            let h_scroll = TextInput::calculate_scroll(cursor_col, available_width, 0);
            let cursor_col_clamped = (cursor_col.saturating_sub(h_scroll) as u16)
                .min(inner_area.width.saturating_sub(1));
            let cursor_row_clamped = (cursor_line.saturating_sub(v_scroll) as u16)
                .min(inner_area.height.saturating_sub(1));
            let cursor_x = inner_area.x + cursor_col_clamped;
            let cursor_y = inner_area.y + cursor_row_clamped;
            frame.set_cursor_position((cursor_x, cursor_y));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn key_ctrl(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::CONTROL)
    }

    fn key_alt(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::ALT)
    }

    fn key_shift(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::SHIFT)
    }

    #[test]
    fn test_text_input_creation_and_defaults() {
        let input = TextInput::new();
        assert_eq!(input.value, "");
        assert_eq!(input.cursor, 0);
        assert_eq!(input.mode, TextInputMode::SingleLine);
        assert!(!input.is_focused);
        assert!(!input.is_password);
        assert_eq!(input.display_text(), "");

        let prompt = TextInput::new_multiline_prompt()
            .with_placeholder("Ketik pesan...")
            .with_focus(true);
        assert_eq!(prompt.mode, TextInputMode::MultiLinePrompt);
        assert_eq!(prompt.placeholder, "Ketik pesan...");
        assert!(prompt.is_focused);

        let editor = TextInput::new_multiline_editor()
            .with_value("{\n  \"key\": \"value\"\n}");
        assert_eq!(editor.mode, TextInputMode::MultiLineEditor);
        assert_eq!(editor.cursor, editor.value.len());
    }

    #[test]
    fn test_insert_ascii_and_multibyte_utf8() {
        let mut input = TextInput::new();

        input.insert_char('h');
        input.insert_char('a');
        input.insert_char('l');
        input.insert_char('o');
        assert_eq!(input.value, "halo");
        assert_eq!(input.cursor, 4);

        // UTF-8 multibyte characters (Indonesian, Japanese, Emojis)
        input.insert_char(' ');
        input.insert_char('d');
        input.insert_char('u');
        input.insert_char('n');
        input.insert_char('i');
        input.insert_char('a');
        input.insert_char(' ');
        input.insert_char('🌍'); // 4 bytes
        assert_eq!(input.value, "halo dunia 🌍");
        assert_eq!(input.cursor, 11 + 4);

        // Test cursor movement over multibyte UTF-8
        input.cursor_left();
        assert_eq!(input.cursor, 11);
        input.cursor_right();
        assert_eq!(input.cursor, 15);

        // Backspace multibyte UTF-8
        input.backspace();
        assert_eq!(input.value, "halo dunia ");
        assert_eq!(input.cursor, 11);

        // Delete forward multibyte UTF-8
        input.cursor_left(); // at ' ' (idx 10)
        input.cursor_left(); // at 'a' (idx 9)
        input.insert_char('🔥'); // 4 bytes at idx 9
        assert_eq!(input.value, "halo duni🔥a ");
        input.cursor_left(); // back to idx 9 before '🔥'
        assert_eq!(input.cursor, 9);
        input.delete_forward(); // deletes '🔥'
        assert_eq!(input.value, "halo dunia ");
        assert_eq!(input.cursor, 9);
    }

    #[test]
    fn test_edge_cases_cursor_bounds() {
        let mut input = TextInput::new();

        // Operasi pada string kosong
        input.backspace();
        assert_eq!(input.cursor, 0);
        input.delete_forward();
        assert_eq!(input.cursor, 0);
        input.cursor_left();
        assert_eq!(input.cursor, 0);
        input.cursor_right();
        assert_eq!(input.cursor, 0);
        input.cursor_home();
        assert_eq!(input.cursor, 0);
        input.cursor_end();
        assert_eq!(input.cursor, 0);
        input.clear_to_start();
        assert_eq!(input.cursor, 0);
        input.clear_to_end();
        assert_eq!(input.cursor, 0);
        input.delete_word_backward();
        assert_eq!(input.cursor, 0);
        input.delete_word_forward();
        assert_eq!(input.cursor, 0);

        // Operasi di akhir string
        input.set_value("ragasukma");
        assert_eq!(input.cursor, 9);
        input.cursor_right();
        assert_eq!(input.cursor, 9);
        input.delete_forward();
        assert_eq!(input.value, "ragasukma");
        assert_eq!(input.cursor, 9);
        input.clear_to_end();
        assert_eq!(input.value, "ragasukma");

        // Operasi di awal string
        input.cursor_home();
        assert_eq!(input.cursor, 0);
        input.cursor_left();
        assert_eq!(input.cursor, 0);
        input.backspace();
        assert_eq!(input.value, "ragasukma");
        assert_eq!(input.cursor, 0);
        input.clear_to_start();
        assert_eq!(input.value, "ragasukma");
        assert_eq!(input.cursor, 0);
    }

    #[test]
    fn test_paste_single_line_sanitization() {
        let mut input = TextInput::new_single_line();
        input.paste("line1\r\nline2\nline3\rline4");
        // Di single-line mode, newline dibuang
        assert_eq!(input.value, "line1line2line3line4");
        assert_eq!(input.cursor, input.value.len());

        let mut pwd_input = TextInput::new_single_line().with_password(true);
        pwd_input.paste("secret_token");
        assert_eq!(pwd_input.value, "secret_token");
        assert_eq!(pwd_input.display_text(), "••••••••••••");
    }

    #[test]
    fn test_paste_multiline_mode() {
        let mut input = TextInput::new_multiline_prompt();
        input.paste("function test() {\r\n    return 42;\r\n}");
        assert_eq!(input.value, "function test() {\n    return 42;\n}");
        assert_eq!(input.cursor, input.value.len());
    }

    #[test]
    fn test_max_length_enforcement() {
        let mut input = TextInput::new().with_max_length(5);
        input.paste("hello world");
        assert_eq!(input.value, "hello");
        assert_eq!(input.cursor, 5);

        input.insert_char('!');
        assert_eq!(input.value, "hello");
    }

    #[test]
    fn test_readline_delete_word_backward_ctrl_w() {
        let mut input = TextInput::new().with_value("cargo build --bin ragasukma");
        assert_eq!(input.cursor, 27);

        // Ctrl+W: hapus "ragasukma"
        input.handle_key(key_ctrl(KeyCode::Char('w')));
        assert_eq!(input.value, "cargo build --bin ");
        assert_eq!(input.cursor, 18);

        // Ctrl+W: hapus "--bin "
        input.handle_key(key_ctrl(KeyCode::Char('w')));
        assert_eq!(input.value, "cargo build ");
        assert_eq!(input.cursor, 12);

        // Ctrl+W: hapus "build "
        input.handle_key(key_ctrl(KeyCode::Char('w')));
        assert_eq!(input.value, "cargo ");
        assert_eq!(input.cursor, 6);

        // Ctrl+W: hapus "cargo "
        input.handle_key(key_ctrl(KeyCode::Char('w')));
        assert_eq!(input.value, "");
        assert_eq!(input.cursor, 0);

        // Ctrl+W pada buffer kosong tidak panik
        input.handle_key(key_ctrl(KeyCode::Char('w')));
        assert_eq!(input.value, "");
        assert_eq!(input.cursor, 0);
    }

    #[test]
    fn test_readline_clear_to_start_and_end() {
        let mut input = TextInput::new().with_value("hello brave new world");
        input.cursor = 11; // setelah "brave " ("hello brave| new world")

        // Ctrl+U: hapus hingga awal baris
        input.handle_key(key_ctrl(KeyCode::Char('u')));
        assert_eq!(input.value, " new world");
        assert_eq!(input.cursor, 0);

        // Ctrl+K: hapus hingga akhir baris
        input.handle_key(key_ctrl(KeyCode::Char('k')));
        assert_eq!(input.value, "");
        assert_eq!(input.cursor, 0);
    }

    #[test]
    fn test_readline_clear_to_start_and_end_multiline() {
        let mut input = TextInput::new_multiline_editor()
            .with_value("first line\nsecond line\nthird line");
        // Posisi kursor di baris kedua pada kata "line"
        // "first line\nsecond line" -> index 11 + 7 = 18 ("second ")
        input.cursor = 18;

        // Ctrl+U: hapus dari awal baris kedua ("second ")
        input.clear_to_start();
        assert_eq!(input.value, "first line\nline\nthird line");
        assert_eq!(input.cursor, 11);

        // Ctrl+K: hapus sisa baris kedua ("line")
        input.clear_to_end();
        assert_eq!(input.value, "first line\n\nthird line");
        assert_eq!(input.cursor, 11);

        // Ctrl+K lagi di atas '\n': hapus karakter newline
        input.clear_to_end();
        assert_eq!(input.value, "first line\nthird line");
        assert_eq!(input.cursor, 11);
    }

    #[test]
    fn test_readline_home_end_and_word_navigation() {
        let mut input = TextInput::new().with_value("hello to the terminal world");
        input.cursor = 12; // di "the"

        // Home (Ctrl+A)
        input.handle_key(key_ctrl(KeyCode::Char('a')));
        assert_eq!(input.cursor, 0);

        // End (Ctrl+E)
        input.handle_key(key_ctrl(KeyCode::Char('e')));
        assert_eq!(input.cursor, input.value.len());

        // Alt+B / Alt+Left (word backward)
        input.handle_key(key_alt(KeyCode::Char('b')));
        assert_eq!(input.cursor, 22); // awal kata "world"

        input.handle_key(key_alt(KeyCode::Left));
        assert_eq!(input.cursor, 13); // awal kata "terminal"

        // Alt+F / Alt+Right (word forward)
        input.handle_key(key_alt(KeyCode::Char('f')));
        assert_eq!(input.cursor, 21); // akhir kata "terminal"

        input.handle_key(key_alt(KeyCode::Right));
        assert_eq!(input.cursor, 27); // akhir kata "world"
    }

    #[test]
    fn test_multiline_cursor_up_down() {
        let mut input = TextInput::new_multiline_prompt()
            .with_value("abc\ndefghi\njk");
        // abc (0..3), \n (3), defghi (4..10), \n (10), jk (11..13)

        input.cursor = 7; // pada 'f' di baris kedua (col 3)
        assert!(input.cursor_up());
        assert_eq!(input.cursor, 3); // col 3 di baris 1 adalah akhir "abc"

        // Dari baris 1, cursor_up mengembalikan false
        assert!(!input.cursor_up());

        // Pindah ke bawah
        assert!(input.cursor_down()); // ke baris 2
        assert_eq!(input.cursor, 7); // col 3 di baris 2 ("def")
        assert!(input.cursor_down()); // ke baris 3
        assert_eq!(input.cursor, 13); // baris 3 hanya 2 char ("jk"), col 3 diclamp ke 13

        // Dari baris 3, cursor_down mengembalikan false
        assert!(!input.cursor_down());
    }

    #[test]
    fn test_handle_key_newline_and_enter_policies() {
        // 1. SingleLine:
        let mut single = TextInput::new_single_line();
        assert_eq!(single.handle_key(key(KeyCode::Enter)), InputResult::Submit);
        assert_eq!(single.handle_key(key_alt(KeyCode::Char('j'))), InputResult::Ignored);
        assert_eq!(single.handle_key(key(KeyCode::Tab)), InputResult::NextField);
        assert_eq!(single.handle_key(key(KeyCode::BackTab)), InputResult::PrevField);
        assert_eq!(single.handle_key(key(KeyCode::Esc)), InputResult::Cancelled);

        // 2. MultiLinePrompt:
        let mut prompt = TextInput::new_multiline_prompt();
        // Bare Enter -> Submit
        assert_eq!(prompt.handle_key(key(KeyCode::Enter)), InputResult::Submit);
        // Shift+Enter / Ctrl+Enter -> Swallowed dead key
        assert_eq!(prompt.handle_key(key_shift(KeyCode::Enter)), InputResult::Consumed);
        assert_eq!(prompt.handle_key(key_ctrl(KeyCode::Enter)), InputResult::Consumed);
        // Alt+J -> Insert newline
        assert_eq!(prompt.handle_key(key_alt(KeyCode::Char('j'))), InputResult::NewlineInserted);
        assert_eq!(prompt.value, "\n");
        assert_eq!(prompt.cursor, 1);

        // 3. MultiLineEditor:
        let mut editor = TextInput::new_multiline_editor();
        // Bare Enter -> Insert newline
        assert_eq!(editor.handle_key(key(KeyCode::Enter)), InputResult::NewlineInserted);
        assert_eq!(editor.value, "\n");
        // Alt+J -> Insert newline
        assert_eq!(editor.handle_key(key_alt(KeyCode::Char('j'))), InputResult::NewlineInserted);
        assert_eq!(editor.value, "\n\n");
    }

    #[test]
    fn test_handle_key_typing_characters() {
        let mut input = TextInput::new();
        assert_eq!(input.handle_key(key(KeyCode::Char('a'))), InputResult::Consumed);
        assert_eq!(input.handle_key(key(KeyCode::Char('b'))), InputResult::Consumed);
        assert_eq!(input.handle_key(key(KeyCode::Char('c'))), InputResult::Consumed);
        assert_eq!(input.value, "abc");

        // Backspace
        assert_eq!(input.handle_key(key(KeyCode::Backspace)), InputResult::Consumed);
        assert_eq!(input.value, "ab");

        // Left arrow
        assert_eq!(input.handle_key(key(KeyCode::Left)), InputResult::Consumed);
        assert_eq!(input.cursor, 1);

        // Insert in middle
        assert_eq!(input.handle_key(key(KeyCode::Char('z'))), InputResult::Consumed);
        assert_eq!(input.value, "azb");
        assert_eq!(input.cursor, 2);
    }

    #[test]
    fn test_calculate_scroll_bounds() {
        assert_eq!(TextInput::calculate_scroll(5, 0, 0), 0);
        // Cursor inside viewport
        assert_eq!(TextInput::calculate_scroll(0, 10, 0), 0);
        assert_eq!(TextInput::calculate_scroll(5, 10, 0), 0);
        assert_eq!(TextInput::calculate_scroll(9, 10, 0), 0);
        // Cursor at boundary or past
        assert_eq!(TextInput::calculate_scroll(10, 10, 0), 1);
        assert_eq!(TextInput::calculate_scroll(15, 10, 0), 6);
        // Cursor moved left
        assert_eq!(TextInput::calculate_scroll(3, 10, 6), 3);
    }

    #[test]
    fn test_cursor_line_and_col_and_display() {
        let mut input = TextInput::new_multiline_editor().with_value("hello\nworld!");
        input.cursor = 0;
        assert_eq!(input.cursor_line_and_col(), (0, 0));
        input.cursor = 5; // right after "hello"
        assert_eq!(input.cursor_line_and_col(), (0, 5));
        input.cursor = 6; // start of "world!"
        assert_eq!(input.cursor_line_and_col(), (1, 0));
        input.cursor = 8; // "wo"
        assert_eq!(input.cursor_line_and_col(), (1, 2));

        let pwd = TextInput::new().with_value("secret").with_password(true);
        assert_eq!(pwd.display_content(), "••••••");
    }

    #[test]
    fn test_render_text_input_empty_placeholder_and_cursor() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let input = TextInput::new_single_line()
            .with_placeholder("Enter name...")
            .with_focus(true);

        let backend = TestBackend::new(30, 3);
        let mut terminal = Terminal::new(backend).expect("terminal created");
        terminal
            .draw(|f| {
                render_text_input(f, f.area(), &input, "Profile", &Theme);
            })
            .expect("draw succeeded");

        // Cursor should be at (inner_x, inner_y) = (1, 1)
        assert_eq!(terminal.get_cursor_position().ok(), Some(ratatui::layout::Position::new(1, 1)));
    }

    #[test]
    fn test_render_text_input_unfocused_no_cursor() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let input = TextInput::new_single_line()
            .with_value("Some value")
            .with_focus(false);

        let backend = TestBackend::new(30, 3);
        let mut terminal = Terminal::new(backend).expect("terminal created");
        terminal.set_cursor_position((99, 99)).expect("set cursor");
        terminal
            .draw(|f| {
                render_text_input(f, f.area(), &input, "Profile", &Theme);
            })
            .expect("draw succeeded");

        // When not focused, cursor was not set by render_text_input, remains (99, 99)
        assert_eq!(terminal.get_cursor_position().ok(), Some(ratatui::layout::Position::new(99, 99)));
    }

    #[test]
    fn test_render_text_input_password_masking() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let input = TextInput::new_single_line()
            .with_value("super_secret_key")
            .with_password(true)
            .with_focus(true);

        let backend = TestBackend::new(30, 3);
        let mut terminal = Terminal::new(backend).expect("terminal created");
        terminal
            .draw(|f| {
                render_text_input(f, f.area(), &input, "API Key", &Theme);
            })
            .expect("draw succeeded");

        let buffer_str = format!("{:?}", terminal.backend().buffer());
        assert!(buffer_str.contains('•'));
        assert!(!buffer_str.contains("super_secret_key"));
    }

    #[test]
    fn test_render_text_input_horizontal_autoscroll() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        // Area width 10 -> inner width 8
        let mut input = TextInput::new_single_line()
            .with_value("0123456789ABCDEF") // 16 chars
            .with_focus(true);
        input.cursor = input.value.len(); // cursor at 16

        let backend = TestBackend::new(10, 3);
        let mut terminal = Terminal::new(backend).expect("terminal created");
        terminal
            .draw(|f| {
                render_text_input(f, f.area(), &input, "", &Theme);
            })
            .expect("draw succeeded");

        // Inner area x starts at 1, max_col is 7 (inner width 8)
        // Cursor x should be at 1 + 7 = 8
        assert_eq!(terminal.get_cursor_position().ok(), Some(ratatui::layout::Position::new(8, 1)));
    }

    #[test]
    fn test_render_text_input_multiline_editor() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let mut input = TextInput::new_multiline_editor()
            .with_value("first line\nsecond line\nthird line")
            .with_focus(true);
        // Place cursor on second line, 3rd char ("sec")
        input.cursor = "first line\nsec".len();

        let backend = TestBackend::new(30, 5);
        let mut terminal = Terminal::new(backend).expect("terminal created");
        terminal
            .draw(|f| {
                render_text_input(f, f.area(), &input, "Config", &Theme);
            })
            .expect("draw succeeded");

        // Cursor line is 1 (second line), col is 3
        // inner_x = 1, inner_y = 1
        // cursor_x = 1 + 3 = 4, cursor_y = 1 + 1 = 2
        assert_eq!(terminal.get_cursor_position().ok(), Some(ratatui::layout::Position::new(4, 2)));
    }
}