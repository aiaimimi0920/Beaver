//! Strict process ownership for object writers. Legacy RPC callers keep their
//! existing shutdown behavior. Unsupported platforms fail before spawning.
use tokio::process::{Child, Command};

#[cfg(windows)]
use std::{
    mem::size_of,
    os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
};
#[cfg(windows)]
use windows_sys::Win32::{
    Foundation::{
        ERROR_INVALID_PARAMETER, ERROR_MORE_DATA, INVALID_HANDLE_VALUE, WAIT_OBJECT_0, WAIT_TIMEOUT,
    },
    System::{
        Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Thread32First, Thread32Next, TH32CS_SNAPTHREAD, THREADENTRY32,
        },
        JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, JobObjectBasicAccountingInformation,
            JobObjectBasicProcessIdList, JobObjectExtendedLimitInformation,
            QueryInformationJobObject, SetInformationJobObject, TerminateJobObject,
            JOBOBJECT_BASIC_ACCOUNTING_INFORMATION, JOBOBJECT_BASIC_PROCESS_ID_LIST,
            JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_LIMIT_ACTIVE_PROCESS,
            JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        },
        Threading::{
            OpenProcess, OpenThread, ResumeThread, WaitForSingleObject, CREATE_NO_WINDOW,
            CREATE_SUSPENDED, PROCESS_SYNCHRONIZE, THREAD_SUSPEND_RESUME,
        },
    },
};

pub(crate) struct ProcessTree {
    #[cfg(windows)]
    job: OwnedHandle,
}

impl ProcessTree {
    pub(crate) fn new() -> Result<Self, String> {
        #[cfg(windows)]
        unsafe {
            let handle = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if handle.is_null() {
                return Err(os_error());
            }
            let job = OwnedHandle::from_raw_handle(handle);
            let limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION {
                BasicLimitInformation:
                    windows_sys::Win32::System::JobObjects::JOBOBJECT_BASIC_LIMIT_INFORMATION {
                        LimitFlags: JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
                        ..Default::default()
                    },
                ..Default::default()
            };
            if SetInformationJobObject(
                job.as_raw_handle(),
                JobObjectExtendedLimitInformation,
                &limits as *const _ as _,
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            ) == 0
            {
                return Err(os_error());
            }
            Ok(Self { job })
        }
        #[cfg(not(windows))]
        Err("OBJECT_ATTEMPT_PROCESS_TREE_UNSUPPORTED".into())
    }

    pub(crate) fn configure(&self, command: &mut Command) {
        #[cfg(windows)]
        command.creation_flags(CREATE_NO_WINDOW | CREATE_SUSPENDED);
        #[cfg(not(windows))]
        let _ = command;
    }

    pub(crate) fn attach(&self, child: &Child) -> Result<(), String> {
        #[cfg(windows)]
        unsafe {
            let process = child.raw_handle().ok_or("OBJECT_ATTEMPT_PROCESS_MISSING")?;
            if AssignProcessToJobObject(self.job.as_raw_handle(), process) == 0 {
                return Err(os_error());
            }
            // The primary thread has never run, so no descendant can escape
            // between spawn and assignment. Resume only after owning the tree.
            resume_primary(child.id().ok_or("OBJECT_ATTEMPT_PROCESS_MISSING")?)
        }
        #[cfg(not(windows))]
        {
            let _ = child;
            Err("OBJECT_ATTEMPT_PROCESS_TREE_UNSUPPORTED".into())
        }
    }

