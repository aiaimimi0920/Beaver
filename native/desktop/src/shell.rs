use crate::{
    asset_protocol, business_api, instance, setup_runtime, task_runtime, template_runtime, Backend,
};
use beaver_core::{files::Files, journal::Journal, store::Store};
use serde_json::{json, Value};
use std::{
    fs,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Emitter, Manager,
};

#[tauri::command]
fn beaver_window(window: tauri::WebviewWindow, command: String) -> Result<Value, String> {
    let action = match command.as_str() {
        "state" => Ok(()),
        "minimize" => window.minimize(),
        "startDragging" => window.start_dragging(),
        "toggleMaximize" => window.is_maximized().and_then(|maximized| {
            if maximized {
                window.unmaximize()
            } else {
                window.maximize()
            }
        }),
        "close" if window.label() == "main" => window.hide(),
        "close" => {
            window.close().map_err(|e| e.to_string())?;
            return Ok(Value::Null);
        }
        _ => return Err("未知窗口操作".into()),
    };
    action.map_err(|e| e.to_string())?;
    Ok(json!({"maximized":window.is_maximized().map_err(|e| e.to_string())?}))
}

pub(crate) fn run() {
    tauri::Builder::default()
        .register_asynchronous_uri_scheme_protocol("beaver-asset", |context, request, responder| {
            let backend = context.app_handle().state::<Arc<Backend>>().inner().clone();
            asset_protocol::serve(
                backend,
                context.webview_label().to_owned(),
                request,
                responder,
            );
        })
        .setup(|app| {
            // Native preview never opens the Electron data root implicitly.
            let root = std::env::var_os("BEAVER_DATA_DIR")
                .map(PathBuf::from)
                .unwrap_or(app.path().app_data_dir()?);
            beaver_core::migration_bundle::ensure_activated(&root)?;
            fs::create_dir_all(&root)?;
            let Some(mut instance) = instance::Instance::acquire(&root)? else {
                std::process::exit(0);
            };
            let handle = app.handle().clone();
            instance.listen(move || {
                let app = handle.clone();
                let _ = handle.run_on_main_thread(move || show_main(&app));
            })?;
            let mut store = Store::open(&root)?;
            Journal::new(&mut store, &Files::new(root.clone())).recover()?;
            store.recover_tasks()?;
            beaver_core::validation::repository::recover(&store)?;
            setup_runtime::recover(&store)?;
            let store = Arc::new(Mutex::new(store));
            let scheduler = tauri::async_runtime::block_on(async {
                task_runtime::start(app.handle().clone(), &root, store.clone())
            })?;
            let validation = crate::validation_runtime::start(
                app.handle().clone(),
                &root,
                store.clone(),
                scheduler.clone(),
            )?;
            let backend = Arc::new(Backend {
                store,
                scheduler,
                validation,
                players: beaver_core::game_play::Players::default(),
                root,
                closing: AtomicBool::new(false),
                api_calls: Arc::new(tokio::sync::Semaphore::new(
                    business_api::MAX_CALLS as usize,
                )),
                operations: Mutex::new(()),
                setup: beaver_core::tool_setup::Setup::default(),
                setup_gate: Mutex::new(()),
                template_gate: Mutex::new(()),
                template_cancel: Mutex::new(None),
                captures: Mutex::new(beaver_core::capture::Capture::default()),
                _instance: instance,
            });
            business_api::start(app.handle().clone(), backend.clone())?;
            app.manage(backend);
            let show = MenuItem::with_id(app, "show", "打开 Beaver", true, None::<&str>)?;
            let exit = MenuItem::with_id(app, "exit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &exit])?;
            let icon = app.default_window_icon().ok_or("missing app icon")?.clone();
            TrayIconBuilder::new()
                .icon(icon)
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        show_main(app);
                    }
                    "exit" => {
                        let backend = app.state::<Arc<Backend>>().inner().clone();
                        if !backend.closing.swap(true, Ordering::SeqCst) {
                            let app = app.clone();
                            tauri::async_runtime::spawn(async move {
                                let _ = backend.setup.cancel();
                                let _ = template_runtime::cancel(&backend);
                                if let Err(error) = backend.players.shutdown().await {
                                    let _ = app.emit("beaver:shutdown-error", error.to_string());
                                }
                                if let Err(error) = backend.scheduler.shutdown().await {
                                    let _ = app.emit("beaver:shutdown-error", error);
                                }
                                let _api_calls = backend
                                    .api_calls
                                    .clone()
                                    .acquire_many_owned(business_api::MAX_CALLS)
                                    .await;
                                tauri::async_runtime::spawn_blocking(move || {
                                    if let Err(error) = backend.validation.shutdown() {
                                        let _ =
                                            app.emit("beaver:shutdown-error", error.to_string());
                                    }
                                    let _setup = backend.setup_gate.lock();
                                    let _templates = backend.template_gate.lock();
                                    let _captures = backend.captures.lock();
                                    let _operations = backend.operations.lock();
                                    let _guard = backend.store.lock();
                                    app.exit(0);
                                })
                                .await
                                .ok();
                            });
                        }
                    }
                    _ => {}
                })
                .build(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![crate::beaver_call, beaver_window])
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } if window.label() == "main" => {
                api.prevent_close();
                let _ = window.hide();
            }
            tauri::WindowEvent::Destroyed => {
                crate::asset_task_windows::release(window.app_handle(), window.label());
            }
            tauri::WindowEvent::Resized(_) => {
                let _ = window.emit(
                    "beaver:window-state",
                    json!({"maximized":window.is_maximized().unwrap_or(false)}),
                );
            }
            _ => {}
        })
        .run(tauri::generate_context!())
        .expect("Beaver native runtime failed");
}

fn show_main(app: &tauri::AppHandle) {
    if app
        .try_state::<Arc<Backend>>()
        .is_some_and(|backend| backend.closing.load(Ordering::SeqCst))
    {
        return;
    }
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        if window.is_minimized().unwrap_or(false) {
            let _ = window.unminimize();
        }
        let _ = window.set_focus();
    }
}
