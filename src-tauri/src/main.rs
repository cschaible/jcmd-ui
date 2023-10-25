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

static CACHE: Lazy<Mutex<MetricsCache>> = Lazy::new(|| {
    Mutex::new(MetricsCache {
        total_memory: NamedMetric::new("Total".to_string()),
        class_metrics: NamedMetric::new("Class".to_string()),
        heap_metrics: NamedMetric::new("Heap".to_string()),
        metaspace_metrics: NamedMetric::new("Metaspace".to_string()),
        thread_metrics: NamedMetric::new("Thread".to_string()),
        thread_count_metrics_application: NamedMetric::new("ThreadCountApplication".to_string()),
        thread_count_metrics_jvm: NamedMetric::new("ThreadCountJvm".to_string()),
        other_metrics: HashMap::new(),
    })
});

static THREAD_CACHE: Lazy<Mutex<HashMap<String, ThreadCacheEntry>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

static JCMD: Lazy<Mutex<String>> = Lazy::new(|| Mutex::new("".to_string()));

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
    let mut c = CACHE.lock().unwrap();
    c.class_metrics.values.clear();
    c.heap_metrics.values.clear();
    c.metaspace_metrics.values.clear();
    c.other_metrics.clear();
    c.total_memory.values.clear();
    c.thread_count_metrics_application.values.clear();
    c.thread_count_metrics_jvm.values.clear();
    c.thread_metrics.values.clear();
}

