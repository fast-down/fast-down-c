use fast_down_ffi::{Proxy, WriteMethod};
use std::ffi::CStr;
use std::net::IpAddr;
use std::os::raw::c_char;
use std::time::Duration;

#[derive(Debug, Clone, Default)]
pub struct Config {
    pub config: fast_down_ffi::Config,
}

/// 创建一个新的配置句柄，所有字段为合理的默认值
#[unsafe(no_mangle)]
pub extern "C" fn config_new() -> *mut Config {
    Box::into_raw(Box::new(Config::default()))
}

/// 销毁配置句柄，释放内存
#[unsafe(no_mangle)]
pub extern "C" fn config_free(ptr: *mut *mut Config) {
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

#[unsafe(no_mangle)]
pub extern "C" fn config_set_threads(handle: *mut Config, threads: usize) {
    if let Some(cfg) = unsafe { handle.as_mut() } {
        cfg.config.threads = threads;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn config_set_proxy(handle: *mut Config, proxy: *const c_char) {
    if handle.is_null() || proxy.is_null() {
        return;
    }
    let cfg = unsafe { &mut *handle };
    let proxy = unsafe { CStr::from_ptr(proxy) }.to_string_lossy();
    cfg.config.proxy = match proxy.as_ref() {
        "no" => Proxy::No,
        "system" => Proxy::System,
        custom => Proxy::Custom(custom.to_string()),
    };
}

#[unsafe(no_mangle)]
pub extern "C" fn config_insert_header(
    handle: *mut Config,
    key: *const c_char,
    value: *const c_char,
) {
    if handle.is_null() || key.is_null() || value.is_null() {
        return;
    }
    let cfg = unsafe { &mut *handle };
    let key = unsafe { CStr::from_ptr(key) }
        .to_string_lossy()
        .into_owned();
    let value = unsafe { CStr::from_ptr(value) }
        .to_string_lossy()
        .into_owned();
    cfg.config.headers.insert(key, value);
}

#[unsafe(no_mangle)]
pub extern "C" fn config_remove_header(handle: *mut Config, key: *const c_char) -> bool {
    if handle.is_null() || key.is_null() {
        return false;
    }
    let cfg = unsafe { &mut *handle };
    let key = unsafe { CStr::from_ptr(key) }.to_string_lossy();
    cfg.config.headers.remove(&*key).is_some()
}

#[unsafe(no_mangle)]
pub extern "C" fn config_clear_headers(handle: *mut Config) {
    if let Some(cfg) = unsafe { handle.as_mut() } {
        cfg.config.headers.clear();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn config_set_min_chunk_size(handle: *mut Config, size: u64) {
    if let Some(cfg) = unsafe { handle.as_mut() } {
        cfg.config.min_chunk_size = size;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn config_set_write_buffer_size(handle: *mut Config, size: usize) {
    if let Some(cfg) = unsafe { handle.as_mut() } {
        cfg.config.write_buffer_size = size;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn config_set_write_queue_cap(handle: *mut Config, cap: usize) {
    if let Some(cfg) = unsafe { handle.as_mut() } {
        cfg.config.write_queue_cap = cap;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn config_set_retry_gap_ms(handle: *mut Config, ms: u64) {
    if let Some(cfg) = unsafe { handle.as_mut() } {
        cfg.config.retry_gap = Duration::from_millis(ms);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn config_set_pull_timeout_ms(handle: *mut Config, ms: u64) {
    if let Some(cfg) = unsafe { handle.as_mut() } {
        cfg.config.pull_timeout = Duration::from_millis(ms);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn config_set_accept_invalid_certs(handle: *mut Config, accept: bool) {
    if let Some(cfg) = unsafe { handle.as_mut() } {
        cfg.config.accept_invalid_certs = accept;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn config_set_accept_invalid_hostnames(handle: *mut Config, accept: bool) {
    if let Some(cfg) = unsafe { handle.as_mut() } {
        cfg.config.accept_invalid_hostnames = accept;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn config_set_write_method(handle: *mut Config, method: *const c_char) {
    if handle.is_null() || method.is_null() {
        return;
    }
    let cfg = unsafe { &mut *handle };
    let method = unsafe { CStr::from_ptr(method) }.to_string_lossy();
    cfg.config.write_method = match method.as_ref() {
        "std" => WriteMethod::Std,
        _ => WriteMethod::Mmap,
    };
}

#[unsafe(no_mangle)]
pub extern "C" fn config_set_retry_times(handle: *mut Config, times: usize) {
    if let Some(cfg) = unsafe { handle.as_mut() } {
        cfg.config.retry_times = times;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn config_add_local_address(handle: *mut Config, addr: *const c_char) -> bool {
    if handle.is_null() || addr.is_null() {
        return false;
    }
    let cfg = unsafe { &mut *handle };
    let addr = unsafe { CStr::from_ptr(addr) }
        .to_string_lossy()
        .parse::<IpAddr>();
    if let Ok(addr) = addr {
        cfg.config.local_address.push(addr);
        true
    } else {
        false
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn config_remove_local_address(handle: *mut Config, addr: *const c_char) -> bool {
    if handle.is_null() || addr.is_null() {
        return false;
    }
    let cfg = unsafe { &mut *handle };
    let addr = unsafe { CStr::from_ptr(addr) }
        .to_string_lossy()
        .parse::<IpAddr>();
    if let Ok(addr) = addr {
        cfg.config.local_address.retain(|a| a != &addr);
        true
    } else {
        false
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn config_clear_local_addresses(handle: *mut Config) {
    if let Some(cfg) = unsafe { handle.as_mut() } {
        cfg.config.local_address.clear();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn config_set_max_speculative(handle: *mut Config, max: usize) {
    if let Some(cfg) = unsafe { handle.as_mut() } {
        cfg.config.max_speculative = max;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn config_add_downloaded_chunk(handle: *mut Config, start: u64, end: u64) {
    if let Some(cfg) = unsafe { handle.as_mut() } {
        cfg.config.downloaded_chunk.lock().push(start..end);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn config_clear_downloaded_chunks(handle: *mut Config) {
    if let Some(cfg) = unsafe { handle.as_mut() } {
        cfg.config.downloaded_chunk.lock().clear();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn config_set_chunk_window(handle: *mut Config, window: u64) {
    if let Some(cfg) = unsafe { handle.as_mut() } {
        cfg.config.chunk_window = window;
    }
}
