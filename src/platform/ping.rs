//! ICMP ping for the server list's ping column.

use windows::Win32::NetworkManagement::IpHelper::{IcmpCloseHandle, IcmpCreateFile, IcmpSendEcho, ICMP_ECHO_REPLY};

/// Round trip in milliseconds, None when the address doesn't answer in time
pub fn ping(ip: &str, timeout_ms: u32) -> Option<u32> {
    let address: std::net::Ipv4Addr = ip.parse().ok()?;
    let request = [0x41u8; 32];
    let mut reply = vec![0u8; std::mem::size_of::<ICMP_ECHO_REPLY>() + request.len() + 8];

    unsafe {
        let handle = IcmpCreateFile().ok()?;
        let count = IcmpSendEcho(handle, u32::from_ne_bytes(address.octets()), request.as_ptr() as _, request.len() as u16, None, reply.as_mut_ptr() as _, reply.len() as u32, timeout_ms);
        let _ = IcmpCloseHandle(handle);

        if count == 0 {
            return None;
        }
        let answer = &*(reply.as_ptr() as *const ICMP_ECHO_REPLY);
        (answer.Status == 0).then_some(answer.RoundTripTime)
    }
}
