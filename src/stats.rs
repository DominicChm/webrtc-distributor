use std::{
    sync::{Arc, Weak},
    time::Duration,
};

use anyhow::{Error, Result};
use serde::Serialize;
use sysinfo::{
    CpuExt, CpuRefreshKind, Pid, PidExt, ProcessExt, ProcessRefreshKind, RefreshKind, System,
    SystemExt,
};
use tokio::{
    sync::RwLock,
    task::JoinHandle,
    time::{self, Sleep},
};

const POLL_DELAY: u64 = 1000;

#[derive(Clone, Debug, Serialize, Default)]
pub struct SystemStatus {
    mem_total: u64,
    mem_used: u64,
    proc_mem: u64,
    cpu_num: usize,
    cpu_used: f32,
    proc_cpu: f32,
    uptime: u64,
    proc_id: u32,
}

pub struct SystemStatusReader {
    sys: Arc<RwLock<System>>,
    stats: Arc<RwLock<SystemStatus>>,
}
/**
 * Generates, tracks, and makes accessible statistics.
 */
impl SystemStatusReader {
    pub fn new() -> SystemStatusReader {
        let sys = Arc::new(RwLock::new(System::new()));
        let stats = Arc::new(RwLock::new(SystemStatus::default()));
        let task = SystemStatusReader::updater_task(Arc::downgrade(&sys), Arc::downgrade(&stats));

        SystemStatusReader { sys, stats }
    }

    /**
     * Spawns a task responsible for updating the system's info
     * on a regular basis.
     */
    fn updater_task(sys: Weak<RwLock<System>>, stats: Weak<RwLock<SystemStatus>>) {
        let specifics = RefreshKind::new()
            .with_networks()
            .with_cpu(CpuRefreshKind::new().with_cpu_usage())
            .with_memory();

        let pid = Pid::from(std::process::id() as usize);

        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_millis(POLL_DELAY));
            loop {
                if SystemStatusReader::update_stats(&sys, &stats, specifics.clone(), pid.clone())
                    .await
                    .is_err()
                {
                    break;
                }
                interval.tick().await;
            }
        });
    }

    async fn update_stats(
        sys: &Weak<RwLock<System>>,
        stats: &Weak<RwLock<SystemStatus>>,
        specifics: RefreshKind,
        pid: Pid,
    ) -> Result<()> {
        let sys_arc = sys
            .upgrade()
            .ok_or(Error::msg("Couldn't upgrade system weak"))?;
        let mut s = sys_arc.write().await;

        s.refresh_specifics(specifics);
        s.refresh_process(pid);

        let d_proc = s
            .processes()
            .get(&pid)
            .expect("Couldn't get this process. How?");

        let a = SystemStatus {
            cpu_num: s.cpus().len(),
            cpu_used: s.global_cpu_info().cpu_usage(),

            mem_total: s.total_memory(),
            mem_used: s.used_memory(),

            proc_cpu: d_proc.cpu_usage(),
            proc_id: d_proc.pid().as_u32(),
            proc_mem: d_proc.memory(),

            uptime: s.uptime(),
        };

        drop(s);

        let stats_arc = stats
            .upgrade()
            .ok_or(Error::msg("Couldn't upgrade stats weak"))?;
        //println!("{:?}", a);
        *stats_arc.write().await = a;

        Ok(())
    }

    pub async fn stats(&self) -> SystemStatus {
        self.stats.read().await.clone()
    }
}
