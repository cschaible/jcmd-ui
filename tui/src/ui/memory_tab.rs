use crate::ui::tab_view::Tab;
use jcmd_data_collector::JvmMetrics;
use jcmd_parser::NativeMemoryMetricValue;
use ratatui::crossterm::event::KeyCode;
use ratatui::layout::{Alignment, Constraint, Flex, Layout, Margin, Rect};
use ratatui::prelude::Stylize;
use ratatui::style::{Color, Style};
use ratatui::symbols::Marker;
use ratatui::widgets::{
    Axis, Block, Borders, Chart, Dataset, GraphType, LegendPosition, Row, Scrollbar,
    ScrollbarOrientation, ScrollbarState, Table,
};
use ratatui::Frame;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;
use crate::ui::MemoryStatsDetails;

pub struct MemoryTab {
    state: ScrollbarState,
    state_position: usize,
    state_length: usize,
    title: String,
    metrics: Option<JvmMetrics>,
    sender: Sender<JvmMetrics>,
    receiver: Receiver<JvmMetrics>,
    charts_display_num: usize,
}

struct ChartData {
    committed: Vec<(f64, f64)>,
    committed_max: u64,
    min: u64,
    name: String,
    reserved: Vec<(f64, f64)>,
    reserved_max: u64,
    used: Option<Vec<(f64, f64)>>,
}

struct MemoryStats {
    committed: MemoryStatsDetails<u64>,
    reserved: MemoryStatsDetails<u64>,
    used: Option<MemoryStatsDetails<u64>>,
}

// https://github.com/ratatui-org/ratatui/blob/main/examples/demo/ui.rs#L152
impl MemoryTab {
    pub fn new(title: String) -> MemoryTab {
        let (tx, rv) = channel();

        MemoryTab {
            state: ScrollbarState::new(1),
            state_position: 0,
            state_length: 1,
            title,
            metrics: None,
            sender: tx,
            receiver: rv,
            charts_display_num: 2,
        }
    }

    pub fn new_sender(&self) -> Sender<JvmMetrics> {
        self.sender.clone()
    }

    #[allow(clippy::too_many_lines)]
    fn draw_charts(&mut self, f: &mut Frame, area: Rect) {
        if let Ok(metrics) = self.receiver.recv_timeout(Duration::from_millis(1)) {
            let changed = self.metrics.is_none() || metrics != self.metrics.clone().unwrap();
            if changed {
                self.metrics = Some(metrics);
            }
        }

        // larger number than 6 results in a gap on the right in the chart
        let width = area.width - 6;

        if let Some(metrics) = self.metrics.clone() {
            let metrics_num =
                (metrics.metrics.len() as f32 / self.charts_display_num as f32).ceil() as usize;
            self.state_length = metrics_num * self.charts_display_num - 2;
            self.state = self
                .state
                .content_length(metrics_num * self.charts_display_num - 2)
                .viewport_content_length(self.charts_display_num);
            if self.state_position >= self.state_length {
                self.state_position = self.state_length;
                self.state.last()
            }

            let mut inner_area_constraints: Vec<Constraint> = Vec::new();
            for _ in 0..self.charts_display_num {
                inner_area_constraints.push(Constraint::Min(20));
            }
            let inner_scroll_area =
                Layout::vertical(inner_area_constraints).split(area.inner(Margin::new(2, 0)));

            let mut chart_data_vec: Vec<ChartData> = metrics
                .metrics
                .iter()
                .map(|values| {
                    Self::memory_metrics_to_bins(values.name.clone(), &values.values, width)
                })
                .collect();

            // Sort descending
            chart_data_vec.sort_by(|a, b| b.committed_max.cmp(&a.committed_max));

            let num_of_charts = chart_data_vec.len();
            for idx in 0..num_of_charts {
                if self.state_position == idx {
                    for i in 0..self.charts_display_num {
                        if idx + i <= num_of_charts - 1 {
                            self.draw_header_chart_stats(
                                f,
                                width,
                                inner_scroll_area[i],
                                &chart_data_vec[idx + i],
                            );
                        }
                    }
                }
            }

            f.render_stateful_widget(
                Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(Some("↑"))
                    .end_symbol(Some("↓")),
                area,
                &mut self.state,
            );
        }
    }

