mod pipeline;

use std::sync::Arc;

use tauri::Manager;
use tauri::http::{Method, Response, StatusCode, header};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let memory = pipeline::memory_gb()
        .map(|gb| format!("window.__DESKTOP_MEMORY_GB__ = {gb};"))
        .unwrap_or_default();
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
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
        .setup(|app| {
            #[cfg(desktop)]
            app.handle()
                .plugin(tauri_plugin_updater::Builder::new().build())?;
            if let Some(pool) = native_pool(app) {
                app.manage(Arc::new(pool));
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
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
