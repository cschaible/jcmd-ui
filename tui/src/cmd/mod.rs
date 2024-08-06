use jcmd_data_collector::{JvmMetrics, JvmProcesses, Threads, VmInformation};
use anyhow::Result;

pub fn get_jvm_processes() -> Result<JvmProcesses> {
    jcmd_data_collector::get_jvm_processes()
}

pub fn get_jvm_metrics(pid: String) -> Result<JvmMetrics> {
    jcmd_data_collector::get_jvm_metrics(&pid)
}

pub fn get_thread_metrics(pid: String) -> Result<Threads> {
    jcmd_data_collector::get_threads(&pid)
}

pub fn get_vm_information(pid: String) -> Result<VmInformation> {
    jcmd_data_collector::get_vm_information(&pid)
}
