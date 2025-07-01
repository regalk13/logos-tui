use color_eyre::Result;
use crossterm::event::KeyEvent;
use ratatui::prelude::{Constraint, Rect};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::{debug, info};

use crate::{
    action::Action,
    bible::Bible,
    components::{Component, fps::FpsCounter, index::Index, reader::Reader},
    config::Config,
    tui::{Event, Tui},
};

pub struct App {
    config: Config,
    tick_rate: f64,
    frame_rate: f64,
    components: Vec<Box<dyn Component>>,
    should_quit: bool,
    should_suspend: bool,
    mode: Mode,
    last_tick_key_events: Vec<KeyEvent>,
    action_tx: mpsc::UnboundedSender<Action>,
    action_rx: mpsc::UnboundedReceiver<Action>,
    focus: Focus,
}

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Mode {
    #[default]
    Home,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Focus {
    Index,
    Reader,
}

impl App {
    pub fn new(tick_rate: f64, frame_rate: f64) -> Result<Self> {
        let bible = Bible::load_tsv("books/kjv.tsv")?;
        let (action_tx, action_rx) = mpsc::unbounded_channel();
        Ok(Self {
            tick_rate,
            frame_rate,
            components: vec![
                Box::new(Index::new(bible.clone())),
                Box::new(Reader::new(bible.clone())),
                Box::new(FpsCounter::default()),
            ],
            should_quit: false,
            should_suspend: false,
            config: Config::new()?,
            mode: Mode::Home,
            last_tick_key_events: Vec::new(),
            action_tx,
            action_rx,
            focus: Focus::Index,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        let mut tui = Tui::new()?
            .tick_rate(self.tick_rate)
            .frame_rate(self.frame_rate);
        tui.enter()?;

        for component in self.components.iter_mut() {
            component.register_action_handler(self.action_tx.clone())?;
        }
        for component in self.components.iter_mut() {
            component.register_config_handler(self.config.clone())?;
        }
        for component in self.components.iter_mut() {
            component.init(tui.size()?)?;
        }

        let action_tx = self.action_tx.clone();
        loop {
            self.handle_events(&mut tui).await?;
            self.handle_actions(&mut tui)?;
            if self.should_suspend {
                tui.suspend()?;
                action_tx.send(Action::Resume)?;
                action_tx.send(Action::ClearScreen)?;
                // tui.mouse(true);
                tui.enter()?;
            } else if self.should_quit {
                tui.stop()?;
                break;
            }
        }
        tui.exit()?;
        Ok(())
    }

    async fn handle_events(&mut self, tui: &mut Tui) -> Result<()> {
        let Some(event) = tui.next_event().await else {
            return Ok(());
        };
        let action_tx = self.action_tx.clone();
        match event {
            Event::Quit => action_tx.send(Action::Quit)?,
            Event::Tick => action_tx.send(Action::Tick)?,
            Event::Render => action_tx.send(Action::Render)?,
            Event::Resize(x, y) => action_tx.send(Action::Resize(x, y))?,
            Event::Key(key) => self.handle_key_event(key)?,
            _ => {}
        }
        for component in self.components.iter_mut() {
            let owns_focus = (self.focus == Focus::Index && component.as_any().is::<Index>())
                || (self.focus == Focus::Reader && component.as_any().is::<Reader>());

            if owns_focus {
                if let Some(action) = component.handle_events(Some(event.clone()))? {
                    action_tx.send(action)?;
                }
                break;
            }
        }
        Ok(())
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> Result<()> {
        let action_tx = self.action_tx.clone();
        let Some(keymap) = self.config.keybindings.get(&self.mode) else {
            return Ok(());
        };
        match keymap.get(&vec![key]) {
            Some(action) => {
                info!("Got action: {action:?}");
                action_tx.send(action.clone())?;
            }
            _ => {
                self.last_tick_key_events.push(key);

                if let Some(action) = keymap.get(&self.last_tick_key_events) {
                    info!("Got action: {action:?}");
                    action_tx.send(action.clone())?;
                }
            }
        }
        Ok(())
    }

    fn handle_actions(&mut self, tui: &mut Tui) -> Result<()> {
        while let Ok(action) = self.action_rx.try_recv() {
            if action != Action::Tick && action != Action::Render {
                debug!("{action:?}");
            }
            match action {
                Action::Tick => {
                    self.last_tick_key_events.drain(..);
                }
                Action::Quit => self.should_quit = true,
                Action::Suspend => self.should_suspend = true,
                Action::Resume => self.should_suspend = false,
                Action::ClearScreen => tui.terminal.clear()?,
                Action::Resize(w, h) => self.handle_resize(tui, w, h)?,
                Action::Render => self.render(tui)?,
                Action::ChangeFocus => {
                    self.focus = match self.focus {
                        Focus::Index => Focus::Reader,
                        Focus::Reader => Focus::Index,
                        // ...
                    }
                }
                _ => {}
            }
            for component in self.components.iter_mut() {
                if let Some(action) = component.update(action.clone())? {
                    self.action_tx.send(action)?
                };
            }
        }
        Ok(())
    }

    fn handle_resize(&mut self, tui: &mut Tui, w: u16, h: u16) -> Result<()> {
        tui.resize(Rect::new(0, 0, w, h))?;
        self.render(tui)?;
        Ok(())
    }

    fn render(&mut self, tui: &mut Tui) -> Result<()> {
        tui.draw(|frame| {
            let [left, right] = ratatui::layout::Layout::horizontal([
                Constraint::Percentage(25),
                Constraint::Percentage(75),
            ])
            .areas(frame.area());

            for component in self.components.iter_mut() {
                let area = if component.as_any().is::<Index>() {
                    left
                } else if component.as_any().is::<Reader>() {
                    right
                } else {
                    frame.area()
                };
                let _ = component.draw(frame, area, self.focus);
            }
        })?;
        Ok(())
    }
}
