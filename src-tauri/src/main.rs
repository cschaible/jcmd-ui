#![cfg_attr(
all(not(debug_assertions), target_os = "windows"),
windows_subsystem = "windows"
)]

use std::collections::HashMap;
use std::process::Command;
use std::string::ToString;
use std::sync::Mutex;
use std::time::SystemTime;

use once_cell::sync::Lazy;
use serde::Serialize;

static CACHE: Lazy<Mutex<MetricsCache>> = Lazy::new(|| Mutex::new(MetricsCache {
    total_memory: TotalMemoryMetric {
        values: Vec::new()
    },
    class_metrics: ClassMemoryMetric {
        name: "Class".to_string(),
        values: Vec::new(),
    },
    heap_metrics: HeapMemoryMetric {
        name: "Heap".to_string(),
        values: Vec::new(),
    },
    metaspace_metrics: MetaspaceMemoryMetric {
        name: "Metaspace".to_string(),
        values: Vec::new(),
    },
    thread_metrics: ThreadMemoryMetric {
        name: "Thread".to_string(),
        values: Vec::new(),
    },
    other_metrics: HashMap::new(),
}));

static JCMD: Lazy<Mutex<String>> = Lazy::new(|| Mutex::new("".to_string()));

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .invoke_handler(tauri::generate_handler![get_jvm_processes, get_jvm_metrics, get_vm_information, reset])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn reset() {
    let mut c = CACHE.lock().unwrap();
    c.class_metrics.values.clear();
    c.heap_metrics.values.clear();
    c.metaspace_metrics.values.clear();
    c.other_metrics.clear();
    c.total_memory.values.clear();
    c.thread_metrics.values.clear();
}

fn jcmd() -> Command {
    let mut cmd = JCMD.lock().unwrap();
    let mut path = (*cmd).clone();
    if *cmd == "".to_string() {
        let p = match std::env::var("JAVA_HOME") {
            Ok(p) => format!("{}/bin/jcmd", p),
            Err(_) => match std::env::var("JCMD") {
                Ok(p) => p,
                Err(_) => "jcmd".to_string()
            }
        };
        *cmd = p.clone();
        path = p
    }
    Command::new(path)
}

