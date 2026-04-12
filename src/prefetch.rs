use crate::{CancellationToken, Config, DownloadTask, RUNTIME};
use fast_down_ffi::create_channel;
use std::ffi::CStr;
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
/// 成功返回 `DownloadTask*`，失败返回 NULL。
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
    match task_result {
        Some(Ok(task)) => Box::into_raw(Box::new(DownloadTask::new(task, rx, cancel_token))),
        _ => std::ptr::null_mut(),
    }
}
