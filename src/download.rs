use crate::{
    CPusher, CallbackContext, Config, EventCallback, FlushCallback, ForceSendExt, PushCallback,
    RUNTIME, UrlInfo,
};
use arc_swap::{ArcSwap, ArcSwapOption};
use fast_down_ffi::{BoxPusher, Error, Rx};
use parking_lot::Mutex;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_void};
use std::path::PathBuf;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

pub struct DownloadTask {
    info: UrlInfo,
    task: Arc<Mutex<Option<fast_down_ffi::DownloadTask>>>,
    rx: Rx,
    token: CancellationToken,
    child_token: ArcSwap<CancellationToken>,
    error: Arc<ArcSwapOption<CString>>,
}

impl DownloadTask {
    pub fn new(task: fast_down_ffi::DownloadTask, rx: Rx, token: CancellationToken) -> Self {
        let child_token = token.child_token();
        child_token.cancel();
        Self {
            info: (&task.info).into(),
            task: Arc::new(Mutex::new(Some(task))),
            rx,
            child_token: ArcSwap::from_pointee(child_token),
            token,
            error: Arc::default(),
        }
    }

    /// 用于 prefetch 失败时创建一个带错误信息的空任务
    #[must_use]
    pub fn new_failed(error: CString, rx: Rx, token: CancellationToken) -> Self {
        let child_token = token.child_token();
        child_token.cancel();
        Self {
            info: UrlInfo::default(),
            task: Arc::default(),
            rx,
            child_token: ArcSwap::from_pointee(child_token),
            token,
            error: Arc::new(ArcSwapOption::from_pointee(Some(error))),
        }
    }

    fn refresh_child_token(&self) -> CancellationToken {
        let child_token = self.token.child_token();
        self.child_token.store(Arc::new(child_token.clone()));
        child_token
    }
}

