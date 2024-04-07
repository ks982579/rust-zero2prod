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
We update now the subscriptions endpoint...

Another quote, "We will happily settle for an application that is _sufficiently_ observable
to endable us to deliver the level of service we promised to our users."
This implies that we should debug errors with little effort. 
If a user gives us an email address and says, "I tried to sign up and got an error... please help,"
could we help them?
Currently, we would try searching logs for the email but would have to ask the user for more information.
That's a failure on _sufficient_ observability.
Improve subscriptions endpoint.

Note: SQLX logs use INFO level by default as well.

Because our system handles multiple requests concurrently,
the stream of logs can mix requests and become hard to read.
We need to correlate all logs somehow.
This can be done with a **request id** (sometimes called a "correlation id").
Basically, generate a random (UUID) id to associate logs.

This is good for an endpoint, but Actix-Web's Logger middleware isn't aware of the request id.
Side-effect is we do not know what status code is sent to our new user.
We _could_ remove actix_web's Logger and write our own middleware with a request id...
There would be such a rewrite effort, from Actix-Web to imported crates that this approach cannot scale.
Basically, "Logs are the wrong abstraction."

So, we have been using the wrong tool. 
We migrate from `log` to `tracing`

This is good for an endpoint, but Actix-Web's Logger middleware isn't aware of the request id.
Side-effect is we do not know what status code is sent to our new user.
We _could_ remove actix_web's Logger and write our own middleware with a request id...
There would be such a rewrite effort, from Actix-Web to imported crates that this approach cannot scale.
Basically, "Logs are the wrong abstraction."

So, we have been using the wrong tool. 
We migrate from `log` to `tracing`.

```bash
cargo add tracing --features log
```

If anything, I'm getting good at importing crates. 
Because we use the `log` feature, we can do a find-and-replace for "tracing".
But... we want to use tracing _span_ to represent the whole HTTP request.

```rust
// {...}
pub async fn subscribe(
    form: web::Form<FormData>,
    // recieving connection from application state!
    pool: web::Data<PgPool>,
) -> HttpResponse {
    // Generate random unique identifier
    let request_id = Uuid::new_v4();
    // Improving observability
    let request_span = tracing::info_span!(
        "Adding a new subscriber.",
        %request_id,
        subscriber_email = %form.email,
        subscriber_name = %form.name
    );
    // Don't actually use `enter` in async function, it is bad.
    let _request_span_guard = request_span.enter();
// {...}
```

