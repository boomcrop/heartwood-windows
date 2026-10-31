// Signing in with Google through the browser. Google won't sign in inside an app's own window, so the app
// listens on a free port of this PC only (127.0.0.1), opens the game's sign-in page in the browser with that
// port and a one-time code, and waits. The page signs in with Google and goes to http://127.0.0.1:<port>/done
// with Google's tokens in the address's fragment (which the browser never sends anywhere); the little page
// served there posts them back to this listener, which takes them only with the right code.
// Asking again while one waits starts afresh: the earlier ask ends as superseded.
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use tauri::Manager;
use tauri_plugin_opener::OpenerExt;

/// the page the browser opens
const PAGE: &str = "https://heartwood-game.web.app/desktop-signin.html";
/// how long the app waits for the browser
const WAIT: Duration = Duration::from_secs(5 * 60);
/// the most a request may carry (Google's tokens are a few kilobytes)
const MAX_REQUEST: usize = 64 * 1024;

#[derive(Default)]
pub struct SignIn {
    /// which ask is the newest: an older one sees it change and ends
    newest: Arc<AtomicU64>,
}

#[tauri::command]
pub async fn google_sign_in(app: tauri::AppHandle, flows: tauri::State<'_, SignIn>) -> Result<Value, String> {
    let newest = flows.newest.clone();
    let mine = newest.fetch_add(1, Ordering::SeqCst) + 1;
    tauri::async_runtime::spawn_blocking(move || {
        let listener = match TcpListener::bind(("127.0.0.1", 0)) {
            Ok(l) => l,
            Err(e) => return json!({ "error": format!("listen: {e}") }),
        };
        let port = match listener.local_addr() {
            Ok(a) => a.port(),
            Err(e) => return json!({ "error": format!("listen: {e}") }),
        };
        let code = match one_time_code() {
            Some(c) => c,
            None => return json!({ "error": "random" }),
        };
        let page = page_url();
        let url = format!("{page}?port={port}&state={code}");
        if let Err(e) = app.opener().open_url(url, None::<&str>) {
            return json!({ "error": format!("browser: {e}") });
        }
        let answer = serve(listener, &code, Instant::now() + WAIT, || newest.load(Ordering::SeqCst) == mine);
        // back to the game once the browser is done with
        if answer.get("superseded").is_none() {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.set_focus();
            }
        }
        answer
    })
    .await
    .map_err(|e| e.to_string())
}

/// the sign-in page, which a development build may be pointed elsewhere for testing (HW_SIGNIN_PAGE)
fn page_url() -> String {
    #[cfg(debug_assertions)]
    if let Ok(p) = std::env::var("HW_SIGNIN_PAGE") {
        return p;
    }
    PAGE.to_string()
}

/// 32 random bytes as hex: the code the answer must carry
fn one_time_code() -> Option<String> {
    let mut b = [0u8; 32];
    getrandom::getrandom(&mut b).ok()?;
    Some(b.iter().map(|x| format!("{x:02x}")).collect())
}

