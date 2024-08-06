use ratatui::crossterm::event::KeyCode;
use ratatui::layout::{Constraint, Layout, Margin, Rect};
use ratatui::prelude::{Color, Span, Style};
use ratatui::widgets::{Block, Tabs};
use ratatui::{text, Frame};

pub struct TabView {
    tabs_state: TabsState,
}

impl TabView {
    pub(crate) fn new(tabs: Vec<Box<dyn Tab>>) -> Self {
        let tabs_state = TabsState::new(tabs);
        TabView { tabs_state }
    }

    pub fn on_key(&mut self, key_code: KeyCode) {
        match key_code {
            KeyCode::Right | KeyCode::Char('l') => self.tabs_state.next(),
            KeyCode::Left | KeyCode::Char('h') => self.tabs_state.previous(),
            _ => self.tabs_state.tabs[self.tabs_state.index].on_key(key_code),
        }
    }

    pub fn on_tick(&mut self) {
        self.tabs_state.tabs[self.tabs_state.index].on_tick();
    }

    pub fn draw(&mut self, f: &mut Frame) {
        let render_tabs = self
            .tabs_state
            .titles()
            .iter()
            .map(|t| text::Line::from(Span::styled(t.clone(), Style::default().fg(Color::DarkGray))))
            .collect::<Tabs>()
            .block(Block::default().title(""))
            .highlight_style(Style::default().fg(Color::White))
            .select(self.tabs_state.index);

        let chunks = Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).split(f.area());
        f.render_widget(render_tabs, chunks[0].inner(Margin{
            horizontal: 1,
            vertical: 0,
        }));

        let tab = &mut self.tabs_state.tabs[self.tabs_state.index];
        tab.draw(f, chunks[1]);
    }
}

pub struct TabsState {
    pub tabs: Vec<Box<dyn Tab>>,
    pub index: usize,
}

impl TabsState {
    pub fn new(tabs: Vec<Box<dyn Tab>>) -> Self {
        Self { tabs, index: 0 }
    }
    pub fn next(&mut self) {
        self.index = (self.index + 1) % self.tabs.len();
    }

    pub fn previous(&mut self) {
        if self.index > 0 {
            self.index -= 1;
        } else {
            self.index = self.tabs.len() - 1;
        }
    }

    pub fn titles(&self) -> Vec<String> {
        self.tabs.iter().map(|t| t.get_title()).collect()
    }
}

pub trait Tab {
    fn get_title(&self) -> String;
    fn draw(&mut self, f: &mut Frame, area: Rect);
    fn on_key(&mut self, key_code: KeyCode);
    fn on_tick(&mut self);
}
