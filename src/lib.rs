mod cancel;
mod config;
mod download;
mod event;
mod prefetch;
mod pusher;
mod url_info;

pub use cancel::*;
pub use config::*;
pub use download::*;
pub use event::*;
pub use prefetch::*;
pub use pusher::*;
pub use url_info::*;

use std::sync::LazyLock;
pub static RUNTIME: LazyLock<tokio::runtime::Runtime> =
    LazyLock::new(|| tokio::runtime::Runtime::new().unwrap());
