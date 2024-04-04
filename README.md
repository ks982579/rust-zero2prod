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

So, we wrap the connection and stuff into the `.app_data(thing.clone())` method with a clone.
Then, we make the actual connection in the `main()` function to pass into `run()` function.

In the `subscribe` function, we bring in a new `_connection: web::Data<PgConnection>` parameter.
Actix-web uses a _type-map_ to represent its application state.
This is a `HashMap` that stores arbitrary data (with `Any` type) against its unique type identifier (obtained via `TypeId::of`).
Other languages mightt call this "dependency injection".
Interesting to have it done in Rust.

Passing information to the database requires some additional imports.

```
cargo add uuid --features v4
cargo add chrono --no-default-features --features clock
```

Setting up the `subscribe` API with a `query!` to the database still gives and error.
More specifically, the `.execute(connection.get_ref().deref())` does not implement `sqlx_core::executor:Executor<'_>`.
Only the `&mut PgConnection` implements that trait because sqlx has an asynchronous interface. 
However, you cannot run multiple queries _concurrently_ over the same dabase connection.
Think of a mutable reference as a unique reference.
It guarantees to `execute` that they have exclusive access the PgConnection because Rust rules only allows one mutuable reference at a time. 

The problem, `web::Data` will not give us mutuable access to application state. 
Using a lock (Mutex) would allow for synchronise access to underlying TCP socket, leveraging interior mutability.
But not a great solution.

Instead of `PgConnection`, `PgPool` is a pool of connections to a Postgres database. 
It implements the `Executor` trait, and uses a kind of interior mutability.
When you run a query with `&PgPool`, sqlx borrows a PgConnection from the pool to execute the query,
or wait for one to free up,
or create a new one!
This increases number of concurrent queries our application can run, improving resiliency. 
A slow query will not impact the performance of all incoming requests by locking the connection.

We must begin our update in `main.rs`.
Then, we update the `run()` function in `startup.rs`.
Then the endpoint for subscriptions.

The `health_check.rs` tests also need to be updated. 
I guess the idea is to create a new stuct to hold necessary information and pass around.
We will call it `TestApp`.

The book leaves updating the tests to the reader.
When using the test pool, it's interesting to know that the `query!().fetch_one(&pool: &PgPool)`
requires something with the `Executor<'_>` trait.
Luckily, the compiler was helpful enough to suggest that "`Executor<'p>` is implemented for `&Pool<DB>`"
when I tried to pass in a clone.

The test _intent_ is clear now,
and we removed most biolerplate of establishing the DB connection (in the test itself).
I can happily report that my tests also passed.
But they will only pass once because the data persists in the database. 
And one of the constraints on the data is a unique field.

There are 2 ways to handle:
1. Wrap each tests in an SQL transaction and rollback at end of test.
2. spin up new database for each integration test.

The former would be much faster but for our integration tests,
unsure how to _capture_ the connection in the SQL transaction context.
The latter is slower but will be easier to implement. 
This means creating a new logical database with a unique name and running migrations on it.
To do this, we randomize the database name with UUID.
But then we need to not pass that into the connection, so we create a new method in configurations.
This will connect to the Postgres instance and not a specific database.

Then we create a new function in our test to connect to the database and run migrations...
Word of caution, you must `use sqlx::{Executor}` to execute the "CREATE DATABASE" command.
Interesting that we create the database with the `PgConnection` struct,
then migrate and return a `PgPool`.
The tests now run.
I wonder if we would want to implement our own drop function for this database connection.
The drop function could delete the database, else would we not rack up databases in our tests?

The book addresses the point above, saying we can add a clean-up step.
But if performance starts to suffer from the hundreds of near empty test databases,
we can just create a new instance. 
This database is only for testing after all. 

## Telemetry

p. 89

Don't deploy the app just yet, we don't know what we don't know.
There are too many "unknown unknowns".
That's why we need to collect telemetry data.

Things to consider:
+ What happens if we lose database connection? Will it try to reconnect or is that it?
+ What happens if an attacker tries passing malicious payloads into POST body? 

Those are actually **known unknowns**, we are aware but they are unmanaged. 
Unknown unknowns can happen when:
+ system pushed beyond usual operating conditions.
+ multiple components failures at same time.
+ No changes introduced for long time (system not restarted for while and memory leaks emerge).

They are similar in that they are nearly impossible to reporduce outside live environments.
And we cannot attach a debugger to a process in production.
**Telemetry Data** is information about the running application that is collected automatically,
which can be reviewed later.

Goal: Have an **Observable Application**.

For this we need to collect _high-quality_ telemetry data.

### Logging

Logs are the most common type of telemetry data. 
Rust has a [log | crates.io](https://crates.io/crates/log) crate.
It has 5 macros, each emitting a log at a different level:
+ error = used if an operation fails (user impact).
+ warn
+ info = used to record success of operation.
+ debug
+ trace = verbose and used to record things like when TCP packet is received.

But wait, Actix-Web provides a logger middleware?
We add to `startup.rs`.

```rust
// {...}
    let server: Server = HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .route("/health-check", web::get().to(health_check))
// {...}
```

Middleware is added with the `.wrap()` method. 
But, what happens to the logs when they are generated?
If you start the application, still nothing in the consolse...

The book goes into the "Facade Pattern", 
a structural design pattern.
I have the book, "Design Patterns", but there's also [Refactoring.guru](https://refactoring.guru/design-patterns).
This pattern provides an interface to a library to simplify working with it. 

The `log` crate leverages the facad pattern.
Basically, you have the tools and you get to decide how logs are displayed.
There's a `set_logger` function we can call in `main()`.
If we don't, logs are basically just discarded.

There are many Log implementations, listed in the docs of `log`.
We will use... `env_logger`, nice to print log records to the terminal.

```bash
cargo add env_logger
```

It should print logs in the following format to terminal:

```bash
[<timestamp> <level> <module path>] <log message>
```

So, you pass in something like `RUST_LOG=debug cargo run` so it can know what to print out. 
Sending `RUST_LOG=zero2prod` would filter out dependencies.
Update your main function!

We default setting to "info", but go ahead with `RUST_LOG=trace` to see some lower level events being logged.

Now, import the `log` dependency with `cargo add log`.
A rule of thumb, "any interaction with external systems over the network should be closely monitored."