What the hell is a [`tracing::span` | docs.rs](https://docs.rs/tracing/latest/tracing/span/index.html)?
The docs say a **span** represents a period of time the program was executing in a _particular_ context. 
You create the span and enter it in 2 separate steps in the manual implementation.

The book warns not to actually use `request_span.enter()` in async functions.
We will exit the span when the request span guard is dropped at the end of the function.
Also note how we attache values to the span context with the `%`.
This tell `tracing` to use the `Display` implementation for logging.
`tracing` allows us to associate structured information to spans as collection of key-value pairs.
See [tracing docs | crates.io](https://crates.io/crates/tracing) for more info.
They say the guard won't exit until the async block is complete. 
This leads to confusing, and incorrect, output.

Why?
We must explicitly step into the tracing with `.enter()` method to activate.
It returns an instance of `Entered`, which is a guard.
As long as the value is not dropped,
all downstream spans and logs are registered as children of the entered span.
Check our the Rust pattern "Resource Acquisition Is Initialization" (RAII).

To work asynchronously, we need to `use tracing::Instrument`!
It enters the span when the future is polled,
and exits the span when the future is parked.

Updating subscriptions endpoint,
we tuck the instrument method into the `sqlx::query`. 
We need to fix the `main()` function still, it's using `env_logger`. 
Following a facade pattern like `log`,
implementing the `Subscriber` trait exposes many methods to manange the lifecycle of a `Span`.

Of course, tracing doesn't provide a subscriber out of the box:

```bash
cargo add tracing-subscriber --features registry,env-filter
cargo add tracing-bunyan-formatter
```

We get another trait here called `Layer` that allows us to build a processing pipeline.
Check out [tracing-subscriber | crates.io](https://crates.io/crates/tracing-subscriber).
The `Registry` struct implements the `Subscriber` trait!

We will also use `tracing-bunyan-formatter` becauce it implements metadata inheritance. 
When you have this, update your main function (finally!).

Wow, So if you update the `main()` and curl "health-check"... you won't see anything. 
I don't think that endpoint is set with tracing.
But create a new user...

```bash
curl --request POST --data 'name=tom%20brady&email=tb%40tb.com' 127.0.0.1:8000/subscriptions --verbose
```

I think it hits the console in Json format.
When a Span is closed, JSON message is printed to the console.
But an issue arises because our terminal only shows logs directly emitted by our application.
Basically, tracing's log feature flag ensures a log record is emitted when tracing even happens.
But the opposite is not true. 
To get this...

```bash
cargo add tracing-log
```

Then, hook up in the `main()` function.
Looks like just initializing `LogTracer`.

### Remove Unused Dependencies

We have come a long way, still a ways to go, and already we have accumulated many dependencies.
Some, we aren't using anymore already.
Use this to remove

```bash
cargo install cargo-udeps
cargo +nightly udeps
```

That'll take a minute to fetch the tool.
And building literally takes a few minutes...
Then run on the nightly compiler (if necessary).
And if you don't have the [nightly compiler](https://doc.rust-lang.org/book/appendix-07-nightly-rust.html):

```bash
rustup toolchain list
rustup toolchain install nightly
```

It won't get everything though (like logs), which you may remove manually.

### Clean up

Starting with the `main()` function, we want to refactor.
Each function should have like one duty. 
Then, we split the functions into a new "telemetry.rs" file.

They are moved out so we can use them for our test suite as well. 
Rule of thumb; everything used in the application should be reflected in integration tests.
We then update the `spawn_app()` function in our testing suite. 
However, this app is called multiple times, once per test.
As mentioned before, we only want to initialize our subscriber once.
I ran the tests and only one passes, the rest will fail.

The book mentions we could use `std::sync::Once.call_once()` method.
But we may need to use our subscriber after initialization.
That means we will want `std::sync::SyncOnceCell`, which is not yet stable. 
Doing a little digging, I think it's moved to `std::cell::OnceCell`.
And the docs say for a thread-safe version use `std::sync::OnceLock`.

I want to try use this instead.
Ok, I read the [`OnceLock` docs | doc.rust-lang.org](https://doc.rust-lang.org/std/sync/struct.OnceLock.html),
this is a thread-safe cell that can only be written to once.
So, you call `get_or_init(fn -> F) -> &T` which either sets the cell with you function,
or gets you a reference of it. 
If it doesn't work then go back to page 118 to setup with correct 3rd-party package.

Cargo test by default eats output unless a test fails. 
You can run `cargo test -- --nocapture` to opt back into viewing those logs.
We will now add a new parameter to `get_subscriber` to allow customisation of what
_sink_ logs should be written to. 
The internet says, "A log sink is a place where logs from a system or application are 
collected and stored for later analysis."
Back to "telemetry.rs"!

We are adding a special `Sink` type using the `where` clause.
It uses a strange format though: `Sink: for<'a> MakeWriter<'a> + Send + Sync + 'static`.
This is a [**Higher-Ranked Trait Bound**](https://doc.rust-lang.org/nomicon/hrtb.html) (HRTB),
something straight out of the Nomicon...
It is basically like, you know when you return a reference from a function,
the function requires the lifetime to ensure it isn't returning like a _null_ reference?
This is syntax to help pass in a lifetime when needed by traits.
For the trait, it is read "for all choices of `'a`..." and produces an infinite list of trait bounds 
that our trait, usually a function, must satisfy. 
Adding an additional parameter to this function means passing `std::io::stdout` into it 
in the `main()` function also.

What is the [`MakeWriter`](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/fmt/trait.MakeWriter.html) trait?

How do we handle this in our test suite now?
We initialize the subscriber based on `TEST_LOG` environment variable coming in.
Technically the `std::io::{stdout, sink}` are different types,
making it challenging to return a value and set it.
So we do this work-around of setting everything in the if statement. 

So, run `TEST_LOG=true cargo test health_check_works | bunyan`...
You can _prettify_ output if you `cargo install bunyan` and pass output to it (i guess).
Just passing in the environment variable helps to see output even on passing tests.

### Refactoring

Good time to look back at our subscriptions route.
As pointed out in the book, implementing logging has bloated out function,
or added noise as they say.
Interestingly enough, the `tracing` crate gives us the `tracing::instrument` procedural macro.
It can _wrap_ our `subscribe` function in a span.

Normally, function arguments aren't displayable on log records.
We can specify more explicitly what and how things are captured,
like using `skip()` to ignore things.
It also takes care to use `Instrument::instrument` if it is applied to asynchronous function.

What is the [`tracing::instrument` | docs.rs](https://docs.rs/tracing/latest/tracing/attr.instrument.html) macro?
Coming from a _Python_ background, I think of it like a decorator.
The docs suggest it is like a wrapper to create and enter a tracing span every time the function is called. 
We have implemented `skip()`, but there's also a `skip_all` which is not invoked like a function.

I lost a bunch of notes, that's great...

So, the default behaviour of that macro is to display all arguments by their debug trait.
That is a security event waiting to happen.
Luckily, there's a [`secrecy` | crates.io](https://crates.io/crates/secrecy) crate we can utilize.
You wrap your sensitive value in a struct which updates the `Debug` trait to say "Redacted" (or something).

Try wrapping the database password in the configuration Rust file.
It ends up breaking the connection string functions.
This is because secrecy has also removed the `Display` trait. 
We need to bring in the `ExposeSecret` trait, see the file for example.

This also means in the `main.rs` we must also expose the secret.
The tests also need to be updated, since they use a similar function that now returns `Secret<String>`.

### Adding Middleware!!!

Logs should be labeled with a reqeust id. 
We can bring in another crate.

```bash
cargo add tracing-actix-web
```

It is designed to be a drop-in replacement of actix-web logger, but based on tracing instead of log.
Just adding it basically to the `run()` function.
Note that we create a `request_id` on the `subscribe()` function that will override this new tracing one.
Remove it.

As a side note:
+ `tracing-actix-web` is OpenTelemetry-compatible. So, if you brought in `tracing-opentelemetry`,
you could ship your spans to an OpenTelemetry-compatible service!
+ `tracing-error` can make error types better for troubleshooting.

## Going Live?

This section will be very handy as we learn to dockerize our application and deploy it (to DigitalOcean).
Committing to the main brach will _automatically_ trigger the deployment of the latest version of our application.
The focus is on _philosphy_ because there are too many moving parts. 

### Dockerization of our App

First task is to write a Dockerfile.
Think of it like a recipe for the application environment. 
Getting our database ready is more trouble than you'd think.
We need to run the following:

```bash
cargo sqlx prepare --workspace
```

Ok, so I updated the version of the book when the command wasn't working.
The above command is working, and no need to add any weird "offline" feature.
We actually then pass in `SQLX_OFFLINE` environment variable as `true` in the Dockerfile.
We can build a Docker container now with:

```bash
docker build --tag zero2prod --file Dockerfile .
```

Something about using the `--check` flag to ensure our database doesn't fall out of sync with the json file.

Once the image is built, we can run it:

```bash
docker run zero2prod
```

However, it won't work because of the connection with Postgres.
In `main()` we use `connect_lazy`, which is not a future, so we don't await it.

We will also have issues with using '127.0.0.1' as our host address.
We instruct our application to only accept connections coming from the same machine. 
Using '0.0.0.0' instructs our application to accept connections from _any_ network interface.
We will use that for Docker only, and leave localhost for local development. 
Making adjustments to `configurations.rs` and `configuration.yaml`.

#To differentiate between the environments, we make our configuration _hierarchical_.
So, there isn't a lot more we can do with what we currently have.
The idea is to create an environment varialbe, `APP_ENVIRONMNET`,
that we can set to "production" or "local".
Based on its value, we load environment-specific configuration files. 

We can start with updates to the `configuration::get_configuration()` function.
Ok, we added an Enum, implemented some traits and created some new files.
Now, we update our Dockerfile.

The Docker image takes a while to build...

```bash
docker build --tag zero2prod --file Dockerfile .
docker run -p 8000:8000 zero2prod
```

Because we didn't run this in detach mode, get a new terminal:

```bash
curl -v http://127.0.0.1:8000/health-check
```

I was very happy to see this finally work.
It takes a long time to copy in everything, that needs to be trimmed down _a lot_.
I also forgot to create the `./configuration/` directory.
That meant the build didn't work right and I had to rebuild.

### Database Connection to Docker

Ok, the Application works but it isn't connected to the database correctly.
This has to do with using `connect_lazy`.
We can specify the network as `--network=host` and put Postgres on the same network.
Or we could use `docker-compose` because it puts all containers on the same network by default.
We can also create a user-defined network.

However, we leave it be for the moment... ok.

### Optimise Docker Image

The actual host machine won't really run `docker build`.
Instead, it'll use `docker pull` to _download_ a prebuilt image. 
To use an image we pay for its download cost.
That is directly related to _size_.

Wow, I am officially very impressed. 
Somehow the image is just 97.2MB currently and the build time is very fast.
I didn't make changes to the source code so it's just pulling from cache,
but then the post build after compilation is quick. 
```bash
docker images zero2prod  # it will display the size
```

My image is like almost 11GB!

Start with a `.dockerignore` file.
Ignoring unnecessary files and directories will greatly reduce build size.

But Rust's binaries are _statically linked_ (mostly).
This means we do not need to keep source code nor intermediate compilation artifacts.
We will create a multi-stage build in Docker:
1. Builder stage to generate a compiled binary.
2. A Runtime stage to run said binary.

The `runtime` is our final image.
The `builder` stage is thrown away at the end of the build. 
Will we try to build again?

```bash
docker build --tag zero2prod --file Dockerfile .
```

Wow, not only was it much faster not copying in all of those extra files,
but the image is only 1.43GB now.

And if you really know what you are doing you can actually run the binary on just the bare OS as the base image.
The book uses `debian:bookworm-slim`.
You have to know what your program requires though and install packages accordingly.
For example, ours will use OpenSSL because it is dynamically linked by some of the dependencies.
It also requires "ca-certifactes" to verify TLS certificates for HTTPS connections.

Is that as small as we can go?
Actually, you can look into `rust:1.XX-alpine` image.
Alpine is a Linux distribution designed to be small and secure.
It's out of scope because you would have to cross-compile to linux-musl.
The book (p. 150) also suggest "stripping symbols" from the binary and provides a link for more information.

Also worth noting that because of caching, the order of operations is important.
Docker stops pulling cache once it hits a change. 
So, things that change often, like source code, should be written as low as possible. 

Cargo is also a little weird.
Most languages copy in a "lock-file" to build the dependencies.
Then you copy in the source code and build the project.
The `cargo build` is unfortunately a one-stop-shop for all building. 

The author made a tool, [`cargo-chef`| github](https://github.com/LukeMathWalker/cargo-chef)
that works nicely into docker containers.
There's explicit instructions as well.
