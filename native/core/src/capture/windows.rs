use anyhow::{bail, ensure, Context, Result};
use base64::Engine;
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use windows::{
    core::{factory, IInspectable, Interface, BOOL},
    Foundation::TypedEventHandler,
    Graphics::{
        Capture::{Direct3D11CaptureFramePool, GraphicsCaptureItem, GraphicsCaptureSession},
        DirectX::{Direct3D11::IDirect3DDevice, DirectXPixelFormat},
    },
    Win32::{
        Foundation::{E_POINTER, HMODULE, HWND, LPARAM},
        Graphics::{
            Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_WARP},
            Direct3D11::{
                D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D,
                D3D11_CPU_ACCESS_READ, D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_MAPPED_SUBRESOURCE,
                D3D11_MAP_READ, D3D11_SDK_VERSION, D3D11_TEXTURE2D_DESC, D3D11_USAGE_STAGING,
            },
            Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED},
            Dxgi::IDXGIDevice,
        },
        System::WinRT::{
            Direct3D11::{CreateDirect3D11DeviceFromDXGIDevice, IDirect3DDxgiInterfaceAccess},
            Graphics::Capture::IGraphicsCaptureItemInterop,
            RoInitialize, RoUninitialize, RO_INIT_MULTITHREADED,
        },
        UI::WindowsAndMessaging::{
            EnumWindows, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId, IsIconic,
            IsWindow, IsWindowVisible,
        },
    },
};

struct Apartment;
impl Apartment {
    fn new() -> Result<Self> {
        unsafe {
            RoInitialize(RO_INIT_MULTITHREADED)?;
        }
        Ok(Self)
    }
}
impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe {
            RoUninitialize();
        }
    }
}

struct Source {
    window: usize,
    process: u32,
    item: GraphicsCaptureItem,
    closed: Arc<AtomicBool>,
    token: i64,
}
impl Source {
    fn check(&self, cancelled: &AtomicBool) -> Result<()> {
        ensure!(!cancelled.load(Ordering::SeqCst), "截图已取消");
        let hwnd = HWND(self.window as _);
        let mut process = 0;
        unsafe {
            GetWindowThreadProcessId(hwnd, Some(&mut process));
        }
        ensure!(
            !self.closed.load(Ordering::SeqCst)
                && unsafe { IsWindow(Some(hwnd)) }.as_bool()
                && process == self.process,
            "目标窗口已关闭，请重新选择"
        );
        ensure!(
            !unsafe { IsIconic(hwnd) }.as_bool(),
            "目标窗口已最小化，请还原后截图"
        );
        Ok(())
    }
}
impl Drop for Source {
    fn drop(&mut self) {
        let _ = self.item.RemoveClosed(self.token);
    }
}

#[derive(Default)]
pub struct Capture {
    sources: BTreeMap<String, Source>,
}
impl Capture {
    pub fn sources(&mut self, cancelled: &AtomicBool) -> Result<Value> {
        ensure!(!cancelled.load(Ordering::SeqCst), "截图已取消");
        let _apartment = Apartment::new()?;
        ensure!(
            GraphicsCaptureSession::IsSupported().unwrap_or(false),
            "此系统不支持原生窗口采集，请使用 Windows 10 1903 或更新系统"
        );
        let device = Device::new()?;
        let interop: IGraphicsCaptureItemInterop = factory::<GraphicsCaptureItem, _>()?;
        let mut windows = Vec::<(usize, u32, String)>::new();
        unsafe {
            EnumWindows(Some(enumerate), LPARAM(&mut windows as *mut _ as isize))?;
        }
        let mut next = BTreeMap::new();
        let mut result = Vec::new();
        for (window, process, name) in windows {
            ensure!(!cancelled.load(Ordering::SeqCst), "截图已取消");
            let Ok(item) =
                (unsafe { interop.CreateForWindow::<GraphicsCaptureItem>(HWND(window as _)) })
            else {
                continue;
            };
            let closed = Arc::new(AtomicBool::new(false));
            let flag = closed.clone();
            let token = item.Closed(
                &TypedEventHandler::<GraphicsCaptureItem, IInspectable>::new(move |_, _| {
                    flag.store(true, Ordering::SeqCst);
                    Ok(())
                }),
            )?;
            let source = Source {
                window,
                process,
                item,
                closed,
                token,
            };
            let Ok(png) =
                device.capture(&source, (640, 360), Duration::from_millis(700), cancelled)
            else {
                continue;
            };
            let id = uuid::Uuid::new_v4().to_string();
            result.push(json!({"id":id, "name":name, "image":format!("data:image/png;base64,{}",base64::engine::general_purpose::STANDARD.encode(png))}));
            next.insert(id, source);
        }
        ensure!(!cancelled.load(Ordering::SeqCst), "截图已取消");
        // Keep the capture item, not just an HWND which Windows can recycle.
        self.sources = next;
        Ok(json!(result))
    }

    pub fn capture(&self, id: &str, cancelled: &AtomicBool) -> Result<Vec<u8>> {
        let _apartment = Apartment::new()?;
        let source = self
            .sources
            .get(id)
            .context("目标窗口已关闭或选择已过期，请重新选择")?;
        source.check(cancelled)?;
        Device::new()?.capture(source, (1920, 1080), Duration::from_secs(3), cancelled)
    }
}

