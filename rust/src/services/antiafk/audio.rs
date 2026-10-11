//! Muting single Roblox windows through their Windows audio session.

use windows::core::Interface;
use windows::Win32::Media::Audio::{eConsole, eRender, IAudioSessionControl2, IAudioSessionManager2, IMMDeviceEnumerator, ISimpleAudioVolume, MMDeviceEnumerator};
use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED};

/// Mutes or unmutes every audio session of these processes, returns how many sessions changed
pub fn set_muted(pids: &[u32], muted: bool) -> usize {
    if pids.is_empty() {
        return 0;
    }

    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);

        let result = (|| -> windows::core::Result<usize> {
            let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
            let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole)?;
            let manager: IAudioSessionManager2 = device.Activate(CLSCTX_ALL, None)?;
            let sessions = manager.GetSessionEnumerator()?;
            let mut changed = 0;

            for i in 0..sessions.GetCount()? {
                let Ok(control) = sessions.GetSession(i) else { continue };
                let Ok(control) = control.cast::<IAudioSessionControl2>() else { continue };
                let Ok(pid) = control.GetProcessId() else { continue };
                if !pids.contains(&pid) {
                    continue;
                }
                if let Ok(volume) = control.cast::<ISimpleAudioVolume>() {
                    if volume.SetMute(muted, std::ptr::null()).is_ok() {
                        changed += 1;
                    }
                }
            }
            Ok(changed)
        })();

        result.unwrap_or(0)
    }
}
