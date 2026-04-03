use cfg_if::cfg_if;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AutoStartState {
    Unsupported,
    Disabled,
    Enabled,
    DisabledByUser,
    DisabledByPolicy,
}

pub trait AutoStartManager {
    fn state(&self) -> AutoStartState;
    fn set_enabled(&self, enabled: bool) -> Result<AutoStartState, String>;
}

cfg_if! {
    if #[cfg(target_os = "windows")] {
        mod platform {
            use crate::platform::autostart::{AutoStartManager, AutoStartState};
            use std::{
                thread,
                time::{Duration, Instant}
            };
            use windows::{
                ApplicationModel::{StartupTask, StartupTaskState},
                core::HSTRING,
                Win32::{
                    Foundation::APPMODEL_ERROR_NO_PACKAGE,
                    Storage::Packaging::Appx::GetCurrentPackageFullName
                }
            };
            use windows_future::AsyncStatus;

            const STARTUP_TASK_ID: &str = "OLEDShiftStartup";
            const ASYNC_TIMEOUT: Duration = Duration::from_secs(20);

            #[derive(Default)]
            pub struct PlatformAutoStartManager;

            impl AutoStartManager for PlatformAutoStartManager {
                fn state(&self) -> AutoStartState {
                    return match get_task() {
                        Ok(task) => match task.State() {
                            Ok(state) => map_state(state),
                            Err(_) => AutoStartState::Unsupported,
                        },
                        Err(_) => AutoStartState::Unsupported,
                    };
                }

                fn set_enabled(&self, enabled: bool) -> Result<AutoStartState, String> {
                    let task = get_task()?;

                    if enabled {
                        let operation = task.RequestEnableAsync().map_err(|err| err.to_string())?;
                        return wait_for_async_result(
                            || operation.Status().map_err(|err| err.to_string()),
                            || operation.GetResults().map(map_state).map_err(|err| err.to_string()),
                            || operation.ErrorCode().map(|code| code.to_string()).map_err(|err| err.to_string()),
                        );
                    }

                    task.Disable().map_err(|err| err.to_string())?;
                    task.State().map(map_state).map_err(|err| err.to_string())
                }
            }

            /// Retrieves the StartupTask for this application, returning an error if it fails or if we're not running as a packaged app
            fn get_task() -> Result<StartupTask, String> {
                if !is_packaged() {
                    return Err("Auto start is unsupported for unpackaged builds.".to_string());
                }

                let operation = StartupTask::GetAsync(&HSTRING::from(STARTUP_TASK_ID)).map_err(|err| err.to_string())?;
                wait_for_async_result(
                    || operation.Status().map_err(|err| err.to_string()),
                    || operation.GetResults().map_err(|err| err.to_string()),
                    || operation.ErrorCode().map(|code| code.to_string()).map_err(|err| err.to_string()),
                )
            }

            /// Returns true if the application is running as a packaged app, false otherwise
            fn is_packaged() -> bool {
                let mut length = 0;
                let result = unsafe { GetCurrentPackageFullName(&mut length, None) };

                return result != APPMODEL_ERROR_NO_PACKAGE;
            }

            /// Spins until the provided async operation completes, returning the results or an error message
            /// Times out after a reasonable amount of time to avoid infinite loops in case of unexpected issues
            fn wait_for_async_result<T>(
                mut status: impl FnMut() -> Result<AsyncStatus, String>,
                mut get_results: impl FnMut() -> Result<T, String>,
                mut error_code: impl FnMut() -> Result<String, String>,
            ) -> Result<T, String> {
                let started_at = Instant::now();

                loop {
                    match status()? {
                        AsyncStatus::Started => {
                            if started_at.elapsed() >= ASYNC_TIMEOUT {
                                return Err("The startup task operation timed out!".to_string());
                            }
                            thread::sleep(Duration::from_millis(10));
                        }
                        AsyncStatus::Completed => return get_results(),
                        AsyncStatus::Canceled => return Err("The startup task operation was canceled".to_string()),
                        AsyncStatus::Error => return Err(error_code()?),
                        value => return Err(format!("Unexpected async status: {:?}", value)),
                    }
                }
            }

            #[inline]
            fn map_state(state: StartupTaskState) -> AutoStartState {
                return match state {
                    StartupTaskState::Disabled => AutoStartState::Disabled,
                    StartupTaskState::Enabled | StartupTaskState::EnabledByPolicy => AutoStartState::Enabled,
                    StartupTaskState::DisabledByUser => AutoStartState::DisabledByUser,
                    StartupTaskState::DisabledByPolicy => AutoStartState::DisabledByPolicy,
                    _ => AutoStartState::Unsupported,
                };
            }
        }
    }
    else {
        mod platform {
            use crate::platform::autostart::{AutoStartManager, AutoStartState};

            #[derive(Default)]
            pub struct PlatformAutoStartManager;

            impl AutoStartManager for PlatformAutoStartManager {
                fn state(&self) -> AutoStartState {
                    AutoStartState::Unsupported
                }

                fn set_enabled(&self, _enabled: bool) -> Result<AutoStartState, String> {
                    Ok(AutoStartState::Unsupported)
                }
            }
        }
    }
}

pub use platform::PlatformAutoStartManager;
