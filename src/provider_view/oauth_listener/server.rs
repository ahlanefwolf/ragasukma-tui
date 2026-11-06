use std::collections::HashMap;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

pub struct OAuthCallbackData {
    pub code: String,
    pub state: Option<String>,
}

pub struct OAuthServerResponse {
    pub success_html: String,
    pub error_html: String,
}

pub async fn listen_for_oauth_callback(
    port: u16,
    expected_state: &str,
) -> Result<(tokio::net::TcpStream, OAuthCallbackData), String> {
    let addr = format!("127.0.0.1:{}", port);
    let listener = TcpListener::bind(&addr)
        .await
        .map_err(|e| format!("Failed to listen on {}: {}", addr, e))?;

    let timeout_duration = Duration::from_secs(180);

    let accept_res = tokio::time::timeout(timeout_duration, listener.accept()).await;
    let (mut stream, _) = match accept_res {
        Ok(Ok(pair)) => pair,
        Ok(Err(e)) => return Err(format!("Socket accept error: {}", e)),
        Err(_) => return Err("Authentication timed out (3 minute limit)".to_string()),
    };

    let mut buffer = [0u8; 4096];
    let bytes_read = stream
        .read(&mut buffer)
        .await
        .map_err(|e| format!("Failed to read request from browser: {}", e))?;

    let request_str = String::from_utf8_lossy(&buffer[..bytes_read]);
    let first_line = request_str.lines().next().unwrap_or_default();
    let query_string = if let Some(path) = first_line.split_whitespace().nth(1) {
        path.split('?').nth(1).unwrap_or_default()
    } else {
        ""
    };

    let query_params: HashMap<String, String> = query_string
        .split('&')
        .filter_map(|pair| {
            let (key, value) = pair.split_once('=')?;
            let decoded = urlencoding_decode(value);
            Some((key.to_string(), decoded))
        })
        .collect();

    if let Some(err) = query_params.get("error") {
        let err_desc = query_params
            .get("error_description")
            .cloned()
            .unwrap_or_default();
        let fallback = if err_desc.trim().is_empty() {
            err
        } else {
            &err_desc
        };
        let html_response = format!(
            "HTTP/1.1 400 Bad Request\r\nContent-Type: text/html; charset=utf-8\r\n\r\n\
            <html><body style='font-family: sans-serif; text-align: center; padding: 50px;'>\
            <h1 style='color: #ef4444;'>Authentication Denied</h1>\
            <p>{}</p>\
            <p>You can close this window now.</p>\
            </body></html>",
            fallback
        );
        let _ = stream.write_all(html_response.as_bytes()).await;
        return Err(format!("{}: {}", err, err_desc));
    }

    let code = match query_params.get("code") {
        Some(c) if !c.is_empty() => c.clone(),
        _ => {
            let html_response = "HTTP/1.1 400 Bad Request\r\nContent-Type: text/html; charset=utf-8\r\n\r\n\
                <html><body><h1>Error: OAuth code not found</h1></body></html>";
            let _ = stream.write_all(html_response.as_bytes()).await;
            return Err("OAuth code not found in callback request".to_string());
        }
    };

    if let Some(state) = query_params.get("state")
        && state != expected_state
    {
        let html_response = "HTTP/1.1 400 Bad Request\r\nContent-Type: text/html; charset=utf-8\r\n\r\n\
            <html><body><h1>Error: Invalid OAuth state (CSRF mismatch)</h1></body></html>";
        let _ = stream.write_all(html_response.as_bytes()).await;
        return Err("State mismatch: potential CSRF attack or session expired".to_string());
    }

    Ok((
        stream,
        OAuthCallbackData {
            code,
            state: query_params.get("state").cloned(),
        },
    ))
}

pub async fn respond_success(mut stream: tokio::net::TcpStream, email: &str) {
    let html_response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\n\r\n\
        <!DOCTYPE html>\
        <html>\
        <head><title>Ragasukma OAuth - Success</title></head>\
        <body style='background: #0f172a; color: #f8fafc; font-family: -apple-system, BlinkMacSystemFont, Segoe UI, Roboto, Helvetica, Arial, sans-serif; text-align: center; padding-top: 80px;'>\
            <div style='background: #1e293b; display: inline-block; padding: 40px 60px; border-radius: 12px; box-shadow: 0 10px 25px -5px rgba(0, 0, 0, 0.5); border: 1px solid #334155;'>\
                <h1 style='color: #10b981; margin: 0 0 10px 0;'>Authentication Successful!</h1>\
                <p style='color: #94a3b8; font-size: 16px; margin: 0 0 24px 0;'>Account <b>{}</b> is now connected to Ragasukma TUI.</p>\
                <p style='color: #64748b; font-size: 14px;'>You may safely close this tab and return to your terminal.</p>\
            </div>\
            <script>setTimeout(function() {{ window.close(); }}, 3000);</script>\
        </body>\
        </html>",
        email
    );
    let _ = stream.write_all(html_response.as_bytes()).await;
}

pub async fn respond_error(mut stream: tokio::net::TcpStream, err: &str) {
    let html_response = format!(
        "HTTP/1.1 500 Internal Server Error\r\nContent-Type: text/html; charset=utf-8\r\n\r\n\
        <!DOCTYPE html>\
        <html>\
        <head><title>Ragasukma OAuth - Error</title></head>\
        <body style='background: #0f172a; color: #f8fafc; font-family: sans-serif; text-align: center; padding-top: 80px;'>\
            <div style='background: #1e293b; display: inline-block; padding: 40px 60px; border-radius: 12px; border: 1px solid #ef4444;'>\
                <h1 style='color: #ef4444;'>Failed to Complete OAuth</h1>\
                <p style='color: #94a3b8;'>{}</p>\
            </div>\
        </body>\
        </html>",
        err
    );
    let _ = stream.write_all(html_response.as_bytes()).await;
}

fn urlencoding_decode(val: &str) -> String {
    let replaced = val.replace('+', " ");
    let mut bytes = Vec::new();
    let mut chars = replaced.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '%' {
            let h1 = chars.next();
            let h2 = chars.next();
            if let (Some(c1), Some(c2)) = (h1, h2) {
                let hex = format!("{}{}", c1, c2);
                if let Ok(b) = u8::from_str_radix(&hex, 16) {
                    bytes.push(b);
                    continue;
                }
            }
        }
        let mut buf = [0u8; 4];
        let encoded = ch.encode_utf8(&mut buf);
        bytes.extend_from_slice(encoded.as_bytes());
    }
    String::from_utf8_lossy(&bytes).to_string()
}
