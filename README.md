# Flamer

***Express.js inspired backend framework for Flame.***

- *Leverages Rust **tokio** and **axum*** for maximum throughput and native async I/O.
- *Easy to use inside **Flame*** via scope-injecting annotations and camelCase helpers.
- *Built-in path filtering, dynamic route parameter extraction, query string parsing, and response utilities.*

---

## Installation

Add `flamer` as a dependency using the Flame package manager:

```shell
fmp add https://github.com/shoya-129/flamer
```

Alternatively, manually add it to your `flame.toml`:

```toml
[dependencies]
flamer = "https://github.com/shoya-129/flamer"
```

---

## Quickstart

```flame
import flamer

// Define route handlers outside @Flamer scope for full access to flamer package helpers:
fn handleHome() {
    return flamer.html("<h1>Welcome to Flamer!</h1>")
}

fn handleUsers() {
    return flamer.json(["Alice", "Bob", "Charlie"])
}

fn handleEcho(body: Formula) {
    return body
}

@flamer.Flamer(port: 3000)
async fn main() {
    // Register routes
    flamer.get("/", handleHome)
    flamer.get("/api/users", handleUsers)
    flamer.post("/api/echo", handleEcho)

    await flamer.listen()
}

await main()
```

---

## ⚠️ Scope Notice & Handler Architecture

### Why does `flamer.json()` or `flamer.html()` fail inside `@Flamer`?
When you annotate a function with `@Flamer(port: 3000)` (or `@flamer.Flamer`), Flame injects a local variable named `flamer` into the function scope. This local variable is the **`FlamerServer` instance** (providing `.get()`, `.post()`, `.listen()`, etc.), which **shadows** the imported package `flamer`.

Because of this shadowing:
- Calling `flamer.json(...)` or other package-level helpers inside the `@Flamer` function or its inline closures fails because `flamer` is the server instance, not the package.
- Calling bare `html(...)` or `json(...)` without named imports fails because they are not imported into the global namespace.

---

### Solution 1: Define Handlers Outside `@Flamer` Scope (Recommended)
Defining route handler functions **outside** of the `@Flamer` function scope is the cleanest, most modular architectural pattern:

```flame
import flamer

// Handlers are defined in module scope:
fn getHome() {
    return flamer.html("<h1>Hello from Flamer!</h1>")
}

fn getUsers() {
    return flamer.json(["Alice", "Bob"])
}

@flamer.Flamer(port: 3000)
async fn main() {
    flamer.get("/", getHome)
    flamer.get("/api/users", getUsers)

    await flamer.listen()
}

await main()
```

---

### Solution 2: Named Imports (for Inline Closures)
If you prefer writing inline closures inside `@Flamer`, import the helper functions directly:

```flame
import flamer.{Flamer, html, json, text, status, notFound}

@Flamer(port: 3000)
async fn main() {
    flamer.get("/", () {
        html("<h1>Hello from Flamer!</h1>")
    })

    flamer.get("/api/users", () {
        json(["Alice", "Bob"])
    })

    await flamer.listen()
}

await main()
```

---

### Solution 3: Alias the Package Import
You can also alias the package when importing to prevent any name collision between the package and the injected `flamer` server instance:

```flame
import flamer as f

@f.Flamer(port: 3000)
async fn main() {
    // `flamer` is the FlamerServer instance
    // `f` is the flamer package
    flamer.get("/", () {
        f.html("<h1>Hello from Flamer!</h1>")
    })

    flamer.get("/api/users", () {
        f.json(["Alice", "Bob"])
    })

    await flamer.listen()
}

await main()
```

---

## Routing Shortcuts

`flamer` provides route shortcut methods matching standard HTTP verbs:

- `flamer.get(path, handler)` - Registers a GET endpoint
- `flamer.post(path, handler)` - Registers a POST endpoint (passes incoming body payload to handler)
- `flamer.put(path, handler)` - Registers a PUT endpoint (passes incoming body payload to handler)
- `flamer.delete(path, handler)` - Registers a DELETE endpoint
- `flamer.patch(path, handler)` - Registers a PATCH endpoint (passes incoming body payload to handler)
- `flamer.head(path, handler)` - Registers a HEAD endpoint
- `flamer.options(path, handler)` - Registers an OPTIONS endpoint
- `flamer.trace(path, handler)` - Registers a TRACE endpoint
- `flamer.any(path, handler)` - Matches any HTTP method

---

## Path Filtering & Route Parameter Extraction

### Dynamic Path Matching (`matchPath`)

Test whether an incoming request path matches a route pattern, including `:param` placeholders and `*` wildcards:

```flame
import flamer

flamer.matchPath("/users/:id", "/users/42")          // true
flamer.matchPath("/static/*", "/static/css/app.css") // true
flamer.matchPath("/api/v1", "/dashboard")            // false
```

### Parameter Extraction (`extractPathParams`)

Extract named route segments directly into a typed Formula object:

```flame
import flamer

let params = flamer.extractPathParams("/teams/:teamId/members/:memberId", "/teams/alpha/members/42")

println(params.teamId)   // "alpha"
println(params.memberId) // "42"
```

