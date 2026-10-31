// Heartwood for Windows: the game in a desktop window, served from the files bundled with the app (so
// single-player works offline), the same way the iPhone, iPad and Mac apps carry it. The page learns it
// is in this app from window.HW_APP, which also says what this app can't do (no App Store, Game Center
// or Sign in with Apple), and it signs in with Google through the browser (signin.rs).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod signin;
mod updates;

use tauri::{Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use tauri_plugin_opener::OpenerExt;

/// the game's website: invite links to it open in the game, its other pages in the browser
const SITE: &str = "heartwood-game.web.app";

/// what the page reads before its first line runs (native.js, core.js, platform.js); F11 goes full screen
fn app_script(version: &str) -> String {
    let info = serde_json::json!({
        "os": "windows",
        "device": "desktop",
        "version": version,
        "can": { "iap": false, "gc": false, "apple": false },
        "signIn": "browser",
    });
    format!(
        "window.HW_APP = {info};\n\
         window.addEventListener('keydown', function (e) {{\n\
           if (e.key === 'F11' && window.__TAURI__) {{ e.preventDefault(); window.__TAURI__.core.invoke('toggle_fullscreen'); }}\n\
         }}, true);"
    )
}

/// the game's own address (https://tauri.localhost/...): it stays in the window. Saves and the guest
/// account live with that address, so it must never change between versions
fn is_game(url: &url::Url) -> bool {
    url.host_str() == Some("tauri.localhost") || matches!(url.scheme(), "tauri" | "about" | "data" | "blob")
}

/// a link to the website that the game handles itself: an invite to a server (/join/... or ?join=...)
fn is_join_link(url: &url::Url) -> bool {
    url.scheme() == "https"
        && url.host_str() == Some(SITE)
        && (url.path().starts_with("/join/") || url.query_pairs().any(|(k, _)| k == "join"))
}

#[tauri::command]
fn toggle_fullscreen(window: WebviewWindow) {
    let full = window.is_fullscreen().unwrap_or(false);
    let _ = window.set_fullscreen(!full);
}

fn main() {
    tauri::Builder::default()
        // a second launch brings the open window forward instead of starting another copy on the same saves
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.unminimize();
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(signin::SignIn::default())
        .invoke_handler(tauri::generate_handler![signin::google_sign_in, toggle_fullscreen])
        .setup(|app| {
            let version = app.package_info().version.to_string();
            let handle = app.handle().clone();
            WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .title("Heartwood")
                .inner_size(1280.0, 800.0)
                .min_inner_size(640.0, 360.0)
                .center()
                .use_https_scheme(true)
                .initialization_script(app_script(&version))
                // anything outside the game opens in the browser; an invite link joins the server in the game
                .on_navigation(move |url| {
                    if is_game(url) {
                        return true;
                    }
                    if is_join_link(url) {
                        if let Some(w) = handle.get_webview_window("main") {
                            let link = serde_json::to_string(url.as_str()).unwrap_or_default();
                            let _ = w.eval(format!("window.__hwOpenLink && window.__hwOpenLink({link})"));
                        }
                    } else if matches!(url.scheme(), "http" | "https" | "mailto") {
                        let _ = handle.opener().open_url(url.as_str(), None::<&str>);
                    }
                    false
                })
                .build()?;
            updates::check_on_launch(app.handle().clone());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Heartwood could not start");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn u(s: &str) -> url::Url {
        url::Url::parse(s).unwrap()
    }

    #[test]
    fn the_game_stays_in_the_window() {
        assert!(is_game(&u("https://tauri.localhost/index.html")));
        assert!(is_game(&u("http://tauri.localhost/js/main.js")));
        assert!(!is_game(&u("https://heartwood-game.web.app/privacy.html")));
        assert!(!is_game(&u("https://tauri.localhost.example.com/")));
    }

    #[test]
    fn invites_join_in_the_game() {
        assert!(is_join_link(&u("https://heartwood-game.web.app/join/abc123")));
        assert!(is_join_link(&u("https://heartwood-game.web.app/?join=abc123")));
        assert!(!is_join_link(&u("https://heartwood-game.web.app/privacy.html")));
        assert!(!is_join_link(&u("https://example.com/join/abc123")));
        assert!(!is_join_link(&u("http://heartwood-game.web.app/join/abc123")));
    }

    #[test]
    fn the_page_learns_it_is_in_the_windows_app() {
        let s = app_script("1.2.3");
        assert!(s.starts_with("window.HW_APP = {"));
        let json = &s["window.HW_APP = ".len()..s.find(";\n").unwrap()];
        let v: serde_json::Value = serde_json::from_str(json).unwrap();
        assert_eq!(v["os"], "windows");
        assert_eq!(v["version"], "1.2.3");
        assert_eq!(v["signIn"], "browser");
        assert_eq!(v["can"]["iap"], false);
        assert_eq!(v["can"]["gc"], false);
        assert_eq!(v["can"]["apple"], false);
    }
}
