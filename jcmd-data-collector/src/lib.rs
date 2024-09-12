#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use anyhow::{anyhow, Result};
use jcmd_parser::NativeMemoryMetricValue;
pub use jcmd_parser::{
    parse_heap_info, parse_jvm_processes, parse_native_memory, parse_threads, parse_vm_information,
    ApplicationThread, JvmHeapInfo, JvmProcesses, JvmThread, ThreadCountMetricValue, VmInformation,
};
use once_cell::sync::Lazy;
use serde::Serialize;
use std::collections::HashMap;
use std::process::Command;
use std::string::ToString;
use std::sync::Mutex;
use std::time::SystemTime;

static CACHE: Lazy<Mutex<MetricsCache>> = Lazy::new(|| {
    Mutex::new(MetricsCache {
        memory_metrics: HashMap::new(),
        thread_count_metrics_application: NamedMetric::new("ThreadCountApplication".to_string()),
        thread_count_metrics_jvm: NamedMetric::new("ThreadCountJvm".to_string()),
    })
});

static THREAD_CACHE: Lazy<Mutex<HashMap<String, ThreadCacheEntry>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

static JCMD: Lazy<Mutex<String>> = Lazy::new(|| Mutex::new("".to_string()));

pub trait DataCollector: Sync + Send {
    fn get_vm_information(&self) -> Result<VmInformation>;
    fn get_jvm_metrics(&self) -> Result<JvmMetrics>;
    fn get_threads(&self) -> Result<Threads>;
}

pub struct DefaultDataCollector {
    pid: Option<String>,
}

impl DefaultDataCollector {
    pub fn new() -> DefaultDataCollector {
        DefaultDataCollector { pid: None }
    }

    pub fn get_pid(&self) -> Option<String> {
        self.pid.clone()
    }

    pub fn set_pid(&mut self, pid: String) {
        self.pid = Some(pid);
    }

    pub fn get_jvm_processes(&self) -> Result<JvmProcesses> {
        match jcmd().output() {
            Ok(o) => {
                if o.status.success() {
                    let output = String::from_utf8_lossy(o.stdout.as_slice()).to_string();
                    return parse_jvm_processes(output);
                }
                Err(anyhow!("Data couldn't be read successfully".to_string()))
            }
            Err(e) => Err(anyhow!(e.to_string())),
        }
    }

    pub fn reset(&self) {
        let mut c = CACHE.lock().unwrap();
        c.memory_metrics.clear();
        c.thread_count_metrics_application.values.clear();
        c.thread_count_metrics_jvm.values.clear();
    }
}

impl DataCollector for DefaultDataCollector {
    fn get_vm_information(&self) -> Result<VmInformation> {
        if self.pid.is_none() {
            return Err(anyhow!("No pid specified"));
        }
        match jcmd()
            .arg(self.pid.clone().unwrap())
            .arg("VM.info")
            .output()
        {
            Ok(o) => {
                if o.status.success() {
                    let output = String::from_utf8_lossy(o.stdout.as_slice()).to_string();
                    return parse_vm_information(output);
                }
                Err(anyhow!("Data couldn't be read successfully".to_string()))
            }
            Err(e) => Err(anyhow!(e.to_string())),
        }
    }

