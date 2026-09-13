use anyhow::{Context, Result};
use fs2::FileExt;
use std::{
    fs::{File, OpenOptions},
    path::Path,
};

pub struct Instance {
    // Stop the listener before releasing the data directory lock.
    #[cfg(windows)]
    activation: windows::Activation,
    _lock: File,
}

impl Instance {
    pub fn acquire(root: &Path) -> Result<Option<Self>> {
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(root.join(".beaver-native.lock"))?;
        match lock.try_lock_exclusive() {
            Ok(()) => Ok(Some(Self {
                #[cfg(windows)]
                activation: windows::Activation::new(root)?,
                _lock: lock,
            })),
            Err(error) if error.raw_os_error() == fs2::lock_contended_error().raw_os_error() => {
                #[cfg(windows)]
                {
                    windows::activate(root)?;
                    Ok(None)
                }
                #[cfg(not(windows))]
                Err(error).context("另一个 Beaver 实例正在使用此数据目录")
            }
            Err(error) => Err(error).context("无法锁定 Beaver 数据目录"),
        }
    }

    pub fn listen(&mut self, notify: impl Fn() + Send + 'static) -> Result<()> {
        #[cfg(windows)]
        self.activation.listen(notify)?;
        #[cfg(not(windows))]
        let _ = notify;
        Ok(())
    }
}

#[cfg(windows)]
mod windows {
    use super::*;
    use sha2::{Digest, Sha256};
    use std::{
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
        },
        thread,
        time::{Duration, Instant},
    };
    use windows_sys::Win32::{
        Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0},
        System::Threading::{
            CreateEventW, OpenEventW, SetEvent, WaitForSingleObject, EVENT_MODIFY_STATE, INFINITE,
        },
    };

    fn name(root: &Path) -> Result<Vec<u16>> {
        let root = std::fs::canonicalize(root)?;
        let digest = Sha256::digest(root.as_os_str().to_string_lossy().to_lowercase().as_bytes());
        Ok(format!("Local\\Beaver.Native.Activate.{digest:x}")
            .encode_utf16()
            .chain(Some(0))
            .collect())
    }

    pub fn activate(root: &Path) -> Result<()> {
        let name = name(root)?;
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            let handle = unsafe { OpenEventW(EVENT_MODIFY_STATE, 0, name.as_ptr()) };
            if !handle.is_null() {
                let result = unsafe { SetEvent(handle) };
                let error = std::io::Error::last_os_error();
                unsafe {
                    CloseHandle(handle);
                }
                anyhow::ensure!(result != 0, "无法唤醒 Beaver: {error}");
                return Ok(());
            }
            if Instant::now() >= deadline {
                anyhow::bail!("此数据目录已被占用，无法唤醒现有 Beaver 窗口");
            }
            thread::sleep(Duration::from_millis(25));
        }
    }

    pub struct Activation {
        handle: usize,
        stopped: Arc<AtomicBool>,
        listener: Option<thread::JoinHandle<()>>,
    }
    impl Activation {
        pub fn new(root: &Path) -> Result<Self> {
            let name = name(root)?;
            let handle = unsafe { CreateEventW(std::ptr::null(), 0, 0, name.as_ptr()) };
            anyhow::ensure!(
                !handle.is_null(),
                "无法创建 Beaver 激活事件: {}",
                std::io::Error::last_os_error()
            );
            Ok(Self {
                handle: handle as usize,
                stopped: Arc::new(AtomicBool::new(false)),
                listener: None,
            })
        }
        pub fn listen(&mut self, notify: impl Fn() + Send + 'static) -> Result<()> {
            anyhow::ensure!(self.listener.is_none(), "Beaver 激活监听已启动");
            let handle = self.handle;
            let stopped = self.stopped.clone();
            self.listener = Some(
                thread::Builder::new()
                    .name("beaver-activation".into())
                    .spawn(move || loop {
                        let result = unsafe { WaitForSingleObject(handle as HANDLE, INFINITE) };
                        if stopped.load(Ordering::SeqCst) || result != WAIT_OBJECT_0 {
                            break;
                        }
                        notify();
                    })?,
            );
            Ok(())
        }
    }
    impl Drop for Activation {
        fn drop(&mut self) {
            self.stopped.store(true, Ordering::SeqCst);
            unsafe {
                SetEvent(self.handle as HANDLE);
            }
            if let Some(listener) = self.listener.take() {
                let _ = listener.join();
            }
            unsafe {
                CloseHandle(self.handle as HANDLE);
            }
        }
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use std::{sync::mpsc, time::Duration};

    #[test]
    fn same_directory_activates_primary_without_reopening_store() -> Result<()> {
        let root = tempfile::tempdir()?;
        let other = tempfile::tempdir()?;
        let mut first = Instance::acquire(root.path())?.context("first instance")?;
        // A launch during startup is retained until the listener is ready.
        assert!(Instance::acquire(root.path())?.is_none());
        let (send, receive) = mpsc::channel();
        first.listen(move || {
            let _ = send.send(());
        })?;
        receive.recv_timeout(Duration::from_secs(3))?;
        assert!(Instance::acquire(other.path())?.is_some());
        assert!(receive.recv_timeout(Duration::from_millis(50)).is_err());
        assert!(Instance::acquire(root.path().join(".").as_path())?.is_none());
        receive.recv_timeout(Duration::from_secs(3))?;
        drop(first);
        assert!(Instance::acquire(root.path())?.is_some());
        Ok(())
    }
}
