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

### Deployment to DigitalOcean

p. 152 / 171

I tried to sign up on DigitalOcean, they charged my credit card, and then said they needed more information.
They reverted the money, but I still need to wait 24 hours for a manual review of my account. 
Try to continue whilst under review.

DigitalOcean uses what they call an _App Spec_ file. 

p. 154...

DigitalOcean also takes care of HTTPS for us?
That's actually very nice. 

We will need to _provision_ a database.

---

Honestly, going to have to skip this section because DigitalOcean is making it really hard to sign-up.
I'm about as real human as it gets so something is wrong with their software.
I'm borderline taking the side of never recommending their platform.

---

Making update to allow environment variables from a platform to be passed it.
We have a struct called `settings`.
By doing this

```rust
let settings = config::Config::builder()
    // {...}
    .add_source(
        config::Environment::with_prefix("APP")
            .prefix_separator("_")
            .separator("__")
    )
    .build()?;
```

We can now dynamically set environment variables on the platform.
We pass in something like `APP_APPLICATION__PORT=4269`,
and the program sets `Settings.application.port`.

Also, we must import:

```bash
cargo add serde-aux
```

Environment variables are stings for the config crate.
It will fail to pick up integers if using standard deserialization from serde.

DigitalOcean (hard to sign-up) requires SSL mode,
our database does not handle currently.

In a Crazy turn of events, I filed a couple of tickets and the platform recognized my legitimate interest in their platform.
I switched to using PayPal, as I do not think that the bank / debit card I was using was working for them.
If you are turned down, don't give up. 
Or just use PayPal from the start.

Information about linking Github and DigitalOcean isn't exactly forthcoming.
1. Log into DigitalOcean
2. On left side pannel, under "MANAGE", click "App Platform".
3. The first button should say "Create App" or something, which begins process for linking.
4. Once linked you do not have to proceed on the website.

Now, create an API token on the website.

Then:

```bash
doctl auto init
```

This will give link your terminal to DigitalOcean.

Finally:

```bash
doctl apps create --spec spec.yaml
docts apps list
```

This spins up a droplet or whatever.
For more information on the CLI, [`doctl` reference | digitalocean.com](https://docs.digitalocean.com/reference/doctl/reference/).
Watching the deployment build logs is exciting. 
But note that this process can take a while as it compiles your project. 
And then it goes live :)

```bash
curl -v https://zero2prod-abc123.ondigitalocean.app/health-check
```

The URL may differ, that's not my acutal one.
But it did return status 200.

Now we did update the `spec.yaml` and I pushed to GitHub.
I think that triggers a new build with out settings.
But the books suggests:

```bash
doctl apps list # get app ID
docts apps update <APP-ID> --spec=spec.yaml
```

This terminated the build from the push, and started another build.
We give it a while, updating project to take in environment variables from everywhere.
Like a while...

In the app, under settings, there's a "Components" tab. 
You can find the "Connection String", which is quite long.
Notice it has `postgresql://newletter:.../newsletter?sslmode=require`.
We must refactor our `DatabaseSettings` now for this.

I did those refactors previous, just didn't write about it.
We have the database struct actually return a connection, or `PgConnectOptions`.
Great, returning a string then couples functions to the database with connection setup.

Update `main.rs` and the tests, which connect with the database.

Now we add `require_ssl` property fo the `DatabaseSettings` struct.
We also update the local and production configuation YAML files.
Updating DataBase Settings to trace database as well with a new `LevelFilter`.

Then, we update the `spec.yaml` with loads of environment variables.
There's a [How To | digitalocean.com](https://docs.digitalocean.com/products/app-platform/how-to/update-app-spec/) section.
Still looking for docs for this app spec tho. 

Here's what you do:

```bash
doctl apps list
doctl apps update <APP-ID> --spec=spec.yaml
git commit -am "<Commit-message>"
git push origin main
```

That will trigger a new deployment.

The last thing to do is to push our migrations. 
Note, it's a small note in the book, but you must disable "Trusted Sources" to push migations locally.
The database will not connect with you else.
You can do this from your app's settings.

```bash
DATABASE_URL=postgresql://... sqlx migrate run
```

Should work.

There's not really a way to "turn off" the drop unless you destroy the container.
Probably work looking into.

## Rejecting Invalid Subscribers - Part I

We cut corners to get this far.
Let's write a test to _probe_ troublesome inputs.

The test is set to pass with bad data.
It should fail, probably a rewrite later.
We want a name and email, and should validate that data.

We aren't checking passports, so we can settle on the name not being empty.
But what about SQL injections or other attacks?
+ Denial of Service
+ Data Theft
+ Phishing

A "Layered Security Approach" is also called _defense in depth_.
We cannot handle every threat, but can mitigate risk substantially by introducing measures on multiple levels.
+ input validation
+ parametrised queries
+ escaping parametrised input in emails
+ etc...

What validation can we perform on names?
+ enforce maximum length, 256 characters should be plenty.
+ Reject troublesome characters such as `/()"<>\{}`.

Of course, Rust has a small issue with Strings because... Strings are complicated.
I've seen many videos, but this [UTF-8 article | wikipedia.org](https://en.wikipedia.org/wiki/UTF-8) explains UTF-8 nicely.
A "grapheme", which is the actual smallest unit of writing (a letter), can be comprised of 1 to 4 bytes. 
Some graphemes, like "å" are compose of two character!
This saves space, not storing the unnecessary trailing 3 bytes all of the time, but makes it hard to... traverse strings.

My understanding is the first byte encodes the length of the character, 
and ajoining bytes begin with `10xxxxxx`.

```bash
cargo add unicode-segmentation
```

We put it into our endpoint but then the book discusses local and global validation approaches.
We have a validation function, but using it doesn't scale well. 
We need a _parsing function_ to tranfrom unstructured data into structured data.

### Type-Driven Development

This leads to a new topic I have been hearing about called **Type-Driven Development**.
And so, we add a new "domain" module!
Suggested "Parse, don't validate" by Alexis King is a good starting point on type-driven development.
And "Domain Modelling Made Functional" by Scott Wlaschin is a good book for a deeper look into the topic.

