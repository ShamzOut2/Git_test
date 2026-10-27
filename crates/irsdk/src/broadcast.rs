//! Commands sent to the simulator (irsdk_broadcastMsg): a window message broadcast
//! to every top-level window, which iRacing picks up.

use windows_sys::Win32::UI::WindowsAndMessaging::{RegisterWindowMessageW, SendNotifyMessageW, HWND_BROADCAST};

/// irsdk_BroadcastMsg (the ones used here).
#[derive(Debug, Clone, Copy)]
#[repr(u16)]
pub enum BroadcastMsg {
    /// Car number (CarNumberRaw), camera group, camera.
    CamSwitchNum = 1,
    /// Speed, slow motion (0/1).
    ReplaySetPlaySpeed = 3,
    /// Session number, session time in milliseconds.
    ReplaySearchSessionTime = 12,
}

/// Send a command; `var2` is a full 32-bit value for messages that take one, otherwise the
/// low and high halves carry two 16-bit arguments.
pub fn broadcast(msg: BroadcastMsg, var1: i32, var2: i32) {
    let name: Vec<u16> = "IRSDK_BROADCASTMSG\0".encode_utf16().collect();
    // SAFETY: `name` is a NUL-terminated UTF-16 string that outlives the call.
    let id = unsafe { RegisterWindowMessageW(name.as_ptr()) };
    if id == 0 {
        return;
    }
    let wparam = (msg as u16 as u32 | ((var1 as u16 as u32) << 16)) as usize;
    // SAFETY: a fire-and-forget broadcast with plain integer arguments.
    unsafe { SendNotifyMessageW(HWND_BROADCAST, id, wparam, var2 as isize) };
}
