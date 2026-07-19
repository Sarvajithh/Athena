//! Minimal localhost loopback listener for the OAuth 2.0 Authorization
//! Code redirect (07_INTEGRATIONS.md §1.8-§1.10, 2026-07-17 amendment).
//! Hand-rolled HTTP parsing over a `tokio::net::TcpListener` rather than
//! pulling in a web-server crate (`warp`/`axum`/`tiny_http`) for the one
//! thing this app ever needs one for: reading a single GET request's
//! query string, once, per OAuth connect flow (Implementation Plan §4,
//! "cut, don't add" — the existing connectors' own hand-rolled
//! ISO-8601/base64 helpers set the precedent for this style of
//! minimalism).

use std::collections::HashMap;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/// Binds an ephemeral local port and waits for exactly one GET request
/// carrying `code`/`state` query params (or an `error` param if the
/// user declined), then responds with a short static page telling the
/// user they can close the tab.
pub struct LoopbackListener {
    listener: TcpListener,
    pub port: u16,
}

impl LoopbackListener {
    /// Binds to `127.0.0.1:0` (OS-assigned ephemeral port) — never a
    /// fixed port, so this never collides with another app already
    /// listening, and never needs a firewall exception beyond loopback.
    pub async fn bind() -> Result<Self, String> {
        let listener = TcpListener::bind("127.0.0.1:0").await.map_err(|e| e.to_string())?;
        let port = listener.local_addr().map_err(|e| e.to_string())?.port();
        Ok(Self { listener, port })
    }

    /// Waits for the redirect, with a timeout so an abandoned browser
    /// tab (user closes it, never approves) doesn't hang the calling
    /// Tauri command forever — the command returns a normal `Err`
    /// string in that case, same as any other recoverable failure in
    /// this file's caller (`commands::integrations`).
    pub async fn wait_for_code(self, timeout: Duration) -> Result<(String, String), String> {
        let accept = async {
            loop {
                let (mut stream, _) = self.listener.accept().await.map_err(|e| e.to_string())?;

                let mut buf = [0u8; 8192];
                let n = stream.read(&mut buf).await.map_err(|e| e.to_string())?;
                let request = String::from_utf8_lossy(&buf[..n]);

                let Some(query) = parse_query_from_request_line(&request) else {
                    let _ = write_response(&mut stream, "Athena: invalid callback request.").await;
                    continue;
                };

                if let Some(error) = query.get("error") {
                    let _ = write_response(
                        &mut stream,
                        "Athena: authorization was not granted. You can close this tab.",
                    )
                    .await;
                    return Err(format!("provider returned an oauth error: {error}"));
                }

                match (query.get("code"), query.get("state")) {
                    (Some(code), Some(state)) => {
                        let _ = write_response(
                            &mut stream,
                            "Athena: connected. You can close this tab and return to the app.",
                        )
                        .await;
                        return Ok((code.clone(), state.clone()));
                    }
                    _ => {
                        // Not the callback we expected (a stray favicon
                        // request, browser prefetch, etc.) — keep
                        // waiting for the real one instead of failing
                        // the whole flow over it.
                        let _ = write_response(&mut stream, "Athena: waiting for authorization.").await;
                        continue;
                    }
                }
            }
        };

        tokio::time::timeout(timeout, accept)
            .await
            .map_err(|_| "timed out waiting for the browser to complete authorization".to_string())?
    }
}

fn parse_query_from_request_line(request: &str) -> Option<HashMap<String, String>> {
    let line = request.lines().next()?; // "GET /callback?code=...&state=... HTTP/1.1"
    let mut parts = line.split_whitespace();
    let _method = parts.next()?;
    let path = parts.next()?;
    let (_, query) = path.split_once('?')?;

    let mut map = HashMap::new();
    for pair in query.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            map.insert(url_decode(k), url_decode(v));
        }
    }
    Some(map)
}

fn url_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                if let Ok(byte) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                    out.push(byte);
                    i += 3;
                    continue;
                }
                out.push(bytes[i]);
                i += 1;
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

async fn write_response(stream: &mut TcpStream, body: &str) -> std::io::Result<()> {
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    stream.write_all(response.as_bytes()).await
}

/// Opens the user's default browser to `url` — one `std::process::Command`
/// per OS, the standard desktop-app pattern for this need, rather than a
/// new Tauri plugin dependency for a single `open` call.
pub fn open_in_browser(url: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let result = std::process::Command::new("open").arg(url).spawn();
    #[cfg(target_os = "windows")]
    let result = std::process::Command::new("cmd").args(["/C", "start", "", url]).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let result = std::process::Command::new("xdg-open").arg(url).spawn();

    result.map(|_| ()).map_err(|e| format!("could not open browser: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_code_and_state_from_a_request_line() {
        let request = "GET /callback?code=abc123&state=xyz HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n";
        let query = parse_query_from_request_line(request).unwrap();
        assert_eq!(query.get("code").map(String::as_str), Some("abc123"));
        assert_eq!(query.get("state").map(String::as_str), Some("xyz"));
    }

    #[test]
    fn parses_percent_encoded_and_plus_encoded_values() {
        let request = "GET /callback?state=a%20b+c HTTP/1.1\r\n\r\n";
        let query = parse_query_from_request_line(request).unwrap();
        assert_eq!(query.get("state").map(String::as_str), Some("a b c"));
    }

    #[test]
    fn returns_none_for_a_request_line_with_no_query_string() {
        let request = "GET /favicon.ico HTTP/1.1\r\n\r\n";
        assert!(parse_query_from_request_line(request).is_none());
    }
}
