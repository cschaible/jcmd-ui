use crate::ui::MemoryStatsDetails;
use jcmd_data_collector::NamedMetric;
use jcmd_parser::ThreadCountMetricValue;
use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::{Color, Style, Stylize};
use ratatui::symbols::Marker;
use ratatui::widgets::{
    Axis, Block, Chart, Dataset, GraphType, LegendPosition, Row
    , Table,
};
use ratatui::Frame;

pub struct ChartData {
    blocked: Vec<(f64, f64)>,
    blocked_stats: MemoryStatsDetails<u32>,
    new: Vec<(f64, f64)>,
    new_stats: MemoryStatsDetails<u32>,
    runnable: Vec<(f64, f64)>,
    runnable_stats: MemoryStatsDetails<u32>,
    timed_waiting: Vec<(f64, f64)>,
    timed_waiting_stats: MemoryStatsDetails<u32>,
    waiting: Vec<(f64, f64)>,
    waiting_stats: MemoryStatsDetails<u32>,
    total: Vec<(f64, f64)>,
    total_stats: MemoryStatsDetails<u32>,
}

fn format_value(value: u32) -> String {
    format!("{}", value)
}

pub fn draw_stats(f: &mut Frame, area: Rect, chart_data: &ChartData) {
    let rows = vec![
        Row::new(vec!["", "Min:", "Max:", "Avg:", "Median:"]),
        Row::new(vec![
            "New:".to_string(),
            format_value(chart_data.new_stats.min),
            format_value(chart_data.new_stats.max),
            format_value(chart_data.new_stats.avg),
            format_value(chart_data.new_stats.median),
        ]),
        Row::new(vec![
            "Runnable:".to_string(),
            format_value(chart_data.runnable_stats.min),
            format_value(chart_data.runnable_stats.max),
            format_value(chart_data.runnable_stats.avg),
            format_value(chart_data.runnable_stats.median),
        ]),
        Row::new(vec![
            "Waiting:".to_string(),
            format_value(chart_data.waiting_stats.min),
            format_value(chart_data.waiting_stats.max),
            format_value(chart_data.waiting_stats.avg),
            format_value(chart_data.waiting_stats.median),
        ]),
        Row::new(vec![
            "Timed Waiting:".to_string(),
            format_value(chart_data.timed_waiting_stats.min),
            format_value(chart_data.timed_waiting_stats.max),
            format_value(chart_data.timed_waiting_stats.avg),
            format_value(chart_data.timed_waiting_stats.median),
        ]),
        Row::new(vec![
            "Blocked:".to_string(),
            format_value(chart_data.blocked_stats.min),
            format_value(chart_data.blocked_stats.max),
            format_value(chart_data.blocked_stats.avg),
            format_value(chart_data.blocked_stats.median),
        ]),
        Row::new(vec![
            "Total:".to_string(),
            format_value(chart_data.total_stats.min),
            format_value(chart_data.total_stats.max),
            format_value(chart_data.total_stats.avg),
            format_value(chart_data.total_stats.median),
        ]),
    ];

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

pub fn draw_legend(f: &mut Frame, area: Rect) {
    let new_title = Block::new().title("New");
    let new_legend = Block::new()
        .title("   ")
        .style(Style::default().bg(Color::Rgb(212, 12, 243)));

    let runnable_title = Block::new().title("Runnable");
    let runnable_legend = Block::new()
        .title("   ")
        .style(Style::default().bg(Color::Rgb(158, 243, 12)));

    let waiting_title = Block::new().title("Waiting");
    let waiting_legend = Block::new()
        .title("   ")
        .style(Style::default().bg(Color::Rgb(12, 162, 243)));

    let timed_waiting_title = Block::new().title("Timed Waiting");
    let timed_waiting_legend = Block::new()
        .title("   ")
        .style(Style::default().bg(Color::Rgb(243, 224, 12)));

    let blocked_title = Block::new().title("Blocked");
    let blocked_legend = Block::new()
        .title("   ")
        .style(Style::default().bg(Color::Rgb(243, 101, 12)));

    let total_title = Block::new().title("Total");
    let total_legend = Block::new()
        .title("   ")
        .style(Style::default().bg(Color::Rgb(123, 123, 123)));

    let layout_chunks = Layout::horizontal(vec![
        Constraint::Length(1),
        Constraint::Percentage(100),
        Constraint::Length(1),
    ])
        .split(area);

    let legend_chunks = Layout::horizontal(vec![
        // New
        Constraint::Length(3),
        Constraint::Length(1),
        Constraint::Length(3),
        Constraint::Length(1),
        // Runnable
        Constraint::Length(8),
        Constraint::Length(1),
        Constraint::Length(3),
        Constraint::Length(1),
        // Waiting
        Constraint::Length(7),
        Constraint::Length(1),
        Constraint::Length(3),
        Constraint::Length(1),
        // Timed Waiting
        Constraint::Length(13),
        Constraint::Length(1),
        Constraint::Length(3),
        Constraint::Length(1),
        // Blocked
        Constraint::Length(7),
        Constraint::Length(1),
        Constraint::Length(3),
        Constraint::Length(1),
        // Total
        Constraint::Length(5),
        Constraint::Length(1),
        Constraint::Length(3),
        Constraint::Length(1),
    ])
        .flex(Flex::Center)
        .split(layout_chunks[1]);

    f.render_widget(new_title, legend_chunks[0]);
    f.render_widget(new_legend, legend_chunks[2]);
    f.render_widget(runnable_title, legend_chunks[4]);
    f.render_widget(runnable_legend, legend_chunks[6]);
    f.render_widget(waiting_title, legend_chunks[8]);
    f.render_widget(waiting_legend, legend_chunks[10]);
    f.render_widget(timed_waiting_title, legend_chunks[12]);
    f.render_widget(timed_waiting_legend, legend_chunks[14]);
    f.render_widget(blocked_title, legend_chunks[16]);
    f.render_widget(blocked_legend, legend_chunks[18]);
    f.render_widget(total_title, legend_chunks[20]);
    f.render_widget(total_legend, legend_chunks[22]);
}

pub fn draw_chart(f: &mut Frame, area: Rect, width: u16, chart_data: &ChartData) {
    let ds_new = Dataset::default()
        .name("New")
        .data(&chart_data.new)
        .marker(Marker::Dot)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(Color::Rgb(212, 12, 243)));
    let ds_runnable = Dataset::default()
        .name("Runnable")
        .data(&chart_data.runnable)
        .marker(Marker::Dot)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(Color::Rgb(158, 243, 12)));
    let ds_waiting = Dataset::default()
        .name("Waiting")
        .data(&chart_data.waiting)
        .marker(Marker::Dot)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(Color::Rgb(12, 162, 243)));
    let ds_timed_waiting = Dataset::default()
        .name("Timed Waiting")
        .data(&chart_data.timed_waiting)
        .marker(Marker::Dot)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(Color::Rgb(243, 224, 12)));
    let ds_blocked = Dataset::default()
        .name("Blocked")
        .data(&chart_data.blocked)
        .marker(Marker::Dot)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(Color::Rgb(243, 101, 12)));
    let ds_total = Dataset::default()
        .name("Total")
        .data(&chart_data.total)
        .marker(Marker::Dot)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(Color::Rgb(123, 123, 123)));

    let datasets = vec![
        ds_new,
        ds_runnable,
        ds_waiting,
        ds_timed_waiting,
        ds_blocked,
        ds_total,
    ];

    let x_axis_min_label = format!("-{}s", width);
    let x_axis_middle_label = format!("-{}s", width / 2);
    let x_axis = Axis::default()
        .title("Time")
        .style(Style::default().white())
        .bounds([0.0, width as f64])
        .labels([&x_axis_min_label, &x_axis_middle_label, "now"]);

    let y_max = chart_data.total_stats.max;
    let y_min = 0;

    let mut y_axis_labels: Vec<String> = Vec::new();
    y_axis_labels.push(format!("{:7.0}", y_min));
    let steps = 9;
    let step_size = (y_max - y_min) / steps;
    for i in 1..steps {
        y_axis_labels.push(format!("{:7.0}", y_min + step_size * i));
    }
    y_axis_labels.push(format!("{:7.0}", y_max));
    let y_axis = Axis::default()
        .title("Threads")
        .style(Style::default().white())
        .bounds([y_min as f64, y_max as f64])
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

