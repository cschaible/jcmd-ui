use jcmd_data_collector::VmInformation;
use ratatui::crossterm::event::KeyCode;
use ratatui::layout::{Constraint, Rect};
use ratatui::widgets::{Block, Borders, Row, Table, TableState};
use ratatui::Frame;
use ratatui::prelude::*;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;
use ui::table_utils::as_rows;

use crate::ui;
use crate::ui::tab_view::Tab;

pub struct ProcessInformationTab {
    state: ProcessInformationTabState,
    title: String,
    java_command: String,
    jvm_arguments: String,
    jvm_resources: String,
    sender: Sender<VmInformation>,
    receiver: Receiver<VmInformation>,
}

impl ProcessInformationTab {
    pub fn new(title: String) -> ProcessInformationTab {
        let (tx, rv) = channel();

        ProcessInformationTab {
            state: ProcessInformationTabState::new(),
            title,
            java_command: "".to_string(),
            jvm_arguments: "".to_string(),
            jvm_resources: "".to_string(),
            sender: tx,
            receiver: rv,
        }
    }

    pub fn new_sender(&self) -> Sender<VmInformation> {
        self.sender.clone()
    }

    pub fn update_java_command(&mut self, java_command: String) {
        self.java_command = java_command
    }

    pub fn update_jvm_arguments(&mut self, jvm_arguments: String) {
        self.jvm_arguments = jvm_arguments
    }

    pub fn update_jvm_resources(&mut self, jvm_resources: String) {
        self.jvm_resources = jvm_resources
    }
}

impl Tab for ProcessInformationTab {
    fn get_title(&self) -> String {
        self.title.clone()
    }

    fn draw(&mut self, f: &mut Frame, area: Rect) {
        if let Ok(vm_info) = self.receiver.recv_timeout(Duration::from_millis(1)) {
            if let Some(vm_resources) = vm_info.vm_resources {
                let resources = format!(
                    "CPUs: {}\nMemory: {}\nMin Heap Size: {}\nInit Heap Size: {}\nMax Heap Size: {}",
                    vm_resources.cpus.unwrap_or("-".to_string()),//.replace("\"", ""),
                    vm_resources.memory.unwrap_or("-".to_string()),//.replace("\"", ""),
                    vm_resources.heap_size_min.unwrap_or("-".to_string()),//.replace("\"", ""),
                    vm_resources.heap_size_init.unwrap_or("-".to_string()),//.replace("\"", ""),
                    vm_resources.heap_size_max.unwrap_or("-".to_string()),//.replace("\"", ""),
                );
                self.update_jvm_resources(resources);
            }
            if let Some(arguments) = vm_info.vm_arguments {
                if let Some(java_command) = arguments.java_command {
                    self.update_java_command(java_command)
                }
                if let Some(jvm_args) = arguments.jvm_args {
                    self.update_jvm_arguments(jvm_args.replace(" ", "\n"))
                }
            }
        }

        let mut jvm_arguments_rows = as_rows(&self.jvm_arguments);
        let mut java_command_rows = as_rows(&self.java_command);
        let mut resources_rows = as_rows(&self.jvm_resources);

        let mut rows: Vec<Row> = Vec::new();
        rows.push(Row::new(vec!["JVM Arguments:"]));
        rows.append(&mut jvm_arguments_rows);
        rows.push(Row::new(vec![""]));
        rows.push(Row::new(vec!["Java Command:"]));
        rows.append(&mut java_command_rows);
        rows.push(Row::new(vec![""]));
        rows.push(Row::new(vec!["Resources:"]));
        rows.append(&mut resources_rows);

        self.state.table_state_max = rows.len() - 1;

        let widths = [Constraint::Percentage(100)];
        let table = Table::new(rows, widths)
            .highlight_style(Style::new().bg(Color::Rgb(41, 41, 41)))
            .highlight_symbol("");

        let chunks = Layout::vertical(vec![Constraint::Length(2), Constraint::Min(1)]).split(area);

        let separator = Block::new()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(Color::DarkGray));
        f.render_widget(
            separator,
            chunks[0].inner(Margin {
                vertical: 0,
                horizontal: 2,
            }),
        );

        //f.render_widget(block, area);
        f.render_stateful_widget(
            table,
            chunks[1].inner(Margin {
                vertical: 0,
                horizontal: 2,
            }),
            &mut self.state.table_state,
        );
    }

    fn on_key(&mut self, key_code: KeyCode) {
        match key_code {
            KeyCode::Down | KeyCode::Char('j') => self.state.on_down(),
            KeyCode::Up | KeyCode::Char('k') => self.state.on_up(),
            _ => {}
        }
    }

    fn on_tick(&mut self) {
        // do nothing
    }
}

pub struct ProcessInformationTabState {
    pub table_state: TableState,
    pub table_state_max: usize,
}

impl ProcessInformationTabState {
    pub fn new() -> ProcessInformationTabState {
        ProcessInformationTabState {
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
