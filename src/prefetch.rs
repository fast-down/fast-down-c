use crate::{CancellationToken, Config, DownloadTask, ToNapiError};
use fast_down_ffi::create_channel;
use napi_derive::napi;
use url::Url;

#[napi]
#[allow(clippy::trailing_empty_array)]
pub async fn prefetch(
  url: String,
  config: Option<Config>,
  token: Option<&CancellationToken>,
) -> napi::Result<DownloadTask> {
  let url: Url = url.parse().convert_err("Invalid URL")?;
  let config = config.map(|c| c.to_ffi_config()).unwrap_or_default();
  let token = token.map(CancellationToken::get_token).unwrap_or_default();
  let (tx, rx) = create_channel();
  let task = tokio::select! {
    () = token.cancelled() => Err(napi::Error::new(napi::Status::Cancelled, "Prefetch Cancelled"))?,
    t = fast_down_ffi::prefetch(url, config, tx) => t.convert_err("Prefetch Failed")?
  };
  Ok(DownloadTask::new(task, rx, token))
}
