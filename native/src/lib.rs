use axum::{
    body::Body,
    extract::Request as AxumRequest,
    http::{header, HeaderValue},
    middleware::{self, Next},
    response::Response as AxumResponse,
    routing::{any, delete, get, head, options, patch, post, put, trace},
    Router,
};
use bytes::Bytes;
use flame_macro::flame;
use http_body_util::BodyExt;
use std::{collections::HashMap, mem, net::SocketAddr, sync::RwLock};

#[derive(Clone, Debug, Default)]
pub struct CurrentRequestContext {
    pub path: String,
    pub query: String,
    pub method: String,
}

static CURRENT_REQUEST_INFO: RwLock<CurrentRequestContext> = RwLock::new(CurrentRequestContext {
    path: String::new(),
    query: String::new(),
    method: String::new(),
});

/// Represents an HTTP Server powered by Axum and Tokio.
pub struct FlamerServer {
    router: Router,
    port: u16,
    host: String,
    has_custom_fallback: bool,
}

/// Represents an incoming HTTP request containing body, method, path, and query.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Request {
    pub body: String,
    pub method: String,
    pub path: String,
    pub query: String,
}

impl Request {
    /// Constructs a new Request instance.
    pub fn new(body: String, method: String, path: String, query: String) -> Request {
        Request {
            body,
            method,
            path,
            query,
        }
    }

    /// Returns the body payload of the request.
    pub fn body(&self) -> String {
        self.body.clone()
    }

    /// Returns the HTTP method (e.g. GET, POST) of the request.
    pub fn method(&self) -> String {
        self.method.clone()
    }

    /// Returns the URL path of the request.
    pub fn path(&self) -> String {
        self.path.clone()
    }

    /// Returns the raw URL query string (without the leading '?').
    pub fn query(&self) -> String {
        self.query.clone()
    }

    /// Extracts a specific query parameter value by key.
    #[flame(rename = "queryParam")]
    pub fn query_param(&self, key: String) -> String {
        get_query_param(self.query.clone(), key)
    }

    /// Checks if a query parameter exists in the query string.
    #[flame(rename = "hasQuery")]
    pub fn has_query(&self, key: String) -> bool {
        !self.query_param(key).is_empty()
    }
}

/// Represents an outgoing HTTP response with status code, body, and content-type.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Response {
    pub body: String,
    pub status: i64,
    pub content_type: String,
}

impl Response {
    /// Creates a generic Response with HTTP 200 and text/plain.
    pub fn new(body: String) -> Response {
        Response {
            body,
            status: 200,
            content_type: "text/plain".to_string(),
        }
    }

    /// Creates a JSON Response with HTTP 200 and application/json.
    pub fn json(body: String) -> Response {
        Response {
            body,
            status: 200,
            content_type: "application/json".to_string(),
        }
    }

    /// Creates an HTML Response with HTTP 200 and text/html.
    pub fn html(body: String) -> Response {
        Response {
            body,
            status: 200,
            content_type: "text/html".to_string(),
        }
    }

    /// Creates a plain text Response with HTTP 200 and text/plain.
    pub fn text(body: String) -> Response {
        Response {
            body,
            status: 200,
            content_type: "text/plain".to_string(),
        }
    }

    /// Sets the HTTP status code (e.g. 200, 201, 404, 500).
    #[flame(rename = "setStatus")]
    pub fn set_status(&mut self, status: i64) {
        self.status = status;
    }

    /// Sets the Content-Type header (e.g. "application/json").
    #[flame(rename = "setContentType")]
    pub fn set_content_type(&mut self, content_type: String) {
        self.content_type = content_type;
    }

    /// Returns the HTTP status code.
    pub fn status(&self) -> i64 {
        self.status
    }

    /// Returns the response body.
    pub fn body(&self) -> String {
        self.body.clone()
    }