fn jcmd() -> Command {
    let mut cmd = JCMD.lock().unwrap();
    let mut path = (*cmd).clone();
    if (*cmd).is_empty() {
        let p = match std::env::var("JAVA_HOME") {
            Ok(p) => format!("{}/bin/jcmd", p),
            Err(_) => match std::env::var("JCMD") {
                Ok(p) => p,
                Err(_) => "jcmd".to_string(),
            },
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
                let lines: Vec<&str> = output.split('\n').collect();
                let mut processes: Vec<JvmProcessRef> = Vec::new();
                for line in lines {
                    if (line).contains("jdk.jcmd") {
                        continue;
                    }
                    let parts: Vec<&str> = line.split(' ').collect();
                    if parts.len() < 2 {
                        continue;
                    }
                    if parts.len() >= 3 {
                        let mut path: Vec<String> = Vec::new();
                        for p in 2..(parts.len()) {
                            let value = parts.get(p);
                            if let Some(v) = value {
                                path.push(v.to_string());
                            }
                        }
                        processes.push(JvmProcessRef {
                            id: parts.first().unwrap().to_string(),
                            name: parts.get(1).unwrap().to_string(),

                            path: Some(path.join(" ")),
                        })
                    } else {
                        processes.push(JvmProcessRef {
                            id: parts.first().unwrap().to_string(),
                            name: parts.get(1).unwrap().to_string(),
                            path: None,
                        })
                    }
                }

                return Ok(JvmProcesses { processes });
            }
            Err("Data couldn't be read successfully".to_string())
        }
        Err(e) => Err(e.to_string()),
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
                    let rows: Vec<&str> = block.split('\n').collect();
                    let row1 = rows.first();
                    if let Some(first_row) = row1 {
                        if first_row.starts_with("VM Arguments:") {
                            let mut jvm_args = None;
                            let mut java_command = None;
                            for row in rows {
                                if row.starts_with("jvm_args:") {
                                    jvm_args = Some(row.replace("jvm_args:", "").trim().to_string())
                                } else if row.starts_with("java_command:") {
                                    java_command =
                                        Some(row.replace("java_command:", "").trim().to_string())
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
                                    heap_size_min = Some(
                                        row.replace(" Heap Min Capacity:", "").trim().to_string(),
                                    )
                                } else if row.starts_with(" Min Capacity:") {
                                    // zgc
                                    heap_size_min =
                                        Some(row.replace(" Min Capacity:", "").trim().to_string())
                                } else if row.starts_with(" Heap Initial Capacity:") {
                                    heap_size_init = Some(
                                        row.replace(" Heap Initial Capacity:", "")
                                            .trim()
                                            .to_string(),
                                    )
                                } else if row.starts_with(" Initial Capacity:") {
                                    // zgc
                                    heap_size_init = Some(
                                        row.replace(" Initial Capacity:", "").trim().to_string(),
                                    )
                                } else if row.starts_with(" Heap Max Capacity:") {
                                    heap_size_max = Some(
                                        row.replace(" Heap Max Capacity:", "").trim().to_string(),
                                    )
                                } else if row.starts_with(" Max Capacity:") {
                                    // zgc
                                    heap_size_max =
                                        Some(row.replace(" Max Capacity:", "").trim().to_string())
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
            Err("Data couldn't be read successfully".to_string())
        }
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
fn get_jvm_metrics(pid: &str) -> Result<JvmMetrics, String> {
    let time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis();
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
            if output.contains("IOException: No such process") {
                return Err("No such process".to_string());
            }

            let rows: Vec<&str> = output.split('\n').collect();
            let mut is_shenandoah = false;
            for row in rows {
                if row.starts_with("Shenandoah") {
                    is_shenandoah = true;
                } else if is_shenandoah && heap_size.is_none() {
                    // parse used size from second line
                    heap_size = parse_memory_from_heap_info(row, "used", true);
                } else if row.starts_with(" ZHeap")
                    || row.starts_with(" garbage-first")
                    // def new generation is serial - new gen
                    || row.starts_with(" def new generation")
                {
                    heap_size = parse_memory_from_heap_info(row, "used", false);
                } else if row.starts_with(" tenured generation") && heap_size.is_some() {
                    // serial - old gen
                    let old_gen_size = parse_memory_from_heap_info(row, "used", false);
                    if let Some(old_gen) = old_gen_size {
                        heap_size = Some(heap_size.unwrap() + old_gen);
                    }
                } else if row.starts_with(" PSYoungGen") {
                    // parallel - new gen
                    heap_size = parse_memory_from_heap_info(row, "used", false);
                } else if row.starts_with(" ParOldGen") && heap_size.is_some() {
                    // parallel - old gen
                    let old_gen_size = parse_memory_from_heap_info(row, "used", false);
                    if let Some(old_gen) = old_gen_size {
                        heap_size = Some(heap_size.unwrap() + old_gen);
                    }
                } else if row.starts_with(" Metaspace") {
                    metaspace_size = parse_memory_from_heap_info(row, "used", false);
                    metaspace_committed = parse_memory_from_heap_info(row, "committed", false);
                    metaspace_reserved = parse_memory_from_heap_info(row, "reserved", false);
                } else if row.starts_with("  class space") {
                    class_space_size = parse_memory_from_heap_info(row, "used", false);
                    class_space_committed = parse_memory_from_heap_info(row, "committed", false);
                    class_space_reserved = parse_memory_from_heap_info(row, "reserved", false);
                }
            }
        }
        Err(e) => return Err(e.to_string()),
    };
    match jcmd()
        .arg(pid)
        .arg("VM.native_memory")
        .arg("scale=b")
        .output()
    {
        Ok(o) => {
            if o.status.success() {
                let output = String::from_utf8_lossy(o.stdout.as_slice()).to_string();
                if output.contains("IOException: No such process") {
                    return Err("No such process".to_string());
                } else if output.contains("Native memory tracking is not enabled") {
                    return Err(
                        "Native memory tracking not activated. Start application with java \
                    -XX:NativeMemoryTracking=summary -jar ..."
                            .to_string(),
                    );
                }

                let rows: Vec<&str> = output.split('\n').collect();
                let mut buffer: Vec<String> = Vec::new();

                for row in rows {
                    if row.trim() == "" {
                        for buffer_row in buffer {
                            if buffer_row.starts_with("Total:") {
                                let total_memory_parts: Vec<&str> = buffer_row.split(' ').collect();
                                let (reserved, committed) =
                                    parse_reserved_committed(total_memory_parts);
                                let total_memory_metric = TotalMemoryMetricValue {
                                    time,
                                    reserved,
                                    committed,
                                };
                                CACHE
                                    .lock()
                                    .unwrap()
                                    .total_memory
                                    .values
                                    .push(total_memory_metric)
                            } else if buffer_row.starts_with('-') && buffer_row.contains("Class") {
                                let class_memory_metric = ClassMemoryMetricValue {
                                    time,
                                    reserved: class_space_reserved,
                                    committed: class_space_committed,
                                    class_count: 0,
                                    used: class_space_size,
                                };
                                CACHE
                                    .lock()
                                    .unwrap()
                                    .class_metrics
                                    .values
                                    .push(class_memory_metric)
                            } else if buffer_row.starts_with('-') && buffer_row.contains("Thread") {
                                let cleaned_row = buffer_row
                                    .replace(['(', ')', ','], "")
                                    .replace("Thread", "");
                                let values: Vec<&str> = cleaned_row.split(' ').collect();
                                let (reserved, committed) = parse_reserved_committed(values);
                                let thread_memory_metric = ThreadMemoryMetricValue {
                                    time,
                                    reserved,
                                    committed,
                                    thread_count: 0,
                                };
                                CACHE
                                    .lock()
                                    .unwrap()
                                    .thread_metrics
                                    .values
                                    .push(thread_memory_metric);
                            } else if buffer_row.starts_with('-')
                                && buffer_row.contains("Java Heap")
                            {
                                let cleaned_row = buffer_row
                                    .replace(['(', ')', ','], "")
                                    .replace("Java Heap", "");
                                let values: Vec<&str> = cleaned_row.split(' ').collect();

                                let (reserved, committed) = parse_reserved_committed(values);
                                let heap_memory_metric = HeapMemoryMetricValue {
                                    time,
                                    reserved,
                                    committed,
                                    used: heap_size,
                                };
                                CACHE
                                    .lock()
                                    .unwrap()
                                    .heap_metrics
                                    .values
                                    .push(heap_memory_metric);
                            } else if buffer_row.starts_with('-')
                                && buffer_row.contains("Metaspace")
                            {
                                let metaspace_memory_metric = MetaspaceMemoryMetricValue {
                                    time,
                                    reserved: metaspace_reserved,
                                    committed: metaspace_committed,
                                    used: metaspace_size,
                                };
                                CACHE
                                    .lock()
                                    .unwrap()
                                    .metaspace_metrics
                                    .values
                                    .push(metaspace_memory_metric);
                            } else if buffer_row.starts_with('-') {
                                let cleaned_row = buffer_row.replace(['-', '(', ')', ','], "");
                                let values: Vec<&str> = cleaned_row.split(' ').collect();

                                let (name, reserved, committed) =
                                    parse_name_reserved_committed(values);
                                let metric = GenericMemoryMetricValue {
                                    time,
                                    reserved,
                                    committed,
                                };
                                let n = name.unwrap();
                                let mut c = CACHE.lock().unwrap();
                                if !c.other_metrics.contains_key(&n) {
                                    c.other_metrics.insert(
                                        n.clone(),
                                        GenericMemoryMetric {
                                            name: n,
                                            values: vec![metric],
                                        },
                                    );
                                } else {
                                    c.other_metrics.get_mut(&n).unwrap().values.push(metric);
                                }
                            } // else ignore
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
                    other: c.other_metrics.values().cloned().collect(),
                };
                return Ok(jvm_metrics);
            }
            Err("Data couldn't be read successfully".to_string())
        }
        Err(e) => Err(e.to_string()),
    }
}

// Intro to thread dumps: https://dzone.com/articles/how-to-read-a-thread-dump
#[tauri::command]
fn get_threads(pid: &str) -> Result<Threads, String> {
    let time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    match jcmd().arg(pid).arg("Thread.print").arg("-e").output() {
        Ok(o) => {
            let output = String::from_utf8_lossy(o.stdout.as_slice()).to_string();
            let blocks: Vec<&str> = output.split("\n\n").collect();

            let mut new_thread_count_application = 0;
            let mut new_thread_count_jvm = 0;
            let mut runnable_thread_count_application = 0;
            let mut runnable_thread_count_jvm = 0;
            let mut timed_waiting_thread_count_application = 0;
            let mut waiting_thread_count_application = 0;
            let mut waiting_thread_count_jvm = 0;
            let mut blocked_thread_count_application = 0;
            let mut blocked_thread_count_jvm = 0;

            let mut application_threads = Vec::new();
            let mut jvm_threads = Vec::new();
            for block in blocks {
                let rows: Vec<&str> = block.split('\n').collect();
                let row1 = rows.first();
                if let Some(first_row) = row1 {
                    if let Some(first_row_without_first_quote) = first_row.strip_prefix('\"') {
                        let title_quote_indexes = first_row
                            .chars()
                            .enumerate()
                            .filter(|(_, c)| *c == '\"')
                            .map(|(i, _)| i)
                            .collect::<Vec<_>>();

                        let thread_name_end_quote = title_quote_indexes.last().unwrap();
                        let name = first_row_without_first_quote
                            .chars()
                            .take(thread_name_end_quote - 1)
                            .collect::<String>();
                        let rest = first_row[thread_name_end_quote + 2..]
                            .chars()
                            .take(first_row.len() - thread_name_end_quote + 2)
                            .collect::<String>();

                        let mut status = "".to_string();
                        if rows.len() > 1 {
                            let row2 = rows[1];
                            let thread_state_input = row2.replace("java.lang.Thread.State:", "");
                            let thread_state_parts: Vec<&str> =
                                thread_state_input.split(' ').collect();
                            let thread_state_parts: Vec<&str> = thread_state_parts
                                .into_iter()
                                .filter(|p| p != &"")
                                .collect();
                            status = thread_state_parts.join(" ");
                        }

                        if rest.contains('#') {
                            let elements: Vec<&str> = rest.split(' ').collect();
                            let mut id = 0;
                            let mut daemon = false;
                            let mut prio = 0;
                            let mut os_thread_prio = 0;
                            let mut cpu = 0.0;
                            let mut elapsed = 0.0;
                            let mut allocated = "".to_string();
                            let mut defined_classes = 0;
                            let mut tid = "".to_string();
                            let mut nid = "".to_string();
                            //let mut status: Vec<&str> = Vec::new();
                            let mut last_known_java_stack_pointer = "".to_string();
                            for e in elements {
                                if e.starts_with('#') {
                                    id = e.replace('#', "").parse::<u32>().unwrap();
                                } else if e == "daemon" {
                                    daemon = true;
                                } else if e.starts_with("prio=") {
                                    prio = e.replace("prio=", "").parse::<u16>().unwrap();
                                } else if e.starts_with("cpu=") {
                                    cpu = calculate_thread_time(e, "cpu=")
                                } else if e.starts_with("elapsed=") {
                                    elapsed = calculate_thread_time(e, "elapsed=")
                                } else if e.starts_with("allocated=") {
                                    allocated = e.replace("allocated=", "");
                                } else if e.starts_with("defined_classes=") {
                                    defined_classes =
                                        e.replace("defined_classes=", "").parse::<u16>().unwrap();
                                } else if e.starts_with("os_prio=") {
                                    os_thread_prio =
                                        e.replace("os_prio=", "").parse::<u16>().unwrap();
                                } else if e.starts_with("tid=") {
                                    tid = e.replace("tid=", "");
                                } else if e.starts_with("nid=") {
                                    nid = e.replace("nid=", "");
                                } else if !nid.is_empty() && !e.starts_with('[') {
                                    //status.push(e);
                                } else if e.starts_with('[') {
                                    last_known_java_stack_pointer = e.replace(['[', ']'], "");
                                }
                            }

                            cpu = update_thread_cache(cpu, name.clone(), false);

                            let thread = ApplicationThread {
                                name,
                                id,
                                daemon,
                                prio,
                                os_thread_prio,
                                cpu,
                                elapsed,
                                allocated,
                                defined_classes,
                                thread_id: tid.to_string(),
                                os_thread_id: nid.to_string(),
                                status: status.clone(), //status.join(" ").to_string(),
                                last_known_java_stack_pointer: last_known_java_stack_pointer
                                    .to_string(),
                            };
                            application_threads.push(thread);

                            // The java.lang.Thread class contains a static State enum
                            if status.starts_with("NEW") {
                                new_thread_count_application += 1;
                            } else if status.starts_with("RUNNABLE") {
                                runnable_thread_count_application += 1;
                            } else if status.starts_with("TIMED_WAITING") {
                                timed_waiting_thread_count_application += 1;
                            } else if status.starts_with("WAITING") {
                                waiting_thread_count_application += 1;
                            } else if status.starts_with("BLOCKED") {
                                blocked_thread_count_application += 1;
                            }
                        } else {
                            let elements: Vec<&str> = rest.split(' ').collect();
                            let mut os_thread_prio = 0;
                            let mut cpu = 0.0;
                            let mut elapsed = 0.0;
                            let mut tid = "".to_string();
                            let mut nid = "".to_string();
                            let mut status: Vec<&str> = Vec::new();
                            for e in elements {
                                if e.starts_with("os_prio=") {
                                    os_thread_prio =
                                        e.replace("os_prio=", "").parse::<u16>().unwrap();
                                } else if e.starts_with("cpu=") {
                                    cpu = calculate_thread_time(e, "cpu=");
                                } else if e.starts_with("elapsed=") {
                                    elapsed = calculate_thread_time(e, "elapsed=")
                                } else if e.starts_with("tid=") {
                                    tid = e.replace("tid=", "");
                                } else if e.starts_with("nid=") {
                                    nid = e.replace("nid=", "");
                                } else if !nid.is_empty() && !e.starts_with('[') {
                                    status.push(e);
                                }
                            }

                            cpu = update_thread_cache(cpu, name.clone(), true);
                            let status_name = status.join(" ").to_string();

                            let thread = JvmThread {
                                name,
                                os_thread_prio,
                                cpu,
                                elapsed,
                                thread_id: tid,
                                os_thread_id: nid,
                                status: status_name.clone(),
                            };
                            jvm_threads.push(thread);

                            // The java.lang.Thread class contains a static State enum
                            if status_name.starts_with("new") {
                                new_thread_count_jvm += 1;
                            } else if status_name.starts_with("runnable") {
                                runnable_thread_count_jvm += 1;
                            } else if status_name.starts_with("waiting on condition") {
                                // There's no separation of thread state waiting and timed waiting for jvm threads
                                waiting_thread_count_jvm += 1;
                            } else if status_name.starts_with("blocked") {
                                blocked_thread_count_jvm += 1;
                            }
                        }
                    }
                }
            }

            let thread_count_application = ThreadCountMetricValue {
                time,
                new_count: new_thread_count_application,
                runnable_count: runnable_thread_count_application,
                waiting_count: waiting_thread_count_application,
                timed_waiting_count: timed_waiting_thread_count_application,
                blocked_count: blocked_thread_count_application,
            };

            let mut c = CACHE.lock().unwrap();
            c.thread_count_metrics_application
                .values
                .push(thread_count_application);

            let thread_count_jvm = ThreadCountMetricValue {
                time,
                new_count: new_thread_count_jvm,
                runnable_count: runnable_thread_count_jvm,
                waiting_count: waiting_thread_count_jvm,
                timed_waiting_count: 0,
                blocked_count: blocked_thread_count_jvm,
            };
            c.thread_count_metrics_jvm.values.push(thread_count_jvm);

            Ok(Threads {
                application_threads,
                jvm_threads,
                thread_count_application: c.thread_count_metrics_application.clone(),
                thread_count_jvm: c.thread_count_metrics_jvm.clone(),
            })
        }
        Err(e) => Err(e.to_string()),
    }
}

fn calculate_thread_time(e: &str, column: &str) -> f32 {
    let factor = if e.contains("ms") {
        1
    } else if e.contains('m') {
        60 * 1000
    } else if e.contains('s') {
        1000
    } else {
        1
    };
    e.replace(column, "")
        .replace("ms", "")
        .replace('s', "")
        // On linux it seem to be required to replace the comma
        // with a dot to be able to parse the number
        .replace(',', ".")
        .parse::<f32>()
        .unwrap()
        * factor as f32
}

fn parse_reserved_committed(parts: Vec<&str>) -> (Option<u64>, Option<u64>) {
    let mut reserved = None;
    let mut committed = None;
    for part in parts {
        if part.starts_with("reserved=") {
            reserved = Some(
                part.replace("reserved=", "")
                    .replace(',', "")
                    .trim()
                    .parse::<u64>()
                    .unwrap(),
            )
        } else if part.starts_with("committed=") {
            committed = Some(
                part.replace("committed=", "")
                    .replace(',', "")
                    .trim()
                    .parse::<u64>()
                    .unwrap(),
            )
        }
    }
    (reserved, committed)
}

fn parse_name_reserved_committed(parts: Vec<&str>) -> (Option<String>, Option<u64>, Option<u64>) {
    let mut name_buffer = Vec::new();
    for p in &parts {
        if p.starts_with("reserved") {
            break;
        } else if !p.is_empty() {
            name_buffer.push(p.trim().to_string());
        }
    }
    let name = Some(name_buffer.join(" "));
    let (reserved, committed) = parse_reserved_committed(parts);
    (name, reserved, committed)
}

fn parse_memory_from_heap_info(row: &str, memory_type: &str, reversed: bool) -> Option<u64> {
    let parts: Vec<&str> = row.split(' ').collect();
    for (i, p) in parts.iter().enumerate() {
        if p == &memory_type {
            let size_index = if reversed { i - 1 } else { i + 1 };
            let size_string = parts.get(size_index).unwrap();
            let size_unit_factor = if size_string.contains('K') {
                1024
            } else if size_string.contains('M') {
                1024 * 1024
            } else if size_string.contains('G') {
                1024 * 1024 * 1024
            } else {
                1
            };

            let size = size_string
                .replace(['K', 'M', 'G', ','], "")
                .trim()
                .parse::<u64>()
                .unwrap();
            return Some(size * size_unit_factor);
        }
    }
    None
}

fn update_thread_cache(cpu: f32, name: String, jvm_tread: bool) -> f32 {
    let mut thread_cpu = 0.0f32;
    let mut mutex_guard = THREAD_CACHE.lock().unwrap();
    let existing_thread = mutex_guard.get_mut(&name);
    if let Some(thread_entry) = existing_thread {
        let existing_cpu = thread_entry.cpu;
        thread_cpu = (cpu - existing_cpu).abs();
        thread_entry.cpu = cpu;
    } else {
        mutex_guard.insert(
            name.clone(),
            ThreadCacheEntry {
                name: name.clone(),
                jvm_thread: jvm_tread,
                cpu,
            },
        );
    }
    thread_cpu
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
    total_memory: NamedMetric<TotalMemoryMetricValue>,
    class: NamedMetric<ClassMemoryMetricValue>,
    heap: NamedMetric<HeapMemoryMetricValue>,
    metaspace: NamedMetric<MetaspaceMemoryMetricValue>,
    thread: NamedMetric<ThreadMemoryMetricValue>,
    other: Vec<GenericMemoryMetric>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NamedMetric<T> {
    name: String,
    values: Vec<T>,
}

impl<T> NamedMetric<T> {
    fn new(name: String) -> NamedMetric<T> {
        NamedMetric {
            name,
            values: Vec::new(),
        }
    }
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
pub struct ClassMemoryMetricValue {
    time: u128,
    class_count: u32,
    reserved: Option<u64>,
    committed: Option<u64>,
    used: Option<u64>,
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
pub struct HeapMemoryMetricValue {
    time: u128,
    reserved: Option<u64>,
    committed: Option<u64>,
    used: Option<u64>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadCountMetricValue {
    time: u128,
    new_count: u32,
    runnable_count: u32,
    waiting_count: u32,
    timed_waiting_count: u32,
    blocked_count: u32,
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
    total_memory: NamedMetric<TotalMemoryMetricValue>,
    class_metrics: NamedMetric<ClassMemoryMetricValue>,
    heap_metrics: NamedMetric<HeapMemoryMetricValue>,
    metaspace_metrics: NamedMetric<MetaspaceMemoryMetricValue>,
    thread_metrics: NamedMetric<ThreadMemoryMetricValue>,
    thread_count_metrics_application: NamedMetric<ThreadCountMetricValue>,
    thread_count_metrics_jvm: NamedMetric<ThreadCountMetricValue>,
    other_metrics: HashMap<String, GenericMemoryMetric>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationThread {
    name: String,
    id: u32,
    daemon: bool,
    prio: u16,
    os_thread_prio: u16,
    //https://www.linkedin.com/pulse/getting-java-thread-cpu-utilization-vishalendu-pandey
    cpu: f32,
    elapsed: f32,
    allocated: String,
    defined_classes: u16,
    thread_id: String,
    os_thread_id: String,
    status: String,
    last_known_java_stack_pointer: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JvmThread {
    name: String,
    os_thread_prio: u16,
    cpu: f32,
    elapsed: f32,
    thread_id: String,
    os_thread_id: String,
    status: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Threads {
    application_threads: Vec<ApplicationThread>,
    jvm_threads: Vec<JvmThread>,
    thread_count_application: NamedMetric<ThreadCountMetricValue>,
    thread_count_jvm: NamedMetric<ThreadCountMetricValue>,
}

#[derive(Clone, Debug, Serialize)]
struct ThreadCacheEntry {
    name: String,
    jvm_thread: bool,
    cpu: f32,
}
