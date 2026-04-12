use crate::{
    CPusher, CallbackContext, EventCallback, FlushCallback, PushCallback, RUNTIME, UrlInfo,
};
use arc_swap::ArcSwap;
use fast_down_ffi::{BoxPusher, Error, Rx};
use parking_lot::Mutex;
use std::ffi::CStr;
use std::os::raw::{c_char, c_void};
use std::path::PathBuf;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

pub struct DownloadTask {
    info: UrlInfo,
    task: fast_down_ffi::DownloadTask,
    rx: Mutex<Option<Rx>>,
    token: CancellationToken,
    child_token: ArcSwap<CancellationToken>,
}

impl DownloadTask {
    pub fn new(task: fast_down_ffi::DownloadTask, rx: Rx, token: CancellationToken) -> Self {
        let child_token = token.child_token();
        child_token.cancel();
        Self {
            info: (&task.info).into(),
            task,
            rx: Mutex::new(Some(rx)),
            child_token: ArcSwap::from_pointee(child_token),
            token,
        }
    }

    fn refresh_child_token(&self) -> CancellationToken {
        let child_token = self.token.child_token();
        self.child_token.store(Arc::new(child_token.clone()));
        child_token
    }
}

/// 释放任务句柄
#[unsafe(no_mangle)]
pub unsafe extern "C" fn download_task_free(ptr: *mut *mut DownloadTask) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        let inner = *ptr;
        if inner.is_null() {
            return;
        }
        drop(Box::from_raw(inner));
        *ptr = std::ptr::null_mut();
    }
}

/// 彻底取消下载任务（不可恢复）
#[unsafe(no_mangle)]
pub unsafe extern "C" fn download_task_cancel(handle: *const DownloadTask) {
    if let Some(h) = unsafe { handle.as_ref() } {
        h.token.cancel();
    }
}

/// 检查是否已被彻底取消，空指针永远返回 true
#[unsafe(no_mangle)]
pub unsafe extern "C" fn download_task_is_cancelled(handle: *const DownloadTask) -> bool {
    unsafe { handle.as_ref().is_none_or(|h| h.token.is_cancelled()) }
}

/// 暂停下载任务（可恢复）
#[unsafe(no_mangle)]
pub unsafe extern "C" fn download_task_pause(handle: *const DownloadTask) {
    if let Some(h) = unsafe { handle.as_ref() } {
        h.child_token.load().cancel();
    }
}

/// 检查是否处于暂停状态，空指针永远返回 true
#[unsafe(no_mangle)]
pub unsafe extern "C" fn download_task_is_paused(handle: *const DownloadTask) -> bool {
    unsafe {
        handle
            .as_ref()
            .is_none_or(|h| h.child_token.load().is_cancelled())
    }
}

/// 获取 `UrlInfo` 句柄（禁止用 `url_info_free` 释放，这只是一个可变借用）
#[unsafe(no_mangle)]
pub unsafe extern "C" fn download_task_get_info(handle: *mut DownloadTask) -> *mut UrlInfo {
    unsafe {
        handle
            .as_mut()
            .map_or(std::ptr::null_mut(), |h| &raw mut h.info)
    }
}

fn download_inner<R>(
    download_fut: impl Future<Output = Result<R, Error>>,
    rx: Rx,
    ctx: Option<CallbackContext>,
) -> (Result<R, String>, Rx) {
    RUNTIME.block_on(async move {
        tokio::pin!(download_fut);
        let res = loop {
            tokio::select! {
              res = &mut download_fut => break res,
              event = rx.recv() => {
                match event {
                  Ok(e) => {
                    if let Some(ref ctx) = ctx {
                        ctx.emit_c_event(e);
                    }
                  }
                  Err(_) => break download_fut.await,
                }
              }
            }
        };
        while let Ok(e) = rx.try_recv() {
            if let Some(ref ctx) = ctx {
                ctx.emit_c_event(e);
            }
        }
        let res = res.map_err(|e| format!("Download task failed: {e:?}"));
        (res, rx)
    })
}

