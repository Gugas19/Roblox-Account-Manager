//! Several Roblox clients at once. A Roblox client only stays the only one when it gets its two singleton objects
//! first: the mutex `ROBLOX_singletonMutex` and, in current clients, the event `ROBLOX_singletonEvent`. Holding both
//! before Roblox starts lets every client run. Only the mutex used to be held, which current clients ignore.
//!
//! A mutex belongs to the thread that took it (it's let go when that thread ends), so one thread of its own takes
//! both and keeps them for as long as the setting is on, and takes them again by itself once Roblox let go of them
//! (every Roblox closed). The event is never signalled, nothing is changed inside Roblox.
//!
//! What usually holds them before the app starts: Roblox's own tray process (`RobloxPlayerBeta.exe
//! --launch-to-tray`, which Roblox starts with Windows), see [`tray_process`].

use crate::platform::win::{self, wide};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HANDLE, WAIT_ABANDONED, WAIT_OBJECT_0};
use windows::Win32::System::Threading::{CreateEventW, CreateMutexW, ReleaseMutex, WaitForSingleObject};

const MUTEX: &str = "ROBLOX_singletonMutex";
const EVENT: &str = "ROBLOX_singletonEvent";
/// How often the thread tries again while Roblox still holds them
const RETRY: Duration = Duration::from_secs(2);

static WANTED: AtomicBool = AtomicBool::new(false);
static HELD: AtomicBool = AtomicBool::new(false);
static THREAD: OnceLock<std::thread::Thread> = OnceLock::new();
/// Bumped by the thread after each look, so `set` can wait for the answer
static ROUNDS: Mutex<u64> = Mutex::new(0);

struct Handles {
    mutex: HANDLE,
    event: HANDLE,
}

/// Both objects, or nothing (whatever was taken is given back): the mutex must be ours, the event only open
fn take() -> Option<Handles> {
    unsafe {
        let name = wide(MUTEX);
        let mutex = CreateMutexW(None, true, PCWSTR(name.as_ptr())).ok()?;
        if GetLastError() == ERROR_ALREADY_EXISTS {
            let wait = WaitForSingleObject(mutex, 0);
            if wait != WAIT_OBJECT_0 && wait != WAIT_ABANDONED {
                let _ = CloseHandle(mutex);
                return None;
            }
        }
        let name = wide(EVENT);
        // manual reset, not signalled; an existing one is only opened, never signalled or reset
        match CreateEventW(None, true, false, PCWSTR(name.as_ptr())) {
            Ok(event) => Some(Handles { mutex, event }),
            Err(_) => {
                let _ = ReleaseMutex(mutex);
                let _ = CloseHandle(mutex);
                None
            }
        }
    }
}

fn give_back(h: Handles) {
    unsafe {
        let _ = ReleaseMutex(h.mutex);
        let _ = CloseHandle(h.mutex);
        let _ = CloseHandle(h.event);
    }
}

fn run() {
    let mut held: Option<Handles> = None;
    loop {
        let wanted = WANTED.load(Ordering::SeqCst);
        if wanted && held.is_none() {
            held = take();
            if held.is_some() {
                log::info!("Multi Roblox on: both Roblox singleton objects are held");
            }
        } else if !wanted {
            if let Some(h) = held.take() {
                give_back(h);
            }
        }
        HELD.store(held.is_some(), Ordering::SeqCst);
        *ROUNDS.lock().unwrap() += 1;
        std::thread::park_timeout(RETRY);
    }
}

/// Turns it on or off; true when it works right now (always when off). When Roblox holds the objects, it keeps
/// trying in the background and works as soon as they're free.
pub fn set(enabled: bool) -> bool {
    WANTED.store(enabled, Ordering::SeqCst);
    let before = *ROUNDS.lock().unwrap();
    let thread = THREAD.get_or_init(|| std::thread::Builder::new().name("multi-roblox".into()).spawn(run).expect("multi roblox thread").thread().clone());
    thread.unpark();
    // the thread answers within a moment
    let end = Instant::now() + Duration::from_millis(500);
    while *ROUNDS.lock().unwrap() == before && Instant::now() < end {
        std::thread::sleep(Duration::from_millis(5));
    }
    !enabled || HELD.load(Ordering::SeqCst)
}

/// Whether it's on and working right now
pub fn working() -> bool {
    HELD.load(Ordering::SeqCst)
}

/// Roblox's background tray process, which starts with Windows and holds the singleton objects
pub fn tray_process() -> Option<u32> {
    win::roblox_processes().into_iter().find(|p| p.command_line.contains("--launch-to-tray")).map(|p| p.pid)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Only two names of Roblox's own: nothing else on the PC uses them, and the test holds them a moment only
    #[test]
    fn holds_both_objects_and_gives_them_back() {
        // Roblox, or your own copy of the app with Multi Roblox on, holds them: nothing to test here
        let free = std::thread::spawn(|| take().map(give_back).is_some()).join().unwrap();
        if !free {
            eprintln!("the Roblox singleton objects are held by someone else: skipped");
            return;
        }
        assert!(set(true), "not held");
        // someone else (a Roblox starting now) can't own the mutex
        let other = std::thread::spawn(|| take().map(give_back).is_some()).join().unwrap();
        assert!(!other, "the mutex isn't owned");
        assert!(set(false));
        assert!(!working());
        let free = std::thread::spawn(|| take().map(give_back).is_some()).join().unwrap();
        assert!(free, "not given back");
    }
}
