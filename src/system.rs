use std::collections::HashMap;
use sysinfo::{
    CpuRefreshKind, Disks, Networks,
    ProcessRefreshKind, System,
};

pub struct SystemMonitor {
    sys: System,
    disks: Disks,
    networks: Networks,
}

#[derive(Debug, Clone)]
pub struct CombinedProcess {
    pub name: String,
    pub cpu_usage: f32,
    pub memory_usage: u64,
    pub pids: Vec<u32>,
}

impl SystemMonitor {
    pub fn new() -> Self {
        println!("SystemMonitor::new() -> creating System with new_all()");
        let mut sys = System::new_all();
        sys.refresh_all();

        let disks = Disks::new_with_refreshed_list();
        let networks = Networks::new_with_refreshed_list();
        Self { sys, disks, networks }
    }

    /// Refresh the data with two calls and a short pause, to normalize CPU usage
    pub fn refresh(&mut self) {
        println!("SystemMonitor: refresh() start");
        self.do_refresh_cycle();
        println!("SystemMonitor: refresh() complete");
    }

    fn do_refresh_cycle(&mut self) {
        self.sys.refresh_cpu_specifics(CpuRefreshKind::everything());
        self.sys.refresh_memory();
        self.disks.refresh();
        self.networks.refresh();

        let proc_kind = ProcessRefreshKind::new()
            .with_cpu()
            .with_memory();
        self.sys.refresh_processes_specifics(proc_kind);
    }

    /// CPU usage in percentage (0-100).
    pub fn global_cpu_usage(&self) -> f32 {
        self.sys.global_cpu_info().cpu_usage()
    }

    /// (used_mem, free_mem, total_mem, avail_mem, swap_used, swap_total)
    pub fn memory_info(&self) -> (u64, u64, u64, u64, u64, u64) {
        (
            self.sys.used_memory(),
            self.sys.free_memory(),
            self.sys.total_memory(),
            self.sys.available_memory(),
            self.sys.used_swap(),
            self.sys.total_swap(),
        )
    }

    /// Return (disk_name, file_system, mount_point, used, available, total)
    pub fn disk_info(&self) -> Vec<(String, String, String, u64, u64, u64)> {
        let mut info = Vec::new();
        for disk in self.disks.list() {
            let total = disk.total_space();
            let avail = disk.available_space();
            let used = total.saturating_sub(avail);
            info.push((
                disk.name().to_string_lossy().into_owned(),
                disk.file_system().to_string_lossy().into_owned(),
                disk.mount_point().to_string_lossy().into_owned(),
                used,
                avail,
                total,
            ));
        }
        info
    }

    /// Return (iface_name, total_rx, total_tx)
    pub fn network_info(&self) -> Vec<(String, u64, u64)> {
        let mut out = Vec::new();
        for (iface, data) in self.networks.iter() {
            let usage = data.total_received() + data.total_transmitted();
            if usage > 0 {
                out.push((iface.clone(), data.total_received(), data.total_transmitted()));
            }
        }
        out
    }

    /// Combine all processes by name. Each unique process name has sums of CPU/mem usage.
    pub fn combined_process_list(&self) -> Vec<CombinedProcess> {
        let mut map: HashMap<String, CombinedProcess> = HashMap::new();

        for proc_ in self.sys.processes().values() {
            let pid_val = proc_.pid().as_u32();
            let proc_name = proc_.name().to_string();

            let entry = map.entry(proc_name.clone()).or_insert_with(|| CombinedProcess {
                name: proc_name,
                cpu_usage: 0.0,
                memory_usage: 0,
                pids: Vec::new(),
            });

            entry.cpu_usage += proc_.cpu_usage();
            entry.memory_usage += proc_.memory();
            entry.pids.push(pid_val);
        }
        map.into_values().collect()
    }
}