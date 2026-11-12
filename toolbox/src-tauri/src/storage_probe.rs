use crate::contracts::{StorageCapacity, ToolboxError};
use crate::workspace::{error, reject_links};
use std::path::Path;
#[allow(clippy::unnecessary_cast)] // statvfs field widths differ between supported Unix targets.
pub fn available_space(path: &Path) -> Result<u64, ToolboxError> {
    reject_links(path)?;
    if !path.exists() {
        return Err(error("UnknownCapacity"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        let path =
            std::ffi::CString::new(path.as_os_str().as_bytes()).map_err(|_| error("UnsafePath"))?;
        let mut stats = std::mem::MaybeUninit::<libc::statvfs>::uninit();
        // statvfs writes exactly the platform-defined structure for this existing path.
        if unsafe { libc::statvfs(path.as_ptr(), stats.as_mut_ptr()) } != 0 {
            return Err(error("UnknownCapacity"));
        }
        let stats = unsafe { stats.assume_init() };
        (stats.f_bavail as u64)
            .checked_mul(stats.f_frsize as u64)
            .ok_or_else(|| error("UnknownCapacity"))
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn GetDiskFreeSpaceExW(
                path: *const u16,
                available: *mut u64,
                total: *mut u64,
                free: *mut u64,
            ) -> i32;
        }
        let wide = path
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<_>>();
        let mut available = 0u64;
        if unsafe {
            GetDiskFreeSpaceExW(
                wide.as_ptr(),
                &mut available,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        } == 0
        {
            return Err(error("UnknownCapacity"));
        }
        Ok(available)
    }
    #[cfg(not(any(unix, windows)))]
    {
        Err(error("UnknownCapacity"))
    }
}
pub fn capacity(
    workspace: &Path,
    local_daemon_root: Option<&Path>,
) -> Result<StorageCapacity, ToolboxError> {
    let workspace_free_bytes = available_space(workspace).ok();
    // A daemon directory is provided only after proving it belongs to a native
    // Linux daemon on this host; Desktop/OrbStack VM paths never use this branch.
    let daemon_free_bytes = local_daemon_root.and_then(|p| available_space(p).ok());
    Ok(StorageCapacity {
        workspace_free_bytes,
        daemon_free_bytes,
        daemon_status: if daemon_free_bytes.is_some() {
            "Known"
        } else {
            "UnknownCapacity"
        }
        .into(),
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn workspace_disk_does_not_stand_in_for_daemon() {
        let d = tempfile::tempdir().unwrap();
        let c = capacity(d.path(), None).unwrap();
        assert!(c.workspace_free_bytes.unwrap() > 0);
        assert_eq!(c.daemon_free_bytes, None);
        assert_eq!(c.daemon_status, "UnknownCapacity");
    }
    #[test]
    fn nonexistent_path_is_not_zero_capacity() {
        assert!(available_space(Path::new("/codeark-nonexistent-volume-7b1777")).is_err());
    }
}
