//! Lighter background Roblox windows: Windows' efficiency mode (EcoQoS) and a lower priority.
//! Safer than the original AntiAFK-RBX FPS capper, which suspends Roblox's threads: if anything goes wrong here
//! Roblox keeps running normally.

use windows::Win32::Foundation::CloseHandle;
use windows::Win32::System::Threading::{
    OpenProcess, ProcessPowerThrottling, SetPriorityClass, SetProcessInformation, BELOW_NORMAL_PRIORITY_CLASS, NORMAL_PRIORITY_CLASS, PROCESS_POWER_THROTTLING_CURRENT_VERSION,
    PROCESS_POWER_THROTTLING_EXECUTION_SPEED, PROCESS_POWER_THROTTLING_STATE, PROCESS_SET_INFORMATION,
};

pub fn set_efficient(pid: u32, efficient: bool) -> bool {
    unsafe {
        let Ok(process) = OpenProcess(PROCESS_SET_INFORMATION, false, pid) else { return false };

        let state = PROCESS_POWER_THROTTLING_STATE {
            Version: PROCESS_POWER_THROTTLING_CURRENT_VERSION,
            ControlMask: PROCESS_POWER_THROTTLING_EXECUTION_SPEED,
            StateMask: if efficient { PROCESS_POWER_THROTTLING_EXECUTION_SPEED } else { 0 },
        };
        let throttled = SetProcessInformation(process, ProcessPowerThrottling, &state as *const _ as *const _, std::mem::size_of::<PROCESS_POWER_THROTTLING_STATE>() as u32).is_ok();
        let priority = SetPriorityClass(process, if efficient { BELOW_NORMAL_PRIORITY_CLASS } else { NORMAL_PRIORITY_CLASS }).is_ok();

        let _ = CloseHandle(process);
        throttled || priority
    }
}