We created an `.inner_ref()` method which works very well.
However, Rust library has a trait called `AsRef`.
You use it when you want a reference to something that is, or is similar, to what  you have.

A popular technique / pattern is something like:

```rust
pub fn this_slice_function<T: AsRef<str>>(s: T) {
    let s = s.as_ref();
    // {...}
}
```

We require `T` to implement the `AsRef` trait, which is a _trait bound_.
Apparently the Rustt standard library `std::fs` does something like this.
functions take arguments `P: AsRef<Path>` instead of forcing the user to convert everything into `Path`.
Examples to convert to `Path` could be from `String`, `PathBuf`, `OsString`, etc...

So, we update our things and now there's an `hyper::Error(IncompleteMessage)` error.
This means the API is terminating the requst processing abruptly and is not graceful. 
This is because we panic when the parsing fails.
Panicing is for **unrecoverable** errors. 
If you application panics in response to user input, it's probably a bug. 

In the foot-note of page 182, the author states that Actix-Web is resilient to crashing.
A panic will not crash the whole application, hopefully just one of the workers. 
Also, it will just spawn a new worker to replace the ones that failed. 

The `Result<T, E>` type is used when a return type is _fallible_.

We add a new crate:

```rust
cargo add claims
```

Regular `assert!(result.is_ok())` only prints that an assertion failed during testing.
It doesn't print the error message which can be critical. 
You could match first and print... or download this package.
It has nice features such as:

```rust
#[test]
fn dummy_test() {
    let result: Result<&str, &str> = Err("Failure Message.");
    claims::assert_ok!(result);
}
```

That is very clean.

With that, we can add unit tests to our `src/domain.rs` file.
They aren't exactly passing though.
This is because we are calling `.expect()` on our `SubscriberName::parse()`,
And that will panic if it gets an error.
We much change the `subscribe` return to "400 BAD REQUEST" on errrors. 

Ok, to run the integration tests:

```bash
cargo test --test <INTEGRATION_TEST_FILE_NAME>
```