#[tauri::command]
fn get_jvm_processes() -> Result<JvmProcesses, String> {
    match jcmd().output() {
        Ok(o) => {
            if o.status.success() {
                let output = String::from_utf8_lossy(o.stdout.as_slice()).to_string();
                let lines: Vec<&str> = output.split("\n").collect();
                let mut processes: Vec<JvmProcessRef> = Vec::new();
                for line in lines {
                    if (&line).contains("jdk.jcmd") {
                        continue;
                    }
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
            return Err("Data couldn't be read successfully".to_string());
        }
        Err(e) => Err(e.to_string())
    }
}

#[tauri::command]
fn get_vm_information(pid: &str) -> Result<VmInformation, String> {
    match jcmd().arg(pid).arg("VM.info").output() {
        Ok(o) => {
            if o.status.success() {
                let output = String::from_utf8_lossy(o.stdout.as_slice()).to_string();
                let blocks: Vec<&str> = output.split("\n\n").collect();

                let mut vm_arguments = None;
                let mut vm_resources = None;
                for block in blocks {
                    let rows: Vec<&str> = block.split("\n").collect();
                    let row1 = (&rows).get(0);
                    if let Some(first_row) = row1 {
                        if first_row.starts_with("VM Arguments:") {
                            let mut jvm_args = None;
                            let mut java_command = None;
                            for row in rows {
                                if row.starts_with("jvm_args:") {
                                    jvm_args = Some(row.replace("jvm_args:", "").trim().to_string())
                                } else if row.starts_with("java_command:") {
                                    java_command = Some(row.replace("java_command:", "").trim().to_string())
                                }
                            }
                            vm_arguments = Some(VmArguments {
                                jvm_args,
                                java_command,
                            });
                        } else if first_row.starts_with("GC Precious Log:") {
                            let mut cpus = None;
                            let mut memory = None;
                            let mut heap_size_min = None;
                            let mut heap_size_init = None;
                            let mut heap_size_max = None;
                            for row in rows {
                                if row.starts_with(" CPUs:") {
                                    cpus = Some(row.replace(" CPUs:", "").trim().to_string())
                                } else if row.starts_with(" Memory:") {
                                    memory = Some(row.replace(" Memory:", "").trim().to_string())
                                } else if row.starts_with(" Heap Min Capacity:") {
                                    heap_size_min = Some(row.replace(" Heap Min Capacity:", "").trim().to_string())
                                } else if row.starts_with(" Heap Initial Capacity:") {
                                    heap_size_init = Some(row.replace(" Heap Initial Capacity:", "").trim().to_string())
                                } else if row.starts_with(" Heap Max Capacity:") {
                                    heap_size_max = Some(row.replace(" Heap Max Capacity:", "").trim().to_string())
                                }
                            }
                            vm_resources = Some(VmResources {
                                cpus,
                                memory,
                                heap_size_min,
                                heap_size_init,
                                heap_size_max,
                            })
                        }
                    }
                }

                return Ok(VmInformation {
                    vm_arguments,
                    vm_resources,
                });
            }
            return Err("Data couldn't be read successfully".to_string());
        }
        Err(e) => Err(e.to_string())
    }
}

#[tauri::command]
fn get_jvm_metrics(pid: &str) -> Result<JvmMetrics, String> {
    let time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis();
    let mut heap_size = None;
    let mut metaspace_size = None;
    let mut class_space_size = None;

    // Parse committed / reserved sizes from heap info as they seem to be more accurate
    // than the values from the native memory tracking.
    let mut metaspace_committed = None;
    let mut metaspace_reserved = None;
    let mut class_space_committed = None;
    let mut class_space_reserved = None;

    match jcmd().arg(pid).arg("GC.heap_info").output() {
        Ok(o) => {
            let output = String::from_utf8_lossy(o.stdout.as_slice()).to_string();
            if (&output).contains("IOException: No such process") {
                return Err("No such process".to_string());
            }

            let rows: Vec<&str> = output.split("\n").collect();
            for row in rows {
                if row.starts_with(" garbage-first") {
                    heap_size = parse_memory_from_heap_info(row, "used");
                } else if row.starts_with(" Metaspace") {
                    metaspace_size = parse_memory_from_heap_info(row, "used");
                    metaspace_committed = parse_memory_from_heap_info(row, "committed");
                    metaspace_reserved = parse_memory_from_heap_info(row, "reserved");
                } else if row.starts_with("  class space") {
                    class_space_size = parse_memory_from_heap_info(row, "used");
                    class_space_committed = parse_memory_from_heap_info(row, "committed");
                    class_space_reserved = parse_memory_from_heap_info(row, "reserved");
                }
            }
        }
        Err(e) => return Err(e.to_string())
    };
    match jcmd().arg(pid).arg("VM.native_memory").arg("scale=b").output() {
        Ok(o) => {
            if o.status.success() {
                let output = String::from_utf8_lossy(o.stdout.as_slice()).to_string();
                if (&output).contains("IOException: No such process") {
                    return Err("No such process".to_string());
                } else if (&output).contains("Native memory tracking is not enabled") {
                    return Err("Native memory tracking not activated. Start application with java \
                    -XX:NativeMemoryTracking=summary -jar ...".to_string());
                }

                let rows: Vec<&str> = output.split("\n").collect();
                let mut buffer: Vec<String> = Vec::new();

                for row in rows {
                    if row.trim() == "" {
                        for buffer_row in buffer {
                            if buffer_row.starts_with("Total:") {
                                let total_memory_parts: Vec<&str> = buffer_row.split(" ").collect();
                                let (reserved, committed) = parse_reserved_committed(total_memory_parts);
                                let total_memory_metric = TotalMemoryMetricValue {
                                    time,
                                    reserved,
                                    committed,
                                };
                                CACHE.lock().unwrap().total_memory.values.push(total_memory_metric)
                            } else if buffer_row.starts_with("-") && buffer_row.contains("Class") {
                                let class_memory_metric = ClassMemoryMetricValue {
                                    time,
                                    reserved: class_space_reserved,
                                    committed: class_space_committed,
                                    class_count: 0,
                                    used: class_space_size,
                                };
                                CACHE.lock().unwrap().class_metrics.values.push(class_memory_metric)
                            } else if buffer_row.starts_with("-") && buffer_row.contains("Thread") {
                                let cleaned_row = buffer_row.replace("-", "").replace("Thread", "").replace("(", "").replace(")", "")
                                    .replace(",", "");
                                let values: Vec<&str> = cleaned_row.split(" ").collect();
                                let (reserved, committed) = parse_reserved_committed(values);
                                let thread_memory_metric = ThreadMemoryMetricValue {
                                    time,
                                    reserved,
                                    committed,
                                    thread_count: 0,
                                };
                                CACHE.lock().unwrap().thread_metrics.values.push(thread_memory_metric);
                            } else if buffer_row.starts_with("-") && buffer_row.contains("Java Heap") {
                                let cleaned_row = buffer_row.replace("-", "").replace("Java Heap", "").replace("(", "").replace(")", "")
                                    .replace(",", "");
                                let values: Vec<&str> = cleaned_row.split(" ").collect();

                                let (reserved, committed) = parse_reserved_committed(values);
                                let heap_memory_metric = HeapMemoryMetricValue {
                                    time,
                                    reserved,
                                    committed,
                                    used: heap_size,
                                };
                                CACHE.lock().unwrap().heap_metrics.values.push(heap_memory_metric);
                            } else if buffer_row.starts_with("-") && buffer_row.contains("Metaspace") {
                                let metaspace_memory_metric = MetaspaceMemoryMetricValue {
                                    time,
                                    reserved: metaspace_reserved,
                                    committed: metaspace_committed,
                                    used: metaspace_size,
                                };
                                CACHE.lock().unwrap().metaspace_metrics.values.push(metaspace_memory_metric);
                            } else if buffer_row.starts_with("-") {
                                let cleaned_row = buffer_row.replace("-", "").replace("(", "").replace(")", "")
                                    .replace(",", "");
                                let values: Vec<&str> = cleaned_row.split(" ").collect();

                                let (name, reserved, committed) = parse_name_reserved_committed(values);
                                let metric = GenericMemoryMetricValue {
                                    time,
                                    reserved,
                                    committed,
                                };
                                let n = name.unwrap();
                                let mut c = CACHE.lock().unwrap();
                                if !c.other_metrics.contains_key(&n) {
                                    c.other_metrics.insert(n.clone(), GenericMemoryMetric {
                                        name: n,
                                        values: vec![metric],
                                    });
                                } else {
                                    c.other_metrics.get_mut(&n).unwrap().values.push(metric);
                                }
                            }  // else ignore
                        }

                        // reset buffer
                        buffer = Vec::new();
                    } else {
                        buffer.push(row.to_string());
                    }
                }

                let c = CACHE.lock().unwrap();
                let jvm_metrics = JvmMetrics {
                    total_memory: c.total_memory.clone(),
                    class: c.class_metrics.clone(),
                    heap: c.heap_metrics.clone(),
                    metaspace: c.metaspace_metrics.clone(),
                    thread: c.thread_metrics.clone(),
                    other: c.other_metrics.values().into_iter().map(|o| o.clone()).collect(),
                };
                return Ok(jvm_metrics);
            }
            return Err("Data couldn't be read successfully".to_string());
        }
        Err(e) => Err(e.to_string())
    }
}

fn parse_reserved_committed(parts: Vec<&str>) -> (Option<u64>, Option<u64>) {
    let mut reserved = None;
    let mut committed = None;
    for part in parts {
        if part.starts_with("reserved=") {
            reserved = Some(part.replace("reserved=", "").replace(",", "").trim().parse::<u64>().unwrap())
        } else if part.starts_with("committed=") {
            committed = Some(part.replace("committed=", "").replace(",", "").trim().parse::<u64>().unwrap())
        }
    }
    return (reserved, committed);
}

fn parse_name_reserved_committed(parts: Vec<&str>) -> (Option<String>, Option<u64>, Option<u64>) {
    let mut name_buffer = Vec::new();
    for p in &parts {
        if p.starts_with("reserved") {
            break;
        } else {
            if p.len() > 0 {
                name_buffer.push(p.trim().to_string());
            }
        }
    }
    let name = Some(name_buffer.join(" "));
    let (reserved, committed) = parse_reserved_committed(parts);
    return (name, reserved, committed);
}

fn parse_memory_from_heap_info(row: &str, memory_type: &str) -> Option<u64> {
    let parts: Vec<&str> = row.split(" ").collect();
    for (i, p) in parts.iter().enumerate() {
        if p == &memory_type {
            let size_string = parts.get(i + 1).unwrap();
            let size = size_string.replace("K", "").replace(",", "").trim().parse::<u64>().unwrap();
            return Some(size * 1024);
        }
    }
    None
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VmInformation {
    vm_arguments: Option<VmArguments>,
    vm_resources: Option<VmResources>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VmArguments {
    jvm_args: Option<String>,
    java_command: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VmResources {
    cpus: Option<String>,
    memory: Option<String>,
    heap_size_min: Option<String>,
    heap_size_init: Option<String>,
    heap_size_max: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JvmMetrics {
    total_memory: TotalMemoryMetric,
    class: ClassMemoryMetric,
    heap: HeapMemoryMetric,
    metaspace: MetaspaceMemoryMetric,
    thread: ThreadMemoryMetric,
    other: Vec<GenericMemoryMetric>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenericMemoryMetric {
    name: String,
    values: Vec<GenericMemoryMetricValue>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenericMemoryMetricValue {
    time: u128,
    reserved: Option<u64>,
    committed: Option<u64>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassMemoryMetric {
    name: String,
    values: Vec<ClassMemoryMetricValue>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassMemoryMetricValue {
    time: u128,
    class_count: u32,
    reserved: Option<u64>,
    committed: Option<u64>,
    used: Option<u64>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetaspaceMemoryMetric {
    name: String,
    values: Vec<MetaspaceMemoryMetricValue>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetaspaceMemoryMetricValue {
    time: u128,
    reserved: Option<u64>,
    committed: Option<u64>,
    used: Option<u64>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HeapMemoryMetric {
    name: String,
    values: Vec<HeapMemoryMetricValue>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HeapMemoryMetricValue {
    time: u128,
    reserved: Option<u64>,
    committed: Option<u64>,
    used: Option<u64>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadMemoryMetric {
    name: String,
    values: Vec<ThreadMemoryMetricValue>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadMemoryMetricValue {
    time: u128,
    thread_count: u32,
    reserved: Option<u64>,
    committed: Option<u64>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TotalMemoryMetric {
    values: Vec<TotalMemoryMetricValue>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TotalMemoryMetricValue {
    time: u128,
    reserved: Option<u64>,
    committed: Option<u64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct JvmProcesses {
    processes: Vec<JvmProcessRef>,
}

#[derive(Clone, Debug, Serialize)]
pub struct JvmProcessRef {
    id: String,
    name: String,
    path: Option<String>,
}

struct MetricsCache {
    total_memory: TotalMemoryMetric,
    class_metrics: ClassMemoryMetric,
    heap_metrics: HeapMemoryMetric,
    metaspace_metrics: MetaspaceMemoryMetric,
    thread_metrics: ThreadMemoryMetric,
    other_metrics: HashMap<String, GenericMemoryMetric>,
}