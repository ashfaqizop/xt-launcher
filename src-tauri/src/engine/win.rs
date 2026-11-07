//! Native Windows calls the engine needs, so it never has to start helper programs
//! (PowerShell, reg.exe, taskkill.exe) in the background to get things done.
//!
//! - memory: installed and free RAM (GlobalMemoryStatusEx)
//! - display adapters: the graphics cards Device Manager lists (SetupAPI)
//! - GPU preference: Windows' per-app graphics setting (registry API)
//! - jobs: the game's process tree, so Stop ends everything it started (job objects)
//! - recycle: delete to the Recycle Bin (shell file operation)

use std::path::{Path, PathBuf};

/// Prefer `javaw.exe` (a windowed program, so no console ever appears) next to `java.exe`.
pub fn windowless_java(java: &Path) -> PathBuf {
    let is_java = java
        .file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.eq_ignore_ascii_case("java.exe"))
        .unwrap_or(false);
    if is_java {
        let javaw = java.with_file_name("javaw.exe");
        if javaw.is_file() {
            return javaw;
        }
    }
    java.to_path_buf()
}

#[cfg(windows)]
mod imp {
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;
    use windows_sys::core::GUID;
    use windows_sys::Win32::Devices::DeviceAndDriverInstallation::{
        SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInfo, SetupDiGetClassDevsW,
        SetupDiGetDeviceRegistryPropertyW, DIGCF_PRESENT, SPDRP_DEVICEDESC, SP_DEVINFO_DATA,
    };
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, TerminateJobObject,
    };
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegCreateKeyExW, RegDeleteKeyValueW, RegSetValueExW, HKEY,
        HKEY_CURRENT_USER, KEY_SET_VALUE, REG_OPTION_NON_VOLATILE, REG_SZ,
    };
    use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    use windows_sys::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE};
    use windows_sys::Win32::UI::Shell::{
        SHFileOperationW, FOF_ALLOWUNDO, FOF_NOCONFIRMATION, FOF_NOERRORUI, FOF_SILENT, FO_DELETE,
        SHFILEOPSTRUCTW,
    };

    /// The "Display adapters" device class.
    const DISPLAY_CLASS: GUID = GUID::from_u128(0x4d36e968_e325_11cd_bfc1_08002be10318);

    fn wide(text: &std::ffi::OsStr) -> Vec<u16> {
        text.encode_wide().chain(std::iter::once(0)).collect()
    }

    /// (installed, available) memory in bytes.
    pub fn memory() -> Option<(u64, u64)> {
        // SAFETY: MEMORYSTATUSEX is plain data; all-zero is a valid value and the length is set.
        let mut status: MEMORYSTATUSEX = unsafe { std::mem::zeroed() };
        status.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
        // SAFETY: `status` is a valid, writable MEMORYSTATUSEX.
        if unsafe { GlobalMemoryStatusEx(&mut status) } == 0 {
            return None;
        }
        Some((status.ullTotalPhys, status.ullAvailPhys))
    }

    /// Names of the display adapters that are present (what Device Manager shows).
    pub fn display_adapters() -> Vec<String> {
        let mut names = Vec::new();
        // SAFETY: standard SetupAPI enumeration; the set is destroyed before returning and
        // every buffer passed in is valid for the size given.
        unsafe {
            let set = SetupDiGetClassDevsW(
                &DISPLAY_CLASS,
                std::ptr::null(),
                std::ptr::null_mut(),
                DIGCF_PRESENT,
            );
            if set as isize == -1 || set as isize == 0 {
                return names;
            }
            let mut index = 0u32;
            loop {
                let mut device: SP_DEVINFO_DATA = std::mem::zeroed();
                device.cbSize = std::mem::size_of::<SP_DEVINFO_DATA>() as u32;
                if SetupDiEnumDeviceInfo(set, index, &mut device) == 0 {
                    break;
                }
                index += 1;

                let mut buffer = [0u16; 512];
                let mut needed = 0u32;
                let ok = SetupDiGetDeviceRegistryPropertyW(
                    set,
                    &device,
                    SPDRP_DEVICEDESC,
                    std::ptr::null_mut(),
                    buffer.as_mut_ptr() as *mut u8,
                    (buffer.len() * 2) as u32,
                    &mut needed,
                );
                if ok != 0 {
                    let len = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
                    names.push(String::from_utf16_lossy(&buffer[..len]));
                }
                if index > 64 {
                    break;
                }
            }
            SetupDiDestroyDeviceInfoList(set);
        }
        names
    }

    /// Windows' per-app graphics preference (Settings > Display > Graphics) for `exe`:
    /// Some("GpuPreference=2;") for high performance, Some("GpuPreference=1;") for power
    /// saving, None to let Windows decide.
    pub fn set_gpu_preference(exe: &Path, value: Option<&str>) {
        let key = wide(std::ffi::OsStr::new(r"Software\Microsoft\DirectX\UserGpuPreferences"));
        let name = wide(exe.as_os_str());
        // SAFETY: all strings are NUL-terminated UTF-16 that outlive the calls; the key handle
        // is closed after use.
        unsafe {
            match value {
                Some(v) => {
                    let mut hkey: HKEY = std::ptr::null_mut();
                    let created = RegCreateKeyExW(
                        HKEY_CURRENT_USER,
                        key.as_ptr(),
                        0,
                        std::ptr::null(),
                        REG_OPTION_NON_VOLATILE,
                        KEY_SET_VALUE,
                        std::ptr::null(),
                        &mut hkey,
                        std::ptr::null_mut(),
                    );
                    if created == 0 {
                        let data = wide(std::ffi::OsStr::new(v));
                        let _ = RegSetValueExW(
                            hkey,
                            name.as_ptr(),
                            0,
                            REG_SZ,
                            data.as_ptr() as *const u8,
                            (data.len() * 2) as u32,
                        );
                        let _ = RegCloseKey(hkey);
                    }
                }
                None => {
                    let _ = RegDeleteKeyValueW(HKEY_CURRENT_USER, key.as_ptr(), name.as_ptr());
                }
            }
        }
    }

    /// A job object holding the game's process; ending the job ends every process the
    /// game started too. Closing it (on drop) does NOT end the game.
    pub struct Job(usize);

    impl Job {
        pub fn adopt(process: HANDLE) -> Option<Job> {
            // SAFETY: `process` is a live process handle owned by the caller; the job handle
            // is owned by the returned value and closed on drop.
            unsafe {
                let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
                if job.is_null() {
                    return None;
                }
                if AssignProcessToJobObject(job, process) == 0 {
                    CloseHandle(job);
                    return None;
                }
                Some(Job(job as usize))
            }
        }

        pub fn terminate(&self) -> bool {
            // SAFETY: the handle is valid until drop.
            unsafe { TerminateJobObject(self.0 as HANDLE, 1) != 0 }
        }
    }

    impl Drop for Job {
        fn drop(&mut self) {
            // SAFETY: we own this handle and close it exactly once.
            unsafe {
                CloseHandle(self.0 as HANDLE);
            }
        }
    }

    /// End one process by id (fallback when there is no job).
    pub fn terminate_pid(pid: u32) -> bool {
        // SAFETY: the handle is checked and closed.
        unsafe {
            let process = OpenProcess(PROCESS_TERMINATE, 0, pid);
            if process.is_null() {
                return false;
            }
            let ok = TerminateProcess(process, 1) != 0;
            CloseHandle(process);
            ok
        }
    }

    /// Move a file to the Recycle Bin. True when it was moved.
    pub fn recycle(path: &Path) -> bool {
        // The source list is double NUL-terminated
        let mut from: Vec<u16> = path.as_os_str().encode_wide().collect();
        from.push(0);
        from.push(0);
        // SAFETY: SHFILEOPSTRUCTW is plain data; zero is valid for every field we don't set,
        // and `from` outlives the call.
        unsafe {
            let mut op: SHFILEOPSTRUCTW = std::mem::zeroed();
            op.wFunc = FO_DELETE as _;
            op.pFrom = from.as_ptr();
            op.fFlags = (FOF_ALLOWUNDO | FOF_NOCONFIRMATION | FOF_SILENT | FOF_NOERRORUI) as _;
            SHFileOperationW(&mut op) == 0 && op.fAnyOperationsAborted == 0
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use std::path::Path;

    pub fn memory() -> Option<(u64, u64)> {
        None
    }
    pub fn display_adapters() -> Vec<String> {
        Vec::new()
    }
    pub fn set_gpu_preference(_exe: &Path, _value: Option<&str>) {}
    pub struct Job;
    impl Job {
        pub fn terminate(&self) -> bool {
            false
        }
    }
    pub fn terminate_pid(_pid: u32) -> bool {
        false
    }
    pub fn recycle(_path: &Path) -> bool {
        false
    }
}

pub use imp::*;

/// Put a freshly started child process into its own job (Windows only).
#[cfg(windows)]
pub fn job_for(child: &tokio::process::Child) -> Option<Job> {
    let handle = child.raw_handle()?;
    Job::adopt(handle as windows_sys::Win32::Foundation::HANDLE)
}

#[cfg(not(windows))]
pub fn job_for(_child: &tokio::process::Child) -> Option<Job> {
    None
}
