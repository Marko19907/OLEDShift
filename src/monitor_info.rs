use std::{
    ffi::OsString,
    os::windows::ffi::{OsStrExt, OsStringExt},
    mem
};
use windows::{
    core::{BOOL, PCWSTR},
    Win32::{
        Foundation::{FALSE, LPARAM, RECT, TRUE},
        Graphics::Gdi::{DISPLAY_DEVICEW, EnumDisplayDevicesW, EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO, MONITORINFOEXW},
    },
};

unsafe extern "system" fn enum_display_monitors_collect_callback(
    hmonitor: HMONITOR,
    _hdc: HDC,
    _lprect: *mut RECT,
    lparam: LPARAM
) -> BOOL {
    let monitors_info = &mut *(lparam.0 as *mut Vec<MONITORINFOEXW>);

    let mut monitor_info: MONITORINFOEXW = mem::zeroed();
    monitor_info.monitorInfo.cbSize = mem::size_of::<MONITORINFOEXW>() as u32;

    if GetMonitorInfoW(hmonitor, &mut monitor_info as *mut _ as *mut MONITORINFO) == FALSE {
        return TRUE;
    }

    monitors_info.push(monitor_info);
    return TRUE;
}

/// Helper to get all monitors (device name, area, etc.)
pub fn get_all_monitors_info() -> Vec<MONITORINFOEXW> {
    let mut monitors_info: Vec<MONITORINFOEXW> = Vec::new();
    unsafe {
        let _ = EnumDisplayMonitors(
            None,
            None,
            Some(enum_display_monitors_collect_callback),
            LPARAM(&mut monitors_info as *mut _ as isize),
        );
    }
    return monitors_info;
}

/// A small helper to extract the device name from MONITORINFOEXW
pub fn monitor_device_name(monitor_info: &MONITORINFOEXW) -> String {
    // Convert `szDevice` from wide char to Rust String
    let len = monitor_info.szDevice.iter()
        .position(|&c| c == 0)
        .unwrap_or(monitor_info.szDevice.len());
    let os_str = OsString::from_wide(&monitor_info.szDevice[..len]);
    return os_str.to_string_lossy().to_string();
}

/// A small helper to extract the friendly name from the device name, this is the device name that comes from MONITORINFOEXW, monitor_info.szDevice
pub fn get_display_device_info(device_name: &str) -> Option<(String, String)> {
    unsafe {
        let mut display_device: DISPLAY_DEVICEW = mem::zeroed();
        display_device.cb = mem::size_of::<DISPLAY_DEVICEW>() as u32;

        // EnumDisplayDevices expects a wide string for deviceName
        let wide_devname: Vec<u16> = OsString::from(device_name)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        // iDevNum=0 fetches info about the primary device entry
        // If the call fails, return None
        let success = EnumDisplayDevicesW(PCWSTR(wide_devname.as_ptr()), 0, &mut display_device, 0) != FALSE;
        if !success {
            return None;
        }

        // Convert `DeviceString` (friendly name)
        let friendly_name = {
            let slice = &display_device.DeviceString;
            let len = slice.iter().position(|&c| c == 0).unwrap_or(slice.len());
            OsString::from_wide(&slice[..len]).to_string_lossy().to_string()
        };

        // Convert `DeviceID` (often a PnP hardware path)
        let device_id = {
            let slice = &display_device.DeviceID;
            let len = slice.iter().position(|&c| c == 0).unwrap_or(slice.len());
            OsString::from_wide(&slice[..len]).to_string_lossy().to_string()
        };

        return Some((friendly_name, device_id));
    }
}

/// Retrieve the extended monitor information structure (MONITORINFOEXW) for the given HMONITOR.
pub fn get_monitor_info_ex(h_monitor: HMONITOR) -> Option<MONITORINFOEXW> {
    unsafe {
        let mut mon_info_ex: MONITORINFOEXW = mem::zeroed();
        mon_info_ex.monitorInfo.cbSize = mem::size_of::<MONITORINFOEXW>() as u32;

        let success = GetMonitorInfoW(h_monitor, &mut mon_info_ex as *mut _ as *mut MONITORINFO);
        return if success == FALSE {
            None
        } else {
            Some(mon_info_ex)
        }
    }
}
