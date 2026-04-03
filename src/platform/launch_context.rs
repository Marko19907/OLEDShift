use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(target_os = "windows")] {
        
        /// Returns true if the application was started by a Windows startup task, false otherwise
        pub fn started_by_startup_task() -> bool {
            use windows::ApplicationModel::{Activation::ActivationKind, AppInstance};

            return AppInstance::GetActivatedEventArgs()
                .and_then(|args| args.Kind())
                .map(|kind| kind == ActivationKind::StartupTask)
                .unwrap_or(false);
        }
    }
    else {
        pub fn started_by_startup_task() -> bool {
            return false;
        }
    }
}