    fn draw_header_chart_stats(&self, f: &mut Frame, width: u16, area: Rect, chart_data: &ChartData) {
        let stats_length = if chart_data.used.is_some() { 5 } else { 4 };
        let chunks = Layout::vertical(vec![
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Min(4),
            Constraint::Length(stats_length),
        ])
        .split(area);

        self.draw_title(f, chunks[0], chart_data);
        self.draw_legend(f, chunks[1], chart_data);

        self.draw_chart(f, chunks[2], width, chart_data);
        self.draw_stats(f, chunks[3], chart_data);
    }

    fn draw_title(&self, f: &mut Frame, area: Rect, chart_data: &ChartData) {
        let title_chunks = Layout::vertical(vec![
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);

        // draw separator
        let separator = Block::new()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(Color::DarkGray));
        f.render_widget(separator, title_chunks[0]);

        // draw title
        let name = &chart_data.name;
        let horizontal_layout = Layout::horizontal(vec![
            Constraint::Length(name.len() as u16),
            Constraint::Percentage(100),
        ])
        .split(title_chunks[1]);

        let title = Block::new()
            .title_alignment(Alignment::Left)
            .style(Style::default().underlined())
            .title(format!("{} Memory ", name.clone()));
        f.render_widget(title, horizontal_layout[0]);
    }

    fn draw_legend(&self, f: &mut Frame, area: Rect, chart_data: &ChartData) {
        let reserved_title = Block::new().title("Reserved");
        let reserved_legend = Block::new()
            .title("   ")
            .style(Style::default().bg(Color::Rgb(110, 120, 140)));

        let committed_title = Block::new().title("Committed");
        let committed_legend = Block::new()
            .title("   ")
            .style(Style::default().bg(Color::Rgb(181, 143, 29)));

        let layout_chunks = Layout::horizontal(vec![
            Constraint::Length(1),
            Constraint::Percentage(100),
            Constraint::Length(1),
        ])
        .split(area);

        if chart_data.used.is_some() {
            let used_title = Block::new().title("Used");
            let used_legend = Block::new()
                .title("   ")
                .style(Style::default().bg(Color::Rgb(29, 105, 181)));

            let legend_chunks = Layout::horizontal(vec![
                // Reserved
                Constraint::Length(8),
                Constraint::Length(1),
                Constraint::Length(3),
                Constraint::Length(1),
                // Committed
                Constraint::Length(9),
                Constraint::Length(1),
                Constraint::Length(3),
                Constraint::Length(1),
                // Used
                Constraint::Length(4),
                Constraint::Length(1),
                Constraint::Length(3),
            ])
            .flex(Flex::Center)
            .split(layout_chunks[1]);

            f.render_widget(reserved_title, legend_chunks[0]);
            f.render_widget(reserved_legend, legend_chunks[2]);
            f.render_widget(committed_title, legend_chunks[4]);
            f.render_widget(committed_legend, legend_chunks[6]);
            f.render_widget(used_title, legend_chunks[8]);
            f.render_widget(used_legend, legend_chunks[10]);
        } else {
            let legend_chunks = Layout::horizontal(vec![
                // Reserved
                Constraint::Length(8),
                Constraint::Length(1),
                Constraint::Length(3),
                Constraint::Length(1),
                // Committed
                Constraint::Length(9),
                Constraint::Length(1),
                Constraint::Length(3),
            ])
            .flex(Flex::Center)
            .split(layout_chunks[1]);

            f.render_widget(reserved_title, legend_chunks[0]);
            f.render_widget(reserved_legend, legend_chunks[2]);
            f.render_widget(committed_title, legend_chunks[4]);
            f.render_widget(committed_legend, legend_chunks[6]);
        }
    }

    fn format_value(value: u64) -> String {
        let (unit, factor) = Self::calculate_unit_and_factor(value as f64);
        format!("{:.2} {}", value as f64 / factor, unit)
    }

    fn draw_stats(&self, f: &mut Frame, area: Rect, metrics: &ChartData) {
        let memory_stats = Self::calculate_memory_stats(metrics);

        let mut rows = vec![
            Row::new(vec!["", "Min:", "Max:", "Avg:", "Median:"]),
            Row::new(vec![
                "Reserved:".to_string(),
                Self::format_value(memory_stats.reserved.min),
                Self::format_value(memory_stats.reserved.max),
                Self::format_value(memory_stats.reserved.avg),
                Self::format_value(memory_stats.reserved.median),
            ]),
            Row::new(vec![
                "Committed:".to_string(),
                Self::format_value(memory_stats.committed.min),
                Self::format_value(memory_stats.committed.max),
                Self::format_value(memory_stats.committed.avg),
                Self::format_value(memory_stats.committed.median),
            ]),
        ];

        if let Some(stats) = memory_stats.used {
            rows.push(Row::new(vec![
                "Used:".to_string(),
                Self::format_value(stats.min),
                Self::format_value(stats.max),
                Self::format_value(stats.avg),
                Self::format_value(stats.median),
            ]));
        }

        let widths = [
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Min(0),
        ];
        let stats_table = Table::new(rows, widths);
        f.render_widget(stats_table, area);
    }