unsafe extern "system" fn enumerate(hwnd: HWND, data: LPARAM) -> BOOL {
    if !unsafe { IsWindowVisible(hwnd) }.as_bool() || unsafe { IsIconic(hwnd) }.as_bool() {
        return BOOL(1);
    }
    let length = unsafe { GetWindowTextLengthW(hwnd) };
    if length <= 0 || length > 32767 {
        return BOOL(1);
    }
    let mut cloaked: u32 = 0;
    let _ = unsafe { DwmGetWindowAttribute(hwnd, DWMWA_CLOAKED, &mut cloaked as *mut _ as _, 4) };
    if cloaked != 0 {
        return BOOL(1);
    }
    let mut title = vec![0u16; length as usize + 1];
    let count = unsafe { GetWindowTextW(hwnd, &mut title) };
    let mut process = 0;
    unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut process));
    }
    if count > 0 && process != 0 {
        let windows = unsafe { &mut *(data.0 as *mut Vec<(usize, u32, String)>) };
        windows.push((
            hwnd.0 as usize,
            process,
            String::from_utf16_lossy(&title[..count as usize]),
        ));
    }
    BOOL(1)
}

struct Device {
    device: ID3D11Device,
    context: ID3D11DeviceContext,
    runtime: IDirect3DDevice,
}
impl Device {
    fn new() -> Result<Self> {
        let mut device = None;
        let mut context = None;
        for driver in [D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_WARP] {
            if unsafe {
                D3D11CreateDevice(
                    None,
                    driver,
                    HMODULE::default(),
                    D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                    None,
                    D3D11_SDK_VERSION,
                    Some(&mut device),
                    None,
                    Some(&mut context),
                )
            }
            .is_ok()
            {
                break;
            }
        }
        let device = device.context("无法创建设备进行窗口采集")?;
        let context = context.context("无法创建窗口采集上下文")?;
        let dxgi: IDXGIDevice = device.cast()?;
        let runtime = unsafe { CreateDirect3D11DeviceFromDXGIDevice(&dxgi)? }.cast()?;
        Ok(Self {
            device,
            context,
            runtime,
        })
    }

    fn capture(
        &self,
        source: &Source,
        bounds: (u32, u32),
        timeout: Duration,
        cancelled: &AtomicBool,
    ) -> Result<Vec<u8>> {
        source.check(cancelled)?;
        let size = source.item.Size()?;
        super::dimensions(u32::try_from(size.Width)?, u32::try_from(size.Height)?)?;
        let pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
            &self.runtime,
            DirectXPixelFormat::B8G8R8A8UIntNormalized,
            2,
            size,
        )?;
        let session = match pool.CreateCaptureSession(&source.item) {
            Ok(session) => session,
            Err(error) => {
                let _ = pool.Close();
                return Err(error.into());
            }
        };
        let capture = Session { session, pool };
        let _ = capture.session.SetIsCursorCaptureEnabled(false);
        capture.session.StartCapture()?;
        let deadline = Instant::now() + timeout;
        loop {
            source.check(cancelled)?;
            match capture.pool.TryGetNextFrame() {
                Ok(frame) => {
                    let result = (|| -> Result<Vec<u8>> {
                        let size = frame.ContentSize()?;
                        let texture: ID3D11Texture2D = unsafe {
                            frame
                                .Surface()?
                                .cast::<IDirect3DDxgiInterfaceAccess>()?
                                .GetInterface()?
                        };
                        self.png(
                            &texture,
                            u32::try_from(size.Width)?,
                            u32::try_from(size.Height)?,
                            bounds,
                        )
                    })();
                    let _ = frame.Close();
                    source.check(cancelled)?;
                    return result;
                }
                // A null frame is projected as an error with S_OK by windows-rs.
                Err(error) if error.code().0 == 0 || error.code() == E_POINTER => {}
                Err(error) => return Err(error).context("窗口画面采集失败"),
            }
            if Instant::now() >= deadline {
                bail!("窗口未提供新画面，请确认窗口可见后重试");
            }
            std::thread::sleep(Duration::from_millis(15));
        }
    }

    fn png(
        &self,
        source: &ID3D11Texture2D,
        width: u32,
        height: u32,
        bounds: (u32, u32),
    ) -> Result<Vec<u8>> {
        super::dimensions(width, height)?;
        let mut desc = D3D11_TEXTURE2D_DESC::default();
        unsafe {
            source.GetDesc(&mut desc);
        }
        ensure!(
            width <= desc.Width && height <= desc.Height,
            "窗口尺寸正在变化，请重试截图"
        );
        desc.Usage = D3D11_USAGE_STAGING;
        desc.BindFlags = 0;
        desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
        desc.MiscFlags = 0;
        let mut texture = None;
        unsafe {
            self.device
                .CreateTexture2D(&desc, None, Some(&mut texture))?;
        }
        let texture = texture.context("无法读取截图像素")?;
        unsafe {
            self.context.CopyResource(&texture, source);
        }
        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        unsafe {
            self.context
                .Map(&texture, 0, D3D11_MAP_READ, 0, Some(&mut mapped))?;
        }
        let result = (|| -> Result<Vec<u8>> {
            ensure!(
                !mapped.pData.is_null() && mapped.RowPitch as usize >= width as usize * 4,
                "截图映射数据无效"
            );
            let length = (mapped.RowPitch as usize)
                .checked_mul(height as usize - 1)
                .and_then(|n| n.checked_add(width as usize * 4))
                .context("截图映射长度溢出")?;
            let bytes = unsafe { std::slice::from_raw_parts(mapped.pData as *const u8, length) };
            super::encode_bgra(width, height, mapped.RowPitch as usize, bytes, bounds)
        })();
        unsafe {
            self.context.Unmap(&texture, 0);
        }
        result
    }
}

struct Session {
    session: GraphicsCaptureSession,
    pool: Direct3D11CaptureFramePool,
}
impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.session.Close();
        let _ = self.pool.Close();
    }
}