    fn get_jvm_metrics(&self) -> Result<JvmMetrics> {
        if self.pid.is_none() {
            return Err(anyhow!("No pid specified"));
        }

        let time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis();

        let heap_info: JvmHeapInfo;

        match jcmd()
            .arg(self.pid.clone().unwrap())
            .arg("GC.heap_info")
            .output()
        {
            Ok(o) => {
                let output = String::from_utf8_lossy(o.stdout.as_slice()).to_string();
                heap_info = parse_heap_info(output)?;
                let class_memory_metric = NativeMemoryMetricValue {
                    time,
                    reserved: heap_info.class_space_reserved,
                    committed: heap_info.class_space_committed,
                    used: heap_info.class_space_size,
                };
                add_or_update_metric("Class".to_string(), class_memory_metric);

                let metaspace_memory_metric = NativeMemoryMetricValue {
                    time,
                    reserved: heap_info.metaspace_reserved,
                    committed: heap_info.metaspace_committed,
                    used: heap_info.metaspace_size,
                };
                add_or_update_metric("Metaspace".to_string(), metaspace_memory_metric);
            }
            Err(e) => return Err(anyhow!(e.to_string())),
        };
        match jcmd()
            .arg(self.pid.clone().unwrap())
            .arg("VM.native_memory")
            .arg("scale=b")
            .output()
        {
            Ok(o) => {
                if o.status.success() {
                    let output = String::from_utf8_lossy(o.stdout.as_slice()).to_string();
                    if output.contains("IOException: No such process") {
                        return Err(anyhow!("No such process".to_string()));
                    } else if output.contains("Native memory tracking is not enabled") {
                        return Err(anyhow!(
                            "Native memory tracking not activated. Start application with java \
                    -XX:NativeMemoryTracking=summary -jar ..."
                                .to_string()
                        ));
                    }

                    let native_memory = parse_native_memory(output, time)?;
                    for (name, mut metric) in native_memory.metrics {
                        // Class metrics are taken from GC.heap_info above
                        if name == "Class" {
                            continue;
                        }
                        // Merge "used" heap size from GC.heap_info above
                        if name == "Heap" {
                            metric.used = heap_info.heap_size
                        }
                        add_or_update_metric(name.to_string(), metric);
                    }

                    let c = CACHE.lock().unwrap();
                    let jvm_metrics = JvmMetrics {
                        metrics: c
                            .memory_metrics
                            .iter()
                            .map(|m| NamedMetric {
                                name: m.0.clone(),
                                values: m.1.clone(),
                            })
                            .collect(),
                    };
                    return Ok(jvm_metrics);
                }
                Err(anyhow!("Data couldn't be read successfully".to_string()))
            }
            Err(e) => Err(anyhow!(e.to_string())),
        }
    }

    // Intro to thread dumps: https://dzone.com/articles/how-to-read-a-thread-dump
    fn get_threads(&self) -> Result<Threads> {
        if self.pid.is_none() {
            return Err(anyhow!("No pid specified"));
        }

        let time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        match jcmd()
            .arg(self.pid.clone().unwrap())
            .arg("Thread.print")
            .arg("-e")
            .output()
        {
            Ok(o) => {
                let output = String::from_utf8_lossy(o.stdout.as_slice()).to_string();
                let threads = parse_threads(output, time)?;
                let application_threads = threads
                    .application_threads
                    .into_iter()
                    .map(|mut t| {
                        t.cpu = update_thread_cache(t.cpu, t.name.clone(), false);
                        t
                    })
                    .collect();

                let jvm_threads = threads
                    .jvm_threads
                    .into_iter()
                    .map(|mut t| {
                        t.cpu = update_thread_cache(t.cpu, t.name.clone(), true);
                        t
                    })
                    .collect();

                let mut c = CACHE.lock().unwrap();
                c.thread_count_metrics_application
                    .values
                    .push(threads.thread_count_application);

                c.thread_count_metrics_jvm
                    .values
                    .push(threads.thread_count_jvm);

                Ok(Threads {
                    application_threads,
                    jvm_threads,
                    thread_count_application: c.thread_count_metrics_application.clone(),
                    thread_count_jvm: c.thread_count_metrics_jvm.clone(),
                })
            }
            Err(e) => Err(anyhow!(e.to_string())),
        }
    }
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

fn add_or_update_metric(name: String, metric: NativeMemoryMetricValue) {
    let mut c = CACHE.lock().unwrap();
    if !c.memory_metrics.contains_key(&name) {
        c.memory_metrics.insert(name.clone(), vec![metric]);
    } else {
        c.memory_metrics.get_mut(&name).unwrap().push(metric);
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
    memory_metrics: HashMap<String, Vec<NativeMemoryMetricValue>>,
    thread_count_metrics_application: NamedMetric<ThreadCountMetricValue>,
    thread_count_metrics_jvm: NamedMetric<ThreadCountMetricValue>,
}

#[derive(Clone, Debug, PartialOrd, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Threads {
    pub application_threads: Vec<ApplicationThread>,
    pub jvm_threads: Vec<JvmThread>,
    pub thread_count_application: NamedMetric<ThreadCountMetricValue>,
    pub thread_count_jvm: NamedMetric<ThreadCountMetricValue>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ThreadCacheEntry {
    name: String,
    jvm_thread: bool,
    cpu: f32,
}

#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JvmMetrics {
    pub metrics: Vec<NamedMetric<NativeMemoryMetricValue>>,
}

#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NamedMetric<T> {
    pub name: String,
    pub values: Vec<T>,
}

impl<T> NamedMetric<T> {
    fn new(name: String) -> NamedMetric<T> {
        NamedMetric {
            name,
            values: Vec::new(),
        }
    }
}
