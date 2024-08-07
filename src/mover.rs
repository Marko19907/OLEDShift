use std::{
    mem,
    os::raw::c_int,
    ptr,
};
use std::collections::HashSet;
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::sync::Once;

use lazy_static::lazy_static;
use libloading::Library;
use rand::Rng;
use winapi::{
    shared::minwindef::{BOOL, LPARAM, LPVOID, TRUE, UINT},
    shared::windef::{HDC, HMONITOR, HWND, RECT},
    um::shellapi::{ABM_GETSTATE, ABS_AUTOHIDE, APPBARDATA, SHAppBarMessage},
    um::winuser::{
        AnimateWindow,
        AW_CENTER,
        EnumDisplayMonitors,
        EnumWindows,
        GetClassNameW,
        GetMonitorInfoW,
        GetSystemMetrics,
        GetWindowPlacement,
        HWND_TOP,
        IsWindowVisible,
        MonitorFromWindow,
        MONITORINFO,
        MONITORINFOEXW,
        SetWindowPos,
        SM_CYSCREEN,
        SPI_GETWORKAREA,
        SW_SHOWMAXIMIZED,
        SWP_NOSIZE,
        SWP_NOZORDER,
        SystemParametersInfoW,
        WINDOWPLACEMENT,
    },
};

use crate::controller::MAX_MOVE;

lazy_static! {
    /// A set of window classes that should be excluded from being moved.
    static ref CLASS_EXCLUSIONS: HashSet<&'static str> = {
        [
            "#32768", // OLEDShift right click menu
            "NarratorHelperWindow", // A small circle/line, more info here: https://github.com/Marko19907/OLEDShift/issues/12
            "TopLevelWindowForOverflowXamlIsland", // "Hidden Icon Menu", the flyout menu that appears when you click the arrow on the taskbar
        ].iter().cloned().collect()
    };
}

/// A function pointer to the IsWindowArranged function in user32.dll
static mut IS_WINDOW_ARRANGED: Option<unsafe extern "system" fn(c_int) -> bool> = None;
static INIT: Once = Once::new();

fn is_taskbar_auto_hidden() -> bool {
    let mut app_bar_data: APPBARDATA = unsafe { std::mem::zeroed() };
    app_bar_data.cbSize = std::mem::size_of::<APPBARDATA>() as u32;
    let state = unsafe { SHAppBarMessage(ABM_GETSTATE, &mut app_bar_data) as u32 };
    return (state & ABS_AUTOHIDE) != 0;
}

fn get_taskbar_height() -> i32 {
    let mut work_area_rect: RECT = unsafe { mem::zeroed() };
    unsafe {
        SystemParametersInfoW(SPI_GETWORKAREA, 0, &mut work_area_rect as *mut _ as LPVOID, 0);
    }
    let screen_height = unsafe { GetSystemMetrics(SM_CYSCREEN) };
    return screen_height - (work_area_rect.bottom - work_area_rect.top);
}

fn is_window_snapped(hwnd: HWND) -> bool {
    unsafe {
        INIT.call_once(|| {
            if let Ok(lib) = Library::new("user32.dll") {
                IS_WINDOW_ARRANGED = lib
                    .get::<unsafe extern "system" fn(c_int) -> bool>(b"IsWindowArranged")
                    .ok()
                    .map(|sym| *sym.into_raw()); // Convert the Symbol into a function pointer
            }
        });
        if let Some(func) = IS_WINDOW_ARRANGED {
            return func(hwnd as i32);
        }
    }
    return false;
}

/// Returns the smallest screen size in the form (width, height).
pub fn get_smallest_screen_size() -> Option<(i32, i32)> {
    let mut screen_sizes: Vec<(i32, i32)> = Vec::new();

    unsafe {
        EnumDisplayMonitors(
            ptr::null_mut(),
            ptr::null_mut(),
            Some(enum_display_monitors_callback),
            &mut screen_sizes as *mut _ as LPARAM,
        );
    }

    return screen_sizes.into_iter().min();
}

