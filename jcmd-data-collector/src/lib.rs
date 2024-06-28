#![cfg_attr(
all(not(debug_assertions), target_os = "windows"),
windows_subsystem = "windows"
)]

use std::collections::HashMap;
use std::process::Command;
use std::string::ToString;
use std::sync::Mutex;
use std::time::SystemTime;
pub use jcmd_parser::{ApplicationThread, ClassMemoryMetricValue, GenericMemoryMetric, HeapMemoryMetricValue, JvmHeapInfo, JvmProcesses, JvmThread, MetaspaceMemoryMetricValue, parse_heap_info, parse_jvm_processes, parse_native_memory, parse_threads, parse_vm_information, ThreadCountMetricValue, ThreadMemoryMetricValue, TotalMemoryMetricValue, VmInformation};

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

pub fn reset() {
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
        let jcmd: String = "jcmd".to_string();
        let home_dir = std::env::var("HOME").unwrap_or("".to_string());
        let jcmd_ui_config = format!("{}/.config/jcmd-ui/config", home_dir);
        let jcmd_ui_config_path = std::path::Path::new(&jcmd_ui_config);
        let p = match std::env::var("JAVA_HOME") {
            Ok(p) => format!("{}/bin/jcmd", p),
            Err(_) => {
                if jcmd_ui_config_path.exists() {
                    if let Ok(content) = std::fs::read_to_string(jcmd_ui_config_path) {
                        let lines: Vec<&str> = content.split('\n').collect();
                        let java_home_paths: Vec<&str> = lines
                            .into_iter()
                            .filter(|s| s.starts_with("java.home="))
                            .collect();
                        if !java_home_paths.is_empty() {
                            let jcmd_dir = java_home_paths.first().unwrap().to_string().clone()
                                [10..]
                                .to_string();
                            format!("{}/bin/jcmd", jcmd_dir)
                        } else {
                            jcmd
                        }
                    } else {
                        jcmd
                    }
                } else {
                    jcmd
                }
            }
        };
        *cmd = p.clone();
        path = p
    }
    Command::new(path)
}

pub fn get_jvm_processes() -> Result<JvmProcesses, String> {
    match jcmd().output() {
        Ok(o) => {
            if o.status.success() {
                let output = String::from_utf8_lossy(o.stdout.as_slice()).to_string();
                return parse_jvm_processes(output);
            }
            Err("Data couldn't be read successfully".to_string())
        }
        Err(e) => Err(e.to_string()),
    }
}

pub fn get_vm_information(pid: &str) -> Result<VmInformation, String> {
    match jcmd().arg(pid).arg("VM.info").output() {
        Ok(o) => {
            if o.status.success() {
                let output = String::from_utf8_lossy(o.stdout.as_slice()).to_string();
                return parse_vm_information(output);
            }
            Err("Data couldn't be read successfully".to_string())
        }
        Err(e) => Err(e.to_string()),
    }
}

pub fn get_jvm_metrics(pid: &str) -> Result<JvmMetrics, String> {
    let time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis();

    let heap_info: JvmHeapInfo;

    match jcmd().arg(pid).arg("GC.heap_info").output() {
        Ok(o) => {
            let output = String::from_utf8_lossy(o.stdout.as_slice()).to_string();
            heap_info = parse_heap_info(output)?;
            let class_memory_metric = ClassMemoryMetricValue {
                time,
                reserved: heap_info.class_space_reserved,
                committed: heap_info.class_space_committed,
                class_count: 0,
                used: heap_info.class_space_size,
            };
            CACHE
                .lock()
                .unwrap()
                .class_metrics
                .values
                .push(class_memory_metric);

            let metaspace_memory_metric = MetaspaceMemoryMetricValue {
                time,
                reserved: heap_info.metaspace_reserved,
                committed: heap_info.metaspace_committed,
                used: heap_info.metaspace_size,
            };
            CACHE
                .lock()
                .unwrap()
                .metaspace_metrics
                .values
                .push(metaspace_memory_metric);
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

                let native_memory = parse_native_memory(output, time)?;
                if let Some(total_memory_metric) = native_memory.total_memory_metric {
                    CACHE
                        .lock()
                        .unwrap()
                        .total_memory
                        .values
                        .push(total_memory_metric)
                }

                if let Some(thread_memory_metric) = native_memory.thread_memory_metric {
                    CACHE
                        .lock()
                        .unwrap()
                        .thread_metrics
                        .values
                        .push(thread_memory_metric);
                }

                if let Some(mut heap_memory_metric) = native_memory.heap_memory_metric {
                    heap_memory_metric.used = heap_info.heap_size;
                    CACHE
                        .lock()
                        .unwrap()
                        .heap_metrics
                        .values
                        .push(heap_memory_metric);
                }

                for (name, metric) in native_memory.other_metrics {
                    let mut c = CACHE.lock().unwrap();
                    if !c.other_metrics.contains_key(&name) {
                        c.other_metrics.insert(
                            name.clone(),
                            GenericMemoryMetric {
                                name,
                                values: vec![metric],
                            },
                        );
                    } else {
                        c.other_metrics.get_mut(&name).unwrap().values.push(metric);
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
pub fn get_threads(pid: &str) -> Result<Threads, String> {
    let time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    match jcmd().arg(pid).arg("Thread.print").arg("-e").output() {
        Ok(o) => {
            let output = String::from_utf8_lossy(o.stdout.as_slice()).to_string();
            let threads = parse_threads(output, time)?;
            let application_threads = threads.application_threads.into_iter().map(|mut t| {
                t.cpu = update_thread_cache(t.cpu, t.name.clone(), false);
                t
            }).collect();

            let jvm_threads = threads.jvm_threads.into_iter().map(|mut t| {
                t.cpu = update_thread_cache(t.cpu, t.name.clone(), true);
                t
            }).collect();

            let mut c = CACHE.lock().unwrap();
            c.thread_count_metrics_application
                .values
                .push(threads.thread_count_application);

            c.thread_count_metrics_jvm.values.push(threads.thread_count_jvm);

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
pub struct Threads {
    application_threads: Vec<ApplicationThread>,
    jvm_threads: Vec<JvmThread>,
    thread_count_application: NamedMetric<ThreadCountMetricValue>,
    thread_count_jvm: NamedMetric<ThreadCountMetricValue>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ThreadCacheEntry {
    name: String,
    jvm_thread: bool,
    cpu: f32,
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