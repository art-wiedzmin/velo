#![cfg(windows)]

use super::runner::RunError;
use tokio::process::Child;
use win32job::{ExtendedLimitInfo, Job};

pub(super) fn attach_to_job(child: &Child) -> Result<Job, RunError> {
    let mut info = ExtendedLimitInfo::new();
    // KILL_ON_JOB_CLOSE: when our process exits and the last handle to the
    // job is released, every child in the job is terminated. That's the
    // whole reason this exists.
    info.limit_kill_on_job_close();
    let job = Job::create_with_limit_info(&mut info).map_err(|e| RunError::Job(e.to_string()))?;

    let pid = child.id().ok_or_else(|| RunError::Job("child has no pid".into()))?;
    let handle = open_process_handle(pid)?;
    // SAFETY: `handle` is a valid process HANDLE returned by OpenProcess
    // above; `assign_process` takes ownership semantics by duplicating it
    // into the job. We close ours unconditionally in the guard.
    let result = unsafe { assign_raw_handle(&job, handle) };
    close_handle(handle);
    result.map_err(RunError::Job)?;
    Ok(job)
}

pub(super) fn open_process_handle(pid: u32) -> Result<windows_sys::Win32::Foundation::HANDLE, RunError> {
    use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_ALL_ACCESS};
    // Using ALL_ACCESS because AssignProcessToJobObject wants
    // PROCESS_TERMINATE | PROCESS_SET_QUOTA and the exact set varies by
    // Windows edition; ALL_ACCESS is always sufficient for our own child.
    let h = unsafe { OpenProcess(PROCESS_ALL_ACCESS, 0, pid) };
    if h.is_null() {
        Err(RunError::Job(format!(
            "OpenProcess({pid}) failed: {}",
            std::io::Error::last_os_error()
        )))
    } else {
        Ok(h)
    }
}

pub(super) fn close_handle(handle: windows_sys::Win32::Foundation::HANDLE) {
    use windows_sys::Win32::Foundation::CloseHandle;
    unsafe {
        let _ = CloseHandle(handle);
    }
}

pub(super) unsafe fn assign_raw_handle(
    job: &Job,
    handle: windows_sys::Win32::Foundation::HANDLE,
) -> Result<(), String> {
    use windows_sys::Win32::System::JobObjects::AssignProcessToJobObject;
    let job_handle = job.handle() as windows_sys::Win32::Foundation::HANDLE;
    let ok = AssignProcessToJobObject(job_handle, handle);
    if ok == 0 {
        Err(format!(
            "AssignProcessToJobObject: {}",
            std::io::Error::last_os_error()
        ))
    } else {
        Ok(())
    }
}
