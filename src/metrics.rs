// src/metrics.rs

//! Timing and memory measurement utilities.
//!
//! This module measures:
//! - Wall-clock duration of a single action invocation
//! - Peak memory usage (RSS) of the child process
//!
//! Memory tracking is implemented using the `sysinfo` crate and is best-effort:
//! - Windows: supported
//! - macOS: supported
//! - Linux: supported
//!
//! Notes:
//! - Memory is sampled periodically (polling).
//! - Extremely short-lived spikes may not be captured.
//! - If the platform or PID cannot be inspected, memory tracking
//!   degrades gracefully and returns `None`.

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread;
use std::time::Duration;

use sysinfo::{Pid, System};

/// Metrics collected for a single invocation.
#[derive(Debug, Clone)]
pub struct InvocationMetrics {
    /// Wall-clock duration in milliseconds.
    pub duration_ms: u128,

    /// Peak RSS memory in KB (best-effort).
    pub max_rss_kb: Option<u64>,
}

/// Tracks peak memory usage of a child process while it runs.
pub struct MemoryTracker {
    stop: Arc<AtomicBool>,
    max_kb: Arc<Mutex<u64>>,
    handle: Option<thread::JoinHandle<()>>,
}

impl MemoryTracker {
    /// Start tracking memory usage for a process.
    ///
    /// - `pid_u32`: PID of the child process
    /// - `sample_every`: polling interval (e.g. 20ms)
    ///
    /// Tracking is best-effort and will silently stop if the
    /// process exits or cannot be inspected.
    pub fn start(pid_u32: u32, sample_every: Duration) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let max_kb = Arc::new(Mutex::new(0u64));

        let stop_clone = Arc::clone(&stop);
        let max_clone = Arc::clone(&max_kb);

        let handle = thread::spawn(move || {
            let pid = Pid::from_u32(pid_u32);
            let mut system = System::new();

            loop {
                if stop_clone.load(Ordering::Relaxed) {
                    break;
                }

                system.refresh_process(pid);

                if let Some(process) = system.process(pid) {
                    // RSS memory in KB (sysinfo contract)
                    let mem_kb = process.memory();

                    if let Ok(mut guard) = max_clone.lock() {
                        if mem_kb > *guard {
                            *guard = mem_kb;
                        }
                    }
                } else {
                    // Process exited or is no longer visible
                    break;
                }

                thread::sleep(sample_every);
            }
        });

        Self {
            stop,
            max_kb,
            handle: Some(handle),
        }
    }

    /// Stop tracking and return the peak RSS in KB.
    ///
    /// Returns `None` if no samples were collected.
    pub fn stop_and_take(mut self) -> Option<u64> {
        self.stop.store(true, Ordering::Relaxed);

        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }

        let max = match self.max_kb.lock() {
            Ok(guard) => *guard,
            Err(_) => 0,
        };

        if max > 0 {
            Some(max)
        } else {
            None
        }
    }
}
