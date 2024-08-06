use jcmd_data_collector::JvmProcesses;
use jcmd_parser::JvmProcessRef;
use once_cell::sync::Lazy;
use ratatui::crossterm::event::KeyCode;
use ratatui::layout::{Constraint, Direction, Layout, Margin, Rect};
use ratatui::prelude::Modifier;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Row, Table, TableState};
use ratatui::Frame;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Mutex;
use std::time::Duration;

pub static PID: Lazy<Mutex<String>> = Lazy::new(|| Mutex::new("".to_string()));

pub struct JvmProcessesPopup {
    processes: Vec<JvmProcessRef>,
    state: ProcessesListState,
    sender: Sender<JvmProcesses>,
    receiver: Receiver<JvmProcesses>,
}

impl JvmProcessesPopup {
    pub fn new() -> JvmProcessesPopup {
        let (tx, rv) = channel();
        JvmProcessesPopup {
            processes: Vec::new(),
            state: ProcessesListState::new(),
            sender: tx,
            receiver: rv,
        }
    }

    pub fn new_sender(&self) -> Sender<JvmProcesses> {
        self.sender.clone()
    }

    pub fn on_key(&mut self, key_code: KeyCode) {
        match key_code {
            KeyCode::Down | KeyCode::Char('j') => self.state.on_down(),
            KeyCode::Up | KeyCode::Char('k') => self.state.on_up(),
            KeyCode::Enter => {
                if let Some(selected_idx) = self.state.table_state.selected() {
                    let mut pid = PID.lock().unwrap();
                    *pid = self.processes[selected_idx].id.clone()
                }
            }
            _ => {}
        }
    }

    pub fn draw(&mut self, f: &mut Frame) {
        let mut rows: Vec<Row> = Vec::new();
        if let Ok(processes) = self.receiver.recv_timeout(Duration::from_millis(1)) {
            let changed = self.processes != processes.processes;
            if changed {
                self.processes = processes.processes;
                if self.state.table_state.selected().is_none()
                    || self.state.table_state.selected().unwrap() > self.processes.len()
                {
                    self.state.table_state.select(Some(0))
                }
            }
        }
        let selected_pid_lock = PID.lock().unwrap();
        let selected_pid = (*selected_pid_lock).clone();
        drop(selected_pid_lock);
        for process in &self.processes {
            let selected = if selected_pid == process.id { "*" } else { "" };
            rows.push(Row::new(vec![
                process.id.clone(),
                process.name.clone(),
                selected.to_string(),
            ]));
        }

        self.state.table_state_max = if !rows.is_empty() { rows.len() - 1 } else { 0 };


        let header = Row::new(vec!["PID", "Name", "Selected"]);
        let widths = [Constraint::Length(10), Constraint::Percentage(100),Constraint::Length(10)];
        let table = Table::new(rows, widths).header(header)
            .highlight_style(Style::new().add_modifier(Modifier::REVERSED))
            .highlight_symbol("");

        let area = self.centered_rect(60, 25, f.area());
        f.render_stateful_widget(
            table,
            area.inner(Margin {
                // using an inner vertical margin of 1 unit makes the scrollbar inside the block
                vertical: 2,
                horizontal: 2,
            }),
            &mut self.state.table_state,
        );

        let popup_block = Block::default()
            .title("Select process (press enter)")
            .borders(Borders::NONE)
            .style(Style::default().bg(Color::DarkGray));
        f.render_widget(popup_block, area);
    }

    fn centered_rect(&self, percent_x: u16, percent_y: u16, r: Rect) -> Rect {
        // Cut the given rectangle into three vertical pieces
        let popup_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Min(6),
                Constraint::Percentage((100 - percent_y) / 2),
            ])
            .split(r);

        // Then cut the middle vertical piece into three width-wise pieces
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ])
            .split(popup_layout[1])[1] // Return the middle chunk
    }
}

pub struct ProcessesListState {
    pub table_state: TableState,
    pub table_state_max: usize,
}

impl ProcessesListState {
    pub fn new() -> ProcessesListState {
        ProcessesListState {
            table_state: TableState::default().with_selected(Some(0)),
            table_state_max: 0,
        }
    }

    pub fn on_up(&mut self) {
        let current_offset = self.table_state.selected();
        let mut new_offset = current_offset.unwrap_or(0);
        new_offset = new_offset.saturating_sub(1);
        self.table_state.select(Some(new_offset))
    }

    pub fn on_down(&mut self) {
        let current_offset = self.table_state.selected();
        let mut new_offset = current_offset.unwrap_or(self.table_state_max);
        if new_offset < self.table_state_max {
            new_offset += 1;
        } else {
            new_offset = self.table_state_max
        }

        self.table_state.select(Some(new_offset))
    }
}