    fn calculate_memory_stats(metrics: &ChartData) -> MemoryStats {
        let length = metrics.reserved.len();
        let mut committed_values: Vec<u64> = Vec::with_capacity(length);
        let mut reserved_values: Vec<u64> = Vec::with_capacity(length);
        let mut used_values: Vec<u64> = if metrics.used.is_some() {
            Vec::with_capacity(length)
        } else {
            Vec::new()
        };
        for idx in 0..length {
            committed_values.push(metrics.committed[idx].1 as u64);
            reserved_values.push(metrics.reserved[idx].1 as u64);
            if metrics.used.is_some() {
                used_values.push(metrics.used.clone().unwrap()[idx].1 as u64);
            }
        }

        fn median(numbers: &[u64]) -> u64 {
            let mut sortable_numbers = numbers.to_owned();
            sortable_numbers.sort();
            let mid = sortable_numbers.len() / 2;
            sortable_numbers[mid]
        }

        fn average(numbers: &[u64]) -> u64 {
            (numbers.iter().sum::<u64>() as f64 / numbers.len() as f64).ceil() as u64
        }

        fn min(numbers: &[u64]) -> u64 {
            *numbers.iter().min().unwrap()
        }

        fn max(numbers: &[u64]) -> u64 {
            *numbers.iter().max().unwrap()
        }

        let mut used_metrics = None;
        if !used_values.is_empty() {
            used_metrics = Some(MemoryStatsDetails {
                min: min(&used_values),
                max: max(&used_values),
                avg: average(&used_values),
                median: median(&used_values),
            })
        }

        MemoryStats {
            committed: MemoryStatsDetails {
                min: min(&committed_values),
                max: max(&committed_values),
                avg: average(&committed_values),
                median: median(&committed_values),
            },
            reserved: MemoryStatsDetails {
                min: min(&reserved_values),
                max: max(&reserved_values),
                avg: average(&reserved_values),
                median: median(&reserved_values),
            },
            used: used_metrics,
        }
    }

