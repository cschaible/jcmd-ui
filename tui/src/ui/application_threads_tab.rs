use crate::ui::tab_view::Tab;
use crate::ui::threads_shared::{draw_chart, draw_legend, draw_stats, get_thread_metrics};
use jcmd_data_collector::Threads;
use ratatui::crossterm::event::KeyCode;
use ratatui::layout::{Constraint, Layout, Margin, Rect};
use ratatui::style::{Color, Style, Stylize};
use ratatui::widgets::block::Title;
use ratatui::widgets::{
    Block, Borders, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table,
};
use ratatui::Frame;
use std::cmp::Ordering;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;

pub struct ApplicationThreadTab {
    state: ScrollbarState,
    state_position: usize,
    state_length: usize,
    title: String,
    metrics: Option<Threads>,
    sender: Sender<Threads>,
    receiver: Receiver<Threads>,
    chart_visible: bool,
}

impl ApplicationThreadTab {
    pub fn new(title: String) -> ApplicationThreadTab {
        let (tx, rv) = channel();

        ApplicationThreadTab {
            state: ScrollbarState::new(1),
            state_position: 0,
            state_length: 1,
            title,
            metrics: None,
            sender: tx,
            receiver: rv,
            chart_visible: true,
        }
    }

    pub fn new_sender(&self) -> Sender<Threads> {
        self.sender.clone()
    }

    fn draw_table(&mut self, f: &mut Frame, area: Rect, metrics: &Threads) {
        self.state_length = metrics.application_threads.len();
        self.state = self
            .state
            .content_length(metrics.application_threads.len() - 1)
            .viewport_content_length(1);
        if self.state_position >= self.state_length {
            self.state_position = self.state_length - 1;
            self.state.last()
        }

        let inner_scroll_area = Layout::vertical(vec![Constraint::Percentage(100)])
            .split(area.inner(Margin::new(2, 0)));

        let mut sorted_metrics = metrics.application_threads.clone();
        sorted_metrics.sort_by(|a, b| {
            let ab_status = a.status.cmp(&b.status);
            if ab_status == Ordering::Equal {
                let ab_name = a.name.cmp(&b.name);
                if ab_name == Ordering::Equal {
                    b.elapsed.total_cmp(&a.elapsed)
                } else {
                    ab_name
                }
            } else {
                ab_status
            }
        });
        let data_rows: Vec<Row> = sorted_metrics
            .iter()
            .map(|thread| {
                Row::new(vec![
                    thread.name.clone(),
                    format!("{}", thread.daemon),
                    format!("{:.2}ms", thread.cpu),
                    format!("{:.2}s", thread.elapsed / 1000.0),
                    thread.allocated.clone(),
                    format!("{}", thread.defined_classes),
                    format!("{}", thread.thread_id),
                    thread.os_thread_id.clone(),
                    format!("{}", thread.prio),
                    format!("{}", thread.os_thread_prio),
                    thread.status.clone(),
                ])
            })
            .collect();

        // Sort descending

        let height = area.height as usize;
        let mut min = usize::min(height - 4, self.state_position);
        min = usize::max(0, min);
        let rows: Vec<Row> = data_rows[min..self.state_length].to_vec();

        let widths = [
            Constraint::Min(1),
            Constraint::Min(1),
            Constraint::Min(1),
            Constraint::Min(1),
            Constraint::Min(1),
            Constraint::Min(1),
            Constraint::Min(1),
            Constraint::Min(1),
            Constraint::Min(1),
            Constraint::Min(1),
            Constraint::Min(1),
        ];
        let stats_table = Table::new(rows, widths).header(
            Row::new(vec![
                "Name",
                "Daemon",
                "CPU",
                "Elapsed",
                "Allocated",
                "Defined Classes",
                "Thread ID",
                "OS Thread ID",
                "Prio",
                "OS Thread Prio",
                "State",
            ])
            .style(Style::default().bg(Color::Rgb(41, 41, 41)).underlined()),
        );
        f.render_widget(stats_table, inner_scroll_area[0]);

        // Render scrollbar
        f.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓")),
            area,
            &mut self.state,
        );
    }
}

