use std::collections::HashMap;
use sysinfo::{CpuRefreshKind, Disks, Networks, ProcessRefreshKind, System};

/// System monitor that wraps sysinfo to provide system statistics.
///
/// This struct provides methods to query CPU, memory, disk, network, and process information.
/// It should be refreshed periodically to get up-to-date statistics.
pub struct SystemMonitor {
    pub sys: System,
    pub disks: Disks,
    pub networks: Networks,
    pub last_disk_refresh: std::time::Instant,
    pub cached_processes: Vec<CombinedProcess>,
}

/// A process that may have multiple instances (PIDs) combined together.
///
/// CPU and memory usage are summed across all instances of the process.
#[derive(Debug, Clone)]
pub struct CombinedProcess {
    /// Process name
    pub name: String,
    /// Total CPU usage percentage across all instances
    pub cpu_usage: f32,
    /// Total memory usage in bytes across all instances
    pub memory_usage: u64,
    /// List of all PIDs for this process name
    pub pids: Vec<u32>,
}

impl SystemMonitor {
    /// Create a new SystemMonitor and perform initial refresh.
    ///
    /// # Example
    /// ```
    /// use rust_dashboard_lib::system::SystemMonitor;
    /// let monitor = SystemMonitor::new();
    /// ```
    pub fn new() -> Self {
        log::debug!("SystemMonitor::new() -> creating System with new_all()");
        let mut sys = System::new_all();
        sys.refresh_all();

        let disks = Disks::new_with_refreshed_list();
        let networks = Networks::new_with_refreshed_list();
        Self {
            sys,
            disks,
            networks,
            last_disk_refresh: std::time::Instant::now(),
            cached_processes: Vec::new(),
        }
    }

    /// Refresh all system data.
    ///
    /// This should be called periodically to get up-to-date statistics.
    /// Disk information is only refreshed every 60 seconds to reduce overhead.
    ///
    /// # Example
    /// ```
    /// use rust_dashboard_lib::system::SystemMonitor;
    /// let mut monitor = SystemMonitor::new();
    /// monitor.refresh();
    /// ```
    pub fn refresh(&mut self) {
        log::debug!("SystemMonitor: refresh() start");
        self.do_refresh_cycle();
        log::debug!("SystemMonitor: refresh() complete");
    }

    fn do_refresh_cycle(&mut self) {
        self.sys.refresh_cpu_specifics(CpuRefreshKind::everything());
        self.sys.refresh_memory();
        // require bool arg: false => do not remove unlisted
        if self.last_disk_refresh.elapsed() >= std::time::Duration::from_secs(60) {
            self.disks.refresh(false);
            self.last_disk_refresh = std::time::Instant::now();
        }
        self.networks.refresh(false);

        self.sys.refresh_processes_specifics(
            sysinfo::ProcessesToUpdate::All,
            false,
            ProcessRefreshKind::everything(),
        );

        // Update cached process list
        self.cached_processes = self.compute_combined_process_list();
    }

    /// Get global CPU usage as a percentage (0-100).
    ///
    /// For multi-core systems, this can exceed 100%.
    ///
    /// # Returns
    /// CPU usage percentage as f32
    ///
    /// # Example
    /// ```
    /// use rust_dashboard_lib::system::SystemMonitor;
    /// let monitor = SystemMonitor::new();
    /// let cpu_usage = monitor.global_cpu_usage();
    /// println!("CPU Usage: {:.2}%", cpu_usage);
    /// ```
    pub fn global_cpu_usage(&self) -> f32 {
        self.sys.global_cpu_usage()
    }

    /// Get memory information.
    ///
    /// # Returns
    /// A tuple containing (used_mem, free_mem, total_mem, avail_mem, swap_used, swap_total)
    /// All values are in bytes.
    ///
    /// # Example
    /// ```
    /// use rust_dashboard_lib::system::SystemMonitor;
    /// let monitor = SystemMonitor::new();
    /// let (used, free, total, avail, swap_used, swap_total) = monitor.memory_info();
    /// ```
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

    /// Get disk information for all mounted disks.
    ///
    /// # Note
    /// Disk I/O statistics (read/write speeds) are not available in sysinfo 0.33.1.
    /// The Disk API only provides space information, not I/O metrics.
    ///
    /// # Returns
    /// A vector of tuples containing (disk_name, file_system, mount_point, used, available, total)
    /// All size values are in bytes.
    ///
    /// # Example
    /// ```
    /// use rust_dashboard_lib::system::SystemMonitor;
    /// let monitor = SystemMonitor::new();
    /// let disks = monitor.disk_info();
    /// for (name, fs, mount, used, avail, total) in disks {
    ///     println!("{} mounted at {}: {:.2}% used", name, mount, (used as f64 / total as f64) * 100.0);
    /// }
    /// ```
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