### Path Filtering (`pathFilter`)

Check if a URL path matches a prefix filter for group routing or authentication gates:

```flame
import flamer

if flamer.pathFilter("/api", "/api/v1/checkout") {
    println("Matches API route prefix")
}
```

---

## Query String Parsing & URL Utilities

### Parsing Query Strings (`parseQuery`)

Converts standard URL query strings into accessible Formula key-value objects:

```flame
import flamer

let query = flamer.parseQuery("search=flame&page=2&sort=asc")

println(query.search) // "flame"
println(query.page)   // "2"
println(query.sort)   // "asc"
```

### Single Parameter Lookup (`getQueryParam`)

Extract a single parameter with an optional fallback:

```flame
import flamer

let term = flamer.getQueryParam("search=flame", "search", "")
let limit = flamer.getQueryParam("search=flame", "limit", "10") // returns "10"
```

### URL Encoding & Decoding

```flame
import flamer

let encoded = flamer.urlEncode("hello world & more")
println(encoded) // "hello+world+%26+more"

let decoded = flamer.urlDecode(encoded)
println(decoded) // "hello world & more"
```

---

## Response Helper Functions

Quickly return formatted responses from your handlers:

| Helper | Description | Example |
| :--- | :--- | :--- |
| `flamer.json(data)` | Formats Formula / Array as JSON string | `flamer.json({ ok: true })` |
| `flamer.text(msg)` | Plain text response | `flamer.text("OK")` |
| `flamer.html(markup)` | HTML response string | `flamer.html("<h1>Title</h1>")` |
| `flamer.redirect(url, code)` | Redirect response formula (default: 302) | `flamer.redirect("/login")` |
| `flamer.status(code, body)` | Structured HTTP status formula | `flamer.status(204, "")` |
| `flamer.ok(msg)` | 200 OK text response | `flamer.ok("Success")` |
| `flamer.created(data)` | 201 Created formula | `flamer.created({ id: 101 })` |
| `flamer.badRequest(err)` | 400 Bad Request error formula | `flamer.badRequest("Invalid email")` |
| `flamer.unauthorized(err)`| 401 Unauthorized error formula | `flamer.unauthorized("Auth required")` |
| `flamer.forbidden(err)` | 403 Forbidden error formula | `flamer.forbidden("Access denied")` |
| `flamer.notFound(err)` | 404 Not Found error formula | `flamer.notFound("Item missing")` |
| `flamer.internalError(err)`| 500 Internal Server Error formula | `flamer.internalError("Database down")` |

---

## Route & Filter Annotations

In addition to programmatic routing, `flamer` exports decorators to annotate handlers:

- `@Get(path = "/")` - Marks a GET endpoint
- `@Post(path = "/")` - Marks a POST endpoint
- `@Put(path = "/")` - Marks a PUT endpoint
- `@Delete(path = "/")` - Marks a DELETE endpoint
- `@Patch(path = "/")` - Marks a PATCH endpoint
- `@Head(path = "/")` - Marks a HEAD endpoint
- `@Options(path = "/")` - Marks an OPTIONS endpoint
- `@Route(path = "/", method = "GET")` - Configures custom HTTP method route
- `@PathFilter(pattern = "*")` - Attaches path filter metadata
- `@Query(key = "")` - Declares required query parameter
- `@Cors(origin = "*", methods = "GET,POST,PUT,DELETE,OPTIONS")` - Declares CORS policy
- `@Auth(role = "user")` - Restricts endpoint access by role
- `@Middleware(name = "logger")` - Binds named middleware to a handler

---

## Telegram Bot Example

```flame
import std.net.http
import std.json
import flamer

// Replace with your actual Bot Token from BotFather
let bot_token = "YOUR_BOT_TOKEN_HERE"

async fn webhook(body: Formula) -> Formula {
    // Parse the incoming JSON request body from Telegram
    let data = json.parse(body)

    // Extract the sender's Chat ID and the message text
    let chat_id = data.message.chat.id
    let text = data.message.text

    println($"Received from {chat_id}: {text}")

    let mut reply_text = ""

    // Simple command routing
    if text == "/start" {
        reply_text = "Welcome to Flame Bot! Send me a message."
    } else {
        reply_text = $"You said: {text}"
    }

    // Build the Telegram API request url
    let send_url = $"https://api.telegram.org/bot{bot_token}/sendMessage"

    // Prepare the JSON payload
    let payload = {
        chat_id: chat_id,
        text: reply_text
    }

    // Fire the HTTP POST request to Telegram
    let send_res = await http.post(send_url, payload)

    println($"Telegram response: {send_res.text()}")

    // Return a successful HTTP 200 response
    return {
        ok: true
    }
}

@Flamer(port: 3000)
async fn main() {
    println("--- Flame Telegram Bot Webhook Server ---")
    println("Server listening on port 3000.")
    println("Point your Telegram webhook to: /webhook")

    // Setup routing
    flamer.post("/webhook", webhook)

    // Start listening asynchronously
    await flamer.listen()
}

await main()
```

---

## License

ISC License. Built for [Flame](https://github.com/shoya-129/flame).
