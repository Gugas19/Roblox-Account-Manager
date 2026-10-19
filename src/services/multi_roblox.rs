//! Several Roblox clients at once. A Roblox client stays the only one through two names: `ROBLOX_singletonMutex` and,
//! in current clients, `ROBLOX_singletonEvent`. Taking both names first, as mutexes this app owns, lets every client
//! run: Roblox can neither own the mutex nor create its event under a name that's already a mutex. Only the first
//! name used to be taken, which current clients ignore. (The same as Roblox Manager, Paryx-games/roblox-manager,
//! whose users run several clients with it.)
//!
//! A mutex belongs to the thread that took it (it's let go when that thread ends), so one thread of its own takes
//! both and keeps them for as long as the setting is on, and takes them again by itself once Roblox let go of them
//! (every Roblox closed). When Roblox already made its event, that event is only held open: never signalled or
//! reset, nothing is changed inside Roblox.
//!
//! What usually holds them before the app starts: Roblox's own tray process (`RobloxPlayerBeta.exe
//! --launch-to-tray`, which Roblox starts with Windows), see [`tray_process`].

use crate::platform::win::{self, wide};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, ERROR_INVALID_HANDLE, HANDLE, WAIT_ABANDONED, WAIT_OBJECT_0};
use windows::Win32::System::Threading::{CreateMutexW, OpenEventW, ReleaseMutex, WaitForSingleObject, SYNCHRONIZATION_SYNCHRONIZE};

const MUTEX: &str = "ROBLOX_singletonMutex";
const EVENT: &str = "ROBLOX_singletonEvent";
/// How often the thread tries again while Roblox still holds them
const RETRY: Duration = Duration::from_secs(2);

static WANTED: AtomicBool = AtomicBool::new(false);
static HELD: AtomicBool = AtomicBool::new(false);
static THREAD: OnceLock<std::thread::Thread> = OnceLock::new();
/// Bumped by the thread after each look, so `set` can wait for the answer
static ROUNDS: Mutex<u64> = Mutex::new(0);

/// One name taken: the handle, and whether it's a mutex this thread owns (false: Roblox's event, only held open)
struct Taken(HANDLE, bool);

impl Taken {
    fn give_back(self) {
        unsafe {
            if self.1 {
                let _ = ReleaseMutex(self.0);
            }
            let _ = CloseHandle(self.0);
        }
    }
}

/// A name as a mutex this thread owns, or (`event_ok`) Roblox's existing event under it
fn take_name(name: &str, event_ok: bool) -> Option<Taken> {
    let name = wide(name);
    unsafe {
        match CreateMutexW(None, true, PCWSTR(name.as_ptr())) {
            Ok(mutex) => {
                if GetLastError() == ERROR_ALREADY_EXISTS {
                    let wait = WaitForSingleObject(mutex, 0);
                    if wait != WAIT_OBJECT_0 && wait != WAIT_ABANDONED {
                        let _ = CloseHandle(mutex);
                        return None;
                    }
                }
                Some(Taken(mutex, true))
            }
            // the name is another kind of object: only Roblox's event is fine, held open as it is
            Err(_) if event_ok && GetLastError() == ERROR_INVALID_HANDLE => OpenEventW(SYNCHRONIZATION_SYNCHRONIZE, false, PCWSTR(name.as_ptr())).ok().map(|e| Taken(e, false)),
            Err(_) => None,
        }
    }
}

/// Both names, or nothing (whatever was taken is given back)
fn take() -> Option<Vec<Taken>> {
    let event = take_name(EVENT, true)?;
    match take_name(MUTEX, false) {
        Some(mutex) => Some(vec![event, mutex]),
        None => {
            event.give_back();
            None
        }
    }
}

fn give_back(taken: Vec<Taken>) {
    taken.into_iter().for_each(Taken::give_back);
}

fn run() {
    let mut held: Option<Vec<Taken>> = None;
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

    /// A name that's already an event (Roblox made it first) is held open, never taken as a mutex; with names of
    /// the test's own
    #[test]
    fn an_existing_event_is_only_held_open() {
        use windows::Win32::System::Threading::CreateEventW;
        let name = format!("ram-test-{}-singleton", std::process::id());
        let wide_name = wide(&name);
        let event = unsafe { CreateEventW(None, true, false, PCWSTR(wide_name.as_ptr())) }.unwrap();
        assert!(take_name(&name, false).is_none(), "a mutex was made under an event's name");
        let held = take_name(&name, true).expect("the event isn't held open");
        assert!(!held.1, "an event counted as an owned mutex");
        held.give_back();
        unsafe {
            let _ = CloseHandle(event);
        }
        // a free name becomes a mutex this thread owns
        let free = take_name(&format!("ram-test-{}-free", std::process::id()), true).expect("free name not taken");
        assert!(free.1);
        free.give_back();
    }

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