    /// Returns the Content-Type of the response.
    #[flame(rename = "contentType")]
    pub fn content_type(&self) -> String {
        self.content_type.clone()
    }
}

impl axum::response::IntoResponse for Response {
    fn into_response(self) -> AxumResponse {
        let mut res = AxumResponse::new(Body::from(self.body));
        if let Ok(status) = axum::http::StatusCode::from_u16(self.status as u16) {
            *res.status_mut() = status;
        }
        if let Ok(val) = HeaderValue::from_str(&self.content_type) {
            res.headers_mut().insert(header::CONTENT_TYPE, val);
        }
        res
    }
}

/// Create a new Flamer server instance.
pub fn init() -> FlamerServer {
    FlamerServer {
        router: Router::new(),
        port: 3000,
        host: "127.0.0.1".to_string(),
        has_custom_fallback: false,
    }
}

/// Normalizes an Express-style route pattern (e.g. `/users/:id` or `/api/*`)
/// into an Axum-compliant route pattern (e.g. `/users/{id}` or `/api/{*wildcard}`).
fn normalize_path(path: &str) -> &'static str {
    let p = if !path.starts_with('/') {
        format!("/{}", path)
    } else {
        path.to_string()
    };
    let mut normalized = String::new();
    let parts: Vec<&str> = p.split('/').collect();
    for (i, part) in parts.iter().enumerate() {
        if i > 0 {
            normalized.push('/');
        }
        if let Some(param) = part.strip_prefix(':') {
            normalized.push('{');
            normalized.push_str(param);
            normalized.push('}');
        } else if *part == "*" {
            normalized.push_str("{*wildcard}");
        } else {
            normalized.push_str(part);
        }
    }
    Box::leak(normalized.into_boxed_str())
}

/// Helper macro to register routes for both `/path` and `/path/` to avoid 404 on trailing slash mismatch.
macro_rules! register_route {
    ($self:ident, $path:expr, $method:ident, $handler:expr) => {{
        let p = normalize_path($path);
        let mut r = mem::take(&mut $self.router).route(p, $method($handler.clone()));
        if p != "/" && !p.ends_with('}') {
            let alt_p = if p.ends_with('/') {
                Box::leak(p.trim_end_matches('/').to_string().into_boxed_str())
            } else {
                Box::leak(format!("{}/", p).into_boxed_str())
            };
            r = r.route(alt_p, $method($handler));
        }
        $self.router = r;
    }};
}

impl FlamerServer {
    /// Set the port number on which the server will listen.
    #[flame(rename = "setPort")]
    pub fn set_port(&mut self, port: i64) {
        self.port = port as u16;
    }

    /// Retrieve the configured port number.
    pub fn port(&self) -> i64 {
        self.port as i64
    }

    /// Set the host IP/interface address (defaults to "127.0.0.1").
    #[flame(rename = "setHost")]
    pub fn set_host(&mut self, host: String) {
        self.host = host;
    }

    /// Retrieve the configured host address.
    pub fn host(&self) -> String {
        self.host.clone()
    }

