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
use std::{collections::HashMap, mem, net::SocketAddr};

/// Represents an HTTP Server powered by Axum and Tokio.
pub struct FlamerServer {
    router: Router,
    port: u16,
    host: String,
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

/// Create a new Flamer server instance.
pub fn init() -> FlamerServer {
    FlamerServer {
        router: Router::new(),
        port: 3000,
        host: "127.0.0.1".to_string(),
    }
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
        self.router = mem::take(&mut self.router).route(path, get(handler));
    }

    /// Registers a POST route handler.
    pub fn post<H, T>(&mut self, path: &'static str, handler: H)
    where
        H: axum::handler::Handler<T, ()> + Clone + Send + Sync + 'static,
        T: 'static,
    {
        self.router = mem::take(&mut self.router).route(path, post(handler));
    }

    /// Registers a PUT route handler.
    pub fn put<H, T>(&mut self, path: &'static str, handler: H)
    where
        H: axum::handler::Handler<T, ()> + Clone + Send + Sync + 'static,
        T: 'static,
    {
        self.router = mem::take(&mut self.router).route(path, put(handler));
    }

    /// Registers a DELETE route handler.
    pub fn delete<H, T>(&mut self, path: &'static str, handler: H)
    where
        H: axum::handler::Handler<T, ()> + Clone + Send + Sync + 'static,
        T: 'static,
    {
        self.router = mem::take(&mut self.router).route(path, delete(handler));
    }

    /// Registers a PATCH route handler.
    pub fn patch<H, T>(&mut self, path: &'static str, handler: H)
    where
        H: axum::handler::Handler<T, ()> + Clone + Send + Sync + 'static,
        T: 'static,
    {
        self.router = mem::take(&mut self.router).route(path, patch(handler));
    }

    /// Registers a HEAD route handler.
    pub fn head<H, T>(&mut self, path: &'static str, handler: H)
    where
        H: axum::handler::Handler<T, ()> + Clone + Send + Sync + 'static,
        T: 'static,
    {
        self.router = mem::take(&mut self.router).route(path, head(handler));
    }

    /// Registers an OPTIONS route handler.
    pub fn options<H, T>(&mut self, path: &'static str, handler: H)
    where
        H: axum::handler::Handler<T, ()> + Clone + Send + Sync + 'static,
        T: 'static,
    {
        self.router = mem::take(&mut self.router).route(path, options(handler));
    }

    /// Registers a TRACE route handler.
    pub fn trace<H, T>(&mut self, path: &'static str, handler: H)
    where
        H: axum::handler::Handler<T, ()> + Clone + Send + Sync + 'static,
        T: 'static,
    {
        self.router = mem::take(&mut self.router).route(path, trace(handler));
    }

    /// Registers a route matching any HTTP method.
    pub fn any<H, T>(&mut self, path: &'static str, handler: H)
    where
        H: axum::handler::Handler<T, ()> + Clone + Send + Sync + 'static,
        T: 'static,
    {
        self.router = mem::take(&mut self.router).route(path, any(handler));
    }

    /// Returns the underlying Axum Router instance with auto content-type detection.
    pub fn router(&mut self) -> Router {
        mem::take(&mut self.router).layer(middleware::from_fn(auto_content_type_middleware))
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

        let app = mem::take(&mut self.router).layer(middleware::from_fn(auto_content_type_middleware));

        axum::serve(listener, app)
            .await
            .map_err(std::io::Error::other)
    }

    /// Helper on the server instance to return an HTML response string.
    pub fn html(&self, content: String) -> String {
        content
    }

    /// Helper on the server instance to return a plain text response string.
    pub fn text(&self, content: String) -> String {
        content
    }

    /// Helper on the server instance to return a 200 OK response string.
    pub fn ok(&self, body: String) -> String {
        body
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
        parts.headers.insert(
            header::CONTENT_LENGTH,
            HeaderValue::from(new_bytes.len()),
        );
        return AxumResponse::from_parts(parts, Body::from(new_bytes));
    }

    // Check for explicit JSON marker from flamer.json()
    if let Some(rest) = text_content.strip_prefix("<!--flamer:json-->") {
        parts.headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json; charset=utf-8"),
        );
        let new_bytes = Bytes::copy_from_slice(rest.as_bytes());
        parts.headers.insert(
            header::CONTENT_LENGTH,
            HeaderValue::from(new_bytes.len()),
        );
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
            if i < path_segments.len() {
                map.insert(key.to_string(), path_segments[i].to_string());
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
    pth == pfx || pth.starts_with(&format!("{}/", pfx))
}
