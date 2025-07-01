use color_eyre::Result;
use ratatui::{prelude::*, widgets::*};

use crate::{action::Action, app::Focus, bible::Bible, components::Component};

pub struct Reader {
    bible: Bible,
    book: String,
    chapter: u16,
    scroll: u16,
}
impl Reader {
    pub fn new(bible: Bible) -> Self {
        Self {
            bible,
            book: "Genesis".into(),
            chapter: 1,
            scroll: 0,
        }
    }
}
impl Component for Reader {
    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::OpenPassage { book, chapter } => {
                self.book = book;
                self.chapter = chapter;
                self.scroll = 0;
            }
            Action::Scroll(delta) => {
                self.scroll = self.scroll.saturating_add_signed(delta);
            }
            _ => {}
        }
        Ok(None)
    }
    fn handle_key_event(&mut self, key: crossterm::event::KeyEvent) -> Result<Option<Action>> {
        use crossterm::event::KeyCode::*;
        match key.code {
            Char('j') | Down => Ok(Some(Action::Scroll(1))),
            Char('k') | Up => Ok(Some(Action::Scroll(-1))),
            _ => Ok(None),
        }
    }
    fn draw(&mut self, f: &mut Frame, area: Rect, focus: Focus) -> Result<()> {
        let verses = self.bible.passage(&self.book, self.chapter);
        let text: Vec<Line> = verses
            .iter()
            .map(|v| Line::from(format!("{} {}:{} {}", v.book, v.chapter, v.verse, v.text)))
            .collect();

        let border_style = if focus == Focus::Reader {
            Style::default().fg(Color::Green)
        } else {
            Style::default()
        };

        let para = Paragraph::new(text)
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
