use crate::{Event, FlushFn, ForceSendExt, JsPusher, PushFn, ToNapiError, UrlInfo};
use fast_down_ffi::{BoxPusher, Error, Rx};
use napi::{
  bindgen_prelude::Uint8Array,
  threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode},
  Status,
};
use napi_derive::napi;
use parking_lot::Mutex;
use std::{future::Future, sync::Arc};
use tokio_util::sync::CancellationToken;

#[napi]
pub struct DownloadTask {
  info: UrlInfo,
  inner: Mutex<Option<(fast_down_ffi::DownloadTask, Rx)>>,
  token: CancellationToken,
}

pub type DownloadCallback = ThreadsafeFunction<Event, (), Event, Status, false>;

#[napi]
impl DownloadTask {
  pub fn new(task: fast_down_ffi::DownloadTask, rx: Rx, token: CancellationToken) -> Self {
    let info = (&task.info).into();
    let inner = Mutex::new(Some((task, rx)));
    Self { info, inner, token }
  }

  #[napi]
  pub fn cancel(&self) {
    self.token.cancel();
  }

  #[napi]
  pub fn is_cancelled(&self) -> bool {
    self.token.is_cancelled()
  }

  #[napi(getter)]
  pub fn info(&self) -> UrlInfo {
    self.info.clone()
  }

  fn inner(&self) -> napi::Result<(fast_down_ffi::DownloadTask, Rx)> {
    self
      .inner
      .lock()
      .take()
      .convert_err("Download task has already been started or is invalid")
  }

  /// 开始下载任务写入到指定路径
  /// @param `save_path` 存储路径
  /// @param `callback` 进度与事件回调函数
  #[napi]
  pub async fn start(
    &self,
    save_path: String,
    #[napi(ts_arg_type = "(event: Event) => void")] callback: Option<DownloadCallback>,
  ) -> napi::Result<()> {
    let (task, rx) = self.inner()?;
    let download_fut = task.start(save_path.into(), self.token.clone());
    download_inner(download_fut, rx, callback)
      .force_send()
      .await
  }

  /// 开始下载任务并返回内存中的数据
  /// @param `callback` 进度与事件回调函数
  #[napi]
  pub async fn start_in_memory(
    &self,
    #[napi(ts_arg_type = "(event: Event) => void")] callback: Option<DownloadCallback>,
  ) -> napi::Result<Uint8Array> {
    let (task, rx) = self.inner()?;
    let download_fut = task.start_in_memory(self.token.clone());
    download_inner(download_fut, rx, callback)
      .force_send()
      .await
      .map(Uint8Array::new)
  }

  /// 开始下载任务并使用自定义的 pusher
  /// @param `push_fn` 数据推送回调函数
  /// @param `flush_fn` 缓冲区刷新回调函数
  /// @param `callback` 进度与事件回调函数
  #[napi]
  pub async fn start_with_pusher(
    &self,
    #[napi(ts_arg_type = "(data: [number, Uint8Array]) => Promise<void>")] push_fn: Arc<PushFn>,
    #[napi(ts_arg_type = "() => Promise<void>")] flush_fn: Option<Arc<FlushFn>>,
    #[napi(ts_arg_type = "(event: Event) => void")] callback: Option<DownloadCallback>,
  ) -> napi::Result<()> {
    let (task, rx) = self.inner()?;
    let pusher = JsPusher::new(push_fn, flush_fn, task.config.write_buffer_size);
    let download_fut = task.start_with_pusher(BoxPusher::new(pusher), self.token.clone());
    download_inner(download_fut, rx, callback)
      .force_send()
      .await
  }
}

async fn download_inner<R>(
  download_fut: impl Future<Output = Result<R, Error>>,
  rx: Rx,
  callback: Option<DownloadCallback>,
) -> napi::Result<R> {
  let Some(callback) = callback else {
    return download_fut.await.convert_err("Download Task Error");
  };
  tokio::pin!(download_fut);
  loop {
    tokio::select! {
      res = &mut download_fut => return res.convert_err("Download Task Error"),
      event = rx.recv() => {
        match event {
          Ok(e) => {
            callback.call(
              Event::from(e),
              ThreadsafeFunctionCallMode::NonBlocking,
            );
          }
          Err(_) => break,
        }
      }
    }
  }
  download_fut.await.convert_err("Download Task Error")
}
