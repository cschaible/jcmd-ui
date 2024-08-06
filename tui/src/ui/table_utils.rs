use ratatui::widgets::Row;

pub fn as_rows(input: &str) -> Vec<Row> {
    input
        .split("\n")
        .map(|s| Row::new([format!("  {}", s)]))
        .collect()
}
