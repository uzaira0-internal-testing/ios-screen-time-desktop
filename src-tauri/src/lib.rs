mod pipeline;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use tauri::http::{Method, Response, StatusCode, header};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tauri_plugin_updater::UpdaterExt;
use tauri_plugin_window_state::StateFlags;

struct Ready(AtomicBool);

const SHOW_WINDOW: bool = !cfg!(feature = "background-test");

#[tauri::command]
fn app_ready(window: tauri::WebviewWindow, ready: tauri::State<'_, Ready>) {
    if ready.0.swap(true, Ordering::SeqCst) {
        return;
    }
    if cfg!(feature = "background-test") {
        eprintln!("app_ready");
    }
    if SHOW_WINDOW {
        let _ = window.show();
        #[cfg(not(target_os = "macos"))]
        let _ = window.set_focus();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let memory = pipeline::memory_gb()
        .map(|gb| format!("window.__DESKTOP_MEMORY_GB__ = {gb};"))
        .unwrap_or_default();
    let builder = tauri::Builder::default();
    #[cfg(target_os = "macos")]
    let builder = builder.activate_ignoring_other_apps(false);
    builder
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_process::init())
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_state_flags(StateFlags::all() & !StateFlags::VISIBLE)
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(prevent_browser_shortcuts())
        .plugin(
            tauri::plugin::Builder::<tauri::Wry>::new("desktop-memory")
                .js_init_script(memory)
                .build(),
        )
        .register_asynchronous_uri_scheme_protocol("pipeline", |ctx, request, responder| {
            let pool = ctx
                .app_handle()
                .try_state::<Arc<pipeline::Pool>>()
                .map(|p| Arc::clone(&p));
            std::thread::spawn(move || {
                let (status, body) = match (pool, request.method()) {
                    (_, &Method::OPTIONS) => (StatusCode::NO_CONTENT, Vec::new()),
                    (None, _) => (StatusCode::SERVICE_UNAVAILABLE, Vec::new()),
                    (Some(pool), &Method::POST) if pipeline::is_two_frames(request.body()) => {
                        match pool.call(request.body()) {
                            Ok(reply) => (StatusCode::OK, reply),
                            Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, Vec::new()),
                        }
                    }
                    _ => (StatusCode::BAD_REQUEST, Vec::new()),
                };
                let response = Response::builder()
                    .status(status)
                    .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
                    .header(header::ACCESS_CONTROL_ALLOW_METHODS, "POST")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(body)
                    .expect("static response parts");
                responder.respond(response);
            });
        })
        .manage(Ready(AtomicBool::new(false)))
        .invoke_handler(tauri::generate_handler![app_ready])
        .on_menu_event(|app, event| {
            if event.id() == "quit" {
                match app.get_webview_window("main") {
                    Some(window) => {
                        let _ = window.close();
                    }
                    None => app.exit(0),
                }
                return;
            }
            let _ = app.emit("menu", event.id().as_ref());
        })
        .setup(|app| {
            #[cfg(desktop)]
            app.handle()
                .plugin(tauri_plugin_updater::Builder::new().build())?;
            #[cfg(target_os = "macos")]
            app.set_menu(app_menu(app.handle())?)?;
            startup_watchdog(app.handle().clone());
            if let Some(pool) = native_pool(app) {
                app.manage(Arc::new(pool));
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn prevent_browser_shortcuts<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    use tauri_plugin_prevent_default::Flags;
    let mut flags = Flags::all() - Flags::CONTEXT_MENU - Flags::FOCUS_MOVE;
    if cfg!(debug_assertions) {
        flags -= Flags::DEV_TOOLS | Flags::RELOAD;
    }
    let builder = tauri_plugin_prevent_default::Builder::new().with_flags(flags);
    #[cfg(windows)]
    let builder = builder.platform(
        tauri_plugin_prevent_default::PlatformOptions::new()
            .browser_accelerator_keys(cfg!(debug_assertions))
            .general_autofill(false)
            .password_autosave(false)
            .swipe_navigation(false),
    );
    builder.build()
}

#[cfg(target_os = "macos")]
fn app_menu(app: &AppHandle) -> tauri::Result<tauri::menu::Menu<tauri::Wry>> {
    use tauri::menu::{Menu, MenuItem, MenuItemKind, PredefinedMenuItem};
    let menu = Menu::default(app)?;
    if let Some(MenuItemKind::Submenu(app_menu)) = menu.items()?.into_iter().next() {
        let updates = MenuItem::with_id(
            app,
            "check-updates",
            "Check for Updates…",
            true,
            None::<&str>,
        )?;
        let settings = MenuItem::with_id(app, "settings", "Settings…", true, Some("CmdOrCtrl+,"))?;
        app_menu.insert(&updates, 1)?;
        app_menu.insert(&settings, 3)?;
        app_menu.insert(&PredefinedMenuItem::separator(app)?, 4)?;
        let last = app_menu.items()?.len().saturating_sub(1);
        app_menu.remove_at(last)?;
        let quit = format!("Quit {}", app.package_info().name);
        app_menu.append(&MenuItem::with_id(
            app,
            "quit",
            quit,
            true,
            Some("CmdOrCtrl+Q"),
        )?)?;
    }
    Ok(menu)
}

fn startup_watchdog(app: AppHandle) {
    std::thread::spawn(move || {
        let ready = || app.state::<Ready>().0.load(Ordering::SeqCst);
        std::thread::sleep(Duration::from_secs(5));
        if ready() {
            return;
        }
        if let Some(window) = app.get_webview_window("main").filter(|_| SHOW_WINDOW) {
            let _ = window.show();
        }
        std::thread::sleep(Duration::from_secs(25));
        if ready() {
            return;
        }
        let Ok(updater) = app.updater() else { return };
        let Ok(Some(update)) = tauri::async_runtime::block_on(updater.check()) else {
            return;
        };
        let install = app
            .dialog()
            .message(format!(
                "The app did not finish starting. Version {} is available and may fix this.",
                update.version
            ))
            .title("Update available")
            .kind(MessageDialogKind::Warning)
            .buttons(MessageDialogButtons::OkCancelCustom(
                "Install and Restart".into(),
                "Not Now".into(),
            ))
            .blocking_show();
        if install
            && tauri::async_runtime::block_on(update.download_and_install(|_, _| {}, || {})).is_ok()
        {
            app.restart();
        }
    });
}

fn native_pool(app: &tauri::App) -> Option<pipeline::Pool> {
    let name = if cfg!(windows) {
        "pipeline_native.exe"
    } else {
        "pipeline_native"
    };
    let exe = std::env::current_exe().ok()?.with_file_name(name);
    if !exe.is_file() {
        return None;
    }
    let eng = app
        .asset_resolver()
        .get("pipeline-em/eng.traineddata".into())?;
    let tessdata = app.path().app_cache_dir().ok()?.join("tessdata");
    let file = tessdata.join("eng.traineddata");
    if std::fs::read(&file).ok().as_deref() != Some(eng.bytes()) {
        std::fs::create_dir_all(&tessdata).ok()?;
        std::fs::write(&file, eng.bytes()).ok()?;
    }
    Some(pipeline::Pool::new(exe, tessdata))
}
