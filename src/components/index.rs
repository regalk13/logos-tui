use color_eyre::Result;
use ratatui::{prelude::*, widgets::*};
use tokio::sync::mpsc::UnboundedSender;

use crate::{action::Action, app::Focus, bible::Bible, components::Component};

pub struct Index {
    all_items: Vec<(String, u16)>,
    items: Vec<(String, u16)>,
    selected: usize,
    list_state: ListState,
    mode: Mode,
    action_tx: Option<UnboundedSender<Action>>,
}

enum Mode {
    Normal,
    Filtering { query: String },
}

impl Index {
    pub fn new(bible: Bible) -> Self {
        let all_items = bible.chapters();
        let items = all_items.clone();
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self {
            all_items,
            items,
            selected: 0,
            list_state,
            mode: Mode::Normal,
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

    fn apply_filter(&mut self, query: &str) {
        if query.is_empty() {
            self.items = self.all_items.clone();
        } else {
            let q = query.to_ascii_lowercase();
            self.items = self
                .all_items
                .iter()
                .filter(|(b, _)| b.to_ascii_lowercase().contains(&q))
                .cloned()
                .collect();
        }

        if self.selected >= self.items.len() {
            self.selected = self.items.len().saturating_sub(1);
        }
    }
}
impl Component for Index {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.action_tx = Some(tx);
        Ok(())
    }
    fn handle_key_event(&mut self, key: crossterm::event::KeyEvent) -> Result<Option<Action>> {
        use crossterm::event::KeyCode::*;
        match &mut self.mode {
            Mode::Normal => match key.code {
                Char('/') => {
                    self.mode = Mode::Filtering {
                        query: String::new(),
                    };
                }
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
            },
            Mode::Filtering { query } => match key.code {
                Esc => {
                    self.mode = Mode::Normal;
                }
                Enter => {
                    self.mode = Mode::Normal;
                }
                Backspace => {
                    query.pop();
                    let query_clone = query.clone();
                    self.apply_filter(&query_clone);
                }
                Up => {
                    if self.selected > 0 {
                        self.selected -= 1;
                        self.send_select()?
                    }
                }
                Down => {
                    if self.selected + 1 < self.items.len() {
                        self.selected += 1;
                        self.send_select()?
                    }
                }
                Char(c) => {
                    query.push(c);
                    let query_clone = query.clone();
                    self.apply_filter(&query_clone);
                }
                _ => {}
            },
        }
        Ok(None)
    }
    fn draw(&mut self, f: &mut Frame, area: Rect, focus: Focus) -> color_eyre::Result<()> {
        let (list_area, _) = if let Mode::Filtering { query } = &self.mode {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(0)])
                .split(area);

            let input = Paragraph::new(format!("/{query}"))
                .block(Block::default().borders(Borders::ALL).title("Filter"));
            f.render_widget(input, chunks[0]);

            (chunks[1], true)
        } else {
            (area, false)
        };

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

        let border_style = if focus == Focus::Index {
            Style::default().fg(Color::Green)
        } else {
            Style::default()
        };

        let list = List::new(items)
            .block(
                Block::default()
                    .title("Books")
                    .borders(Borders::ALL)
                    .border_style(border_style),
            )
            .highlight_symbol(">");

        self.list_state.select(Some(self.selected));

        f.render_stateful_widget(list, list_area, &mut self.list_state);
        Ok(())
    }
}
