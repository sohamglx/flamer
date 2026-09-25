# Flamer

***Express.js inspired backend framework for Flame.***

- *Leverages Rust **tokio** and **axum*** for maximum throughput and native async I/O.
- *Easy to use inside **Flame*** via scope-injecting annotations and camelCase helpers.
- *Built-in path filtering, dynamic route parameter extraction, query string parsing, and response utilities.*

---

## Installation

Add `flamer` as a dependency using the Flame package manager:

```shell
fmp add https://github.com/sohamglx/flamer
```

Alternatively, manually add it to your `flame.toml`:

```toml
[dependencies]
flamer = "https://github.com/sohamglx/flamer"
```

---

## Quickstart

```flame
import flamer

@Flamer(port: 3000)
async fn main() {
    // Route shortcuts
    flamer.get("/", () {
        flamer.html("<h1>Welcome to Flamer!</h1>")
    })

    flamer.get("/api/users", () {
        flamer.json(["Alice", "Bob", "Charlie"])
    })

    flamer.post("/api/echo", (body) {
        return body
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
| `json(data)` | Formats Formula / Array as JSON string | `json({ ok: true })` |
| `text(msg)` | Plain text response | `text("OK")` |
| `html(markup)` | HTML response string | `html("<h1>Title</h1>")` |
| `redirect(url, code)` | Redirect response formula (default: 302) | `redirect("/login")` |
| `status(code, body)` | Structured HTTP status formula | `status(204, "")` |
| `ok(msg)` | 200 OK text response | `ok("Success")` |
| `created(data)` | 201 Created formula | `created({ id: 101 })` |
| `badRequest(err)` | 400 Bad Request error formula | `badRequest("Invalid email")` |
| `unauthorized(err)`| 401 Unauthorized error formula | `unauthorized("Auth required")` |
| `forbidden(err)` | 403 Forbidden error formula | `forbidden("Access denied")` |
| `notFound(err)` | 404 Not Found error formula | `notFound("Item missing")` |
| `internalError(err)`| 500 Internal Server Error formula | `internalError("Database down")` |

---

## Route & Filter Annotations

In addition to programmatic routing, `flamer` exports annotations to annotate handlers:

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

ISC License. Built for [Flame](https://github.com/sohamglx/flame).
