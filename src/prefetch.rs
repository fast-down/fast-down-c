use crate::{CancellationToken, Config, DownloadTask, ForceSendExt, RUNTIME};
use fast_down_ffi::create_channel;
use std::ffi::{CStr, CString, c_void};
use std::os::raw::c_char;
use url::Url;

/// 创建下载任务
///
/// # 参数
/// - `url`: 下载链接（UTF-8 字符串）
/// - `config`: 配置句柄（可为 NULL，使用默认配置）
/// - `token`: 取消令牌句柄（可为 NULL，内部自动创建）
///
/// # 返回值
/// - URL 解析失败返回 NULL
/// - 其他情况返回 `DownloadTask*`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn prefetch(
    url: *const c_char,
    config: *mut Config,
    token: *mut CancellationToken,
) -> *mut DownloadTask {
    if url.is_null() {
        return std::ptr::null_mut();
    }
    let url = unsafe { CStr::from_ptr(url) }.to_string_lossy();
    let Ok(url) = Url::parse(&url) else {
        return std::ptr::null_mut();
    };
    let config = unsafe { config.as_ref() }
        .map_or_else(fast_down_ffi::Config::default, |cfg| cfg.config.clone());
    let cancel_token = unsafe { token.as_ref() }
        .map_or_else(tokio_util::sync::CancellationToken::new, |t| {
            t.token.clone()
        });
    let (tx, rx) = create_channel();
    let task_result = RUNTIME.block_on(async {
        cancel_token
            .run_until_cancelled(fast_down_ffi::prefetch(url, config, tx))
            .await
    });
    let res = match task_result {
        Some(Ok(task)) => DownloadTask::new(task, rx, cancel_token),
        Some(Err(e)) => DownloadTask::new_failed(
            CString::new(format!("{e}"))
                .unwrap_or_else(|_| CString::new("Prefetch error").unwrap()),
            rx,
            cancel_token,
        ),
        None => DownloadTask::new_failed(CString::new("Prefetch error").unwrap(), rx, cancel_token),
    };
    Box::into_raw(Box::new(res))
}

pub type PrefetchCallback = Option<extern "C" fn(context: *mut c_void, task: *mut DownloadTask)>;

/// 创建下载任务
///
/// # 参数
/// - `url`: 下载链接（UTF-8 字符串）
/// - `config`: 配置句柄（可为 NULL，使用默认配置）
/// - `token`: 取消令牌句柄（可为 NULL，内部自动创建）
/// - `callback`: 回调函数
/// - `context`: 回调上下文（可为 NULL）
#[unsafe(no_mangle)]
pub unsafe extern "C" fn prefetch_async(
    url: *const c_char,
    config: *mut Config,
    token: *mut CancellationToken,
    callback: PrefetchCallback,
    context: *mut c_void,
) {
    if url.is_null() || callback.is_none() {
        return;
    }
    let url = unsafe { CStr::from_ptr(url) }.to_string_lossy();
    let Ok(url) = Url::parse(&url) else {
        if let Some(cb) = callback {
            cb(context, std::ptr::null_mut());
        }
        return;
    };
    let config = unsafe { config.as_ref() }
        .map_or_else(fast_down_ffi::Config::default, |cfg| cfg.config.clone());
    let cancel_token = unsafe { token.as_ref() }
        .map_or_else(tokio_util::sync::CancellationToken::new, |t| {
            t.token.clone()
        });
    let (tx, rx) = create_channel();
    RUNTIME.spawn(
        async move {
            let task_result = cancel_token
                .run_until_cancelled(fast_down_ffi::prefetch(url, config, tx))
                .await;
            let res = match task_result {
                Some(Ok(task)) => DownloadTask::new(task, rx, cancel_token),
                Some(Err(e)) => DownloadTask::new_failed(
                    CString::new(format!("{e}"))
                        .unwrap_or_else(|_| CString::new("Prefetch error").unwrap()),
                    rx,
                    cancel_token,
                ),
                None => DownloadTask::new_failed(
                    CString::new("Prefetch error").unwrap(),
                    rx,
                    cancel_token,
                ),
            };
            let task_ptr = Box::into_raw(Box::new(res));
            if let Some(cb) = callback {
                cb(context, task_ptr);
            }
        }
        .force_send(),
    );
}
