pub fn schedule_process_restart(delay: std::time::Duration) {
    if restart_disabled() {
        tracing::info!("process restart skipped because AUXM_DISABLE_RESTART is enabled");
        return;
    }

    tokio::spawn(async move {
        tokio::time::sleep(delay).await;
        if let Err(err) = restart_current_process() {
            tracing::error!("failed to restart process after scan: {}", err);
        }
    });
}

fn restart_disabled() -> bool {
    std::env::var("AUXM_DISABLE_RESTART")
        .ok()
        .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false)
}

fn restart_current_process() -> std::io::Result<()> {
    let current_exe = std::env::current_exe()?;
    let args: Vec<_> = std::env::args_os().skip(1).collect();

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;

        let err = std::process::Command::new(&current_exe).args(&args).exec();
        Err(err)
    }

    #[cfg(not(unix))]
    {
        std::process::Command::new(&current_exe)
            .args(&args)
            .spawn()?;
        std::process::exit(0);
    }
}