impl Tab for ApplicationThreadTab {
    fn get_title(&self) -> String {
        self.title.clone()
    }

    fn draw(&mut self, f: &mut Frame, area: Rect) {
        if let Ok(metrics) = self.receiver.recv_timeout(Duration::from_millis(1)) {
            let changed = self.metrics.is_none() || metrics != self.metrics.clone().unwrap();
            if changed {
                self.metrics = Some(metrics);
            }
        }

        if let Some(metrics) = self.metrics.clone() {
            let separator = Block::new()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(Color::DarkGray));

            let mut chart_block = Block::new().title(Title::from("Chart (c)"));
            let chart_table_separator = Block::new().title(Title::from(" | "));
            let mut table_block = Block::new().title(Title::from("Table (t)"));
            if self.chart_visible {
                chart_block = chart_block.style(Style::default().fg(Color::White));
                table_block = table_block.style(Style::default().fg(Color::DarkGray));
            } else {
                chart_block = chart_block.style(Style::default().fg(Color::DarkGray));
                table_block = table_block.style(Style::default().fg(Color::White));
            }

            if self.chart_visible {
                let width = area.width - 6;
                let layout = Layout::vertical(vec![
                    Constraint::Length(1),       // separator
                    Constraint::Length(1),       // chart-table selector
                    Constraint::Length(1),       // space
                    Constraint::Length(1),       // legend
                    Constraint::Percentage(100), // chart
                    Constraint::Length(1),       // space
                    Constraint::Length(7),
                    Constraint::Length(1), // space
                ])
                .split(area.inner(Margin::new(2, 0)));

                f.render_widget(separator, layout[0]);

                let chart_table_layout = Layout::horizontal(vec![
                    Constraint::Length(9),
                    Constraint::Length(3),
                    Constraint::Length(9),
                    Constraint::Percentage(100),
                ])
                .split(layout[1]);
                f.render_widget(chart_block, chart_table_layout[0]);
                f.render_widget(chart_table_separator, chart_table_layout[1]);
                f.render_widget(table_block, chart_table_layout[2]);

                let chart_data = get_thread_metrics(&metrics.thread_count_application, width);
                draw_legend(f, layout[3]);
                draw_chart(f, layout[4], width, &chart_data);
                draw_stats(f, layout[6], &chart_data);
            } else {
                let layout = Layout::vertical(vec![
                    Constraint::Length(1),       // separator
                    Constraint::Length(1),       // chart-table selector
                    Constraint::Length(1),       // space
                    Constraint::Percentage(100), // table
                    Constraint::Length(1),       // space
                ])
                .split(area.inner(Margin::new(2, 0)));

                f.render_widget(separator, layout[0]);

                let chart_table_layout = Layout::horizontal(vec![
                    Constraint::Length(9),
                    Constraint::Length(3),
                    Constraint::Length(9),
                    Constraint::Percentage(100),
                ])
                .split(layout[1]);
                f.render_widget(chart_block, chart_table_layout[0]);
                f.render_widget(chart_table_separator, chart_table_layout[1]);
                f.render_widget(table_block, chart_table_layout[2]);

                self.draw_table(f, layout[3], &metrics);
            }
        }
    }

    fn on_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Down => {
                // Scroll down if there are more than two additional charts to display.
                // Skip the last two as there are always three charts displayed.
                if self.state_position < (self.state_length - 1) {
                    self.state_position += 1;
                }
                self.state = self.state.position(self.state_position);
            }
            KeyCode::Up => {
                if self.state_position >= 1 {
                    self.state_position -= 1;
                }
                self.state = self.state.position(self.state_position);
            }
            KeyCode::Char('c') => self.chart_visible = true,
            KeyCode::Char('t') => self.chart_visible = false,
            _ => {}
        }
    }

    fn on_tick(&mut self) {
        // do nothing
    }
}