When it comes to validating emails, the author recomends [`validator` | crates.io](https://crates.io/crates/validator).
So, sure, we can throw it in.

```rust
cargo add validator --features=derive
```

Well, I think `validator` changed since the book, from version 0.16 to 0.18. 
Must read documentation to figure out the updated way to use it. 
Check out [Validator docs | docs.rs](https://docs.rs/validator/0.18.1/validator/).
It's much more complicated, using the `alloc::borrow::Cow`, or "clone-on-write" smart pointer.

I kind of taped together a solution and it works.
Glad I was able to do it myself.
Had to give our struct a trait to check the validity of the email address.

The Author wants to check if our check allows for valid emails.
This spurs the conversation or, are we checking valid emails or just specific addresses?
Thus, is striving for 100% test coverage even a worthwhile goal?
Even if you touch every line of code with a test, you will probably never test all the allowable cases.

So, we move into the realm of _property-based testing_.
It might test more cases but won't prove our parses is correct.
It does not exhaustively explore the input space.

There's a crate called ["fake" | crates.io](https://crates.io/crates/fake).
As of right now (14-04-2024), apparently `fake:2.9.2` relies on the `rand` crate.
That crate is below version 1, only at 0.8.5. 
Something about not being used by quickcheck, we will settle for `fake:~2.3`.
The create is easy enough to use actually.

There are two mainstream options for _property-based testing_:
+ `quickcheck`
+ `proptest`

We are looking into `quickcheck`.
It has an `Arbitrary` trait we will implement to make our email validation tests compatible with the crate.

```bash
cargo add --dev quickcheck@0.9.2
cargo add --dev quickcheck_macros
```

Don't include the version actually.
The book suggests versions under 1.0, but, maybe recently, they are now at 1.0.
Of course, the API has changed and I cannot get it to work correctly as is. 
So, downgrading gives us passing results.

Maybe a homework assignment, get that working correctly with updated versions. 

Just did some serious debugging because the "returns 200 test keeps failing".
The `dbg!()` macro is amazing for this.
The database is returning that the value is already stored for some reason. 

Hours spent debugging when I was using the ol'foot-gun...
In the test, I was trying to configure the database with the `without_db()` method. 
I don't know what it was doing, but it wasn't configuring a new database.

We can now refactor with `TryFrom`.
When you use `TryFrom`, it also implements `TryInto` on the other struct.
So we do the following:

```rust
impl TryFrom<FormData> for NewSubscriber {
    type Error = String;

    fn try_from(value: FormData) -> Result<Self, Self::Error> {
        // {...}
    }
}
```

And we get both:

```rust
let example_1 = NewSubscriber::try_from(form.0);
// or...
let example_2 = form.0.try_into();
```

## Reject Invalid Subscriber Part II

Seems like the chapter will be about sending a confirmation email.
It is important to obtain subscriber concent. 
European citizens legally require explicit concent from the user.

The idea is we give them a link.
The user clicks the link to confirm intent.
We just return a `200 OK` response, no redirection.

What are the steps?:
1. User sends POST request to our `/subscription` endpoint.
2. We add details to database, "subscriptions" table, status set to `pending_confirmation`.
3. We generate a unique `subscription_token` and store in database linked to user ID.
4. We send confirmation email w/link... `.../subscriptions/confirm?token=<subscription_token>`
5. User clicks link.
6. We return status `200 OK`

We should also then activate their account.
1. The cliked link send GET request with that token.
2. We retrieve that token from query parameters.
3. query the ID associated with that token
4. Update the user's status from pending to "active"
5. return that `200 OK`

Thoughts?
What if they click the link twice?
What if they try to subscribe twice?
One step at a time.

Sending an email probably requires knowing **SMTP** (Simple Mail Transfer Protocol).
It is an application-level protocol to ensure different email servers understand each other.

We will use the `reqwest` crate, which was in the dev-dependencies, to connect to our email API.
Conneting is an expensive operation.
Using HTTPS to create new connections everytime we want to email can lead to _socket exhaustion_ under load.
Most HTTP clients offer _connection pooling_.
When the first requtest to a remote server is complete, the connection hangs open for some time.
This can avoid re-establishing a connection. 
The reqwest create initialises a connection pool _under the hood_. 
We want to take advantage of this and reuse the same `Client` across multiple requests.

To do this, we go to `startup.rs`...
Well, the book explains 2 ways, and we side with the latter.
See page 235 for more details yourself.
We follow the same approach though as we did with our database connection,
wrapping the connection in an ARC pointer and passing clones of that pointer to instances of our application.

We code our way through adding an email client into starting out app.
Because of code duplication, we also must update our tests separately, the `run()` arguments.

While on the subject, we now want to test this, 
which involves testing a REST client.
It is good to start small, testing the `EmailClient` component in isolation. 
The `EmailClient::send_email` must perform an HTTP request.
To test, we need to catch our own HTTP request.
This is where we spin up a mock server!

What's another dependency?

```bash
cargo add --dev wiremock
```

The [`wiremock` | crates.io](https://crates.io/crates/wiremock) is for HTTP mocking.
Also make sure tokio has `rt` and `macros` features.
With that, go to the `email_client.rs` file to write some unit like tests.

The author suggests Postmarkapp.com, and I think it's a good recommendation.
I have a domain through Netlify, hosting my Astro website.
Setting up connection through Netlify with Postmarkapp was simple enough.
Then, they send me an email on how to use their API with `cURL`.

Now we look at Postmark API documentation.
It is also in the email I received earlier. 
It's good to know how to use `cURL`, so here it is...

```bash
curl "https://api.postmarkapp.com/email" \
    -X POST \
    -H "Accept: application/json" \
    -H "Content-Type: application/json" \
    -H "X-Postmark-Server-Token: <YOUR-SERVER-TOKEN>" \
    -d '{
    "From": "sender@example.com",
    "To": "receiver@example.com",
    "Subject": "Postmark test",
    "TextBody": "Hello dear Postmark user.",
    "HtmlBody": "<html><body><strong>Hello</strong> dear Postmark user.</body></html>",
    "MessageStream": "outbound"
}'
```

A successful request returns a header with status 200 OK,
and Content-Type: application/json.
Then the JSON with...

```json
{
    "To": ...,
    "SubmittedAt": ...,
    "MessageID": ...,
    "ErrorCode": 0,
    "Message": "OK"
}
```

We hook up all the moving parts...
Then we refine our testing. 
The `wiremock` crate is very good.
But we want a generic JSON validator because the JSON email body is randomized.
We can create our own using the `wiremock::Match` trait. 

We also need to add `serde_json` specifically. 
I think it has many methods for converting Rust to JSON,
other than serde's derived macros.
You can also import the handy `json!({...})` macro if needed.

There's also an interesting feature for serde.
You can tell it how to rename field names using an attribute macro.
Something like, `#[serde(rename_all = "PascalCase")]`.

After getting the test to work, we look at improvements, which Rust should be good with.
We zoom in on how we create many strings in the email request, a sure sign of waste.
Basically, each field allocates a bunch of new memory to stre a cloned `String`. 
It would be more efficient to reference existing data without additional allocation.
We can use the `&str`, which is just a pointer to a memory buffer owned by something else.

Why didn't we start with that?
Storing a reference in a struct requires a _lifetime_ parameter.
Not the end of the world, just tells the compiler the reference will be alive for the duration of the struct.
This prevents pointers that point to... nothing. 
I think called null/garbage pointers. 

We set up a test to ensure the request is sent ok.
Then a test that errors if a bad response is received.
If the response is 500, the default `reqwest::Client` still says `Ok(())`.
We need to add the method `error_for_status()?` to return the error on bad statuses.

Then, we look at timeout issues.
Very important because if the server begins to hang, requests might build up!
We don't _hang-up_ on the server, so the connection is busy.
When we send an email we open a new connection.
If the server doesn't recover quickly enough, and connections remain open,
we could end up with socket exhaustion/performance degradation.

Rule of thumb is for all IO operations, always set a timeout!

Setting the correct time can be challenging.
Best of luck.
The `reqwest` crate allows for setting `Client`-wide timeout, or per request.
We go with the former for ease.

Then, section 7.3, we look at restructuring the test suite.
Remember: **Test code is still Code**.
It really should be:
+ Modular
+ Well-structured
+ documented
+ _maintained_

We have a look at the test logic for `spawn_app()`.
We know it is similar to the `main()` function.
It's code smells, we had to update our email stuff twice because code duplication.


For testing, we bind a random port in the test setup.
But our new `build()` function also binds a port in it.
We need to somehow pass in port 0 to randomize, but also track the port for later use.
We also pull out some other logic so things are less duplicated.

We can now look back to the mission of sending a confirmation email.
The book explains some of the details around what needs be accomplished around p. 274.
But first, a discussion on **Zero Downtime Deployments**.
Commercially, you might strike a "Service Level Agreement" (SLA), 
which is a contractual obligation to guarantee a certain level of reliability. 
Interesting to note, 99.99% reliability is roughly only 52 minutes of downtime per year.
Basically, if a release triggers a small outage, you cannot release multiple times per day.

The "health-check" endpoint is actually good for **self-healing** applications.
There are also several deployment strategies.
The niave deployment is just... shut it down and reboot with updated version.
**Rolling Updates** involve a _load-balancer_ to introduce nodes casually. 
DigitalOcean is _probably_ relying on Rolling Updates.

This goes into Database schema.
When updates are rolled out, you may have old and new versions communicating with the same database.
Well, they are...
But what if that database schema changes, like adding a new table for `subscription_tokens`?
If new fields are implemented as `NOT NULL`, then old versions of product cannot create new records.
And if we deploy before we migrate, the product tries to make entries not supported.
This means... we need a multi-step approach.

And so we begin by creating a new column...

```bash
sqlx migrate add add_status_to_subscriotions
```

We then add the SQL code.
Run against local database.

```bash
SKIP_DOCKER=true ./scripts/init_db.sh
```

Trying to get the updates to build.
Had to run `cargo sqlx prepare` to update the offline database cache. 
Migrated to production again and pushed code to start deployment another few times. 
I actually wonder if the app is connected to the database on DigitalOcean... I don't remember.
I believe that is in the `spec.yaml` file and why many keys are scoped to `RUN_TIME`.

### 7.7 - sending confirmation email

p. 282 / 301

Now we dive into a test-driven development by trying out _red-gree-refactor_ loop.
We also will use [`linkify` | crates.io](https://crates.io/crates/linkify) in our testing. 

We are adding another environment variable to the `spec.yaml`.
Since we updated this file, we must apply changes to DigitalOcean.
Grab the _app identifier_ and update with:

```bash
doctl apps list --format ID
# prints <$APP_ID>
doctl apps update <$APP_ID> --spec spec.yaml
```

Now we register the value in the application context,
a process to be familiar with!
+ Go to `startups.rs`
+ add parameter into the `server = run(...)` function.
  + This creates errors as the function only takes 3 parameters but we now supply 4.
+ Create a struct wrapper (in needed) since actix-web context is type-based
  + Using multiple `String` types opens us to conflicts
+ add `base_url: String` as a parameter in `fn run(...)`
+ create context with `let base_url = Data::new(ApplicationBaseUrl(base_url))`
+ Add to app like `App::new()...app_data(base_url.clone())`

For production, the base url is OK.
For testing, we need to also know the port. 
But the port we use is $0$ to make it random, what do we do?
Basically, load up any information you need for the tests into the `TestApp` struct.

In this more intenst development cycle of red-green testing,
A theme seems to be write a red test,
make the test green,
then refactor if necessary.
We have hard-coded in a token for the moment on the endpoint to get our test to pass.

Once we write a new test, we begin to look into dynamically setting the token.
We want a token that is tough to guess.
But they are single use and don't acutally grant access to protected information.
Worst case scenario is someone gets a (possibly) unwanted newsletter subscription.
We can use a cryptographically secure pseudo-random number generator (CSPRNG).
We import the `rand` crate....

```bash
cargo add rand --features=std_rng
```

The book has a different approach to generating a random string of characters.
But, following the documentation for `rand::distributions::Alphanumeric`, 
I followed that way.
A small issue is that we related the token to a user in a different table.
We never fetch the user's ID though. 

### 7.8 Database Transactions

We finally have something done, but  now we must consider what happens if...
We have an API call that makes 2 queries to the database. 
What happens if the applications crashes between queries?
You can end up with a subscriber without a token, and that would require intervention.

A database **transaction** groups related operations into a single unit of work.
All operations in a transaction either succeed or fail together. 
The book continues on page 315 about PostGres database transaction syntax.
Luckily, `sqlx` provides an API for transactions usings the `pool.begin().await` method.
Then, when we passed the `pool.get_ref()` into functions, we pass in the mutable transactions.
However, it's a different struct, so some amendments will be required.

The idea of the transaction is it has two methods, `Transaction::{rollback, commit}`.
When the transaction is droped, it will try to rollback if not committed.
We pass in mutable references so we can commit the complete transaction before ending the function.
And if there's a crash, the function drops the transaction before it is committed and that rolls-back the queries. 

Finally, The summary of the chapter!!!
The auther suggests going off and exploring on your own.
I have another project after this so I'll list the ideas, but no time to implement:
+ What about trying to subscribe twice?
+ Clicks on confirmation email twice?
+ What if subscription token is _well-formatted_ but non-existent?
+ Validate incoming token!
  + Code currently passes in raw user input into SQL query but `sqlx` has our back.
+ Proper templating solution for emails? (`tera`)
+ etc...

---

## Ch. 8 - Error Handling

Starts on 323 / 342 and is Error Handling...

### 8.1 - Purpose of Errors?

The author looks at `src::routes::subscriptions::store_token`,
which returns a `Result<(), sqlx::Error>`.
It attempts to store a token into the database.
However, the `execute()` method is a _fallible_ operation. 
The method returns a `Result` so the caller of the method **can _react_ accordingly**.

Knowing how to react is difficutl when there are so many different ways something can fail.
The author shows a massive list of `Error` enum variants from the `sqlx` crate.
Errors should also contain enough **context** to produce a report for the developer.
They need to troubleshoot problems. 
And that is why we use the `...map_err(|e| {tracing::error!(...)})` pattern to log the errors.

Users of an application firstly, expect the application to not fail.
But, secondly, if/when it does, they would expect some kind or signal or message that it has failed.
We have been sending `HttpResponse::InternalServerError().finish()` signals. 
Note, the reponse body of these errors is empty by design.
Most users wouldn't be able to determine why our application is failing anyway.

But, even in `src::domain::subscriber_email::SubscriberEmail::parse()`,
we don't _propagate_ the error message to the user. 
We create an error message and send the `Result`.
The function performs a match,
and if it is `Err(_)`, 
we just send `HttpResponse::BadRequest().finish()`,
which has an empty body.
This is a very poor error because the user wouldn't understand why the call failed.

2 purposes of errors:
+ Control flow
+ Reporting

Location of Errors:
+ Internal (functions in our application calling other functions in our application)
+ At the edge (API request our application fails to fulfill)

Be careful with how much information you give to a user.
They should only be given what is required to adjust their behaviour.

### 8.2 Error reporting for operators

We apparently are going to test our logs by sabotaging our database in a test.
We drop a column?
In my current version of the book... 20240128, 
The author left a footnote that they look forward to revisiting the chapter when better tooling becomes available.
You would ideally write tests to verify the properties of the logs emitted by the application.

The author suggest running the following

```bash
export RUST_LOG="sqlx=error,info"
export TEST_LOG=true
cargo test subscribe_fails_if_there_is_a_fatal_database_error | bunyan
```

Basically, we see there's a failure, but `expection.details` and `exception.message` are both empty.
To understand our poor log records, we start with the "subsriptions" endpoint.
We find were the error would generate and want to use `actix_web::Error`.
Check out the docs for more information.

The [documentation](https://actix.rs/docs/errors#error-logging-1) kind of shows `actix_web::error`, lowercase.
The [actix-web | docs.rs](https://docs.rs/actix-web/4.0.1/actix_web/error/struct.Error.html) documentation
shows that `error` is the module and `Error` is the struct. 
I think the idea is to implement `actix_web::ResponseError` trait on the `sqlx::Error` enum. 
However, Rust has an _orphan rule_:
it is forbidden to implement foreign trait for a foreign type.
It's also just the wrong approach. 
The compiler will suggest wrapping the original error.

This is great but our trait requires `Debug` and `Display` traits.
The Debug one is good and most / all public types should implement this.
Display is tough because most types don't implement it and it cannot be _derived_.
Yes, then were an `sqlx::Error` is emitted, we wrap in our wrapper for returning.
Then, that error type implements the `ResponseError` trait,
Allowing Actix-Web to handle it. 
Looks like the `exception.details = Debug` and `exception.message = Display`

We then look at what an error in Rust really looks like. 
It's a trait that implements Debug and Display. 
In `Result<T, E>` the error type can be anything.
But making it an actually Error type _semantically_ marks our type as an error. 
There's also a "Rust Error Handling Working Group".

Rust error trait:

```Rust
pub trait Error: Debug + Display {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}
```

What is the return type of `source`?
The `dyn Error` is a trait object, we only know it implements the error trait.
They help achieve polymorphism but incur the dynamic dispatch runtime cost.

Now, we implement the `Error` trait on our wrapper.

We have a nice solution, created a `error_chain_fmt()` function,
but we can do better (I guess).

What did we do?

```rust
use actix_web::{web, HttpResponse, ResponseError};
use chrono::Utc;
use rand::{
    distributions::{Alphanumeric, DistString},
    thread_rng, Rng,
};
// use sqlx::PgConnection;
use sqlx::{Executor, PgPool, Postgres, Transaction};
// use tracing::Instrument;
use unicode_segmentation::UnicodeSegmentation;
use uuid::Uuid;

use crate::{
    domain::{NewSubscriber, SubscriberEmail, SubscriberName},
    email_client::EmailClient,
    startup::ApplicationBaseUrl,
};

// New error type, wrapping `sqlx::Error`
pub struct StoreTokenError(sqlx::Error);

/// Iterators over whole chain of errors
/// Can be used in `Debug` implementation for Error types!
fn error_chain_fmt(
    e: &impl std::error::Error,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    // This writes into a _buffer_
    writeln!(f, "{}\n", e)?;
    let mut current = e.source();
    while let Some(cause) = current {
        writeln!(f, "Caused by:\n\t{}", cause)?;
        current = cause.source();
    }
    Ok(())
}

impl std::fmt::Debug for StoreTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // This struct has text, and the one it wraps also does.
        // write!(f, "{}\nCaused by:\n\t{}", self, self.0)
        error_chain_fmt(self, f)
    }
}

impl std::fmt::Display for StoreTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "A database error was encountered while \
            trying to store a subscription token."
        )
    }
}

impl std::error::Error for StoreTokenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        // Compiler casts `&sqlx::Error` into `dyn Error`
        Some(&self.0)
    }
}

impl ResponseError for StoreTokenError {}

#[derive(serde::Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

// Implementing `TryFrom` automagically give you `TryInto`
impl TryFrom<FormData> for NewSubscriber {
    type Error = String;

    fn try_from(value: FormData) -> Result<Self, Self::Error> {
        let name = SubscriberName::parse(value.name)?;
        let email = SubscriberEmail::parse(value.email)?;
        Ok(Self { email, name })
    }
}

// [...]

#[tracing::instrument(name = "Store subscription token in the database", skip_all)]
pub async fn store_token(
    transaction: &mut Transaction<'_, Postgres>,
    subscriber_id: Uuid,
    subscription_token: &str,
) -> Result<(), StoreTokenError> {
    let query = sqlx::query!(
        r#"INSERT INTO subscription_tokens (subscription_token, subscriber_id)
    VALUES ($1, $2)"#,
        subscription_token,
        subscriber_id,
    );
    transaction.execute(query).await.map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        StoreTokenError(e)
    })?;
    Ok(())
}

// [...]
```

Basically, this `actix_web::Error` coming from `store_token()` is basically some random function,
completely unaware of REST or HTTP protocol. 
As such, if the code were reused for a non-REST implementation,
we wouldn't want to return an error of Status 500 on failure.
We will start over now to enfore separation of concerns. 
We create `SubscribeError`.

The book says to nuke the error, but then we use it I think later on.
Just create an enum for error types and we can take it from there.

Suppose it is work talking about the `?` operator.
If we implement `From<err>` for each variant, we should be able to use the `?`.
Implementing that actually makes the code much nicer,
but you must know exactly the returned error type of every operation to map into your enum.
Sure, we will also need to re-map HTTP Status codes in `subscriptions.rs`.
The `enum` allows us to match error variants to status codes very nicely,
as you can see (hopefully) in the code.

I like how this looks:

```rust
impl ResponseError for SubscribeError {
    fn status_code(&self) -> StatusCode {
        match *self {
            SubscribeError::ValidationError(_) => StatusCode::BAD_REQUEST,
            SubscribeError::DatabaseError(_)
            | SubscribeError::StoreTokenError(_)
            | SubscribeError::SendEmailError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
```

The dereference operator isn't necessary, but is explicit in meaning.
While on the subject, the next bit will look like:

```rust
impl std::error::Error for SubscribeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        // Had *self, but the dereference was not compatible with return type
        match self {
            // &str does not implement `Error` - We consider it the root cause
            SubscribeError::ValidationError(_) => None,
            SubscribeError::DatabaseError(e) => Some(e),
            SubscribeError::StoreTokenError(e) => Some(e),
            SubscribeError::SendEmailError(e) => Some(e),
        }
    }
}
```

Before moving on, you can see I did **not** dereference self here.
The return type is a reference.
If you dereference then the returned value will not match the return type.

The logs are shaping up but each message is generic.
For the `SubscribeError`, we can implement our own `Debug` trait that uses the `error_chain_fmt` function.
Then, implement the `std::error::Error` trait for `SubscribeError` to give it a `source` method.

What will be an issue?
The `DatabaseError` wraps over `sqlx::Error`.
That can be a bunch of things, from connection issues, inserting subscriber issue, committing transaction issue, etc...
As such, we much split up our generic `DatabaseError` into different possibilities.
Then, when an error is returned, we'll have to map it to the correct varient.

Because we are mapping the one `sqlx::Error` type to 3 different variants, 
we have to handle mapping with `().await.map_err(SubscribeError::...)` as they occur.
This means knowing what thing returns what error.

Our boilerplate code:

```rust
// #[derive(Debug)] -- Custom implementation
pub enum SubscribeError {
    ValidationError(String),
    // DatabaseError(sqlx::Error),
    StoreTokenError(StoreTokenError),
    SendEmailError(reqwest::Error),
    PoolError(sqlx::Error),
    InsertSubscriberError(sqlx::Error),
    TransactionCommitError(sqlx::Error),
}

// To use the `?` operator we need the `From<err>` trait
impl From<reqwest::Error> for SubscribeError {
    fn from(value: reqwest::Error) -> Self {
        Self::SendEmailError(value)
    }
}
/* We handle mapping errors in function unfortunately
impl From<sqlx::Error> for SubscribeError {
    fn from(value: sqlx::Error) -> Self {
        Self::DatabaseError(value)
    }
}
*/
impl From<StoreTokenError> for SubscribeError {
    fn from(value: StoreTokenError) -> Self {
        Self::StoreTokenError(value)
    }
}
// Yes, even the `String`
impl From<String> for SubscribeError {
    fn from(value: String) -> Self {
        Self::ValidationError(value)
    }
}

impl std::fmt::Debug for SubscribeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl std::fmt::Display for SubscribeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SubscribeError::ValidationError(e) => write!(f, "{}", e),
            SubscribeError::StoreTokenError(_) => {
                write!(f, "Failed to store confirmation token for new subscriber.")
            }
            SubscribeError::SendEmailError(_) => {
                write!(f, "Failed to send a confirmation email.")
            }
            SubscribeError::PoolError(_) => {
                write!(f, "Failed to acquire a Postgres connection from the pool.")
            }
            SubscribeError::InsertSubscriberError(_) => {
                write!(f, "Failed to insert new subscriber in database.")
            }
            SubscribeError::TransactionCommitError(_) => {
                write!(
                    f,
                    "Failed to commit SQL transaction to store new subscriber."
                )
            }
        }
        // write!(f, "Failed to create a new subscriber.")
    }
}

impl std::error::Error for SubscribeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        // Had *self, but the dereference was not compatible with return type
        match self {
            // &str does not implement `Error` - We consider it the root cause
            SubscribeError::ValidationError(_) => None,
            // SubscribeError::DatabaseError(e) => Some(e),
            SubscribeError::StoreTokenError(e) => Some(e),
            SubscribeError::SendEmailError(e) => Some(e),
            SubscribeError::PoolError(e) => Some(e),
            SubscribeError::InsertSubscriberError(e) => Some(e),
            SubscribeError::TransactionCommitError(e) => Some(e),
        }
    }
}

impl ResponseError for SubscribeError {
    fn status_code(&self) -> StatusCode {
        match *self {
            SubscribeError::ValidationError(_) => StatusCode::BAD_REQUEST,
            // SubscribeError::DatabaseError(_)
            SubscribeError::PoolError(_)
            | SubscribeError::TransactionCommitError(_)
            | SubscribeError::InsertSubscriberError(_)
            | SubscribeError::StoreTokenError(_)
            | SubscribeError::SendEmailError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

// New error type, wrapping `sqlx::Error`
pub struct StoreTokenError(sqlx::Error);

impl std::fmt::Debug for StoreTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // This struct has text, and the one it wraps also does.
        // write!(f, "{}\nCaused by:\n\t{}", self, self.0)
        error_chain_fmt(self, f)
    }
}

impl std::fmt::Display for StoreTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "A database error was encountered while \
            trying to store a subscription token."
        )
    }
}

impl std::error::Error for StoreTokenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        // Compiler casts `&sqlx::Error` into `dyn Error`
        Some(&self.0)
    }
}

impl ResponseError for StoreTokenError {}
```

Some of this is pretty general, and would be nice if it could be implemented automatically.
Doesn't Rust have a feature that write code for us?
I think it's called a macro!
There's a crate called `thiserror` that provides a derive macro.
See [thiserror | crates.io] for more details.
The github linked has lovely README as well.

```bash
cargo add thiserror
```

It looks to use the macro and a series of annotations.
Compare biolerplate to this

```rust
// #[derive(Debug)] -- Custom implementation
#[derive(thiserror::Error)]
pub enum SubscribeError {
    #[error("{0}")]
    ValidationError(String),
    // DatabaseError(sqlx::Error),
    #[error("Failed to store the confirmation token for a new subscriber.")]
    StoreTokenError(#[from] StoreTokenError),
    #[error("Failed to send a confirmation email.")]
    SendEmailError(#[from] reqwest::Error),
    #[error("Failed to acquire a Postgres connection from the pool.")]
    PoolError(#[source] sqlx::Error),
    #[error("Failed to insert new subscriber in database.")]
    InsertSubscriberError(#[source] sqlx::Error),
    #[error("Failed to send a confirmation email.")]
    TransactionCommitError(#[source] sqlx::Error),
}

// Keeping our Bespoke implementation of `Debug`
impl std::fmt::Debug for SubscribeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

/* Should be handled by #[derive(thiserror::Error)]
impl std::fmt::Display for SubscribeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        [...]
    }
}

impl std::error::Error for SubscribeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        [...]
    }
}
*/

impl ResponseError for SubscribeError {
    fn status_code(&self) -> StatusCode {
        match *self {
            SubscribeError::ValidationError(_) => StatusCode::BAD_REQUEST,
            // SubscribeError::DatabaseError(_)
            SubscribeError::PoolError(_)
            | SubscribeError::TransactionCommitError(_)
            | SubscribeError::InsertSubscriberError(_)
            | SubscribeError::StoreTokenError(_)
            | SubscribeError::SendEmailError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

// New error type, wrapping `sqlx::Error`
pub struct StoreTokenError(sqlx::Error);

impl std::fmt::Debug for StoreTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // This struct has text, and the one it wraps also does.
        // write!(f, "{}\nCaused by:\n\t{}", self, self.0)
        error_chain_fmt(self, f)
    }
}

impl std::fmt::Display for StoreTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "A database error was encountered while \
            trying to store a subscription token."
        )
    }
}

impl std::error::Error for StoreTokenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        // Compiler casts `&sqlx::Error` into `dyn Error`
        Some(&self.0)
    }
}

impl ResponseError for StoreTokenError {}
```

I am thinking that `#[from]` helps implement the `From<err>` traits.
Where `#[source]` implements the `Error` trait `.source()` method.
It's hidden in the README.
We lost our `From` implementation with `String` errors,
so we have to explicitly map the `SubscribeError::ValidationError`.

The `thiserror::Error` is a procedural macro used with the _derive_ statement.
Other attributes are listed in the book:
+ `#[error(...)]` -> defines `Display`
+ `#[source]` -> what is returned as root cause in `Error::source`
+ `#[from]` -> derives implementation of `From`

Apparently when using the `#[from]` annotation it also implements the _source_,
saving us from annotating twice.

Why don't we use annotations for `ValidationError` variance?
The `String` type does not implement the `Error` trait.
It cannot be returned in `Error::source`. 
This is why, when implemented manually, this branch returned `None`.

The enum `SubscriberError` is explicit but doesn't scale well. 
We should think in terms of **abstraction layers**.
That is, what does the caller of `/subscribe` need to know?

We want to cut down the bulk of that error type, and keep details to ourselves.

```rust
#[derive(thiserror::Error)]
pub enum SubscribeError {
    #[error("{0}")]
    ValidationError(String),
    // "transparent" delegates Display's and source's implementation
    // to the type wrapped by UnexpectedError
    // #[error(transparent)]
    // Adding a string to provide additional information
    #[error("{1}")]
    UnexpectedError(#[source] Box<dyn std::error::Error>, String),
}
// [...]
impl ResponseError for SubscribeError {
    fn status_code(&self) -> StatusCode {
        match *self {
            SubscribeError::ValidationError(_) => StatusCode::BAD_REQUEST,
            SubscribeError::UnexpectedError(_, _) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
```

We did this, adding a `String` to convey additional information.
Example being like:

```rust
pub async fn subscribe(
    [...]
) -> Result<HttpResponse, SubscribeError> {
    // [...]
    let mut transaction = pool.begin().await.map_err(|e| {
        SubscribeError::UnexpectedError(
            Box::new(e),
            "Failed to acquire a Postgres connection from the pool".into(),
        )
    })?;
    // [...]
    Ok(HttpResponse::Ok().finish())
}
```

After all of that refactoring, guess what, there's a crate called `anyhow`. 
Check out [anyhow | crates.io](https://crates.io/crates/anyhow) for details.
Basically, we can replace `Box<dyn std::error::Error>` with `anyhow::Error`.

```rust
use anyhow::Context;

#[derive(thiserror::Error)]
pub enum SubscribeError {
    #[error("{0}")]
    ValidationError(String),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}
// [...]
fn example() -> Result<_, _> {
    let subscriber_id = insert_subscriber(&mut transaction, &new_subscriber)
        .await
        .context("Failed to insert new subscriber into database")?;
}
```

It's a nice touch I guess.

You can again, see logs in tests with...

```bash
export RUST_LOG="sqlx=error,info"
export TEST_LOG=true
cargo test subscribe_fails_if_there_is_a_fatal_database_error | bunyan
```

We see information three times, 
1. from `tracing::error!()` in code.
2. from `actix_web` converting our `SubscribeError` into `actix_web::Error`
3. from `tracing_actix_web::TracingLogger` telemetry middleware.

Rule of thumb is errors should be logged when handled.
Porpagating errors with `?` is not a case of handling, so shouldn't log.

The author suggest to tackle the "confirm" section to practice your error handling skills.

---

## Ch. 9 - Niave Newsletter Deliver

Starts p. 361 / 380

Make a habbit of revisiting user stories throughout project lifecycle.
They can change with implementation.
Start writing test that unconfirmed subscribers should **not** receive newsletters.

We use a _scoped_ mock to ensure that the mock of sending an confirmation email
does not blend with our other mock instance.
The `mount_as_scoped` returns a `MockGuard`, 
which has a custom `Drop` implementation to switch off the mock server when the Guard is out of scope.
Also note when the Guard is dropped it will check it's `expect` clause.

I was confused at first catching request to `{}/email` but
basically the local configuration sends requests back at localhost.

So, we have 2 tests, one to send to confirmed subscribers,
and one to ensure we don't send to unconfirmed subscribers.
As the book says, we begin with a naive approach. 
Looks like we send in the data,
parse it from the request,
fetch a list of newsletter confirmed subscribers,
build up the email request,
and send that to PostMark.

We are using HTML form data for `/subscriptions`.
But it is common to use JSON for REST APIs,
which we will choose here.
Also good to cover all ground.

We continue now building out the `src/routes/newsletters.rs` file.
Writing additional tests now hits me with a too many open files failure.
Per page 260 the limit is usually set to 1024.
We can increase with:

```bash
ulimit -n <number-of-pages (e.g. 10000)>
```

I also made the mistake of having the route accept GET  requests instead of POST.
Things to look out for.
It's an unfortunate consequence that we need to lean on our learnings in Ch. 8
for proper error handling.
The SQL query to get subscribers can be erroneous.
As such, a call to the `/newsletters` endpoint can end in error.
So it should return a result.

A weird `anyhow::Ok` import snuck into my code and messed everything up.
So weird, but then using `Ok()` was returning the wrong thing.

An issue is that the database gives us a `String`,
and sqlx does not naturally convert into other Rust custom types.
We can get around this using a Type for sqlx, and converting that
with the lovely `into` trait maybe?

We almost did this...

```rust
// [...]
    let confirmed_subscribers = rows
        .into_iter()
        .map(|r| ConfirmedSubscriber {
            // Because validation is performed when entering email into
            // database, the `unwrap()` should never panic.
            // Unless we change validation logic in future...
            email: SubscriberEmail::parse(r.email).unwrap(),
        })
        .collect();
// [...]
```

But because we could improve the parser,
we don't want to panic collecting emails and exit the program.
Instead, we can log a warning and move on with other emails.
We instead do this:

```rust
// [...]
    // Map into domain type
    let confirmed_subscribers = rows
        .into_iter()
        .filter_map(|r| match SubscriberEmail::parse(r.email) {
            Ok(email) => Some(ConfirmedSubscriber { email }),
            Err(error) => {
                tracing::warn!(
                    "A confirmed subscriber is using an invalid email address:\n\t{}",
                    error
                );
                None
            }
        })
        .collect();
// [...]
```

The `filter_map` returns an interator containing items that returned a `Some()` variant.
Then, as is typical, we aren't happy with this method and change it again.
What's wrong?
Apparently sending emails to only valid addresses is a business decision.
We shouldn't make that decision in this function.
So return a vector of results instead.

One test is failing, and I cannot figure it out for the life of me.
The rest of this Chapter talks about shortcomings of this approach.
Mainly anyone right now can send a POST request to the endpoint and it would send emails to our clients.
Well, mine might not because that test is failing.
Other things include:
+ We send emails one at a time, bad performance.
+ No chance to review before sending emails.
+ If one email fails, the error returns 500 
    + rest of emails are not sends, we don't retry failure.
+ Networks are unstable, and there's no clause for retry unless you resend to everyone. 

Ok, I am... probably programming on a lack of sleep.
So, the issue I tried debugging earlier was for test `newsletters_are_delivered_to_confirmed_subscribers`.
It was running, but the mock server wasn't seeing the request to `/email`.
We created a function to generate a confirmed user.
The issue was I was using the function for an unconfirmed user.

---

## Ch. 10 - Securing API

Starts p. 387 / 406 and is over 100 pages long...

But it's an important section.
We will look at authentication and authorization. 
Some approaches include:
+ Basic Auth
+ Session-Based Auth
+ OAuth 2.0
+ OpenId Connect

And we will think about the most used token formats **JSON Web Tokens** (JWTs).

You probably know the 3 categories of athentication:
+ Something you know (password)
+ Somthing you have (phone)
+ Something you are (Face, fingerprint)

Each approach has its weaknesses,
which is why we combine them with multi-factor authentication.

We will start with **Basic Authentication**, 
a standard defined by the Internet Engineering Task Force.
The API looks for `Authorization` header on incoming request:

```bash
Authorization: Basic <encoded credentials>
```

The encoding is just [Base 64 Encoding](https://datatracker.ietf.org/doc/html/rfc4648#section-4)
of "{username}:{password}".
Encoding is not encryption, there's no secret.
We create a **realm**, 
a partition of protected space.
We only need a single realm, we will call "publish". 

There's more to it than just this.
The API rejects requests missing the header or using invalid credentials.
It must respond with _401 Unauthorized_.
It also returns a special header with a challenge explaining to the API caller
what type of authentication scheme we expect to see for the relevant realm.

```bash
HTTP/1.1 401 Unauthorized
WWW-Authenticate: Basic realm="publish"
```

That is a log to know.

Start with extracting credentials.
We probably need the `base64` crate...

```bash
cargo add base64
```

I want to put here what we just wrote, 
the complexity around the simplicity of getting credentials:

```rust
/// Using base64 for authentication
fn basic_authentication(headers: &HeaderMap) -> Result<Credentials, anyhow::Error> {
    // Header value, if present, must be valid UTF8
    let header_value: &str = headers
        .get("Authorization")
        // `context` adds a message to the error
        .context("The 'Authorization' header was missing.")?
        .to_str()
        .context("The 'Authorization' header was not a valid UTF8 string.")?;

    let base64encoded_segment: &str = header_value
        // Returns Option<&str> without prefix
        .strip_prefix("Basic ")
        .context("The authorization scheme was not 'Basic'.")?;

    let decoded_bytes: Vec<u8> = base64::engine::general_purpose::STANDARD
        // returns Result<Vec<u8>, base64::DecodeError>
        .decode(base64encoded_segment)
        .context("Failed to base64-decode 'Basic' credentials.")?;

    let decoded_credentials: String = String::from_utf8(decoded_bytes)
        .context("The decoded credential string is not valid UTF8.")?;

    // Split based on ':' delimiter
    // SplitN is an iterator
    let mut credentials: std::str::SplitN<'_, char> = decoded_credentials.splitn(2, ':');
    // Pull out first value in iterator
    let username: String = credentials
        .next()
        // `ok_or()` is _eagerly_ evaluated, this takes a closure
        .ok_or_else(|| anyhow::anyhow!("A username must be provided in 'Basic' auth."))?
        .to_string();
    // Pull out second value in iterator
    let password: String = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("A password must be provided in 'Basic' auth."))?
        .to_string();

    Ok(Credentials {
        username,
        password: Secret::new(password),
    })
}
```

Now we need to update our errors.

So the general workflow appears to be,
helper functions, like `basic_authentication` will return the `anyhow::Error`.
We then must know to map this error to our authentication error.
I guess this makes sense because so many different errors can occur in the process,
we need to map to something generic, like a catch-all,
without implementing `InTo` for every known error.

We get the right status code but the header isn't correct yet.
We implement `ResponseError::status_code()`,
but now need `ResponseError::error_response()`.
The way this trait works, 
It would invoke `error_response` which would call our `status_code`.
We can trash the `status_code` if we implement a bespoke `error_response`.
Unless you want to call `status_code` in your `error_response`.
Hint, don't.

Of course, adding credentials kind of breaks other tests.
We add some random credentials in the `TestApp` that can be parsed.

On to Naive password verification.

```bash
sqlx migrate add create_users_table
```



---

## Ch. 11 - Fault-tolerant Workflows

Starts on p. 525 / 544
