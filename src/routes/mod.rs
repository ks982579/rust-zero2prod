mod health_check;
mod newsletters;
mod subscriptions;
mod subscriptions_confirm;

pub use health_check::*;
pub use newsletters::*;
pub use subscriptions::*;
pub use subscriptions_confirm::*;

/// Iterators over whole chain of errors
/// Can be used in `Debug` implementation for Error types!
pub fn error_chain_fmt(
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
