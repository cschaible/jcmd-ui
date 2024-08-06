pub mod application_threads_tab;
pub mod jvm_threads_tab;
pub mod memory_tab;
pub mod process_information_tab;
pub mod processes;
pub mod tab_view;
mod table_utils;
mod threads_shared;

struct MemoryStatsDetails<T: Copy + Ord> {
    min: T,
    max: T,
    avg: T,
    median: T,
}