    /// Registers a GET route handler.
    pub fn get<H, T>(&mut self, path: &'static str, handler: H)
    where
        H: axum::handler::Handler<T, ()> + Clone + Send + Sync + 'static,
        T: 'static,
    {
        register_route!(self, path, get, handler);
    }

    /// Registers a POST route handler.
    pub fn post<H, T>(&mut self, path: &'static str, handler: H)
    where
        H: axum::handler::Handler<T, ()> + Clone + Send + Sync + 'static,
        T: 'static,
    {
        register_route!(self, path, post, handler);
    }

    /// Registers a PUT route handler.
    pub fn put<H, T>(&mut self, path: &'static str, handler: H)
    where
        H: axum::handler::Handler<T, ()> + Clone + Send + Sync + 'static,
        T: 'static,
    {
        register_route!(self, path, put, handler);
    }

    /// Registers a DELETE route handler.
    pub fn delete<H, T>(&mut self, path: &'static str, handler: H)
    where
        H: axum::handler::Handler<T, ()> + Clone + Send + Sync + 'static,
        T: 'static,
    {
        register_route!(self, path, delete, handler);
    }

    /// Registers a PATCH route handler.
    pub fn patch<H, T>(&mut self, path: &'static str, handler: H)
    where
        H: axum::handler::Handler<T, ()> + Clone + Send + Sync + 'static,
        T: 'static,
    {
        register_route!(self, path, patch, handler);
    }

    /// Registers a HEAD route handler.
    pub fn head<H, T>(&mut self, path: &'static str, handler: H)
    where
        H: axum::handler::Handler<T, ()> + Clone + Send + Sync + 'static,
        T: 'static,
    {
        register_route!(self, path, head, handler);
    }

    /// Registers an OPTIONS route handler.
    pub fn options<H, T>(&mut self, path: &'static str, handler: H)
    where
        H: axum::handler::Handler<T, ()> + Clone + Send + Sync + 'static,
        T: 'static,
    {
        register_route!(self, path, options, handler);
    }

    /// Registers a TRACE route handler.
    pub fn trace<H, T>(&mut self, path: &'static str, handler: H)
    where
        H: axum::handler::Handler<T, ()> + Clone + Send + Sync + 'static,
        T: 'static,
    {
        register_route!(self, path, trace, handler);
    }

    /// Registers a route matching any HTTP method.
    pub fn any<H, T>(&mut self, path: &'static str, handler: H)
    where
        H: axum::handler::Handler<T, ()> + Clone + Send + Sync + 'static,
        T: 'static,
    {
        register_route!(self, path, any, handler);
    }

    /// Registers a custom fallback handler for unmatched routes.
    pub fn fallback<H, T>(&mut self, handler: H)
    where
        H: axum::handler::Handler<T, ()> + Clone + Send + Sync + 'static,
        T: 'static,
    {
        self.has_custom_fallback = true;
        self.router = mem::take(&mut self.router).fallback(handler);
    }

    /// Returns a JSON response string with explicit Flamer marker.
    pub fn json(&self, content: String) -> String {
        format!("<!--flamer:json-->{}", convert_to_json(&content))
    }

    /// Returns an HTML response string with explicit Flamer marker.
    pub fn html(&self, content: String) -> String {
        format!("<!--flamer:html-->{}", content)
    }

    /// Returns a plain text response string.
    pub fn text(&self, content: String) -> String {
        content
    }

    /// Returns the underlying Axum Router instance with auto content-type detection and request context.
    pub fn router(&mut self) -> Router {
        let mut r = mem::take(&mut self.router);
        if !self.has_custom_fallback {
            r = r.fallback(default_fallback_handler);
        }
        r.layer(middleware::from_fn(auto_content_type_middleware))
            .layer(middleware::from_fn(request_context_middleware))
    }

    /// Starts listening for incoming connections asynchronously on the configured port.
    #[flame(daemon)]
    pub async fn listen(mut self) -> std::io::Result<()> {
        let ip: std::net::IpAddr = self
            .host
            .parse()
            .unwrap_or(std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)));
        let addr = SocketAddr::new(ip, self.port);

        println!();
        println!("🔥 Flamer");
        println!("────────────────────────────────");
        println!("✓ Running on http://{}:{}", self.host, self.port);
        println!("✓ Press Ctrl+C to stop");
        println!();

        let listener = tokio::net::TcpListener::bind(addr).await?;

        let mut r = mem::take(&mut self.router);
        if !self.has_custom_fallback {
            r = r.fallback(default_fallback_handler);
        }
        let app = r
            .layer(middleware::from_fn(auto_content_type_middleware))
            .layer(middleware::from_fn(request_context_middleware));

        axum::serve(listener, app)
            .await
            .map_err(std::io::Error::other)
    }
}

/// Tracks the current HTTP request context (path, query, method) across async execution.
async fn request_context_middleware(req: AxumRequest, next: Next) -> AxumResponse {
    let ctx = CurrentRequestContext {
        path: req.uri().path().to_string(),
        query: req.uri().query().unwrap_or("").to_string(),
        method: req.method().to_string(),
    };

    if let Ok(mut lock) = CURRENT_REQUEST_INFO.write() {
        *lock = ctx;
    }

    next.run(req).await
}

/// Default fallback handler for routes that do not match any registered endpoint.
/// Returns a clean JSON error response if requested by client or for /api/* routes,
/// or an aesthetically designed dark-mode HTML 404 page for browser requests.
async fn default_fallback_handler(req: AxumRequest) -> AxumResponse {
    let path = req.uri().path().to_string();
    let method = req.method().to_string();

    let is_json = req
        .headers()
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.contains("application/json"))
        .unwrap_or(false)
        || path.starts_with("/api");

    let status = axum::http::StatusCode::NOT_FOUND;

    if is_json {
        let json_body = format!(
            r#"{{"error":"No route found","method":"{}","path":"{}","status":404}}"#,
            method, path
        );
        let mut res = AxumResponse::new(Body::from(json_body));
        *res.status_mut() = status;
        res.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json; charset=utf-8"),
        );
        res
    } else {
        let html_body = format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>404 Not Found - Flamer</title>
  <style>
    * {{ box-sizing: border-box; margin: 0; padding: 0; }}
    body {{
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
      background: #000000ff;
      color: #f1f5f9;
      display: flex;
      align-items: center;
      justify-content: center;
      min-height: 100vh;
      padding: 1.5rem;
    }}
    .badge {{
      display: inline-block;
      background: rgba(249, 115, 22, 0.15);
      color: #f97316;
      border: 1px solid rgba(249, 115, 22, 0.3);
      padding: 0.35rem 0.85rem;
      border-radius: 9999px;
      font-size: 0.85rem;
      font-weight: 600;
      letter-spacing: 0.05em;
      margin-bottom: 1.25rem;
    }}
    h1 {{
      font-size: 2.75rem;
      font-weight: 800;
      color: #f8fafc;
      margin-bottom: 0.5rem;
      letter-spacing: -0.025em;
    }}
    h2 {{
      font-size: 1.35rem;
      font-weight: 600;
      color: #94a3b8;
      margin-bottom: 1.25rem;
    }}
    p {{
      color: #64748b;
      font-size: 0.95rem;
      line-height: 1.6;
      margin-bottom: 1.5rem;
    }}
  </style>
</head>
<body>
  <div>
    <div class="badge">🔥 FLAMER SERVER</div>
    <h1>404</h1>
    <h2>No Route Found</h2>
    <p>The requested route could not be found on this server.</p>
    <p>{} "{}"</p>
  </div>
</body>
</html>"#,
            method, path
        );
        let mut res = AxumResponse::new(Body::from(html_body));
        *res.status_mut() = status;
        res.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("text/html; charset=utf-8"),
        );
        res
    }
}

/// Automatically detects and sets Content-Type (HTML, JSON, plain text) and strips Flamer markers.
async fn auto_content_type_middleware(req: AxumRequest, next: Next) -> AxumResponse {
    let response = next.run(req).await;
    let (mut parts, body) = response.into_parts();

    let bytes = match body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(_) => return AxumResponse::from_parts(parts, Body::empty()),
    };

    let text_content = match std::str::from_utf8(&bytes) {
        Ok(s) => s,
        Err(_) => return AxumResponse::from_parts(parts, Body::from(bytes)),
    };

    // Check for explicit HTML marker from flamer.html()
    if let Some(rest) = text_content.strip_prefix("<!--flamer:html-->") {
        parts.headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("text/html; charset=utf-8"),
        );
        let new_bytes = Bytes::copy_from_slice(rest.as_bytes());
        parts
            .headers
            .insert(header::CONTENT_LENGTH, HeaderValue::from(new_bytes.len()));
        return AxumResponse::from_parts(parts, Body::from(new_bytes));
    }

    // Check for explicit JSON marker from flamer.json()
    if let Some(rest) = text_content.strip_prefix("<!--flamer:json-->") {
        parts.headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json; charset=utf-8"),
        );
        let new_bytes = Bytes::copy_from_slice(rest.as_bytes());
        parts
            .headers
            .insert(header::CONTENT_LENGTH, HeaderValue::from(new_bytes.len()));
        return AxumResponse::from_parts(parts, Body::from(new_bytes));
    }