/// waits for the browser's answer until the deadline, or until this ask is no longer the newest
pub fn serve(listener: TcpListener, code: &str, deadline: Instant, still_mine: impl Fn() -> bool) -> Value {
    if listener.set_nonblocking(true).is_err() {
        return json!({ "error": "listen" });
    }
    loop {
        if !still_mine() {
            return json!({ "superseded": true });
        }
        if Instant::now() >= deadline {
            return json!({ "error": "timeout" });
        }
        match listener.accept() {
            Ok((stream, _)) => {
                if let Some(answer) = handle(stream, code) {
                    return answer;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => std::thread::sleep(Duration::from_millis(100)),
            Err(_) => std::thread::sleep(Duration::from_millis(100)),
        }
    }
}

/// one request: the page at /done, or the answer posted to /token. Some(..) once the right code came with it
fn handle(mut stream: TcpStream, code: &str) -> Option<Value> {
    let _ = stream.set_nonblocking(false);
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let (head, body) = read_request(&mut stream)?;
    let first = head.lines().next().unwrap_or("");
    let mut parts = first.split(' ');
    let (method, path) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
    match (method, path.split('?').next().unwrap_or("")) {
        ("GET", "/done") => {
            respond(&mut stream, "200 OK", "text/html; charset=utf-8", DONE_PAGE);
            None
        }
        ("POST", "/token") => {
            let (reply, answer) = answer_of(&body, code);
            respond(&mut stream, "200 OK", "text/plain; charset=utf-8", reply);
            answer
        }
        _ => {
            respond(&mut stream, "404 Not Found", "text/plain; charset=utf-8", "not here");
            None
        }
    }
}

/// what the posted fragment says: the reply for the page, and the answer for the game (None: keep waiting)
pub fn answer_of(body: &str, code: &str) -> (&'static str, Option<Value>) {
    let mut state = String::new();
    let (mut id, mut access, mut cancel) = (String::new(), String::new(), false);
    for (k, v) in url::form_urlencoded::parse(body.trim().as_bytes()) {
        match k.as_ref() {
            "state" => state = v.into_owned(),
            "id_token" => id = v.into_owned(),
            "access_token" => access = v.into_owned(),
            "cancel" => cancel = v == "1",
            _ => {}
        }
    }
    if !same(&state, code) {
        return ("expired", None);
    }
    if cancel {
        return ("cancel", Some(json!({ "cancelled": true })));
    }
    if id.is_empty() {
        return ("expired", None);
    }
    ("ok", Some(json!({ "idToken": id, "accessToken": access })))
}

/// compares the codes in a time that doesn't depend on where they differ
fn same(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0u8, |d, (x, y)| d | (x ^ y)) == 0
}

/// the request's head and body (by its Content-Length), at most MAX_REQUEST
fn read_request(stream: &mut TcpStream) -> Option<(String, String)> {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    let head_end = loop {
        let n = stream.read(&mut chunk).ok()?;
        if n == 0 {
            return None;
        }
        buf.extend_from_slice(&chunk[..n]);
        if let Some(i) = find(&buf, b"\r\n\r\n") {
            break i;
        }
        if buf.len() > MAX_REQUEST {
            return None;
        }
    };
    let head = String::from_utf8_lossy(&buf[..head_end]).to_string();
    let len = head
        .lines()
        .find_map(|l| {
            let (k, v) = l.split_once(':')?;
            k.trim().eq_ignore_ascii_case("content-length").then(|| v.trim().parse::<usize>().ok()).flatten()
        })
        .unwrap_or(0);
    if len > MAX_REQUEST {
        return None;
    }
    let mut body = buf[head_end + 4..].to_vec();
    while body.len() < len {
        let n = stream.read(&mut chunk).ok()?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..n]);
    }
    body.truncate(len);
    Some((head, String::from_utf8_lossy(&body).to_string()))
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

fn respond(stream: &mut TcpStream, status: &str, kind: &str, body: &str) {
    let msg = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nReferrer-Policy: no-referrer\r\nX-Content-Type-Options: nosniff\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(msg.as_bytes());
    let _ = stream.flush();
}

