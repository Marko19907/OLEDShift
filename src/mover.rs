use std::ffi::c_void;
use std::ptr;
use std::mem;
use std::time::{SystemTime, UNIX_EPOCH};
use rand::Rng;
use winapi::{
    shared::minwindef::{BOOL, DWORD, LPARAM, TRUE, UINT, LPVOID},
    shared::basetsd::UINT_PTR,
    shared::windef::{RECT, HWND},
    um::winuser::{
        EnumWindows,
        DispatchMessageW,
        AnimateWindow,
        AW_CENTER,
        GetMessageW,
        GetMonitorInfoW,
        GetSystemMetrics,
        GetWindowPlacement,
        HWND_TOP,
        IsWindowVisible,
        KillTimer,
        MONITOR_DEFAULTTONEAREST,
        MonitorFromWindow,
        MONITORINFO,
        MSG,
        SetTimer,
        SetWindowPos,
        SM_CYSCREEN,
        SPI_GETWORKAREA,
        SWP_NOSIZE,
        SWP_NOZORDER,
        SystemParametersInfoW,
        TranslateMessage,
        WINDOWPLACEMENT
    }
};
use winapi::um::shellapi::{ABM_GETSTATE, ABS_AUTOHIDE, APPBARDATA, SHAppBarMessage};
use winapi::um::winuser::SW_SHOWMAXIMIZED;

const MAX_MOVE_X: i32 = 50;
const MAX_MOVE_Y: i32 = 50;

fn is_taskbar_auto_hidden() -> bool {
    let mut app_bar_data: APPBARDATA = unsafe { std::mem::zeroed() };
    app_bar_data.cbSize = std::mem::size_of::<APPBARDATA>() as u32;
    let state = unsafe { SHAppBarMessage(ABM_GETSTATE, &mut app_bar_data) as u32 };
    return (state & ABS_AUTOHIDE) != 0;
}

fn get_taskbar_height() -> i32 {
    unsafe {
        let mut work_area_rect: RECT = mem::zeroed();
        SystemParametersInfoW(SPI_GETWORKAREA, 0, &mut work_area_rect as *mut _ as LPVOID, 0);
        let screen_height = GetSystemMetrics(SM_CYSCREEN);
        return screen_height - (work_area_rect.bottom - work_area_rect.top);
    }
}

/// Returns true if the window is visible.
unsafe fn is_window_visible(hwnd: HWND) -> bool {
    return IsWindowVisible(hwnd) != 0;
}

/// Returns true if the window is maximized.
fn is_window_maximized(wp: &WINDOWPLACEMENT) -> bool {
    return wp.showCmd as i32 == SW_SHOWMAXIMIZED;
}

unsafe extern "system" fn enum_windows_proc(hwnd: HWND, _: LPARAM) -> BOOL {
    if !is_window_visible(hwnd) {
        return TRUE;
    }
    let mut wp: WINDOWPLACEMENT = mem::zeroed();
    wp.length = mem::size_of::<WINDOWPLACEMENT>() as UINT;
    GetWindowPlacement(hwnd, &mut wp);

    if is_window_maximized(&wp) {
        return TRUE;
    }

    let h_monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
    let mut monitor_info: MONITORINFO = mem::zeroed();
    monitor_info.cbSize = mem::size_of::<MONITORINFO>() as UINT;
    GetMonitorInfoW(h_monitor, &mut monitor_info);

    let screen_width = monitor_info.rcMonitor.right - monitor_info.rcMonitor.left;
    let screen_height = monitor_info.rcMonitor.bottom - monitor_info.rcMonitor.top;
    let window_width = wp.rcNormalPosition.right - wp.rcNormalPosition.left;
    let window_height = wp.rcNormalPosition.bottom - wp.rcNormalPosition.top;

    // Check if the window is smaller than the screen, might not be true if the window is a game
    if !(window_width <= screen_width && window_height <= screen_height) {
        return TRUE;
    }

    let max_move_x = i32::min(MAX_MOVE_X, screen_width - window_width);
    let max_move_y = i32::min(MAX_MOVE_Y, screen_height - window_height);

    let mut rng = rand::thread_rng();
    let random_x = wp.rcNormalPosition.left + rng.gen_range(0..(2 * max_move_x + 1)) - max_move_x;
    let random_y = wp.rcNormalPosition.top + rng.gen_range(0..(2 * max_move_y + 1)) - max_move_y;

    let random_x = i32::max(monitor_info.rcMonitor.left, i32::min(random_x, monitor_info.rcMonitor.right - window_width));
    let mut random_y = i32::max(monitor_info.rcMonitor.top, i32::min(random_y, monitor_info.rcMonitor.bottom - window_height));

    let taskbar_height = get_taskbar_height();

    if is_taskbar_auto_hidden() {
        random_y = i32::max(random_y, monitor_info.rcMonitor.top + taskbar_height);
    } else {
        random_y = i32::min(random_y, monitor_info.rcMonitor.bottom - window_height - taskbar_height);
    }

    SetWindowPos(hwnd, HWND_TOP, random_x, random_y, 0, 0, SWP_NOSIZE | SWP_NOZORDER);

    if AnimateWindow(hwnd, 4000, AW_CENTER) == 0 {
        // Failed to animate window movement
    }

    return TRUE;
}

unsafe extern "system" fn timer_proc(_: HWND, _: UINT, _: UINT_PTR, _: DWORD) {
    EnumWindows(Some(enum_windows_proc), 0);
}

/// Moves the windows just once.
pub fn run() {
    unsafe {
        EnumWindows(Some(enum_windows_proc), 0);
    }
    return;
}

/// Leftover code from the proof of concept command line version, not used in the GUI version.
pub fn main() {
    unsafe {
        let _seed = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let _rng = rand::thread_rng();

        let mut interval = String::new();
        println!("Enter the interval in milliseconds: ");
        std::io::stdin().read_line(&mut interval).unwrap();

        let mut interval = interval.trim().parse::<i32>().unwrap_or_else(|_| 0);
        while interval <= 0 {
            println!("Bad entry. Enter a positive number: ");
            let mut new_interval = String::new();
            std::io::stdin().read_line(&mut new_interval).unwrap();
            interval = new_interval.trim().parse::<i32>().unwrap_or_else(|_| 0);
        }

        EnumWindows(Some(enum_windows_proc), 0);

        let timer_id = SetTimer(ptr::null_mut(), 0, interval as u32, Some(timer_proc));
        if timer_id == 0 {
            println!("Failed to set timer!");
            return;
        }

        let mut msg: MSG = mem::zeroed();
        while GetMessageW(&mut msg, ptr::null_mut(), 0, 0) != 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        KillTimer(ptr::null_mut(), timer_id);
    }
}