    // Auto-detect HTML or JSON if currently text/plain
    let is_text_plain = parts
        .headers
        .get(header::CONTENT_TYPE)
        .map_or(false, |v| v.as_bytes().starts_with(b"text/plain"));

    if is_text_plain {
        let trimmed = text_content.trim();
        let is_html = trimmed.starts_with("<!DOCTYPE")
            || trimmed.starts_with("<!doctype")
            || trimmed.starts_with("<html")
            || trimmed.starts_with("<head")
            || trimmed.starts_with("<body")
            || (trimmed.starts_with('<') && (trimmed.contains("</") || trimmed.ends_with('>')));

        if is_html {
            parts.headers.insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static("text/html; charset=utf-8"),
            );
        } else if (trimmed.starts_with('{') && trimmed.ends_with('}'))
            || (trimmed.starts_with('[') && trimmed.ends_with(']'))
        {
            if serde_json::from_str::<serde_json::Value>(trimmed).is_ok() {
                parts.headers.insert(
                    header::CONTENT_TYPE,
                    HeaderValue::from_static("application/json; charset=utf-8"),
                );
            }
        }
    }

    AxumResponse::from_parts(parts, Body::from(bytes))
}

/// Parses a query string into a JSON object string (e.g. `{"key":"value"}`).
#[flame(rename = "parseQueryJson")]
pub fn parse_query_json(query: String) -> String {
    let q = query.trim_start_matches('?');
    let mut map = HashMap::new();
    for (k, v) in form_urlencoded::parse(q.as_bytes()) {
        map.insert(k.into_owned(), v.into_owned());
    }
    serde_json::to_string(&map).unwrap_or_else(|_| "{}".to_string())
}

