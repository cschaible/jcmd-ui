use std::collections::hash_map::Entry;
use std::collections::HashMap;

use anyhow::{anyhow, Result};
use serde::Serialize;

pub fn parse_jvm_processes(output: String) -> Result<JvmProcesses> {
    let lines: Vec<&str> = output.split('\n').collect();
    let mut processes: Vec<JvmProcessRef> = Vec::new();
    for line in lines {
        if line.contains("jdk.jcmd") {
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

    Ok(JvmProcesses { processes })
}

pub fn parse_vm_information(output: String) -> Result<VmInformation> {
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
                        heap_size_min =
                            Some(row.replace(" Heap Min Capacity:", "").trim().to_string())
                    } else if row.starts_with(" Min Capacity:") {
                        // zgc
                        heap_size_min = Some(row.replace(" Min Capacity:", "").trim().to_string())
                    } else if row.starts_with(" Heap Initial Capacity:") {
                        heap_size_init = Some(
                            row.replace(" Heap Initial Capacity:", "")
                                .trim()
                                .to_string(),
                        )
                    } else if row.starts_with(" Initial Capacity:") {
                        // zgc
                        heap_size_init =
                            Some(row.replace(" Initial Capacity:", "").trim().to_string())
                    } else if row.starts_with(" Heap Max Capacity:") {
                        heap_size_max =
                            Some(row.replace(" Heap Max Capacity:", "").trim().to_string())
                    } else if row.starts_with(" Max Capacity:") {
                        // zgc
                        heap_size_max = Some(row.replace(" Max Capacity:", "").trim().to_string())
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

    Ok(VmInformation {
        vm_arguments,
        vm_resources,
    })
}

pub fn parse_heap_info(output: String) -> Result<JvmHeapInfo> {
    let mut heap_size = None;
    let mut metaspace_size = None;
    let mut class_space_size = None;

    // Parse committed / reserved sizes from heap info as they seem to be more accurate
    // than the values from the native memory tracking.
    let mut metaspace_committed = None;
    let mut metaspace_reserved = None;
    let mut class_space_committed = None;
    let mut class_space_reserved = None;

    if output.contains("IOException: No such process") {
        return Err(anyhow!("No such process".to_string()));
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

    Ok(JvmHeapInfo {
        heap_size,
        metaspace_size,
        class_space_size,
        metaspace_committed,
        metaspace_reserved,
        class_space_committed,
        class_space_reserved,
    })
}

pub fn parse_native_memory(output: String, time: u128) -> Result<NativeMemoryMetrics> {
    let rows: Vec<&str> = output.split('\n').collect();
    let mut buffer: Vec<String> = Vec::new();

    let mut metrics: HashMap<String, NativeMemoryMetricValue> = HashMap::new();

    for row in rows {
        if row.trim() == "" {
            for buffer_row in buffer {
                if buffer_row.starts_with("Total:") {
                    let total_memory_parts: Vec<&str> = buffer_row.split(' ').collect();
                    let (reserved, committed) = parse_reserved_committed(total_memory_parts);
                    let metric = NativeMemoryMetricValue {
                        time,
                        reserved,
                        committed,
                        used: None,
                    };
                    add_metric(&mut metrics, metric, "Total".to_string());
                } else if buffer_row.starts_with('-') && buffer_row.contains("Thread") {
                    let cleaned_row = buffer_row
                        .replace(['(', ')', ','], "")
                        .replace("Thread", "");
                    let values: Vec<&str> = cleaned_row.split(' ').collect();

                    let (reserved, committed) = parse_reserved_committed(values);
                    let metric = NativeMemoryMetricValue {
                        time,
                        reserved,
                        committed,
                        used: None,
                    };
                    add_metric(&mut metrics, metric, "Thread".to_string());
                } else if buffer_row.starts_with('-') && buffer_row.contains("Java Heap") {
                    let cleaned_row = buffer_row
                        .replace(['(', ')', ','], "")
                        .replace("Java Heap", "");
                    let values: Vec<&str> = cleaned_row.split(' ').collect();

                    let (reserved, committed) = parse_reserved_committed(values);
                    let metric = NativeMemoryMetricValue {
                        time,
                        reserved,
                        committed,
                        used: None,
                    };
                    add_metric(&mut metrics, metric, "Heap".to_string());
                } else if buffer_row.starts_with('-') && buffer_row.contains("Metaspace") {
                    // do nothing - metric collected in heap info
                } else if buffer_row.starts_with('-') {
                    let cleaned_row = buffer_row.replace(['-', '(', ')', ','], "");
                    let values: Vec<&str> = cleaned_row.split(' ').collect();

                    let (name, reserved, committed) = parse_name_reserved_committed(values);
                    let metric = NativeMemoryMetricValue {
                        time,
                        reserved,
                        committed,
                        used: None,
                    };
                    add_metric(&mut metrics, metric, name.unwrap());
                } // else ignore
            }

            // reset buffer
            buffer = Vec::new();
        } else {
            buffer.push(row.to_string());
        }
    }

    Ok(NativeMemoryMetrics { metrics })
}

fn add_metric(
    metrics: &mut HashMap<String, NativeMemoryMetricValue>,
    metric: NativeMemoryMetricValue,
    name: String,
) {
    if let Entry::Vacant(e) = metrics.entry(name.clone()) {
        e.insert(metric);
    } else {
        // Error
        println!("metric {} detected twice", name);
    }
}

pub fn parse_threads(output: String, time: u128) -> Result<ThreadMetrics> {
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
                    let thread_state_parts: Vec<&str> = thread_state_input.split(' ').collect();
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
                            os_thread_prio = e.replace("os_prio=", "").parse::<u16>().unwrap();
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
                        last_known_java_stack_pointer: last_known_java_stack_pointer.to_string(),
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
                            os_thread_prio = e.replace("os_prio=", "").parse::<u16>().unwrap();
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

    let thread_count_jvm = ThreadCountMetricValue {
        time,
        new_count: new_thread_count_jvm,
        runnable_count: runnable_thread_count_jvm,
        waiting_count: waiting_thread_count_jvm,
        timed_waiting_count: 0,
        blocked_count: blocked_thread_count_jvm,
    };
    Ok(ThreadMetrics {
        application_threads,
        jvm_threads,
        thread_count_application,
        thread_count_jvm,
    })
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

#[derive(Clone, Debug, PartialOrd, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationThread {
    pub name: String,
    pub id: u32,
    pub daemon: bool,
    pub prio: u16,
    pub os_thread_prio: u16,
    //https://www.linkedin.com/pulse/getting-java-thread-cpu-utilization-vishalendu-pandey
    pub cpu: f32,
    pub elapsed: f32,
    pub allocated: String,
    pub defined_classes: u16,
    pub thread_id: String,
    pub os_thread_id: String,
    pub status: String,
    pub last_known_java_stack_pointer: String,
}

#[derive(Clone, Debug, PartialOrd, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JvmThread {
    pub name: String,
    pub os_thread_prio: u16,
    pub cpu: f32,
    pub elapsed: f32,
    pub thread_id: String,
    pub os_thread_id: String,
    pub status: String,
}

#[derive(Clone, Debug, PartialOrd, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadMetrics {
    pub application_threads: Vec<ApplicationThread>,
    pub jvm_threads: Vec<JvmThread>,
    pub thread_count_application: ThreadCountMetricValue,
    pub thread_count_jvm: ThreadCountMetricValue,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VmInformation {
    pub vm_arguments: Option<VmArguments>,
    pub vm_resources: Option<VmResources>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VmArguments {
    pub jvm_args: Option<String>,
    pub java_command: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VmResources {
    pub cpus: Option<String>,
    pub memory: Option<String>,
    pub heap_size_min: Option<String>,
    pub heap_size_init: Option<String>,
    pub heap_size_max: Option<String>,
}

pub struct JvmHeapInfo {
    pub heap_size: Option<u64>,
    pub metaspace_size: Option<u64>,
    pub class_space_size: Option<u64>,

    pub metaspace_committed: Option<u64>,
    pub metaspace_reserved: Option<u64>,
    pub class_space_committed: Option<u64>,
    pub class_space_reserved: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeMemoryMetrics {
    pub metrics: HashMap<String, NativeMemoryMetricValue>,
}

#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeMemoryMetricValue {
    pub time: u128,
    pub reserved: Option<u64>,
    pub committed: Option<u64>,
    pub used: Option<u64>,
}

#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadCountMetricValue {
    pub time: u128,
    pub new_count: u32,
    pub runnable_count: u32,
    pub waiting_count: u32,
    pub timed_waiting_count: u32,
    pub blocked_count: u32,
}

#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Serialize)]
pub struct JvmProcesses {
    pub processes: Vec<JvmProcessRef>,
}

#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Serialize)]
pub struct JvmProcessRef {
    pub id: String,
    pub name: String,
    pub path: Option<String>,
}