    fn draw_chart(&self, f: &mut Frame, area: Rect, width: u16, chart_data: &ChartData) {
        let ds_reserved = Dataset::default()
            .name("reserved")
            .data(&chart_data.reserved)
            .marker(Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Rgb(110, 120, 140)));
        let ds_committed = Dataset::default()
            .name("committed")
            .data(&chart_data.committed)
            .marker(Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Rgb(181, 143, 29)));

        let empty_used: Vec<(f64, f64)> = Vec::new();
        let used_data = &chart_data.used.clone().unwrap_or(empty_used);

        let datasets: Vec<Dataset> = if !used_data.is_empty() {
            let ds_used = Dataset::default()
                .name("used")
                .data(used_data)
                .marker(Marker::Braille)
                .graph_type(GraphType::Line)
                .style(Style::default().fg(Color::Rgb(29, 105, 181)));
            vec![ds_reserved, ds_committed, ds_used]
        } else {
            vec![ds_reserved, ds_committed]
        };

        let x_axis_min_label = format!("-{}s", width);
        let x_axis_middle_label = format!("-{}s", width / 2);
        let x_axis = Axis::default()
            .title("Time")
            .style(Style::default().white())
            .bounds([0.0, width as f64])
            .labels([&x_axis_min_label, &x_axis_middle_label, "now"]);

        let mut y_max = chart_data.reserved_max as f64;
        // add 10 percent below to not start with the minimum
        let y_ten_percent = (chart_data.reserved_max as f64 - chart_data.min as f64) * 0.1;
        let y_min_minus_ten_percent = chart_data.min as f64 - y_ten_percent;
        let mut y_min = if y_min_minus_ten_percent > 0.0 {
            y_min_minus_ten_percent
        } else {
            0.0
        };
        if (y_max - y_min).abs() < 1.0 && y_max > 1.0 {
            y_min -=  1.0;
        } else if (y_max - y_min).abs() < 1.0 {
            y_max = 1.0;
            y_min = 0.0;
        }

        let (unit, factor) = Self::calculate_unit_and_factor(y_max);

        let mut y_axis_labels: Vec<String> = Vec::new();
        y_axis_labels.push(format!("{:7.2}", y_min / factor));
        if self.charts_display_num == 1 {
            let steps = 8;
            let step_size = (y_max - y_min) / steps as f64;
            for i in 1..steps {
                y_axis_labels.push(format!("{:7.2}", (y_min + step_size * i as f64) / factor));
            }
        } else if self.charts_display_num == 2 {
            let steps = 4;
            let step_size = (y_max - y_min) / steps as f64;
            for i in 1..steps {
                y_axis_labels.push(format!("{:7.2}", (y_min + step_size * i as f64) / factor));
            }
        } else if self.charts_display_num == 3 {
            let step_size = (y_max - y_min) / 2f64;
            y_axis_labels.push(format!("{:7.2}", (y_min + step_size) / factor));
        }
        y_axis_labels.push(format!("{:7.2}", y_max / factor));
        let y_axis = Axis::default()
            .title(unit)
            .style(Style::default().white())
            .bounds([y_min, y_max])
            .labels(y_axis_labels);

        // Never show legend
        let constraints = (Constraint::Length(0), Constraint::Min(0));
        let chart = Chart::new(datasets)
            .x_axis(x_axis)
            .y_axis(y_axis)
            .legend_position(Some(LegendPosition::TopRight))
            .hidden_legend_constraints(constraints);

        f.render_widget(chart, area);
    }

    fn calculate_unit_and_factor(value: f64) -> (String, f64) {
        let mut unit = "B";
        let mut factor = 1.0;
        if value > (1024 * 1024) as f64 {
            unit = "MB";
            factor = (1024 * 1024) as f64;
        } else if value > 1024f64 {
            unit = "KB";
            factor = 1024f64;
        }
        (unit.to_string(), factor)
    }

    fn memory_metrics_to_bins(
        name: String,
        metrics: &[NativeMemoryMetricValue],
        bins: u16,
    ) -> ChartData {
        let start: usize =
        if metrics.len() as u16 <= bins {
            0
        } else {
            metrics.len() - bins as usize
        };

        let mut committed: Vec<(f64, f64)> = Vec::new();
        let mut committed_max: u64 = 0;
        let mut reserved: Vec<(f64, f64)> = Vec::new();
        let mut reserved_max: u64 = 0;
        let mut used: Vec<(f64, f64)> = Vec::new();
        let mut min: u64 = u64::MAX;
        // Iterate in reversed order
        for (idx, i) in (start..metrics.len()).rev() .enumerate(){
            let x = bins as f64 - idx as f64;
            committed.push((x, metrics[i].committed.unwrap() as f64));
            reserved.push((x, metrics[i].reserved.unwrap() as f64));
            if let Some(used_value) = &metrics[i].used {
                used.push((x, (*used_value) as f64));

                // check min
                if *used_value < min {
                    min = *used_value
                }
            }

            // check min
            if metrics[i].committed.unwrap() < min {
                min = metrics[i].committed.unwrap()
            }

            if metrics[i].committed.unwrap() > committed_max {
                committed_max = metrics[i].committed.unwrap()
            }
            if metrics[i].reserved.unwrap() > reserved_max {
                reserved_max = metrics[i].reserved.unwrap()
            }
        }

        // reserved.reverse();
        // committed.reverse();
        let used_data = if !used.is_empty() { Some(used) } else { None };
        ChartData {
            committed,
            committed_max,
            min,
            name,
            reserved,
            reserved_max,
            used: used_data,
        }
    }
}

impl Tab for MemoryTab {
    fn get_title(&self) -> String {
        self.title.clone()
    }

    fn draw(&mut self, f: &mut Frame, area: Rect) {
        self.draw_charts(f, area);
    }

    fn on_key(&mut self, code: KeyCode) {
        // do nothing
        match code {
            KeyCode::Down => {
                // Scroll down if there are more than two additional charts to display.
                // Skip the last two as there are always three charts displayed.
                if self.state_position <= (self.state_length - self.charts_display_num) {
                    self.state_position += self.charts_display_num;
                }
                self.state = self.state.position(self.state_position);
            }
            KeyCode::Up => {
                if self.state_position >= self.charts_display_num {
                    self.state_position -= self.charts_display_num;
                }
                self.state = self.state.position(self.state_position);
            }
            KeyCode::Char('1') => self.charts_display_num = 1,
            KeyCode::Char('2') => self.charts_display_num = 2,
            KeyCode::Char('3') => self.charts_display_num = 3,
            _ => {}
        }
    }

    fn on_tick(&mut self) {}
}