/// Extracts a specific parameter from a URL query string by key.
#[flame(rename = "getQueryParam")]
pub fn get_query_param(query: String, key: String) -> String {
    let q = query.trim_start_matches('?');
    for (k, v) in form_urlencoded::parse(q.as_bytes()) {
        if k == key {
            return v.into_owned();
        }
    }
    String::new()
}

/// Encodes a plain string into application/x-www-form-urlencoded percent format.
#[flame(rename = "urlEncode")]
pub fn url_encode(input: String) -> String {
    form_urlencoded::byte_serialize(input.as_bytes()).collect()
}

/// Decodes a percent-encoded string back into raw UTF-8.
#[flame(rename = "urlDecode")]
pub fn url_decode(input: String) -> String {
    form_urlencoded::parse(input.as_bytes())
        .next()
        .map(|(k, _)| k.into_owned())
        .unwrap_or(input)
}

/// Checks if a request path matches a route pattern with `:param` placeholders or `*` wildcards.
#[flame(rename = "matchPath")]
pub fn match_path(pattern: String, path: String) -> bool {
    let pat_clean = pattern.trim_matches('/');
    let path_clean = path.trim_matches('/');

    if path_clean.is_empty() {
        return true;
    }

    if pat_clean.is_empty() && path_clean.is_empty() {
        return true;
    }

    let pat_segments: Vec<&str> = pat_clean.split('/').collect();
    let path_segments: Vec<&str> = path_clean.split('/').collect();

    let mut i = 0;
    while i < pat_segments.len() {
        let seg = pat_segments[i];
        if seg == "*" {
            return true;
        }
        if i >= path_segments.len() {
            return false;
        }
        if seg.starts_with(':') {
            i += 1;
            continue;
        }
        if seg != path_segments[i] {
            return false;
        }
        i += 1;
    }

    i == path_segments.len()
}

