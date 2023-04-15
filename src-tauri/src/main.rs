#![cfg_attr(
all(not(debug_assertions), target_os = "windows"),
windows_subsystem = "windows"
)]

use serde::Serialize;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![get_jvm_processes])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn get_jvm_processes() -> Result<JvmProcesses, String> {
    match std::process::Command::new("jcmd").output() {
        Ok(o) => {
            if o.status.success() {
                let output = String::from_utf8_lossy(o.stdout.as_slice()).to_string();
                let lines: Vec<&str> = output.split("\n").collect();
                let mut processes: Vec<JvmProcessRef> = Vec::new();
                for line in lines {
                    let parts: Vec<&str> = line.split(" ").collect();
                    if parts.len() < 2 {
                        continue;
                    }
                    if parts.len() >= 3 {
                        let mut path: Vec<String> = Vec::new();
                        for p in 2..(parts.len()) {
                            let value = parts.get(p);
                            if value.is_some() {
                                path.push(value.unwrap().to_string());
                            }
                        }
                        processes.push(JvmProcessRef {
                            id: parts.get(0).unwrap().to_string(),
                            name: parts.get(1).unwrap().to_string(),

                            path: Some(path.join(" ")),
                        })
                    } else {
                        processes.push(JvmProcessRef {
                            id: parts.get(0).unwrap().to_string(),
                            name: parts.get(1).unwrap().to_string(),
                            path: None,
                        })
                    }
                }

                return Ok(JvmProcesses { processes });
            }
            return Err("Data couldn't be read successfully from jcmd".to_string());
        }
        Err(e) => Err(e.to_string())
    }
}

#[derive(Serialize)]
pub struct JvmProcesses {
    processes: Vec<JvmProcessRef>,
}

#[derive(Serialize)]
pub struct JvmProcessRef {
    id: String,
    name: String,
    path: Option<String>,
}
