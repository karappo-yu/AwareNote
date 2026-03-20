pub async fn release_unused_memory() -> std::io::Result<u64> {
    tokio::task::spawn_blocking(release_unused_memory_blocking)
        .await
        .map_err(|err| std::io::Error::other(err.to_string()))?
}

#[cfg(target_os = "linux")]
fn release_unused_memory_blocking() -> std::io::Result<u64> {
    let released = unsafe { libc::malloc_trim(0) };
    Ok(if released == 0 { 0 } else { 1 })
}

#[cfg(target_os = "macos")]
fn release_unused_memory_blocking() -> std::io::Result<u64> {
    unsafe extern "C" {
        fn malloc_zone_pressure_relief(zone: *mut libc::c_void, goal: libc::size_t)
            -> libc::size_t;
    }

    let released = unsafe { malloc_zone_pressure_relief(std::ptr::null_mut(), 0) };
    Ok(released as u64)
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn release_unused_memory_blocking() -> std::io::Result<u64> {
    Ok(0)
}