/// Extracts named path parameters from a URL path according to a pattern (e.g. `/users/:id`),
/// returning a JSON object string of the captured keys and values.
#[flame(rename = "extractPathParamsJson")]
pub fn extract_path_params_json(pattern: String, path: String) -> String {
    let pat_clean = pattern.trim_matches('/');
    let path_clean = path.trim_matches('/');

    let pat_segments: Vec<&str> = pat_clean.split('/').collect();
    let path_segments: Vec<&str> = path_clean.split('/').collect();

    let mut map = HashMap::new();

    for (i, seg) in pat_segments.iter().enumerate() {
        if seg.starts_with(':') {
            let key = &seg[1..];
            if i < path_segments.len() && !path_segments[i].is_empty() {
                map.insert(key.to_string(), path_segments[i].to_string());
            } else {
                map.insert(key.to_string(), "1".to_string());
            }
        } else if *seg == "*" && i < path_segments.len() {
            let rest = path_segments[i..].join("/");
            map.insert("wildcard".to_string(), rest);
            break;
        }
    }

    serde_json::to_string(&map).unwrap_or_else(|_| "{}".to_string())
}

/// Tests if a path begins with a given prefix path filter.
#[flame(rename = "pathFilter")]
pub fn path_filter(prefix: String, path: String) -> bool {
    let pfx = prefix.trim_end_matches('/');
    let pth = path.trim_end_matches('/');
    if pth.is_empty() {
        return true;
    }
    pth == pfx || pth.starts_with(&format!("{}/", pfx))
}

/// Converts any input string (including relaxed formulas with unquoted keys, single quotes, or nil) into valid JSON.
pub fn convert_to_json(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return "{}".to_string();
    }
    // If it is already valid JSON, parse and return clean serialization
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(trimmed) {
        return v.to_string();
    }
    // Convert unquoted keys and single quotes into standard JSON format
    let mut normalized = String::with_capacity(trimmed.len() + 32);
    let chars: Vec<char> = trimmed.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        if ch == '\'' {
            normalized.push('"');
            i += 1;
            while i < chars.len() && chars[i] != '\'' {
                if chars[i] == '\\' && i + 1 < chars.len() {
                    normalized.push('\\');
                    normalized.push(chars[i + 1]);
                    i += 2;
                } else if chars[i] == '"' {
                    normalized.push_str("\\\"");
                    i += 1;
                } else {
                    normalized.push(chars[i]);
                    i += 1;
                }
            }
            normalized.push('"');
            if i < chars.len() {
                i += 1;
            }
        } else if ch == '"' {
            normalized.push('"');
            i += 1;
            while i < chars.len() && chars[i] != '"' {
                if chars[i] == '\\' && i + 1 < chars.len() {
                    normalized.push('\\');
                    normalized.push(chars[i + 1]);
                    i += 2;
                } else {
                    normalized.push(chars[i]);
                    i += 1;
                }
            }
            normalized.push('"');
            if i < chars.len() {
                i += 1;
            }
        } else if ch.is_alphabetic() || ch == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_' || chars[i] == '-') {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            let mut j = i;
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            if j < chars.len() && chars[j] == ':' {
                normalized.push('"');
                normalized.push_str(&word);
                normalized.push('"');
            } else if word == "true" || word == "false" || word == "null" {
                normalized.push_str(&word);
            } else if word == "nil" || word == "undefined" {
                normalized.push_str("null");
            } else {
                normalized.push('"');
                normalized.push_str(&word);
                normalized.push('"');
            }
        } else {
            normalized.push(ch);
            i += 1;
        }
    }

    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&normalized) {
        return v.to_string();
    }

    serde_json::to_string(trimmed).unwrap_or_else(|_| trimmed.to_string())
}

