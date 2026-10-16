//! Per-account CPU limit: the account's Roblox process goes into a Windows job object with a hard cap on its CPU rate.
//! Windows enforces it (Roblox ignores the client FPS flag since its September 2025 flag allowlist, it can't ignore
//! this), it holds per window, and it can be changed or lifted while the game runs.
//!
//! The app keeps the job's handle while the Roblox window lives: a job is only reachable through a handle (its name
//! goes with the last one). If the app restarts, the cap stays on that window and can only be lowered (a new job
//! nests in the old one), until the account opens again.

use std::collections::HashMap;
use std::sync::Mutex;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JobObjectCpuRateControlInformation, SetInformationJobObject, JOBOBJECT_CPU_RATE_CONTROL_INFORMATION,
    JOBOBJECT_CPU_RATE_CONTROL_INFORMATION_0, JOB_OBJECT_CPU_RATE_CONTROL_ENABLE, JOB_OBJECT_CPU_RATE_CONTROL_HARD_CAP,
};
use windows::Win32::System::Threading::{OpenProcess, PROCESS_SET_QUOTA, PROCESS_TERMINATE};

/// Percent of the whole PC's CPU, as Task Manager counts it
pub const MIN: u32 = 1;
pub const MAX: u32 = 99;

/// pid -> its job's handle (as an integer, handles aren't Send)
static JOBS: Mutex<Option<HashMap<u32, isize>>> = Mutex::new(None);

/// Caps the process at `percent` of the PC's CPU, or lifts the cap with None
pub fn set(pid: u32, percent: Option<u32>) -> Result<(), String> {
    let mut jobs = JOBS.lock().unwrap();
    let jobs = jobs.get_or_insert_with(HashMap::new);
    let job = match jobs.get(&pid) {
        Some(&h) => HANDLE(h as *mut _),
        // never capped by this app: nothing to lift
        None if percent.is_none() => return Ok(()),
        None => {
            let job = unsafe { create(pid)? };
            jobs.insert(pid, job.0 as isize);
            job
        }
    };
    let info = JOBOBJECT_CPU_RATE_CONTROL_INFORMATION {
        ControlFlags: JOB_OBJECT_CPU_RATE_CONTROL_ENABLE | JOB_OBJECT_CPU_RATE_CONTROL_HARD_CAP,
        // in hundredths of a percent; lifting is a cap at 100% (turning the control off with flags 0 kept the old cap)
        Anonymous: JOBOBJECT_CPU_RATE_CONTROL_INFORMATION_0 { CpuRate: percent.map(|p| p.clamp(MIN, MAX)).unwrap_or(100) * 100 },
    };
    unsafe { SetInformationJobObject(job, JobObjectCpuRateControlInformation, &info as *const _ as *const _, std::mem::size_of::<JOBOBJECT_CPU_RATE_CONTROL_INFORMATION>() as u32) }
        .map_err(|e| format!("Can't limit the CPU: {e}"))
}

/// Lets go of the jobs of processes that are gone (their jobs go with them)
pub fn forget_except(alive: &[u32]) {
    if let Some(jobs) = JOBS.lock().unwrap().as_mut() {
        jobs.retain(|pid, &mut h| {
            let keep = alive.contains(pid);
            if !keep {
                let _ = unsafe { CloseHandle(HANDLE(h as *mut _)) };
            }
            keep
        });
    }
}

/// A new job for the process (a process can be in several, since Windows 8)
unsafe fn create(pid: u32) -> Result<HANDLE, String> {
    let job = CreateJobObjectW(None, None).map_err(|e| format!("Can't limit the CPU: {e}"))?;
    let assigned = match OpenProcess(PROCESS_SET_QUOTA | PROCESS_TERMINATE, false, pid) {
        Ok(process) => {
            let r = AssignProcessToJobObject(job, process);
            let _ = CloseHandle(process);
            r.map_err(|e| format!("Can't limit the CPU: {e}"))
        }
        Err(e) => Err(format!("Can't limit the CPU: {e}")),
    };
    match assigned {
        Ok(()) => Ok(job),
        Err(e) => {
            let _ = CloseHandle(job);
            Err(e)
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};
    use windows::Win32::Foundation::FILETIME;
    use windows::Win32::System::Threading::{GetProcessTimes, PROCESS_QUERY_LIMITED_INFORMATION};

    /// CPU time the process used during `window`, as a share of one core
    fn busy_share(pid: u32, window: Duration) -> f64 {
        let cpu = || unsafe {
            let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).unwrap();
            let (mut a, mut b, mut kernel, mut user) = (FILETIME::default(), FILETIME::default(), FILETIME::default(), FILETIME::default());
            GetProcessTimes(process, &mut a, &mut b, &mut kernel, &mut user).unwrap();
            let _ = CloseHandle(process);
            let t = |f: FILETIME| ((f.dwHighDateTime as u64) << 32) | f.dwLowDateTime as u64;
            (t(kernel) + t(user)) as f64 / 10_000_000.0
        };
        let (start, at) = (cpu(), Instant::now());
        std::thread::sleep(window);
        (cpu() - start) / at.elapsed().as_secs_f64()
    }

    #[test]
    fn caps_a_busy_process_and_lifts_the_cap() {
        // a loop that keeps one core busy
        let mut child = std::process::Command::new("cmd").args(["/d", "/c", "for /l %i in () do @rem"]).spawn().unwrap();
        let pid = child.id();
        std::thread::sleep(Duration::from_millis(500));
        let free = busy_share(pid, Duration::from_millis(1500));

        set(pid, Some(MIN)).unwrap();
        let capped = busy_share(pid, Duration::from_millis(1500));
        set(pid, None).unwrap();
        let lifted = busy_share(pid, Duration::from_millis(1500));
        let _ = child.kill();
        let _ = child.wait();
        forget_except(&[]);

        let cores = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1) as f64;
        assert!(free > 0.6, "the loop isn't busy: {free:.2}");
        // 1% of the PC is cores/100 of one core
        assert!(capped < (cores / 100.0 * 2.0).max(0.15).min(free / 2.0), "not capped: {capped:.2} of a core (free {free:.2}, {cores} cores)");
        assert!(lifted > 0.6, "the cap wasn't lifted: {lifted:.2}");
    }

    #[test]
    fn lifting_a_cap_that_was_never_set_does_nothing() {
        assert!(set(u32::MAX - 7, None).is_ok());
    }
}
