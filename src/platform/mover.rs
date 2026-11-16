use std::{
    collections::HashSet,
    ffi::{
        c_void,
        OsString
    },
    mem,
    os::{
        windows::ffi::OsStringExt
    },
    sync::OnceLock,
    thread,
    time::{Duration, Instant},
};

use lazy_static::lazy_static;
use libloading::Library;
use rand::Rng;
use windows::{
    core::BOOL,
    Win32::{
        Foundation::{FALSE, HWND, LPARAM, RECT, TRUE},
        Graphics::Gdi::{EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO, MONITORINFOEXW, MONITOR_DEFAULTTONEAREST, MonitorFromWindow},
        UI::{
            WindowsAndMessaging::{
                EnumWindows,
                GetClassNameW,
                GetGUIThreadInfo,
                GetWindowRect,
                GetWindowThreadProcessId,
                GUITHREADINFO,
                IsIconic,
                IsWindowVisible,
                IsZoomed,
                SetWindowPos,
                SPI_GETCLIENTAREAANIMATION,
                SWP_ASYNCWINDOWPOS,
                SWP_NOACTIVATE,
                SWP_NOSIZE,
                SWP_NOZORDER,
                SystemParametersInfoW,
            },
        },
    },
};

use crate::app::controller::{ENABLED_MONITORS, MAX_MOVE};
use crate::platform::monitor_info::{get_display_device_info, get_monitor_info_ex, monitor_device_name};


lazy_static! {
    /// A set of window classes that should be excluded from being moved.
    static ref CLASS_EXCLUSIONS: HashSet<&'static str> = {
        [
            "#32768", // OLEDShift right click menu
            "NarratorHelperWindow", // A small circle/line, more info here: https://github.com/Marko19907/OLEDShift/issues/12
            "TopLevelWindowForOverflowXamlIsland", // "Hidden Icon Menu", the flyout menu that appears when you click the arrow on the taskbar
            "Shell_TrayWnd", // Taskbar
            "Shell_SecondaryTrayWnd", // Taskbar on secondary monitors
        ].iter().cloned().collect()
    };
}

struct IsWindowArrangedFn {
    _user32: Library,
    func: unsafe extern "system" fn(HWND) -> BOOL,
}

/// A function pointer to the IsWindowArranged function in user32.dll
static IS_WINDOW_ARRANGED: OnceLock<Option<IsWindowArrangedFn>> = OnceLock::new();

/// Loads the undocumented IsWindowArranged function from user32.dll
/// It's actually provided by the windows crate but I'm leaving it like this to preserve Windows 7 compatibility
fn is_window_snapped(hwnd: HWND) -> bool {
    let is_window_arranged = IS_WINDOW_ARRANGED.get_or_init(|| unsafe {
        let user32 = Library::new("user32.dll").ok()?;
        let func = *user32
            .get::<unsafe extern "system" fn(HWND) -> BOOL>(b"IsWindowArranged")
            .ok()?;

        Some(IsWindowArrangedFn {
            _user32: user32,
            func,
        })
    });

    return is_window_arranged
        .as_ref()
        .map(|is_window_arranged| unsafe { (is_window_arranged.func)(hwnd) != FALSE })
        .unwrap_or(false);
}