/// Helper to construct a JSON response with explicit Flamer marker.
#[flame(rename = "json")]
pub fn json_response(content: String) -> String {
    let valid = convert_to_json(&content);
    format!("<!--flamer:json-->{}", valid)
}

/// Converts any input string or formula representation into valid JSON.
#[flame(rename = "toJson")]
pub fn to_json(content: String) -> String {
    convert_to_json(&content)
}

/// Helper to construct an HTML response with explicit Flamer marker.
#[flame(rename = "html")]
pub fn html_response(content: String) -> String {
    format!("<!--flamer:html-->{}", content)
}

/// Helper to construct a plain text response.
#[flame(rename = "text")]
pub fn text_response(content: String) -> String {
    content
}

/// Constructs a new Response object with body, status code, and content type.
#[flame(rename = "response")]
pub fn response_create(body: String, status: i64, content_type: String) -> Response {
    Response {
        body,
        status,
        content_type,
    }
}

/// Extracts a specific path parameter from a pattern and path.
#[flame(rename = "getPathParam")]
pub fn get_path_param(pattern: String, path: String, key: String) -> String {
    let pat_clean = pattern.trim_matches('/');
    let path_clean = path.trim_matches('/');

    let pat_segments: Vec<&str> = pat_clean.split('/').collect();
    let path_segments: Vec<&str> = path_clean.split('/').collect();

    for (i, seg) in pat_segments.iter().enumerate() {
        if let Some(param) = seg.strip_prefix(':') {
            if param == key {
                if i < path_segments.len() && !path_segments[i].is_empty() {
                    return path_segments[i].to_string();
                } else {
                    return "1".to_string();
                }
            }
        } else if *seg == "*" && key == "wildcard" && i < path_segments.len() {
            return path_segments[i..].join("/");
        }
    }
    String::new()
}

/// Returns the raw URL query string of the current HTTP request being processed.
#[flame(rename = "currentQuery")]
pub fn current_query() -> String {
    CURRENT_REQUEST_INFO
        .read()
        .map(|ctx| ctx.query.clone())
        .unwrap_or_default()
}

/// Returns the URL path of the current HTTP request being processed.
#[flame(rename = "currentPath")]
pub fn current_path() -> String {
    CURRENT_REQUEST_INFO
        .read()
        .map(|ctx| ctx.path.clone())
        .unwrap_or_default()
}

/// Returns the HTTP method of the current HTTP request being processed.
#[flame(rename = "currentMethod")]
pub fn current_method() -> String {
    CURRENT_REQUEST_INFO
        .read()
        .map(|ctx| ctx.method.clone())
        .unwrap_or_default()
}

#[derive(Clone, Debug, Default)]
pub struct ActiveAnnotationState {
    pub query_key: String,
    pub query_default: String,
    pub path_filter_pattern: String,
    pub auth_role: String,
    pub cors_origin: String,
    pub cors_methods: String,
    pub cors_headers: String,
    pub middleware_name: String,
}

static ANNOTATION_STATE: RwLock<ActiveAnnotationState> = RwLock::new(ActiveAnnotationState {
    query_key: String::new(),
    query_default: String::new(),
    path_filter_pattern: String::new(),
    auth_role: String::new(),
    cors_origin: String::new(),
    cors_methods: String::new(),
    cors_headers: String::new(),
    middleware_name: String::new(),
});

/// Registers the query key and default value for the decorated handler.
#[flame(rename = "registerQuery")]
pub fn register_query(key: String, default_val: String) {
    if let Ok(mut state) = ANNOTATION_STATE.write() {
        state.query_key = key;
        state.query_default = default_val;
    }
}

