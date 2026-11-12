//! Windows workspace writes fail closed unless a protected current-user/SYSTEM DACL is applied.
#![cfg(windows)]
use crate::{contracts::ToolboxError, workspace::error};
use std::{ffi::c_void, os::windows::ffi::OsStrExt, path::Path};
#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetCurrentProcess() -> *mut c_void;
    fn CloseHandle(handle: *mut c_void) -> i32;
    fn LocalFree(memory: *mut c_void) -> *mut c_void;
}
#[link(name = "advapi32")]
unsafe extern "system" {
    fn OpenProcessToken(process: *mut c_void, access: u32, token: *mut *mut c_void) -> i32;
    fn GetTokenInformation(
        token: *mut c_void,
        class: u32,
        info: *mut c_void,
        len: u32,
        needed: *mut u32,
    ) -> i32;
    fn ConvertSidToStringSidW(sid: *const c_void, text: *mut *mut u16) -> i32;
    fn ConvertStringSecurityDescriptorToSecurityDescriptorW(
        text: *const u16,
        revision: u32,
        descriptor: *mut *mut c_void,
        size: *mut u32,
    ) -> i32;
    fn SetFileSecurityW(file: *const u16, information: u32, descriptor: *const c_void) -> i32;
}
struct Handle(*mut c_void);
impl Drop for Handle {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}
struct Local(*mut c_void);
impl Drop for Local {
    fn drop(&mut self) {
        unsafe {
            LocalFree(self.0);
        }
    }
}
pub(crate) fn restrict(path: &Path) -> Result<(), ToolboxError> {
    let mut token = std::ptr::null_mut();
    if unsafe { OpenProcessToken(GetCurrentProcess(), 8, &mut token) } == 0 {
        return Err(error("PermissionDenied"));
    }
    let token = Handle(token);
    let mut needed = 0;
    // TokenUser is a TOKEN_USER beginning with an aligned SID pointer. The returned SID lives in info.
    unsafe {
        GetTokenInformation(token.0, 1, std::ptr::null_mut(), 0, &mut needed);
    }
    if needed == 0 || needed > 1024 * 1024 {
        return Err(error("PermissionDenied"));
    }
    let mut info = vec![0usize; (needed as usize).div_ceil(std::mem::size_of::<usize>())];
    if unsafe { GetTokenInformation(token.0, 1, info.as_mut_ptr().cast(), needed, &mut needed) }
        == 0
    {
        return Err(error("PermissionDenied"));
    }
    let sid = info[0] as *const c_void;
    if sid.is_null() {
        return Err(error("PermissionDenied"));
    }
    let mut sid_text = std::ptr::null_mut();
    if unsafe { ConvertSidToStringSidW(sid, &mut sid_text) } == 0 {
        return Err(error("PermissionDenied"));
    }
    let _sid_memory = Local(sid_text.cast());
    let mut length = 0;
    while length < 256 && unsafe { *sid_text.add(length) } != 0 {
        length += 1;
    }
    if length == 256 {
        return Err(error("PermissionDenied"));
    }
    let sid = String::from_utf16(unsafe { std::slice::from_raw_parts(sid_text, length) })
        .map_err(|_| error("PermissionDenied"))?;
    let sddl = format!("D:P(A;OICI;FA;;;{sid})(A;OICI;FA;;;SY)")
        .encode_utf16()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let mut descriptor = std::ptr::null_mut();
    if unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(),
            1,
            &mut descriptor,
            std::ptr::null_mut(),
        )
    } == 0
    {
        return Err(error("PermissionDenied"));
    }
    let _descriptor = Local(descriptor);
    let file = path
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    if unsafe { SetFileSecurityW(file.as_ptr(), 0x80000004, descriptor) } == 0 {
        return Err(error("PermissionDenied"));
    }
    Ok(())
}
pub(crate) fn file_identity(path: &Path) -> Result<String, ToolboxError> {
    use std::os::windows::{fs::OpenOptionsExt, io::AsRawHandle};
    #[repr(C)]
    struct Info {
        attributes: u32,
        creation: [u32; 2],
        access: [u32; 2],
        write: [u32; 2],
        volume: u32,
        size_high: u32,
        size_low: u32,
        links: u32,
        index_high: u32,
        index_low: u32,
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetFileInformationByHandle(handle: *mut c_void, info: *mut Info) -> i32;
    }
    let file = std::fs::OpenOptions::new()
        .access_mode(0x80)
        .custom_flags(0x02000000)
        .open(path)
        .map_err(|_| error("PermissionDenied"))?;
    let mut info = std::mem::MaybeUninit::<Info>::uninit();
    if unsafe { GetFileInformationByHandle(file.as_raw_handle().cast(), info.as_mut_ptr()) } == 0 {
        return Err(error("PermissionDenied"));
    }
    let info = unsafe { info.assume_init() };
    Ok(format!(
        "{}:{}:{}",
        info.volume, info.index_high, info.index_low
    ))
}
