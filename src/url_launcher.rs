use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(target_os = "windows")] {
        use std::{thread, time::{Duration, Instant}};
        use windows::{core::HSTRING, Foundation::Uri, System::Launcher};
        use windows_future::AsyncStatus;

        const ASYNC_TIMEOUT: Duration = Duration::from_secs(15);

        pub fn open_report_problem(url: &str) -> Result<(), String> {
            let uri = Uri::CreateUri(&HSTRING::from(url)).map_err(|err| err.to_string())?;
            let operation = Launcher::LaunchUriAsync(&uri).map_err(|err| err.to_string())?;
            let launched = wait_for_async_result(
                || operation.Status().map_err(|err| err.to_string()),
                || operation.GetResults().map_err(|err| err.to_string()),
                || operation.ErrorCode().map(|code| code.to_string()).map_err(|err| err.to_string()),
            )?;

            if launched {
                Ok(())
            }
            else {
                Err("Windows did not launch the URI.".to_string())
            }
        }

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
                            return Err("The URI launch operation timed out.".to_string());
                        }
                        thread::sleep(Duration::from_millis(10));
                    }
                    AsyncStatus::Completed => return get_results(),
                    AsyncStatus::Canceled => return Err("The URI launch operation was canceled.".to_string()),
                    AsyncStatus::Error => return Err(error_code()?),
                    value => return Err(format!("Unexpected async status: {:?}", value)),
                }
            }
        }
    }
    else {
        pub fn open_report_problem(url: &str) -> Result<(), String> {
            open::that(url).map_err(|err| err.to_string())
        }
    }
}
