use std::sync::Mutex;
use tauri::Manager;
use tauri_plugin_shell::{
    process::{CommandChild, CommandEvent},
    ShellExt,
};

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let (mut rx, child): (_, CommandChild) =
                app.shell().sidecar("server-binary")?.spawn()?;

            // Guardar el handle del proceso en el estado global.
            // Si el CommandChild se dropea, el OS puede matar el proceso hijo.
            app.manage(Mutex::new(child));

            // Monitor de eventos del sidecar — corre en background sin bloquear setup.
            tauri::async_runtime::spawn(async move {
                while let Some(event) = rx.recv().await {
                    match event {
                        CommandEvent::Stderr(line) => {
                            eprintln!("[server] {}", String::from_utf8_lossy(&line));
                        }
                        CommandEvent::Terminated(payload) => {
                            eprintln!(
                                "[server] proceso terminado — código: {:?}, señal: {:?}",
                                payload.code, payload.signal
                            );
                            break;
                        }
                        _ => {}
                    }
                }
            });

            Ok(())
        })
        // Sin comandos IPC registrados — superficie de ataque mínima.
        .invoke_handler(tauri::generate_handler![])
        .run(tauri::generate_context!())
        .expect("error running tauri app");
}
