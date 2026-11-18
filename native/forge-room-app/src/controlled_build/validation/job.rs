//! An unnamed non-inherited job owns the validation process tree after assignment.
//! Windows closes the owning handle on app termination even when Rust destructors cannot run.
use std::{
    os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
    process::Child,
};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
    SetInformationJobObject,
};
pub struct Guard(OwnedHandle);
impl Guard {
    pub fn new() -> Result<Self, String> {
        // Null attributes/name create a private, non-inheritable handle; no unrelated process is assigned.
        let raw = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
        if raw.is_null() {
            return Err(format!(
                "Validation could not create a Windows process job: {}",
                std::io::Error::last_os_error()
            ));
        }
        // SAFETY: CreateJobObjectW returned a live owned handle, released exactly once by OwnedHandle.
        let job = Self(unsafe { OwnedHandle::from_raw_handle(raw) });
        let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        // SAFETY: The initialized repr(C) structure and its exact size match this information class.
        let set = unsafe {
            SetInformationJobObject(
                job.0.as_raw_handle(),
                JobObjectExtendedLimitInformation,
                (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                std::mem::size_of_val(&limits) as u32,
            )
        };
        if set == 0 {
            return Err(format!(
                "Validation could not enable process ownership: {}",
                std::io::Error::last_os_error()
            ));
        }
        Ok(job)
    }
    pub fn assign(&self, child: &Child) -> Result<(), String> {
        // SAFETY: Both borrowed handles stay live throughout the call; Child is the process just spawned here.
        if unsafe { AssignProcessToJobObject(self.0.as_raw_handle(), child.as_raw_handle()) } == 0 {
            return Err(format!(
                "Validation could not own its Windows process: {}",
                std::io::Error::last_os_error()
            ));
        }
        Ok(())
    }
}