/// the browser lands here: it takes the tokens out of the address at once, posts them to the app and says
/// what happened. Same address as the listener, so nothing else is involved
const DONE_PAGE: &str = r#"<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<meta name="referrer" content="no-referrer"><title>Heartwood</title>
<style>html,body{margin:0;background:#0e0a0c;color:#f3ead7;font:16px/1.6 ui-monospace,Menlo,Consolas,monospace}
main{max-width:440px;margin:18vh auto 0;padding:20px;background:#1e1619;border:2px solid #5a4440;border-right-color:#0e0a0c;border-bottom-color:#0e0a0c}
h1{font-size:18px;letter-spacing:2px;margin:0 0 8px;color:#f6c453}</style></head>
<body><main><h1>HEARTWOOD</h1><p id="m">Signing in...</p></main>
<script>
(function () {
  var h = location.hash.slice(1), m = document.getElementById('m');
  history.replaceState(null, '', location.pathname);
  fetch('/token', { method: 'POST', headers: { 'Content-Type': 'text/plain' }, body: h })
    .then(function (r) { return r.text(); })
    .then(function (t) {
      m.textContent = t === 'ok' ? 'Signed in. You can close this tab and go back to Heartwood.'
        : t === 'cancel' ? 'Cancelled. You can close this tab.'
        : 'This sign-in has expired. Start again from the Heartwood app.';
    })
    .catch(function () { m.textContent = 'Heartwood is no longer waiting. Start again from the app.'; });
})();
</script></body></html>
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;

    const CODE: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    fn post(port: u16, path: &str, body: &str) -> String {
        let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
        write!(s, "POST {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{body}", body.len()).unwrap();
        let mut out = String::new();
        s.read_to_string(&mut out).unwrap();
        out
    }

    fn get(port: u16, path: &str) -> String {
        let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
        write!(s, "GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n").unwrap();
        let mut out = String::new();
        s.read_to_string(&mut out).unwrap();
        out
    }

    #[test]
    fn answers() {
        assert_eq!(answer_of("state=nope&id_token=x", CODE).1, None);
        assert_eq!(answer_of(&format!("state={CODE}"), CODE).1, None);
        assert_eq!(answer_of(&format!("state={CODE}&cancel=1"), CODE).1, Some(json!({ "cancelled": true })));
        assert_eq!(
            answer_of(&format!("state={CODE}&id_token=ID%2B1&access_token=AT"), CODE).1,
            Some(json!({ "idToken": "ID+1", "accessToken": "AT" }))
        );
        assert!(!same("abc", "abd") && same(CODE, CODE) && !same(CODE, "short"));
    }

    #[test]
    fn the_browser_hands_back_the_tokens() {
        let l = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = l.local_addr().unwrap().port();
        let t = std::thread::spawn(move || serve(l, CODE, Instant::now() + Duration::from_secs(10), || true));
        let page = get(port, "/done");
        assert!(page.starts_with("HTTP/1.1 200") && page.contains("fetch('/token'"));
        assert!(get(port, "/elsewhere").starts_with("HTTP/1.1 404"));
        // a wrong code changes nothing: it keeps waiting
        assert!(post(port, "/token", "state=guess&id_token=x").ends_with("expired"));
        assert!(post(port, "/token", &format!("state={CODE}&id_token=GOOGLE-ID&access_token=GOOGLE-AT")).ends_with("ok"));
        assert_eq!(t.join().unwrap(), json!({ "idToken": "GOOGLE-ID", "accessToken": "GOOGLE-AT" }));
    }

    #[test]
    fn cancelled_superseded_and_timed_out() {
        let l = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = l.local_addr().unwrap().port();
        let t = std::thread::spawn(move || serve(l, CODE, Instant::now() + Duration::from_secs(10), || true));
        assert!(post(port, "/token", &format!("state={CODE}&cancel=1")).ends_with("cancel"));
        assert_eq!(t.join().unwrap(), json!({ "cancelled": true }));

        let mine = Arc::new(AtomicBool::new(true));
        let m2 = mine.clone();
        let l = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let t = std::thread::spawn(move || serve(l, CODE, Instant::now() + Duration::from_secs(10), move || m2.load(Ordering::SeqCst)));
        mine.store(false, Ordering::SeqCst);
        assert_eq!(t.join().unwrap(), json!({ "superseded": true }));

        let l = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        assert_eq!(serve(l, CODE, Instant::now() + Duration::from_millis(250), || true), json!({ "error": "timeout" }));
    }

    #[test]
    fn codes_are_long_and_fresh() {
        let (a, b) = (one_time_code().unwrap(), one_time_code().unwrap());
        assert_eq!(a.len(), 64);
        assert_ne!(a, b);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