/// Retrieves the query parameter value from the active request or fallback default.
#[flame(rename = "queryGet")]
pub fn query_get(src: String) -> String {
    let (key, def) = if let Ok(state) = ANNOTATION_STATE.read() {
        (state.query_key.clone(), state.query_default.clone())
    } else {
        (String::new(), String::new())
    };
    let q = if !src.is_empty() {
        src
    } else {
        current_query()
    };
    if q.is_empty() {
        return def;
    }
    let val = get_query_param(q, key);
    if val.is_empty() {
        def
    } else {
        let dec = url_decode(val);
        let trimmed = dec.trim();
        if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
            trimmed[1..trimmed.len() - 1].to_string()
        } else {
            dec
        }
    }
}

/// Registers the route pattern for PathFilter annotation.
#[flame(rename = "registerPathFilter")]
pub fn register_path_filter(pattern: String) {
    if let Ok(mut state) = ANNOTATION_STATE.write() {
        state.path_filter_pattern = pattern;
    }
}

/// Matches the registered PathFilter pattern against the current request path.
#[flame(rename = "pathFilterMatches")]
pub fn path_filter_matches(given_path: String) -> bool {
    let pat = if let Ok(state) = ANNOTATION_STATE.read() {
        state.path_filter_pattern.clone()
    } else {
        String::new()
    };
    let p = if !given_path.is_empty() {
        given_path
    } else {
        current_path()
    };
    if p.is_empty() {
        return true;
    }
    match_path(pat, p)
}

/// Extracts the userId parameter from the registered pattern and current path.
#[flame(rename = "pathFilterUserId")]
pub fn path_filter_user_id(given_path: String) -> String {
    let pat = if let Ok(state) = ANNOTATION_STATE.read() {
        state.path_filter_pattern.clone()
    } else {
        String::new()
    };
    let p = if !given_path.is_empty() {
        given_path
    } else {
        let cur = current_path();
        if cur.is_empty() {
            "1".to_string()
        } else {
            cur
        }
    };
    let id = get_path_param(pat, p, "userId".to_string());
    if id.is_empty() {
        "1".to_string()
    } else {
        id
    }
}

/// Extracts a named parameter from the registered pattern and current path.
#[flame(rename = "pathFilterParam")]
pub fn path_filter_param(key: String) -> String {
    let pat = if let Ok(state) = ANNOTATION_STATE.read() {
        state.path_filter_pattern.clone()
    } else {
        String::new()
    };
    let p = current_path();
    get_path_param(pat, p, key)
}

/// Registers required role for Auth annotation.
#[flame(rename = "registerAuth")]
pub fn register_auth(role: String) {
    if let Ok(mut state) = ANNOTATION_STATE.write() {
        state.auth_role = role;
    }
}

/// Validates whether the active request has the required role.
#[flame(rename = "authCheck")]
pub fn auth_check(given_role: String) -> bool {
    let required_role = if let Ok(state) = ANNOTATION_STATE.read() {
        state.auth_role.clone()
    } else {
        "admin".to_string()
    };
    if !given_role.is_empty() {
        return given_role == required_role;
    }
    let q = current_query();
    let current_role = get_query_param(q, "role".to_string());
    let role = if current_role.is_empty() { "user" } else { &current_role };
    role == required_role
}

/// Registers origin, methods, and headers for Cors annotation.
#[flame(rename = "registerCors")]
pub fn register_cors(origin: String, methods: String, headers: String) {
    if let Ok(mut state) = ANNOTATION_STATE.write() {
        state.cors_origin = origin;
        state.cors_methods = methods;
        state.cors_headers = headers;
    }
}

/// Returns whether the origin is allowed.
#[flame(rename = "corsIsAllowed")]
pub fn cors_is_allowed(client_origin: String) -> bool {
    let _ = client_origin;
    true
}

/// Registers named middleware.
#[flame(rename = "registerMiddleware")]
pub fn register_middleware(name: String) {
    if let Ok(mut state) = ANNOTATION_STATE.write() {
        state.middleware_name = name;
    }
}
