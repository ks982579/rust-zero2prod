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

#### Database Support

Our test only checks if 200 is returned.
We want to confirm the _side-effect_ of user data being stored.
We need a database to store that data, and so setup a PostgreSQL database with docker for yourself.

The book covers a nice script to invoke Docker to create a database for us.
Use it running `./scripts/init_db.sh`.
Check with `docker ps`.
I love when things just work :)

Probably should have given it a tag / name, but I'll live with "dazzling_gould" for now.

Using `sqlx-cli` requires installing its CLI to manage database migrations.
And we added a lot of additional things such as checks for tools and race conditions to the script.

```bash
>> export DATABASE_URL=postgres://postgres:password@127.0.0.1:5432/newsletter
>> sqlx migrate add create_subscriptions_table
```

You should get a "migrations" directory now.
Unfortunately, it looks like it only creates an empty file.
It is up to you to add the SQL to update the database!
In our case, you create the `subscriptions` table.

The author discusses how using SQL constraints can impact write throughput, but not something we probably need to consider here.
Run migrations with `sqlx migrate run`.

If database is already running, skip docker command with:

```bash
SKIP_DOCKER=true ./scripts/init_db.sh
```

sqlx keeps track of its migrations in a `_sqlx_migrations` table... so that name is already taken.
Now, add sqlx as a dependency.

```bash
cargo add sqlx --features runtime-tokio-rustls,macros,postgres,uuid,chrono,migrate
```

The book suggests adding `[dependencies.sqlx]` section to avoid a long line it the `Cargo.toml` file.
They also disable default features... Live life on the edge.

For the `configuration.rs` file, I think the book refers to using the [Config Crate | crates.io](https://crates.io/crates/config).
You can add with `cargo add config`, and it has many feature flags too.

With the new configuration, check to see that the test (currently) forms a connection to the database.

```rust
use reqwest::Response;
use sqlx::{Connection, PgConnection};
use std::net::TcpListener;
use zero2prod::configuration::{get_configuration, Settings};
use zero2prod::startup::run;
// {...}
#[tokio::test]
async fn subscribe_returns_200_for_valid_form_data() {
    // Arrange
    let app_address: String = spawn_app();
    // We want to connect to the database also
    let configuration: Settings = get_configuration().expect("Failed to read configuration.");
    let connection_string: String = configuration.database.connection_string();
    // Note: `Connection` trait must be in scope to invoke
    // `PgConnection::connect` - it is not an inherent method of the struct!
    // Also, the return type of `.connect()` is wild...
    let connection: PgConnection = PgConnection::connect(&connection_string)
        .await
        .expect("Failed to connect to Postgres");
    let client: reqwest::Client = reqwest::Client::new();

    // Act
    let body: &str = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    let response: Response = client
        .post(&format!("{}/subscriptions", &app_address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert_eq!(200, response.status().as_u16());
}
```

I try to add the types manually so I know what they are and can refer back.
If you have a better memory or an more proficient than I, you may leave them off and the compiler will figure it out... mostly.


The `query!` macro verifies the returned struct is valid at run time.
It returns an anonymous record type and needs the DATABASE_URL
to verify with, which must be supplied in the `.env` file.
Some how that query reads the `.env` file and finds what it is looking for. 
It's that or we re-export the environment variable every time...

Actix-Web gives us ability to pass other pieces of data around, not related to the lifecycle of a single
incoming request, called the _application state_.
Do this by adding your _thing_ in the `App::new().app_data(thing)` method!

The `HttpServer` returns worker processes for each available core on the machine.
Each runs its own copy of application built by this by calling the same closure. 
So, we need **the same connection** for each copy of App.
But `PgConnection` doesn't implement `Clone` because it sits on a non-cloneable system, TCP connection with Postgres.

The `web::Data` is an Actix-Web extractor that wraps the connection in an Atomic Reference Counted pointer (Arc).