pub fn get_thread_metrics(metrics: &NamedMetric<ThreadCountMetricValue>, bins: u16) -> ChartData {
    let mut blocked: Vec<(f64, f64)> = Vec::new();
    let mut blocked_cnt: Vec<u32> = Vec::new();
    let mut new: Vec<(f64, f64)> = Vec::new();
    let mut new_cnt: Vec<u32> = Vec::new();
    let mut runnable: Vec<(f64, f64)> = Vec::new();
    let mut runnable_cnt: Vec<u32> = Vec::new();
    let mut timed_waiting: Vec<(f64, f64)> = Vec::new();
    let mut timed_waiting_cnt: Vec<u32> = Vec::new();
    let mut waiting: Vec<(f64, f64)> = Vec::new();
    let mut waiting_cnt: Vec<u32> = Vec::new();
    let mut total: Vec<(f64, f64)> = Vec::new();
    let mut total_cnt: Vec<u32> = Vec::new();
    let mut idx = 0;
    metrics.values.clone().into_iter().rev().for_each(|m| {
        let x = bins as f64 - idx as f64;
        blocked.push((x, m.blocked_count as f64));
        blocked_cnt.push(m.blocked_count);
        new.push((x, m.new_count as f64));
        new_cnt.push(m.new_count);
        runnable.push((x, m.runnable_count as f64));
        runnable_cnt.push(m.runnable_count);
        timed_waiting.push((x, m.timed_waiting_count as f64));
        timed_waiting_cnt.push(m.timed_waiting_count);
        waiting.push((x, m.waiting_count as f64));
        waiting_cnt.push(m.waiting_count);
        total.push((
            x,
            (m.blocked_count
                + m.new_count
                + m.runnable_count
                + m.timed_waiting_count
                + m.waiting_count) as f64,
        ));
        total_cnt.push(
            m.blocked_count
                + m.new_count
                + m.runnable_count
                + m.timed_waiting_count
                + m.waiting_count,
        );
        idx += 1;
    });

    let blocked_stats = memory_stats(&blocked_cnt);
    let new_stats = memory_stats(&new_cnt);
    let runnable_stats = memory_stats(&runnable_cnt);
    let timed_waiting_stats = memory_stats(&timed_waiting_cnt);
    let waiting_stats = memory_stats(&waiting_cnt);
    let total_stats = memory_stats(&total_cnt);

    ChartData {
        blocked,
        blocked_stats,
        new,
        new_stats,
        runnable,
        runnable_stats,
        timed_waiting,
        timed_waiting_stats,
        waiting,
        waiting_stats,
        total,
        total_stats,
    }
}

fn memory_stats(values: &[u32]) -> MemoryStatsDetails<u32> {
    fn median(numbers: &[u32]) -> u32 {
        let mut sortable_numbers = numbers.to_owned();
        sortable_numbers.sort();
        let mid = sortable_numbers.len() / 2;
        sortable_numbers[mid]
    }

    fn average(numbers: &[u32]) -> u32 {
        (numbers.iter().sum::<u32>() as f32 / numbers.len() as f32).ceil() as u32
    }

    fn min(numbers: &[u32]) -> u32 {
        *numbers.iter().min().unwrap()
    }

    fn max(numbers: &[u32]) -> u32 {
        *numbers.iter().max().unwrap()
    }

    MemoryStatsDetails {
        min: min(values),
        max: max(values),
        median: median(values),
        avg: average(values),
    }
}