use crate::ui::application_threads_tab::ApplicationThreadTab;
use crate::ui::jvm_threads_tab::JvmThreadTab;
use crate::ui::memory_tab::MemoryTab;
use jcmd_data_collector::{JvmMetrics, Threads, VmInformation};
use jcmd_parser::JvmProcesses;
use ratatui::backend::Backend;
use ratatui::crossterm::event::{DisableMouseCapture, EnableMouseCapture, Event};
use ratatui::crossterm::execute;
use ratatui::{
    backend::CrosstermBackend,
    crossterm::{
        event::{self, KeyCode, KeyEventKind},
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    },
    Terminal,
};
use std::cmp::PartialEq;
use std::error::Error;
use std::io;
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};
use ui::process_information_tab::ProcessInformationTab;
use ui::processes::JvmProcessesPopup;

mod cmd;
mod ui;

#[derive(PartialEq)]
enum CurrentView {
    ProcessDetails,
    ProcessSelection,
}

fn main() -> Result<(), Box<dyn Error>> {
    let tick_rate = Duration::from_millis(250);
    run(tick_rate)
}

pub fn run(tick_rate: Duration) -> Result<(), Box<dyn Error>> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let res = run_app(&mut terminal, tick_rate);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, tick_rate: Duration) -> anyhow::Result<()> {
    let mut last_tick = Instant::now();
    let mut current_view = CurrentView::ProcessSelection;

    let process_information_tab = ProcessInformationTab::new("Process Information".to_string());
    let process_information_sender = process_information_tab.new_sender();

    let memory_tab = MemoryTab::new("Memory".to_string());
    let memory_sender = memory_tab.new_sender();
    let application_thread_tab = ApplicationThreadTab::new("Application Threads".to_string());
    let application_thread_sender = application_thread_tab.new_sender();
    let jvm_thread_tab = JvmThreadTab::new("JVM Threads".to_string());
    let jvm_thread_sender = jvm_thread_tab.new_sender();

    let mut tab_view = ui::tab_view::TabView::new(vec![
        Box::new(process_information_tab),
        Box::new(memory_tab),
        Box::new(application_thread_tab),
        Box::new(jvm_thread_tab),
    ]);

    let mut memory_tick = Instant::now();
    let mut threads_tick = Instant::now();
    let memory_tick_rate = Duration::from_millis(1000);
    let threads_tick_rate = Duration::from_millis(2000);

    let mut process_selection_popup = JvmProcessesPopup::new();
    let process_selection_sender = process_selection_popup.new_sender();
    let mut processes_tick = Instant::now();
    let mut last_known_pid = "".to_string();

    loop {
        terminal.draw(|f| {
            tab_view.draw(f);
            if current_view == CurrentView::ProcessSelection {
                process_selection_popup.draw(f);
            }
        })?;

        let timeout = tick_rate.saturating_sub(last_tick.elapsed());
        let mut should_quit = false;
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    if current_view == CurrentView::ProcessSelection {
                        process_selection_popup.on_key(key.code);
                        match key.code {
                            KeyCode::Enter | KeyCode::Esc => {
                                current_view = CurrentView::ProcessDetails
                            }
                            _ => {}
                        }
                    } else if current_view == CurrentView::ProcessDetails {
                        match key.code {
                            KeyCode::Char('q') => should_quit = true,
                            KeyCode::Char('p') => {
                                current_view = CurrentView::ProcessSelection
                            }
                            _ => tab_view.on_key(key.code),
                        }
                    }
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            tab_view.on_tick();
            last_tick = Instant::now();
        }

        let pid_lock = ui::processes::PID.lock().unwrap();
        let pid = pid_lock.clone();
        drop(pid_lock);

        if current_view == CurrentView::ProcessDetails && !pid.is_empty() {
            update_memory_metrics(
                pid.to_string(),
                &memory_sender,
                &mut memory_tick,
                memory_tick_rate,
            )?;
            update_threads(
                pid.to_string(),
                &application_thread_sender,
                &jvm_thread_sender,
                &mut threads_tick,
                threads_tick_rate,
            )?;
            if last_known_pid != pid {
                update_vm_information(pid.to_string(), &process_information_sender)?;
                last_known_pid = pid;
            }
        }
        if current_view == CurrentView::ProcessSelection {
            update_processes(&process_selection_sender, &mut processes_tick)?;
        }

        if should_quit {
            return Ok(());
        }
    }
}

fn update_processes(sender: &Sender<JvmProcesses>, last_tick: &mut Instant) -> anyhow::Result<()> {
    if last_tick.elapsed() >= Duration::from_secs(1) {
        let processes = cmd::get_jvm_processes()?;
        sender.send(processes)?;
        *last_tick = Instant::now();
    }
    Ok(())
}

fn update_memory_metrics(
    pid: String,
    sender: &Sender<JvmMetrics>,
    last_tick: &mut Instant,
    tick_rate: Duration,
) -> anyhow::Result<()> {
    if last_tick.elapsed() >= tick_rate {
        let metrics = cmd::get_jvm_metrics(pid)?;
        sender.send(metrics)?;
        *last_tick = Instant::now()
    }
    Ok(())
}

fn update_threads(
    pid: String,
    application_thread_sender: &Sender<Threads>,
    jvm_thread_sender: &Sender<Threads>,
    last_tick: &mut Instant,
    tick_rate: Duration,
) -> anyhow::Result<()> {
    if last_tick.elapsed() >= tick_rate {
        let threads = cmd::get_thread_metrics(pid)?;
        application_thread_sender.send(threads.clone())?;
        jvm_thread_sender.send(threads)?;
        *last_tick = Instant::now()
    }
    Ok(())
}

fn update_vm_information(pid: String, sender: &Sender<VmInformation>) -> anyhow::Result<()> {
    let info = cmd::get_vm_information(pid)?;
    sender.send(info)?;
    Ok(())
}