/// 开始下载任务写入到指定路径
///
/// # 返回值
/// - `0` 成功
/// - `-1` 参数错误 (传入了空指针)
/// - `-2` 任务已经运行
/// - `-3` 下载失败
#[unsafe(no_mangle)]
pub unsafe extern "C" fn download_task_start_to_file(
    handle: *mut DownloadTask,
    save_path: *const c_char,
    callback: EventCallback,
    context: *mut c_void,
) -> i32 {
    if handle.is_null() || save_path.is_null() {
        return -1;
    }
    let h = unsafe { &*handle };
    let Some(rx) = h.rx.lock().take() else {
        return -2;
    };
    let path: PathBuf = unsafe { CStr::from_ptr(save_path) }
        .to_string_lossy()
        .as_ref()
        .into();
    let child_token = h.refresh_child_token();
    let fut = h.task.start(path, child_token.clone());
    let ctx = callback.map(|_| CallbackContext { callback, context });
    let (res, rx) = download_inner(fut, rx, ctx);
    h.rx.lock().replace(rx);
    child_token.cancel();
    match res {
        Ok(()) => 0,
        Err(_) => -3,
    }
}

/// 开始下载任务并返回内存中的数据，释放内存需用 `free_downloaded_data` 函数
///
/// # 返回值
/// - `0` 成功
/// - `-1` 参数错误 (传入了空指针)
/// - `-2` 任务已经运行
/// - `-3` 下载失败
#[unsafe(no_mangle)]
pub unsafe extern "C" fn download_task_start_to_memory(
    handle: *mut DownloadTask,
    out_data: *mut *mut u8,
    out_len: *mut usize,
    callback: EventCallback,
    context: *mut c_void,
) -> i32 {
    if handle.is_null() || out_data.is_null() || out_len.is_null() {
        return -1;
    }
    let h = unsafe { &*handle };
    let Some(rx) = h.rx.lock().take() else {
        return -2;
    };
    let child_token = h.refresh_child_token();
    let fut = h.task.start_in_memory(child_token.clone());
    let ctx = callback.map(|_| CallbackContext { callback, context });
    let (res, rx) = download_inner(fut, rx, ctx);
    h.rx.lock().replace(rx);
    child_token.cancel();
    res.map_or(-3, |bytes| {
        let mut boxed = bytes.into_boxed_slice();
        unsafe {
            *out_data = boxed.as_mut_ptr();
            *out_len = boxed.len();
        }
        std::mem::forget(boxed);
        0
    })
}

/// 释放由 `download_task_start_to_memory` 分配的内存
#[unsafe(no_mangle)]
pub unsafe extern "C" fn free_downloaded_data(ptr: *mut *mut u8, len: usize) {
    if ptr.is_null() {
        return;
    }
    let inner = unsafe { *ptr };
    if inner.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
            inner, len,
        )));
        *ptr = std::ptr::null_mut();
    }
}

/// 开始下载任务并使用自定义推送器
///
/// # 返回值
/// - `0` 成功
/// - `-1` 参数错误 (传入了空指针)
/// - `-2` 任务已经运行
/// - `-3` 下载失败
#[unsafe(no_mangle)]
pub unsafe extern "C" fn download_task_start_with_pusher(
    handle: *mut DownloadTask,
    push_cb: PushCallback,
    flush_cb: FlushCallback,
    pusher_ctx: *mut c_void,
    event_cb: EventCallback,
    event_ctx: *mut c_void,
) -> i32 {
    if handle.is_null() {
        return -1;
    }
    let h = unsafe { &*handle };
    let Some(rx) = h.rx.lock().take() else {
        return -2;
    };
    let buffer_size = h.task.config.write_buffer_size;
    let pusher = CPusher::new(push_cb, flush_cb, buffer_size, pusher_ctx);
    let child_token = h.refresh_child_token();
    let fut = h
        .task
        .start_with_pusher(BoxPusher::new(pusher), child_token.clone());
    let event_ctx = event_cb.map(|_| CallbackContext {
        callback: event_cb,
        context: event_ctx,
    });
    let (res, rx) = download_inner(fut, rx, event_ctx);
    h.rx.lock().replace(rx);
    child_token.cancel();
    match res {
        Ok(()) => 0,
        Err(_) => -3,
    }
}
