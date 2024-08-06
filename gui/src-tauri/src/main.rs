// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::string::ToString;

use jcmd_data_collector::{JvmProcesses, VmInformation};

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
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
fn reset() {
    jcmd_data_collector::reset()
}

#[tauri::command]
fn get_jvm_processes() -> Result<JvmProcesses, String> {
    match jcmd_data_collector::get_jvm_processes() {
        Ok(r) => Ok(r),
        Err(e) => Err(e.to_string())
    }
}

#[tauri::command]
fn get_vm_information(pid: &str) -> Result<VmInformation, String> {
    match jcmd_data_collector::get_vm_information(pid) {
        Ok(r) => Ok(r),
        Err(e) => Err(e.to_string())
    }
}

#[tauri::command]
fn get_jvm_metrics(pid: &str) -> Result<jcmd_data_collector::JvmMetrics, String> {
    match jcmd_data_collector::get_jvm_metrics(pid) {
        Ok(r) => Ok(r),
        Err(e) => Err(e.to_string())
    }
}

// Intro to thread dumps: https://dzone.com/articles/how-to-read-a-thread-dump
#[tauri::command]
fn get_threads(pid: &str) -> Result<jcmd_data_collector::Threads, String> {
    match jcmd_data_collector::get_threads(pid) {
        Ok(r) => Ok(r),
        Err(e) => Err(e.to_string())
    }
}
