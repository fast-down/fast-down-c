mod cancel;
mod config;
mod download;
mod event;
mod force_send;
mod prefetch;
mod pusher;
mod url_info;
use std::sync::LazyLock;

pub use cancel::*;
pub use config::*;
pub use download::*;
pub use event::*;
pub use force_send::*;
pub use prefetch::*;
pub use pusher::*;
pub use url_info::*;

pub static RUNTIME: LazyLock<tokio::runtime::Runtime> =
    LazyLock::new(|| tokio::runtime::Runtime::new().unwrap());
