# Zero to Production in Rust

## In the Beginning...

After setting up with cargo, we can 

```bash
cargo install cargo-watch
cargo watch -x check
# OR
cargo watch -x check -x test -x run
```

This runs cargo check after every code change.
The latter command is a chain of commands, to check, test, and run.
The commands stop if the chain breaks. 
This is the _inner development loop_.

To measure code coverage:

```bash
cargo install cargo-tarpaulin
cargo tarpaulin --ignore-tests
```

The latter will measure the test coverage and ignore the tests themselves, which makes sense.
Further, one will check linting and formatting with the following:

```bash
rustup component add clippy
cargo clippy -- -D warnings # fails a pipeline on warnings

rustup component add rustfmt
cargo fmt -- -check
```

It's possible to configure Clippy inline or in a `clippy.toml` file.
Rust formatter can also be configured with a `rustfmt.toml` file as well. 
And if security is a concern:

```bash
cargo install cargo-audit
cargo audit
```

It's a package maintained by The Rust SecureCode working group.
It checks if any of the crates in your dependency tree have reported vulnerabilities.

## Email Newletter Starter Notes

Might be worth noting this is a basic project, and users wont be given ability to _unsubscribe_.
So do not put into production without critical features such as that.

### Signing up

Starting our first server and we use the `HttpServer` struct that handles all _transport level_ concerns. 
Then, the `App` handles the application logic, routing, middlewares, request handlers, etc...
You can checkout [Actix-Web | docs.rs/actix-web](https://docs.rs/actix-web/4.0.1/actix_web/web/fn.get.html) docs.
For context, the `App::new().route(self, path: &str, route: Route) -> Self`. 
This means that `web` is a module, and `web::get() -> Route` is a function in that module that returns a `Route`. 

Opened a can of worms here...
[Route | docs.rs](https://docs.rs/actix-web/4.0.1/actix_web/struct.Route.html) has many methods.
We merely use the `.to<F, Args>(self, handler: F) -> Self` method to call a handler function.
The `Args` parameter is passed into the handler, which should be like a function type, but is `F: Handler<Args>` in the docs.

There's another concept of **Guards**, which specify a condition to be met before passing over to handler.
So a bit like middleware. 
How is this relevant?
The `web::get()` is short for `Route::new().guard(guard::Get())`.
In the docs, you can see (sort of) the macro call to create this.
This essentially says pass the request to the handler iff it is an HTTP GET method. 

Before I change everything to be more realistic with the health check, this is the ground work:

```rust
use actix_web::{web, App, HttpRequest, HttpServer, Responder};

/// A type implements the `Responder` trait if it can be converted into `HttpResponse` type.
async fn greet(req: HttpRequest) -> impl Responder {
    // match_info works with dynamic path segments.
    let name = req.match_info().get("name").unwrap_or("World");
    format!("Hello {}!", name)
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    // HttpServer for binding to TCP socket, maximum number of connections
    // allowing transport layer security, and more.
    HttpServer::new(|| {
        App::new()
            .route("/", web::get().to(greet))
            .route("/{name}", web::get().to(greet))
    })
    .bind("127.0.0.1:8000")?
    .run()
    .await
}
```

With the health check endpoint, test with `curl -v localhost:8000/health-check` or whatever works best for you.

Now, we created the "health-check" endpoint that returns something which implements the `Responder` trait.
It's worth noting the intention of said trait [Responder](https://docs.rs/actix-web/4.0.1/actix_web/trait.Responder.html),
has a method called `respond_to`, which returns an `HttpResponse`. 
So we can cut out the middleman and just return the `HttpResponse`...

```rust
/// We don't have to pass in `_req` surprisingly.
async fn health_check(_req: HttpRequest) -> HttpResponse {
    // HttpResponse is OK because Responder converts to it anyway
    // HttpResponse::Ok gives us a builder with default 200 status code
    // You could use `.finish()` to build, but the builder itself implements the Responder trait
    HttpResponse::Ok().finish()
}
```

#### Testing Endpoints

We don't want to accidently break APIs when we refactor or add features.
Black Box testing validates the behaviour of a system by giving it inputs and examining the outputs.
It does not analyze any of the internal logic.
Could be the worst code, but the test will pass if the output is correct.

We cannot just call the function of the endpoint directly, especially if it takes an HTTP Request argument.
Else, we would have to build the request first.
That would be a unit test though.
It doesn't check that it is invoked with a GET request, or that the endpoint/URL is correct.

What do we want with our integration tests?
+ Highly decoupled from technology underpinning API implementation (in case we change frameworks)
+ We want to test on our server... but cannot write a function that returns `App` due to technical limitations

We want to throw integration tests into a separate directory, but that requires some setup.
Fix the `Cargo.toml` file. Note TOML syntax. 
Specifying the bin of 'src/main.rs' isn't completely necessary,
but spells everything out to provide a complete picture in the configuration.

The book didn't have the library name specified...
But I named my project "rust-zero2prod", so it wasn't coming in _automagically_.

Out tests require `reqwest`, a higher-level HTTP client. 
Add with `cargo add --dev reqwest` to list as "dev-dependencies".
This means it will not complie with final application binary.

The test is nicely decoupled but we need to _spawn_ the app.
We cannot simply call the `run()` method because...
Basically that method is an infinite loop, always listening for requests.
As such, it doesn't return anything and therefore, the tests cannot progress.
This means we need to rework our `run()` method!

We remove the `async` characteristic of the function and merely return a Result holding the server.
The main function can unwrap the result, or error (with the `?`), and await the server.

I am trying to run tests and something isn't working correctly.
The terminal is yelling at me to `sudo apt install pkg-config`... So OK...
That lead to me not having OpenSSL installed on my Ubuntu distro.
Simply...

```bash
sudo apt-get install libssl-dev
```

Or search, in your preferred search engine, how to install OpenSSL on your OS.

Wow, so the server looks like this:

```rust
fn spawn_app() -> () {
    let server = zero2prod::run().expect("Failed to bind address");
    let _ = tokio::spawn(server);
}
```

I was looking at the [Tokio Documentation | tokio.rs](https://tokio.rs/tokio/tutorial/spawning), which is handy.
But hard to get through, quite technical.
I think it's worth reviewing the tutorial section at least.

We also don't like hardcoding in the binding address, what if that socket is taken?
We cannot run tests in parallel if we only use the one socket...
I wasn't aware, but binding to port 0 tells the OS to find an available port for the program.
This means we need to store that information to pass around the tests though.

#### HTML Forms

We consider many things, namely what information we receive and how to receive it.
You can accept the "Content-Type: application/x-www-form-urlencoded".
I would rather use JSON... but the book goes in this direction. 
It uses percent encoding where `%20` is a space and `%40` is the at symbol.
If the information is provided, great; else, return 400 status.
Build up the test first.

Once tests are done we start easy, just always returning status 200:

```rust
// #! src/lib.rs
async fn subscribe() -> HttpResponse {
    HttpResponse::Ok().finish()
}
```

Add the route to the router!
Actix-Web comes with some _extractors_, [`actix_web::web::Form`](https://docs.rs/actix-web/4.0.1/actix_web/web/struct.Form.html) is one.
It looks like you need a struct that can derive **Deserialize**, and then pass the form into your endpoint.
However, we must add `serde` now to our code.

The book then goes into a [Serde | serde.rs](https://serde.rs/) deep-dive to understand how the conversion works.
Or [Serde | docs.rs](https://docs.rs/serde/latest/serde/index.html).
Or the more specific [`serde_urlencoded` | docs.rs](https://docs.rs/serde_urlencoded/latest/serde_urlencoded/index.html).
Serde itself defines a set of interfaces for (de)serialisation from/to data formats.
Serde is agnostic with respect to data formats.
Once your type (struct) implements `Serialize`, you can use any _concrete_ implementation of it.
And most, if not all, commonly used data formats can be found on crates.io.

It is fast thanks to a process called _monomorphization_. 
It (compiler) makes copies of the generic function with concrete types.
This is also called zero-cost abstraction. 
Further, other languages leverage "runtime reflection" to fetch information about types to (de)serialize.
Rust requires everything up front and does not provide runtime reflection.
