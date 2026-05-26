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
    pub last_network_refresh: std::time::Instant,
    /// Duration captured at the most recent network refresh — the actual
    /// interval between the previous and current refresh. Used as the
    /// denominator for rate calculations so the answer doesn't depend on
    /// when `network_info_with_rates` is queried relative to the refresh.
    pub last_network_interval: std::time::Duration,
    pub last_network_snapshot: HashMap<String, (u64, u64)>,
    pub cached_processes: Vec<CombinedProcess>,
}

/// A process that may have multiple instances (PIDs) combined together.
///
/// CPU and memory usage are summed across all instances of the process.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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

impl Default for SystemMonitor {
    fn default() -> Self {
        Self::new()
    }
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
        let mut monitor = Self {
            sys,
            disks,
            networks,
            last_disk_refresh: std::time::Instant::now(),
            last_network_refresh: std::time::Instant::now(),
            // Seed with the refresh threshold so the first computed rate
            // (before any refresh has happened) uses a reasonable denominator
            // rather than zero. Snapshot is empty on first call regardless,
            // so the returned rate is 0.0 — this seed only matters if
            // someone queries between construction and the first refresh.
            last_network_interval: std::time::Duration::from_secs(5),
            last_network_snapshot: HashMap::new(),
            cached_processes: Vec::new(),
        };
        // Populate the cache on initialization
        monitor.cached_processes = monitor.compute_combined_process_list();
        monitor
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
        // Only refresh networks every 5 seconds to reduce overhead
        if self.last_network_refresh.elapsed() >= std::time::Duration::from_secs(5) {
            let now = std::time::Instant::now();
            // Capture the actual interval before resetting the timestamp,
            // so rate calculations divide by the real elapsed time between
            // refreshes — not by "time since last refresh" which is ~0 right
            // after a refresh and produces inflated rates.
            self.last_network_interval = now.duration_since(self.last_network_refresh);
            // Capture pre-refresh totals for rate calculation
            for (iface, data) in self.networks.iter() {
                self.last_network_snapshot.insert(
                    iface.clone(),
                    (data.total_received(), data.total_transmitted()),
                );
            }
            self.networks.refresh(false);
            self.last_network_refresh = now;
        }

        self.sys.refresh_processes_specifics(
            sysinfo::ProcessesToUpdate::All,
            false,
            ProcessRefreshKind::nothing().with_cpu().with_memory(),
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

    /// Get network interface information with throughput rates.
    ///
    /// Returns (iface_name, total_rx, total_tx, rx_rate_bytes_per_sec, tx_rate_bytes_per_sec).
    /// Rates are calculated from the delta since the last network snapshot.
    pub fn network_info_with_rates(&self) -> Vec<(String, u64, u64, f64, f64)> {
        // Use the interval captured at refresh time, not the elapsed time
        // since refresh. The latter approaches zero immediately after a
        // refresh and produced inflated rate spikes (delta / tiny).
        let dt = self.last_network_interval.as_secs_f64().max(0.1);
        let mut out = Vec::new();
        for (iface, data) in self.networks.iter() {
            let total_rx = data.total_received();
            let total_tx = data.total_transmitted();
            let usage = total_rx + total_tx;
            if usage > 0 {
                let (rx_rate, tx_rate) =
                    if let Some(&(prev_rx, prev_tx)) = self.last_network_snapshot.get(iface) {
                        let rx_delta = total_rx.saturating_sub(prev_rx);
                        let tx_delta = total_tx.saturating_sub(prev_tx);
                        (rx_delta as f64 / dt, tx_delta as f64 / dt)
                    } else {
                        (0.0, 0.0)
                    };
                out.push((iface.clone(), total_rx, total_tx, rx_rate, tx_rate));
            }
        }
        out
    }

    /// Get system uptime in seconds.
    pub fn system_uptime(&self) -> u64 {
        System::uptime()
    }

    /// Get system load averages (1, 5, 15 minute).
    pub fn load_average(&self) -> (f64, f64, f64) {
        let load = System::load_average();
        (load.one, load.five, load.fifteen)
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
    pub fn combined_process_list(&self) -> &[CombinedProcess] {
        &self.cached_processes
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
    ///     println!("Command: {}", details.command);
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
        if pid_val <= 1 {
            return Err("Cannot terminate system processes (PID 0 or 1)".to_string());
        }
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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProcessDetails {
    pub command: String,
    pub start_time: u64,
    pub parent: Option<u32>,
}
