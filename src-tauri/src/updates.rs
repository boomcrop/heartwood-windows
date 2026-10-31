// Updates: at launch the app reads latest.json from the newest GitHub release. When it names a newer
// version it asks whether to update now; yes downloads it, checks its signature against the public key in
// tauri.conf.json, installs it with a small progress window and starts Heartwood again. No answer, no
// network or a bad signature: the game simply plays on.
use tauri::AppHandle;
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tauri_plugin_updater::UpdaterExt;

pub fn check_on_launch(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let Ok(updater) = app.updater() else { return };
        let update = match updater.check().await {
            Ok(Some(u)) => u,
            _ => return,
        };
        let text = format!(
            "Heartwood {} is out (you have {}).\n\nUpdate now? It takes a moment, then Heartwood starts again.",
            update.version, update.current_version
        );
        let app2 = app.clone();
        app.dialog()
            .message(text)
            .title("Update Heartwood")
            .kind(MessageDialogKind::Info)
            .buttons(MessageDialogButtons::OkCancelCustom("Update now".into(), "Later".into()))
            .show(move |yes| {
                if !yes {
                    return;
                }
                tauri::async_runtime::spawn(async move {
                    match update.download_and_install(|_, _| {}, || {}).await {
                        // (on Windows the installer takes over and the app closes before this returns)
                        Ok(()) => app2.restart(),
                        Err(e) => {
                            app2.dialog()
                                .message(format!("The update could not be installed ({e}). Heartwood will try again next time."))
                                .title("Update Heartwood")
                                .kind(MessageDialogKind::Warning)
                                .show(|_| {});
                        }
                    }
                });
            });
    });
}
