use anyhow::{ensure, Result};
use std::{
    mem::size_of,
    os::windows::{
        io::{AsRawHandle, FromRawHandle, OwnedHandle},
        process::CommandExt,
    },
    process::{Child, Command},
    time::{Duration, Instant},
};
use windows_sys::Win32::{
    Foundation::{INVALID_HANDLE_VALUE, WAIT_OBJECT_0, WAIT_TIMEOUT},
    System::{
        Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Thread32First, Thread32Next, TH32CS_SNAPTHREAD, THREADENTRY32,
        },
        JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, JobObjectBasicAccountingInformation,
            JobObjectExtendedLimitInformation, QueryInformationJobObject, SetInformationJobObject,
            TerminateJobObject, JOBOBJECT_BASIC_ACCOUNTING_INFORMATION,
            JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        },
        Threading::{
            OpenThread, ResumeThread, WaitForSingleObject, CREATE_NO_WINDOW, CREATE_SUSPENDED,
            THREAD_SUSPEND_RESUME,
        },
    },
};

pub(super) struct Tree(OwnedHandle);
impl Tree {
    pub(super) fn new() -> Result<Self> {
        unsafe {
            let handle = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            ensure!(
                !handle.is_null(),
                "CreateJobObject failed: {}",
                std::io::Error::last_os_error()
            );
            let tree = Self(OwnedHandle::from_raw_handle(handle));
            let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
            limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            ensure!(
                SetInformationJobObject(
                    tree.0.as_raw_handle(),
                    JobObjectExtendedLimitInformation,
                    &limits as *const _ as _,
                    size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32
                ) != 0,
                "Configure owned Blender job failed"
            );
            Ok(tree)
        }
    }
    pub(super) fn configure(&self, command: &mut Command) {
        command.creation_flags(CREATE_NO_WINDOW | CREATE_SUSPENDED);
    }
    pub(super) fn attach(&self, child: &Child) -> Result<()> {
        unsafe {
            ensure!(
                AssignProcessToJobObject(self.0.as_raw_handle(), child.as_raw_handle()) != 0,
                "Could not own suspended Blender process"
            );
            let handle = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
            ensure!(
                handle != INVALID_HANDLE_VALUE,
                "Could not inspect suspended Blender thread"
            );
            let snapshot = OwnedHandle::from_raw_handle(handle);
            let mut entry = THREADENTRY32 {
                dwSize: size_of::<THREADENTRY32>() as u32,
                ..Default::default()
            };
            let mut primary = None;
            let mut present = Thread32First(snapshot.as_raw_handle(), &mut entry);
            while present != 0 {
                if entry.th32OwnerProcessID == child.id() {
                    ensure!(primary.is_none(), "Unexpected Blender startup thread count");
                    primary = Some(entry.th32ThreadID);
                }
                present = Thread32Next(snapshot.as_raw_handle(), &mut entry);
            }
            let handle = OpenThread(
                THREAD_SUSPEND_RESUME,
                0,
                primary.ok_or_else(|| anyhow::anyhow!("Blender primary thread missing"))?,
            );
            ensure!(!handle.is_null(), "Could not open Blender primary thread");
            let thread = OwnedHandle::from_raw_handle(handle);
            ensure!(
                ResumeThread(thread.as_raw_handle()) == 1,
                "Could not resume owned Blender"
            );
            Ok(())
        }
    }
    pub(super) fn exited(&self, child: &Child) -> Result<bool> {
        match unsafe { WaitForSingleObject(child.as_raw_handle(), 0) } {
            WAIT_OBJECT_0 => Ok(true),
            WAIT_TIMEOUT => Ok(false),
            _ => anyhow::bail!("Could not observe owned Blender process"),
        }
    }
    pub(super) fn close(&self) -> Result<()> {
        unsafe {
            ensure!(
                TerminateJobObject(self.0.as_raw_handle(), 1) != 0,
                "Could not stop owned Blender process tree"
            );
            let deadline = Instant::now() + Duration::from_secs(5);
            loop {
                let mut information = JOBOBJECT_BASIC_ACCOUNTING_INFORMATION::default();
                ensure!(
                    QueryInformationJobObject(
                        self.0.as_raw_handle(),
                        JobObjectBasicAccountingInformation,
                        &mut information as *mut _ as _,
                        size_of::<JOBOBJECT_BASIC_ACCOUNTING_INFORMATION>() as u32,
                        std::ptr::null_mut()
                    ) != 0,
                    "Could not verify owned Blender cleanup"
                );
                if information.ActiveProcesses == 0 {
                    return Ok(());
                }
                ensure!(Instant::now() < deadline, "Owned Blender cleanup timed out");
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    }
}