impl Drop for DownloadTask {
    fn drop(&mut self) {
        self.token.cancel();
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

/// 获取 prefetch 阶段的错误信息（如果有的话）
///
/// # 返回值
/// - 非 NULL：返回错误信息字符串
/// - NULL：无错误
#[unsafe(no_mangle)]
pub unsafe extern "C" fn download_task_get_error(handle: *const DownloadTask) -> *const c_char {
    unsafe {
        handle
            .as_ref()
            .and_then(|h| h.error.load().as_ref().map(|s| s.as_ptr()))
            .unwrap_or(std::ptr::null())
    }
}

/// 设置/覆盖下载任务的配置（必须在 start_* 之前调用）
#[unsafe(no_mangle)]
pub unsafe extern "C" fn download_task_set_config(
    handle: *mut DownloadTask,
    config: *const Config,
) {
    if handle.is_null() || config.is_null() {
        return;
    }
    let (h, cfg) = unsafe { (&mut *handle, &*config) };
    if let Some(task) = h.task.lock().as_mut() {
        task.config = cfg.config.clone();
    }
}

async fn download_inner<R>(
    download_fut: impl Future<Output = Result<R, Error>>,
    rx: Rx,
    ctx: Option<&CallbackContext>,
) -> Result<R, CString> {
    tokio::pin!(download_fut);
    let res = loop {
        tokio::select! {
          res = &mut download_fut => break res,
          event = rx.recv() => {
            match event {
              Ok(e) => {
                if let Some(ctx) = &ctx {
                    ctx.emit_c_event(e);
                }
              }
              Err(_) => break download_fut.await,
            }
          }
        }
    };
    while let Ok(e) = rx.try_recv() {
        if let Some(ctx) = &ctx {
            ctx.emit_c_event(e);
        }
    }
    match res {
        Ok(data) => {
            if let Some(ctx) = &ctx {
                ctx.emit_completed();
            }
            Ok(data)
        }
        Err(e) => {
            let e = CString::new(format!("Download task failed: {e:?}"))
                .unwrap_or_else(|_| CString::from(c"Download task failed: Unkown Error"));
            if let Some(ctx) = &ctx {
                ctx.emit_failed(e.as_ptr());
            }
            Err(e)
        }
    }
}

/// 开始下载任务写入到指定路径
///
/// # 返回值
/// - `0` 成功
/// - `-1` 参数错误 (传入了空指针)
/// - `-2` 任务无效(可能是因为 prefetch 失败了)/任务已经运行
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
    let h = unsafe { &mut *handle };
    let Some(task) = h.task.lock().take() else {
        return -2;
    };
    let save_path: PathBuf = unsafe { CStr::from_ptr(save_path) }
        .to_string_lossy()
        .as_ref()
        .into();
    let child_token = h.refresh_child_token();
    let fut = task.start(save_path, child_token.clone());
    let ctx = callback.map(|_| CallbackContext { callback, context });
    let res = RUNTIME.block_on(download_inner(fut, h.rx.clone(), ctx.as_ref()));
    h.task.lock().replace(task);
    child_token.cancel();
    match res {
        Ok(()) => {
            h.error.store(None);
            0
        }
        Err(e) => {
            h.error.store(Some(Arc::new(e)));
            -3
        }
    }
}

/// 开始下载任务并返回内存中的数据，释放内存需用 `free_downloaded_data` 函数
///
/// # 返回值
/// - `0` 成功
/// - `-1` 参数错误 (传入了空指针)
/// - `-2` 任务无效(可能是因为 prefetch 失败了)/任务已经运行
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
    let h = unsafe { &mut *handle };
    let Some(task) = h.task.lock().take() else {
        return -2;
    };
    let child_token = h.refresh_child_token();
    let fut = task.start_in_memory(child_token.clone());
    let ctx = callback.map(|_| CallbackContext { callback, context });
    let res = RUNTIME.block_on(download_inner(fut, h.rx.clone(), ctx.as_ref()));
    h.task.lock().replace(task);
    child_token.cancel();
    match res {
        Ok(bytes) => {
            let mut boxed = bytes.into_boxed_slice();
            unsafe {
                *out_data = boxed.as_mut_ptr();
                *out_len = boxed.len();
            }
            std::mem::forget(boxed);
            h.error.store(None);
            0
        }
        Err(e) => {
            h.error.store(Some(Arc::new(e)));
            -3
        }
    }
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
/// - `-2` 任务无效(可能是因为 prefetch 失败了)/任务已经运行
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
    let h = unsafe { &mut *handle };
    let Some(task) = h.task.lock().take() else {
        return -2;
    };
    let buffer_size = task.config.write_buffer_size;
    let pusher = CPusher::new(push_cb, flush_cb, buffer_size, pusher_ctx);
    let child_token = h.refresh_child_token();
    let fut = task.start_with_pusher(BoxPusher::new(pusher), child_token.clone());
    let event_ctx = event_cb.map(|_| CallbackContext {
        callback: event_cb,
        context: event_ctx,
    });
    let res = RUNTIME.block_on(download_inner(fut, h.rx.clone(), event_ctx.as_ref()));
    h.task.lock().replace(task);
    child_token.cancel();
    match res {
        Ok(()) => {
            h.error.store(None);
            0
        }
        Err(e) => {
            h.error.store(Some(Arc::new(e)));
            -3
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn download_task_start_to_file_async(
    handle: *mut DownloadTask,
    save_path: *const c_char,
    callback: EventCallback,
    context: *mut c_void,
) -> i32 {
    if handle.is_null() || save_path.is_null() {
        return -1;
    }
    let h = unsafe { &mut *handle };
    let task_mutex = h.task.clone();
    let Some(task) = task_mutex.lock().take() else {
        return -2;
    };
    let save_path: PathBuf = unsafe { CStr::from_ptr(save_path) }
        .to_string_lossy()
        .as_ref()
        .into();
    let child_token = h.refresh_child_token();
    let ctx = callback.map(|_| CallbackContext { callback, context });
    let rx = h.rx.clone();
    let err_arc = h.error.clone();
    RUNTIME.spawn(async move {
        let fut = task.start(save_path, child_token.clone()).force_send();
        let res = download_inner(fut, rx, ctx.as_ref()).await;
        match res {
            Ok(()) => err_arc.store(None),
            Err(e) => err_arc.store(Some(Arc::new(e))),
        }
        task_mutex.lock().replace(task);
        child_token.cancel();
    });
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn download_task_start_to_memory_async(
    handle: *mut DownloadTask,
    out_data: *mut *mut u8,
    out_len: *mut usize,
    callback: EventCallback,
    context: *mut c_void,
) -> i32 {
    if handle.is_null() || out_data.is_null() || out_len.is_null() {
        return -1;
    }
    let h = unsafe { &mut *handle };
    let task_mutex = h.task.clone();
    let Some(task) = task_mutex.lock().take() else {
        return -2;
    };
    let child_token = h.refresh_child_token();
    let ctx = callback.map(|_| CallbackContext { callback, context });
    let rx = h.rx.clone();
    let error_arc = h.error.clone();
    RUNTIME.spawn(
        async move {
            let fut = task.start_in_memory(child_token.clone());
            let res = download_inner(fut, rx, ctx.as_ref()).await;
            match res {
                Ok(bytes) => {
                    let mut boxed = bytes.into_boxed_slice();
                    unsafe {
                        *out_data = boxed.as_mut_ptr();
                        *out_len = boxed.len();
                    }
                    std::mem::forget(boxed);
                    error_arc.store(None);
                }
                Err(e) => error_arc.store(Some(Arc::new(e))),
            }
            task_mutex.lock().replace(task);
            child_token.cancel();
        }
        .force_send(),
    );
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn download_task_start_with_pusher_async(
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
    let h = unsafe { &mut *handle };
    let task_mutex = h.task.clone();
    let Some(task) = task_mutex.lock().take() else {
        return -2;
    };
    let child_token = h.refresh_child_token();
    let event_ctx = event_cb.map(|_| CallbackContext {
        callback: event_cb,
        context: event_ctx,
    });
    let rx = h.rx.clone();
    let error_arc = h.error.clone();
    let buffer_size = task.config.write_buffer_size;
    let pusher = CPusher::new(push_cb, flush_cb, buffer_size, pusher_ctx);
    RUNTIME.spawn(async move {
        let fut = task
            .start_with_pusher(BoxPusher::new(pusher), child_token.clone())
            .force_send();
        let res = download_inner(fut, rx, event_ctx.as_ref()).await;
        match res {
            Ok(()) => error_arc.store(None),
            Err(e) => error_arc.store(Some(Arc::new(e))),
        }
        task_mutex.lock().replace(task);
        child_token.cancel();
    });
    0
}