    /// Get network interface information.
    ///
    /// Only returns interfaces that have transmitted or received data.
    ///
    /// # Returns
    /// A vector of tuples containing (iface_name, total_rx_bytes, total_tx_bytes)
    ///
    /// # Example
    /// ```
    /// use rust_dashboard_lib::system::SystemMonitor;
    /// let monitor = SystemMonitor::new();
    /// let networks = monitor.network_info();
    /// for (iface, rx, tx) in networks {
    ///     println!("{}: RX={}, TX={}", iface, rx, tx);
    /// }
    /// ```
    pub fn network_info(&self) -> Vec<(String, u64, u64)> {
        let mut out = Vec::new();
        for (iface, data) in self.networks.iter() {
            let usage = data.total_received() + data.total_transmitted();
            if usage > 0 {
                out.push((
                    iface.clone(),
                    data.total_received(),
                    data.total_transmitted(),
                ));
            }
        }
        out
    }

    /// Get a list of all processes, combined by name.
    ///
    /// Processes with the same name are combined, with CPU and memory usage summed.
    /// This is useful for displaying processes that may have multiple instances.
    ///
    /// # Returns
    /// A vector of CombinedProcess structs, one per unique process name.
    ///
    /// # Example
    /// ```
    /// use rust_dashboard_lib::system::SystemMonitor;
    /// let monitor = SystemMonitor::new();
    /// let processes = monitor.combined_process_list();
    /// for proc in processes {
    ///     println!("{}: CPU={:.2}%, Memory={} bytes", proc.name, proc.cpu_usage, proc.memory_usage);
    /// }
    /// ```
    pub fn combined_process_list(&self) -> Vec<CombinedProcess> {
        self.cached_processes.clone()
    }

    /// Internal method to compute the combined process list.
    fn compute_combined_process_list(&self) -> Vec<CombinedProcess> {
        let mut map: HashMap<String, CombinedProcess> = HashMap::new();

        for proc_ in self.sys.processes().values() {
            let pid_val = proc_.pid().as_u32();
            let proc_name = proc_.name().to_string_lossy().into_owned();

            let entry = map
                .entry(proc_name.clone())
                .or_insert_with(|| CombinedProcess {
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

    /// Get CPU and memory usage for a specific process by PID.
    ///
    /// # Arguments
    /// * `pid_val` - The process ID to query
    ///
    /// # Returns
    /// Some((cpu_usage, memory_usage)) if the process is found, None otherwise.
    /// CPU usage is a percentage (0-100), memory usage is in bytes.
    ///
    /// # Example
    /// ```
    /// use rust_dashboard_lib::system::SystemMonitor;
    /// let monitor = SystemMonitor::new();
    /// let current_pid = std::process::id();
    /// if let Some((cpu, mem)) = monitor.usage_for_pid(current_pid) {
    ///     println!("Current process: CPU={:.2}%, Memory={} bytes", cpu, mem);
    /// }
    /// ```
    pub fn usage_for_pid(&self, pid_val: u32) -> Option<(f32, u64)> {
        self.sys
            .processes()
            .get(&sysinfo::Pid::from_u32(pid_val))
            .map(|p| (p.cpu_usage(), p.memory()))
    }

    /// Get detailed information about a specific process by PID.
    ///
    /// # Arguments
    /// * `pid_val` - The process ID to query
    ///
    /// # Returns
    /// Some(ProcessDetails) if the process is found, None otherwise.
    ///
    /// # Example
    /// ```
    /// use rust_dashboard_lib::system::SystemMonitor;
    /// let monitor = SystemMonitor::new();
    /// let current_pid = std::process::id();
    /// if let Some(details) = monitor.process_details(current_pid) {
    ///     println!("Process: {}, Command: {}", details.name, details.command);
    /// }
    /// ```
    pub fn process_details(&self, pid_val: u32) -> Option<ProcessDetails> {
        self.sys
            .processes()
            .get(&sysinfo::Pid::from_u32(pid_val))
            .map(|p| {
                let cmd_str = p
                    .cmd()
                    .iter()
                    .map(|s| s.to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
                    .join(" ");
                ProcessDetails {
                    command: cmd_str,
                    start_time: p.start_time(),
                    parent: p.parent().map(|pid| pid.as_u32()),
                }
            })
    }

    /// Kill a process by PID (sends SIGKILL).
    ///
    /// # Arguments
    /// * `pid_val` - The process ID to kill
    ///
    /// # Returns
    /// Ok(()) if successful, Err with error message otherwise.
    ///
    /// # Warning
    /// This will forcefully terminate the process. Use with caution.
    ///
    /// # Example
    /// ```
    /// use rust_dashboard_lib::system::SystemMonitor;
    /// let mut monitor = SystemMonitor::new();
    /// // monitor.kill_process(12345)?;
    /// ```
    pub fn kill_process(&mut self, pid_val: u32) -> Result<(), String> {
        if let Some(process) = self.sys.processes().get(&sysinfo::Pid::from_u32(pid_val)) {
            if process.kill() {
                Ok(())
            } else {
                Err("Failed to kill process".to_string())
            }
        } else {
            Err("Process not found".to_string())
        }
    }
}

/// Detailed information about a process.
#[derive(Debug, Clone)]
pub struct ProcessDetails {
    pub command: String,
    pub start_time: u64,
    pub parent: Option<u32>,
}
