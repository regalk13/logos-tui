use color_eyre::Result;
use ratatui::{prelude::*, widgets::*};

use crate::{action::Action, app::Focus, bible::Bible, components::Component};
use arboard::Clipboard;

pub struct Reader {
    bible: Bible,
    book: String,
    chapter: u16,
    row: usize,
    col: usize,
    scroll: u16,
    visual: bool,
    anchor_row: usize,
    anchor_col: usize,
    clipboard: Clipboard,
}

impl Reader {
    pub fn new(bible: Bible) -> Self {
        Self {
            bible,
            book: "Genesis".into(),
            chapter: 1,
            row: 0,
            col: 0,
            scroll: 0,
            visual: false,
            anchor_row: 0,
            anchor_col: 0,
            clipboard: Clipboard::new().unwrap(),
        }
    }

    fn cur_line_len(&self) -> usize {
        self.bible
            .passage(&self.book, self.chapter)
            .get(self.row)
            .map(|v| v.text.len())
            .unwrap_or(0)
    }

    fn ensure_visible(&mut self) {
        let above = 3usize;
        let below = 3usize;
        if self.row < self.scroll as usize {
            self.scroll = self.row as u16;
        } else {
            let low = self.scroll as usize;
            let high = low + above + below;
            if self.row > high {
                self.scroll = self.row.saturating_sub(above + below) as u16;
            }
        }
    }

    fn normalized_range(&self) -> Vec<(usize, usize, usize)> {
        if !self.visual {
            let line_len = self.cur_line_len();
            return vec![(self.row, 0, line_len)];
        }
        let (ar, ac, br, bc) = if (self.anchor_row, self.anchor_col) <= (self.row, self.col) {
            (self.anchor_row, self.anchor_col, self.row, self.col)
        } else {
            (self.row, self.col, self.anchor_row, self.anchor_col)
        };
        let mut out = Vec::new();
        for r in ar..=br {
            let text_len = self
                .bible
                .passage(&self.book, self.chapter)
                .get(r)
                .map(|v| v.text.len())
                .unwrap_or(0);
            let s = if r == ar { ac } else { 0 };
            let e = if r == br { bc + 1 } else { text_len };
            out.push((r, s, e));
        }
        out
    }
}

impl Component for Reader {
    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::OpenPassage { book, chapter } => {
                self.book = book;
                self.chapter = chapter;
                self.row = 0;
                self.col = 0;
                self.scroll = 0;
                self.visual = false;
            }
            Action::MoveRow(dy) => {
                let total = self.bible.passage(&self.book, self.chapter).len() as i32;
                let new_row = ((self.row as i32 + dy).clamp(0, total - 1)) as usize;

                if dy > 0 && new_row > self.row && self.col >= self.cur_line_len() {
                    self.row = new_row;
                    self.col = 0;
                } else if dy < 0 && new_row < self.row && self.col == 0 {
                    self.row = new_row;
                    self.col = self.cur_line_len().saturating_sub(1);
                } else {
                    self.row = new_row;
                    let line_len = self.cur_line_len();
                    if line_len == 0 {
                        self.col = 0;
                    } else {
                        self.col = self.col.min(line_len.saturating_sub(1));
                    }
                }
                self.ensure_visible();
            }
            Action::MoveCol(dx) => {
                let line_len = self.cur_line_len();
                let verses = self.bible.passage(&self.book, self.chapter);

                if dx > 0 {
                    if self.col < line_len.saturating_sub(1) {
                        self.col += 1;
                    } else if self.row + 1 < verses.len() {
                        self.row += 1;
                        self.col = 0;
                        self.ensure_visible();
                    }
                } else if dx < 0 {
                    if self.col > 0 {
                        self.col -= 1;
                    } else if self.row > 0 {
                        self.row -= 1;
                        let prev_line_len = self.cur_line_len();
                        self.col = prev_line_len.saturating_sub(1);
                        self.ensure_visible();
                    }
                }
            }
            Action::ToggleVisual => {
                self.visual = !self.visual;
                if self.visual {
                    self.anchor_row = self.row;
                    self.anchor_col = self.col;
                }
            }
            Action::Yank => {
                let range = self.normalized_range();
                let mut buf = String::new();
                for (r, s, e) in range {
                    if let Some(v) = self.bible.passage(&self.book, self.chapter).get(r) {
                        buf.push_str(&v.text[s.min(v.text.len())..e.min(v.text.len())]);
                        buf.push('\n');
                    }
                }
                self.clipboard.set_text(buf.trim_end())?;
                self.visual = false;
            }
            _ => {}
        }
        Ok(None)
    }

    fn handle_key_event(&mut self, key: crossterm::event::KeyEvent) -> Result<Option<Action>> {
        use crossterm::event::KeyCode::*;
        match key.code {
            Char('j') | Down => Ok(Some(Action::MoveRow(1))),
            Char('k') | Up => Ok(Some(Action::MoveRow(-1))),
            Char('h') | Left => Ok(Some(Action::MoveCol(-1))),
            Char('l') | Right => Ok(Some(Action::MoveCol(1))),
            Char('v') => Ok(Some(Action::ToggleVisual)),
            Char('y') => Ok(Some(Action::Yank)),
            Esc => Ok(Some(Action::ToggleVisual)),
            _ => Ok(None),
        }
    }

    fn draw(&mut self, f: &mut Frame, area: Rect, focus: Focus) -> Result<()> {
        let verses = self.bible.passage(&self.book, self.chapter);

        let (ar, ac, br, bc) = if self.visual {
            if (self.anchor_row, self.anchor_col) <= (self.row, self.col) {
                (self.anchor_row, self.anchor_col, self.row, self.col)
            } else {
                (self.row, self.col, self.anchor_row, self.anchor_col)
            }
        } else {
            (self.row, self.col, self.row, self.col)
        };

        let mut lines = Vec::with_capacity(verses.len());
        for (i, v) in verses.iter().enumerate() {
            let mut spans = vec![Span::raw(format!("{:>3} ", v.verse))];

            if i < ar || i > br {
                spans.push(Span::raw(&v.text));
            } else {
                let start = if i == ar { ac } else { 0 };
                let end = if i == br { bc + 1 } else { v.text.len() };
                let (p1, rest) = v.text.split_at(start.min(v.text.len()));
                let (p2, p3) = rest.split_at((end - start).min(rest.len()));
                spans.extend([
                    Span::raw(p1),
                    Span::styled(p2, Style::default().bg(Color::DarkGray)),
                    Span::raw(p3),
                ]);
            }
            lines.push(Line::from(spans));
        }

        let border_style = if focus == Focus::Reader {
            Style::default().fg(Color::Green)
        } else {
            Style::default()
        };

        let para = Paragraph::new(lines)
            .block(
                Block::default()
                    .title(format!("{} {}", self.book, self.chapter))
                    .borders(Borders::ALL)
                    .border_style(border_style),
            )
            .scroll((self.scroll, 0))
            .wrap(Wrap { trim: true });

        f.render_widget(para, area);

        Ok(())
    }
}