/// Returns the smallest screen size in the form (width, height).
pub fn get_smallest_screen_size() -> Option<(i32, i32)> {
    let mut screen_sizes: Vec<(i32, i32)> = Vec::new();

    unsafe {
        let _ = EnumDisplayMonitors(
            None,
            None,
            Some(enum_display_monitors_callback),
            LPARAM(&mut screen_sizes as *mut _ as isize),
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
    let screen_sizes = &mut *(lparam.0 as *mut Vec<(i32, i32)>);

    let mut monitor_info: MONITORINFOEXW = mem::zeroed();
    monitor_info.monitorInfo.cbSize = mem::size_of::<MONITORINFOEXW>() as u32;

    let _ = GetMonitorInfoW(_hmonitor, &mut monitor_info as *mut _ as *mut MONITORINFO);

    let width = monitor_info.monitorInfo.rcMonitor.right - monitor_info.monitorInfo.rcMonitor.left;
    let height = monitor_info.monitorInfo.rcMonitor.bottom - monitor_info.monitorInfo.rcMonitor.top;

    screen_sizes.push((width, height));

    return TRUE;
}

/// Returns true if the window should be excluded from being moved based on its title or class.
fn is_excluded(hwnd: HWND) -> bool {
    // Get the window class
    let mut class_name = [0u16; 1024];
    let class_length = unsafe { GetClassNameW(hwnd, &mut class_name) } as usize;
    let class_name = OsString::from_wide(&class_name[..class_length]);

    return CLASS_EXCLUSIONS.contains(class_name.to_str().unwrap_or(""));
}

/// Returns true if we should move the window based on the monitor it's on.
/// The window should be moved if it's not in the settings file, or if it's in the settings file and enabled.
fn is_monitor_included(h_monitor: &HMONITOR) -> bool {
    if let Some(mon_info_ex) = get_monitor_info_ex(*h_monitor) {
        let raw_name = monitor_device_name(&mon_info_ex);

        if let Some((_, device_id)) = get_display_device_info(&raw_name) {
            let enabled = ENABLED_MONITORS.lock().unwrap();

            if !enabled.contains_key(&device_id) {
                return true; // If the monitor is not in the settings file, we should move the window.
            }

            return *enabled.get(&device_id).unwrap_or(&false);
        }
    }

    return false;
}

const SHIFT_DURATION: Duration = Duration::from_millis(200);
const FRAME_INTERVAL: Duration = Duration::from_millis(16);

struct WindowShift {
    hwnd: HWND,
    thread_id: u32,
    start: (i32, i32),
    target: (i32, i32),
    confirmed: (i32, i32),
    pending: Option<(i32, i32)>,
}

impl WindowShift {
    fn advance(&mut self, progress: f64) -> bool {
        if !can_move_window(self.hwnd, self.thread_id) {
            return false;
        }

        let Some(rect) = window_rect(self.hwnd) else {
            return false;
        };
        let current = (rect.left, rect.top);
        if self.pending == Some(current) {
            self.confirmed = current;
            self.pending = None;
        } else if current != self.confirmed {
            // Another app or the user repositioned this window during the shift.
            return false;
        }

        // Allow only one outstanding move per window so a busy app cannot
        // replay a backlog of animation frames when it starts responding.
        if self.pending.is_some() {
            return true;
        }

        let next = position_at(self.start, self.target, progress);
        if next != current {
            let result = unsafe {
                SetWindowPos(
                    self.hwnd,
                    None,
                    next.0,
                    next.1,
                    0,
                    0,
                    SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_ASYNCWINDOWPOS,
                )
            };
            if result.is_err() {
                return false;
            }
            self.pending = Some(next);
        }

        true
    }
}

fn position_at(start: (i32, i32), target: (i32, i32), progress: f64) -> (i32, i32) {
    let eased = progress * progress * (3.0 - 2.0 * progress);
    let interpolate = |from: i32, to: i32| {
        (f64::from(from) + (f64::from(to) - f64::from(from)) * eased).round() as i32
    };
    (interpolate(start.0, target.0), interpolate(start.1, target.1))
}

fn window_rect(hwnd: HWND) -> Option<RECT> {
    let mut rect = RECT::default();
    unsafe { GetWindowRect(hwnd, &mut rect) }.ok()?;
    Some(rect)
}

fn can_move_window(hwnd: HWND, thread_id: u32) -> bool {
    if thread_id == 0
        || unsafe { GetWindowThreadProcessId(hwnd, None) } != thread_id
        || unsafe { !IsWindowVisible(hwnd).as_bool() || IsIconic(hwnd).as_bool() || IsZoomed(hwnd).as_bool() }
        || is_window_snapped(hwnd)
    {
        return false;
    }

    let mut info = GUITHREADINFO {
        cbSize: mem::size_of::<GUITHREADINFO>() as u32,
        ..Default::default()
    };
    let has_thread_info = unsafe { GetGUIThreadInfo(thread_id, &mut info) }.is_ok();
    !has_thread_info || info.hwndMoveSize != hwnd
}

fn plan_window_shift(hwnd: HWND) -> Option<WindowShift> {
    let thread_id = unsafe { GetWindowThreadProcessId(hwnd, None) };
    if !can_move_window(hwnd, thread_id) || is_excluded(hwnd) {
        return None;
    }

    let monitor = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) };
    if !is_monitor_included(&monitor) {
        return None;
    }
    let mut info = MONITORINFO {
        cbSize: mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    if !unsafe { GetMonitorInfoW(monitor, &mut info) }.as_bool() {
        return None;
    }

    let rect = window_rect(hwnd)?;
    let width = rect.right - rect.left;
    let height = rect.bottom - rect.top;
    let bounds = info.rcMonitor;
    let work = info.rcWork;
    if width > bounds.right - bounds.left || height > bounds.bottom - bounds.top {
        return None;
    }

    let (max_x, max_y) = MAX_MOVE.lock().map(|guard| *guard).unwrap_or((50, 50));
    let mut rng = rand::rng();
    let mut target_axis = |start: i32, size, distance: i32, work: (i32, i32), monitor| {
        let (min, max) = if size <= work.1 - work.0 {
            work
        } else {
            monitor
        };
        let distance = distance.clamp(0, max - min - size);
        (start + rng.random_range(-distance..=distance)).clamp(min, max - size)
    };
    let start = (rect.left, rect.top);
    let target = (
        target_axis(start.0, width, max_x, (work.left, work.right), (bounds.left, bounds.right)),
        target_axis(start.1, height, max_y, (work.top, work.bottom), (bounds.top, bounds.bottom)),
    );

    (start != target).then_some(WindowShift {
        hwnd,
        thread_id,
        start,
        target,
        confirmed: start,
        pending: None,
    })
}

unsafe extern "system" fn enum_windows_proc(hwnd: HWND, context: LPARAM) -> BOOL {
    let shifts = unsafe { &mut *(context.0 as *mut Vec<WindowShift>) };
    if let Some(shift) = plan_window_shift(hwnd) {
        shifts.push(shift);
    }
    TRUE
}

fn animations_enabled() -> bool {
    let mut enabled = FALSE;
    unsafe {
        SystemParametersInfoW(
            SPI_GETCLIENTAREAANIMATION,
            0,
            Some((&mut enabled as *mut BOOL).cast::<c_void>()),
            Default::default(),
        )
    }
    .is_ok() && enabled.as_bool()
}

pub fn move_all_windows(is_running: impl Fn() -> bool) {
    if !is_running() {
        return;
    }

    let mut shifts = Vec::<WindowShift>::new();
    unsafe {
        let _ = EnumWindows(
            Some(enum_windows_proc),
            LPARAM(&mut shifts as *mut Vec<WindowShift> as isize),
        );
    }

    let duration = if animations_enabled() {
        SHIFT_DURATION
    } else {
        Duration::ZERO
    };
    animate_shifts(shifts, duration, is_running);
}

fn animate_shifts(mut shifts: Vec<WindowShift>, duration: Duration, is_running: impl Fn() -> bool) {
    let started = Instant::now();
    while !shifts.is_empty() && is_running() {
        let elapsed = started.elapsed();
        let finished = elapsed >= duration;
        let progress = if finished { 1.0 } else { elapsed.as_secs_f64() / duration.as_secs_f64() };
        shifts.retain_mut(|shift| shift.advance(progress));
        if finished || shifts.is_empty() {
            break;
        }

        let next_frame = (elapsed + FRAME_INTERVAL).min(duration);
        thread::sleep(next_frame.saturating_sub(started.elapsed()));
    }
}
