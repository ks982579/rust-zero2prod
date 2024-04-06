// src/telemetry.rs

use tracing::{subscriber::set_global_default, Subscriber};
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::{fmt::MakeWriter, layer::SubscriberExt, EnvFilter, Registry};

/// Compose multiple layers into `tracing`'s subscriber.
///
/// # Implementation Notes
///
/// We are using `impl Subscriber` as a return type to avoid having to
/// spell out the actual type of the returned subscriber, which is
/// complex. We need to explicitly call out that the returned subscriber
/// is `Send` and `Sync` to make it possible to pass it to
/// `init_subscriber` later on.
pub fn get_subscriber<Sink>(
    name: String,
    env_filter: String,
    sink: Sink,
) -> impl Subscriber + Send + Sync
where
    Sink: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    // // `init` calles `set_logger` for us, and we default to "info".
    // env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    // Removed env_logger...
    let env_filter: EnvFilter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(env_filter));
    let formatting_layer: BunyanFormattingLayer<Sink> = BunyanFormattingLayer::new(name, sink);
    // `with` is provided by `SubscriberExt`
    Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer)
}

/// Register a subscriber as global default to process span data.
///
/// It should only be called **once**!
pub fn init_subscriber<T>(subscriber: T)
where
    T: Subscriber + Send + Sync,
{
    // Redirecting all log's events to our subscriber...
    LogTracer::init().expect("Failed to set Logger");
    // below specifies what subscriber should be used to procecss spans.
    set_global_default(subscriber).expect("Failed to set subscriber");
}