unsafe extern "system" fn enum_display_monitors_callback(
    _hmonitor: HMONITOR,
    _hdc: HDC,
    _lprect: *mut RECT,
    lparam: LPARAM,
) -> BOOL {
    let screen_sizes = &mut *(lparam as *mut Vec<(i32, i32)>);

    let mut monitor_info: MONITORINFOEXW = mem::zeroed();
    monitor_info.cbSize = mem::size_of::<MONITORINFOEXW>() as u32;

    GetMonitorInfoW(_hmonitor, &mut monitor_info as *mut _ as *mut _);

    let width = monitor_info.rcMonitor.right - monitor_info.rcMonitor.left;
    let height = monitor_info.rcMonitor.bottom - monitor_info.rcMonitor.top;

    screen_sizes.push((width, height));

    return TRUE;
}

/// Returns true if the window is visible.
fn is_window_visible(hwnd: HWND) -> bool {
    return unsafe { IsWindowVisible(hwnd) != 0 };
}

/// Returns true if the window is maximized.
fn is_window_maximized(wp: &WINDOWPLACEMENT) -> bool {
    return wp.showCmd as i32 == SW_SHOWMAXIMIZED;
}

/// Returns true if the window should be excluded from being moved based on its title or class.
fn is_excluded(hwnd: HWND) -> bool {
    // Get the window class
    let mut class_name = [0u16; 1024];
    let class_length = unsafe { GetClassNameW(hwnd, class_name.as_mut_ptr(), 1024) } as usize;
    let class_name = OsString::from_wide(&class_name[..class_length]);

    return CLASS_EXCLUSIONS.contains(class_name.to_str().unwrap_or(""));
}

fn move_window(hwnd: HWND) {
    if !is_window_visible(hwnd) {
        return;
    }

    let mut wp: WINDOWPLACEMENT = unsafe { mem::zeroed() };
    wp.length = mem::size_of::<WINDOWPLACEMENT>() as UINT;
    unsafe { GetWindowPlacement(hwnd, &mut wp) };

    if is_window_maximized(&wp) || is_window_snapped(hwnd) || is_excluded(hwnd) {
        return;
    }

    let h_monitor = unsafe { MonitorFromWindow(hwnd, winapi::um::winuser::MONITOR_DEFAULTTONEAREST) };
    let mut monitor_info: MONITORINFO = unsafe { mem::zeroed() };
    monitor_info.cbSize = mem::size_of::<MONITORINFO>() as UINT;
    unsafe { GetMonitorInfoW(h_monitor, &mut monitor_info) };

    let screen_width = monitor_info.rcMonitor.right - monitor_info.rcMonitor.left;
    let screen_height = monitor_info.rcMonitor.bottom - monitor_info.rcMonitor.top;
    let window_width = wp.rcNormalPosition.right - wp.rcNormalPosition.left;
    let window_height = wp.rcNormalPosition.bottom - wp.rcNormalPosition.top;

    // Check if the window is smaller than the screen, might not be true if the window is a game
    if !(window_width <= screen_width && window_height <= screen_height) {
        return;
    }

    let (max_x, max_y) = MAX_MOVE.lock().map(|guard| *guard).unwrap_or((50, 50));

    let max_move_x = i32::min(max_x, screen_width - window_width);
    let max_move_y = i32::min(max_y, screen_height - window_height);

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

    unsafe { SetWindowPos(hwnd, HWND_TOP, random_x, random_y, 0, 0, SWP_NOSIZE | SWP_NOZORDER) };

    if unsafe { AnimateWindow(hwnd, 4000, AW_CENTER) } == 0 {
        // Failed to animate window movement
    }

    return;
}

unsafe extern "system" fn enum_windows_proc(hwnd: HWND, _: LPARAM) -> BOOL {
    move_window(hwnd);
    return TRUE;
}

/// Moves the windows just once.
pub fn move_all_windows() {
    unsafe {
        EnumWindows(Some(enum_windows_proc), 0);
    }
}
