use std::{thread, time::Duration, collections::HashMap};
use sysinfo::{
    CpuRefreshKind,
    Disks,
    Networks,
    ProcessRefreshKind,
    ProcessesToUpdate,
    System,
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
        let mut sys = System::new_all();
        sys.refresh_all();

        let disks = Disks::new_with_refreshed_list();
        let networks = Networks::new_with_refreshed_list();
        Self { sys, disks, networks }
    }

    /// Refresh the data with 2 calls and a short pause, to normalize CPU usage
    pub fn refresh(&mut self) {
        self.do_refresh_cycle();
        thread::sleep(Duration::from_millis(500));
        self.do_refresh_cycle();
    }

    fn do_refresh_cycle(&mut self) {
        // CPU usage & frequency
        self.sys.refresh_cpu_specifics(CpuRefreshKind::everything());
        // Memory
        self.sys.refresh_memory();
        // Disks
        self.disks.refresh(false);
        // Networks
        self.networks.refresh(false);

        // Processes
        let proc_kind = ProcessRefreshKind::nothing()
            .with_cpu()
            .with_memory();
        self.sys.refresh_processes_specifics(ProcessesToUpdate::All, true, proc_kind);
    }

    /// CPU usage in percentage (0-100).
    pub fn global_cpu_usage(&self) -> f32 {
        self.sys.global_cpu_usage()
    }

    /// Memory usage in bytes: used, free, total, available, swap used/total
    pub fn memory_info(&self) -> (u64, u64, u64, u64, u64, u64) {
        let used = self.sys.used_memory();
        let free = self.sys.free_memory();
        let total = self.sys.total_memory();
        let avail = self.sys.available_memory();
        let swap_used = self.sys.used_swap();
        let swap_total = self.sys.total_swap();
        (used, free, total, avail, swap_used, swap_total)
    }

    /// Gather disk usage info (mount, file system, used/available/total).
    pub fn disk_info(&self) -> Vec<(String, String, String, u64, u64, u64)> {
        // (name, fs, mount, used, available, total)
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

    /// Return total bytes received/transmitted for each interface.
    pub fn network_info(&self) -> Vec<(String, u64, u64)> {
        // (interface name, total rx, total tx)
        let mut out = Vec::new();
        for (iface, data) in self.networks.iter() {
            out.push((iface.clone(), data.total_received(), data.total_transmitted()));
        }
        out
    }

    /// Combine all processes by name. Each unique process name has sums of CPU/mem usage.
    /// Returns a vector of CombinedProcess, one entry per name.
    pub fn combined_process_list(&self) -> Vec<CombinedProcess> {
        let mut map: HashMap<String, CombinedProcess> = HashMap::new();

        for proc_ in self.sys.processes().values() {
            let pid_val = proc_.pid().as_u32();
            let proc_name = proc_.name().to_string_lossy().into_owned();

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

    /// Return top `count` combined processes by CPU
    pub fn top_by_cpu(&self, count: usize) -> Vec<CombinedProcess> {
        let mut procs = self.combined_process_list();
        // Sort descending by CPU usage
        procs.sort_by(|a, b| b.cpu_usage.partial_cmp(&a.cpu_usage).unwrap());
        procs.truncate(count);
        procs
    }

    /// Return top `count` combined processes by memory
    pub fn top_by_memory(&self, count: usize) -> Vec<CombinedProcess> {
        let mut procs = self.combined_process_list();
        // Sort descending by memory usage
        procs.sort_by_key(|p| std::cmp::Reverse(p.memory_usage));
        procs.truncate(count);
        procs
    }
}

fn main() {
    let mut monitor = SystemMonitor::new();
    monitor.refresh();

    // CPU Usage
    let cpu_usage = monitor.global_cpu_usage();
    // Memory breakdown
    let (used_mem, free_mem, total_mem, avail_mem, swap_used, swap_total) = monitor.memory_info();
    // Disks
    let disk_details = monitor.disk_info();
    // Network
    let net_details = monitor.network_info();

    // Combined Processes (top by CPU)
    let top_cpu = monitor.top_by_cpu(5);
    // Combined Processes (top by Memory)
    let top_mem = monitor.top_by_memory(5);

    println!("╔══════════════════════════════════════════╗");
    println!("║        RUST SYSTEM DASHBOARD v1.0        ║");
    println!("╚══════════════════════════════════════════╝\n");

    println!("== CPU ==");
    println!("Global CPU Usage: {:.2}%", cpu_usage);
    println!();

    println!("== MEMORY ==");
    let used_mb = used_mem / 1_048_576;
    let free_mb = free_mem / 1_048_576;
    let total_mb = total_mem / 1_048_576;
    let avail_mb = avail_mem / 1_048_576;
    let swap_used_mb = swap_used / 1_048_576;
    let swap_total_mb = swap_total / 1_048_576;

    println!("Used:        {} MB", used_mb);
    println!("Free:        {} MB", free_mb);
    println!("Available:   {} MB", avail_mb);
    println!("Total:       {} MB", total_mb);
    println!("Swap Used:   {} MB", swap_used_mb);
    println!("Swap Total:  {} MB", swap_total_mb);
    println!();

    // Disks
    println!("== DISKS ==");
    for (name, fs, mount, used, avail, total) in disk_details {
        let used_gb = used as f64 / 1_073_741_824.0;
        let avail_gb = avail as f64 / 1_073_741_824.0;
        let total_gb = total as f64 / 1_073_741_824.0;
        println!(
            "{:>12} ({:>5}) at {:<15} => used {:.2} GiB / total {:.2} GiB (free {:.2} GiB)",
            name, fs, mount, used_gb, total_gb, avail_gb
        );
    }
    println!();

    // Network
    println!("== NETWORK (total bytes so far) ==");
    for (iface, rx, tx) in net_details {
        println!("Interface: {:<10}  RX: {:>12} bytes, TX: {:>12} bytes", iface, rx, tx);
    }
    println!();

    // Top Combined Processes by CPU
    println!("== TOP 5 PROCS (by CPU, COMBINED) ==");
    println!(" CPU%   MEM (bytes)  NAME                     (PIDs...)");
    for proc_group in top_cpu {
        println!(
            "{:>5.1}%  {:>12}   {:<24}  {:?}",
            proc_group.cpu_usage,
            proc_group.memory_usage,
            proc_group.name,
            proc_group.pids
        );
    }
    println!();

    // Top Combined Processes by Memory
    println!("== TOP 5 PROCS (by MEMORY, COMBINED) ==");
    println!(" MEM (bytes)   CPU%   NAME                    (PIDs...)");
    for proc_group in top_mem {
        println!(
            "{:>12}  {:>5.1}   {:<24}  {:?}",
            proc_group.memory_usage,
            proc_group.cpu_usage,
            proc_group.name,
            proc_group.pids
        );
    }
    println!("\nDone.\n");
}
