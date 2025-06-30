use color_eyre::Result;
use ratatui::{prelude::*, widgets::*};
use tokio::sync::mpsc::UnboundedSender;

use crate::{action::Action, bible::Bible, components::Component};

pub struct Index {
    items: Vec<(String, u16)>,
    selected: usize,
    list_state: ListState,
    action_tx: Option<UnboundedSender<Action>>,
}
impl Index {
    pub fn new(bible: Bible) -> Self {
        let items = bible.chapters();
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self {
            items,
            selected: 0,
            list_state,
            action_tx: None,
        }
    }
    fn send_select(&self) -> Result<()> {
        if let Some(tx) = &self.action_tx {
            let (book, chap) = &self.items[self.selected];
            tx.send(Action::OpenPassage {
                book: book.clone(),
                chapter: *chap,
            })?;
        }
        Ok(())
    }
}
impl Component for Index {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.action_tx = Some(tx);
        Ok(())
    }
    fn handle_key_event(&mut self, key: crossterm::event::KeyEvent) -> Result<Option<Action>> {
        use crossterm::event::KeyCode::*;
        match key.code {
            Up | Char('k') => {
                if self.selected > 0 {
                    self.selected -= 1;
                    self.send_select()?
                }
            }
            Down | Char('j') => {
                if self.selected + 1 < self.items.len() {
                    self.selected += 1;
                    self.send_select()?
                }
            }
            _ => {}
        }
        Ok(None)
    }
    fn draw(&mut self, f: &mut Frame, area: Rect) -> color_eyre::Result<()> {
        let items: Vec<ListItem> = self
            .items
            .iter()
            .enumerate()
            .map(|(i, (b, c))| {
                let text = format!("{b} {c}");
                if i == self.selected {
                    ListItem::new(text.bold())
                } else {
                    ListItem::new(text)
                }
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().title("Books").borders(Borders::ALL))
            .highlight_symbol(">");

        self.list_state.select(Some(self.selected));

        f.render_stateful_widget(list, area, &mut self.list_state);
        Ok(())
    }
}
