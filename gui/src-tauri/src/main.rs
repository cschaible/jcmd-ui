// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use jcmd_data_collector::{DefaultDataCollector, JvmProcesses, VmInformation};
use std::string::ToString;
use std::sync::RwLock;
use tauri::State;

fn main() {
    tauri::Builder::default()
        .manage(RwLock::new(DefaultDataCollector::new()))
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            set_pid,
            get_jvm_processes,
            get_jvm_metrics,
            get_threads,
            get_vm_information,
            reset
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn set_pid(pid: &str, collector: State<RwLock<DefaultDataCollector>>) {
    (*collector.write().unwrap()).set_pid(pid.to_string());
}

#[tauri::command]
fn reset(collector: State<RwLock<DefaultDataCollector>>) {
    (*collector.read().unwrap()).reset();
}

#[tauri::command]
fn get_jvm_processes(collector: State<RwLock<DefaultDataCollector>>) -> Result<JvmProcesses, String> {
    match (*collector.read().unwrap()).get_jvm_processes() {
        Ok(r) => Ok(r),
        Err(e) => Err(e.to_string())
    }
}

#[tauri::command]
fn get_vm_information(state: State<RwLock<DefaultDataCollector>>) -> Result<VmInformation, String> {
    let collector = state.read().unwrap();
    if (*collector).get_pid().is_some() {
        match (*collector).get_vm_information() {
            Ok(r) => Ok(r),
            Err(e) => Err(e.to_string())
        }
    } else {
        Err("process id missing".to_string())
    }
}

#[tauri::command]
fn get_jvm_metrics(state: State<RwLock<DefaultDataCollector>>) -> Result<jcmd_data_collector::JvmMetrics, String> {
    let collector = state.read().unwrap();
    if (*collector).get_pid().is_some() {
        match (*collector).get_jvm_metrics() {
            Ok(r) => Ok(r),
            Err(e) => Err(e.to_string())
        }
    } else {
        Err("process id missing".to_string())
    }
}

// Intro to thread dumps: https://dzone.com/articles/how-to-read-a-thread-dump
#[tauri::command]
fn get_threads(state: State<RwLock<DefaultDataCollector>>) -> Result<jcmd_data_collector::Threads, String> {
    let collector = state.read().unwrap();
    if (*collector).get_pid().is_some() {
        match (*collector).get_threads() {
            Ok(r) => Ok(r),
            Err(e) => Err(e.to_string())
        }
    } else {
        Err("process id missing".to_string())
    }
}