    pub(crate) async fn close(&self) -> Result<(), String> {
        #[cfg(windows)]
        {
            // ActiveProcesses can reach zero before process handles are signaled.
            // Stop new descendants, retain the current members, then wait for
            // their actual termination as well as the job accounting boundary.
            let processes = self.closing_processes()?;
            if unsafe { TerminateJobObject(self.job.as_raw_handle(), 1) } == 0 {
                return Err(os_error());
            }
            tokio::time::timeout(std::time::Duration::from_secs(5), async {
                loop {
                    let mut info = JOBOBJECT_BASIC_ACCOUNTING_INFORMATION::default();
                    if unsafe {
                        QueryInformationJobObject(
                            self.job.as_raw_handle(),
                            JobObjectBasicAccountingInformation,
                            &mut info as *mut _ as _,
                            size_of::<JOBOBJECT_BASIC_ACCOUNTING_INFORMATION>() as u32,
                            std::ptr::null_mut(),
                        )
                    } == 0
                    {
                        return Err(os_error());
                    }
                    let mut terminated = true;
                    for process in &processes {
                        match unsafe { WaitForSingleObject(process.as_raw_handle(), 0) } {
                            WAIT_OBJECT_0 => {}
                            WAIT_TIMEOUT => terminated = false,
                            _ => return Err(os_error()),
                        }
                    }
                    if info.ActiveProcesses == 0 && terminated {
                        return Ok(());
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                }
            })
            .await
            .map_err(|_| "OBJECT_ATTEMPT_PROCESS_TREE_CLOSE_TIMEOUT".to_string())?
        }
        #[cfg(not(windows))]
        Err("OBJECT_ATTEMPT_PROCESS_TREE_UNSUPPORTED".into())
    }

    #[cfg(windows)]
    fn closing_processes(&self) -> Result<Vec<OwnedHandle>, String> {
        unsafe {
            let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
            limits.BasicLimitInformation.LimitFlags =
                JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE | JOB_OBJECT_LIMIT_ACTIVE_PROCESS;
            // A live member already occupies this slot. Lowering the limit does
            // not kill existing members, but prevents every member from spawning.
            limits.BasicLimitInformation.ActiveProcessLimit = 1;
            if SetInformationJobObject(
                self.job.as_raw_handle(),
                JobObjectExtendedLimitInformation,
                &limits as *const _ as _,
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            ) == 0
            {
                return Err(os_error());
            }
            let mut buffer = vec![0usize; 64];
            loop {
                if QueryInformationJobObject(
                    self.job.as_raw_handle(),
                    JobObjectBasicProcessIdList,
                    buffer.as_mut_ptr() as _,
                    (buffer.len() * size_of::<usize>()) as u32,
                    std::ptr::null_mut(),
                ) != 0
                {
                    break;
                }
                if std::io::Error::last_os_error().raw_os_error() != Some(ERROR_MORE_DATA as i32) {
                    return Err(os_error());
                }
                buffer.resize(buffer.len() * 2, 0);
            }
            let list = &*(buffer.as_ptr() as *const JOBOBJECT_BASIC_PROCESS_ID_LIST);
            let ids = std::slice::from_raw_parts(
                list.ProcessIdList.as_ptr(),
                list.NumberOfProcessIdsInList as usize,
            );
            let mut processes = Vec::with_capacity(ids.len());
            for &pid in ids {
                let handle = OpenProcess(PROCESS_SYNCHRONIZE, 0, pid as u32);
                if handle.is_null() {
                    // A member may have fully exited while the snapshot was read.
                    if std::io::Error::last_os_error().raw_os_error()
                        != Some(ERROR_INVALID_PARAMETER as i32)
                    {
                        return Err(os_error());
                    }
                } else {
                    processes.push(OwnedHandle::from_raw_handle(handle));
                }
            }
            Ok(processes)
        }
    }
}

#[cfg(windows)]
fn os_error() -> String {
    format!(
        "OBJECT_ATTEMPT_PROCESS_TREE: {}",
        std::io::Error::last_os_error()
    )
}

#[cfg(windows)]
unsafe fn resume_primary(pid: u32) -> Result<(), String> {
    let handle = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
    if handle == INVALID_HANDLE_VALUE {
        return Err(os_error());
    }
    let snapshot = OwnedHandle::from_raw_handle(handle);
    let mut entry = THREADENTRY32 {
        dwSize: size_of::<THREADENTRY32>() as u32,
        ..Default::default()
    };
    let mut found = None;
    let mut present = Thread32First(snapshot.as_raw_handle(), &mut entry);
    while present != 0 {
        if entry.th32OwnerProcessID == pid {
            if found.is_some() {
                return Err("OBJECT_ATTEMPT_UNEXPECTED_STARTUP_THREADS".into());
            }
            found = Some(entry.th32ThreadID);
        }
        present = Thread32Next(snapshot.as_raw_handle(), &mut entry);
    }
    let id = found.ok_or("OBJECT_ATTEMPT_PRIMARY_THREAD_MISSING")?;
    let handle = OpenThread(THREAD_SUSPEND_RESUME, 0, id);
    if handle.is_null() {
        return Err(os_error());
    }
    let thread = OwnedHandle::from_raw_handle(handle);
    if ResumeThread(thread.as_raw_handle()) != 1 {
        return Err("OBJECT_ATTEMPT_PRIMARY_THREAD_RESUME_FAILED".into());
    }
    Ok(())
}
