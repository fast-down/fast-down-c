use arc_swap::ArcSwapOption;
use fast_down_ffi::FileId;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::Arc;

#[derive(Debug)]
pub struct UrlInfo {
    pub size: u64,
    /// 服务器返回的原始文件名，必须清洗掉不合法字符才能安全使用
    pub raw_name: CString,
    pub safe_name: ArcSwapOption<CString>,
    pub supports_range: bool,
    pub fast_download: bool,
    pub final_url: CString,
    pub etag: Option<CString>,
    pub last_modified: Option<CString>,
    pub content_type: Option<CString>,
}

impl From<&fast_down_ffi::UrlInfo> for UrlInfo {
    fn from(inner: &fast_down_ffi::UrlInfo) -> Self {
        Self {
            size: inner.size,
            raw_name: CString::new(&*inner.raw_name).unwrap_or_default(),
            safe_name: ArcSwapOption::from(None),
            supports_range: inner.supports_range,
            fast_download: inner.fast_download,
            final_url: CString::new(inner.final_url.as_ref()).unwrap_or_default(),
            etag: inner
                .file_id
                .etag
                .as_ref()
                .map(|s| CString::new(&**s).unwrap_or_default()),
            last_modified: inner
                .file_id
                .last_modified
                .as_ref()
                .map(|s| CString::new(&**s).unwrap_or_default()),
            content_type: inner
                .content_type
                .as_ref()
                .map(|s| CString::new(&**s).unwrap_or_default()),
        }
    }
}

impl UrlInfo {
    pub fn to_ffi(&self) -> Option<fast_down_ffi::UrlInfo> {
        Some(fast_down_ffi::UrlInfo {
            size: self.size,
            raw_name: self.raw_name.to_string_lossy().to_string(),
            supports_range: self.supports_range,
            fast_download: self.fast_download,
            final_url: self.final_url.to_string_lossy().parse().ok()?,
            file_id: FileId {
                etag: self.etag.as_ref().map(|s| Arc::from(s.to_string_lossy())),
                last_modified: self
                    .last_modified
                    .as_ref()
                    .map(|s| Arc::from(s.to_string_lossy())),
            },
            content_type: self
                .content_type
                .as_ref()
                .map(|s| s.to_string_lossy().to_string()),
        })
    }

    pub fn update_raw_name(&mut self, raw_name: CString) {
        self.raw_name = raw_name;
        self.safe_name.store(None);
    }

    pub fn filename(&self) -> *const c_char {
        if let Some(cached) = self.safe_name.load().as_ref() {
            return cached.as_ptr();
        }
        let sanitized = path_helper::sanitize_filename(self.raw_name.to_string_lossy(), 255);
        let new_arc = Arc::new(CString::new(sanitized).unwrap_or_default());
        self.safe_name.store(Some(new_arc.clone()));
        new_arc.as_ptr()
    }
}

/// 释放 `UrlInfo` 句柄
#[unsafe(no_mangle)]
pub unsafe extern "C" fn url_info_free(ptr: *mut *mut UrlInfo) {
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
pub unsafe extern "C" fn url_info_get_size(handle: *const UrlInfo) -> u64 {
    unsafe { handle.as_ref().map_or(0, |h| h.size) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn url_info_get_raw_name(handle: *const UrlInfo) -> *const c_char {
    unsafe {
        handle
            .as_ref()
            .map_or(std::ptr::null(), |h| h.raw_name.as_ptr())
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn url_info_get_supports_range(handle: *const UrlInfo) -> bool {
    unsafe { handle.as_ref().is_some_and(|h| h.supports_range) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn url_info_get_fast_download(handle: *const UrlInfo) -> bool {
    unsafe { handle.as_ref().is_some_and(|h| h.fast_download) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn url_info_get_final_url(handle: *const UrlInfo) -> *const c_char {
    unsafe {
        handle
            .as_ref()
            .map_or(std::ptr::null(), |h| h.final_url.as_ptr())
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn url_info_get_etag(handle: *const UrlInfo) -> *const c_char {
    unsafe {
        handle
            .as_ref()
            .and_then(|h| h.etag.as_ref().map(|c| c.as_ptr()))
            .unwrap_or(std::ptr::null())
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn url_info_get_last_modified(handle: *const UrlInfo) -> *const c_char {
    unsafe {
        handle
            .as_ref()
            .and_then(|h| h.last_modified.as_ref().map(|c| c.as_ptr()))
            .unwrap_or(std::ptr::null())
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn url_info_get_content_type(handle: *const UrlInfo) -> *const c_char {
    unsafe {
        handle
            .as_ref()
            .and_then(|h| h.content_type.as_ref().map(|c| c.as_ptr()))
            .unwrap_or(std::ptr::null())
    }
}

#[unsafe(no_mangle)]
/// 获取安全的文件名
///
/// 注意悬垂指针问题，例如：
/// 1. 首次调用 `url_info_get_filename` 处理文件名，返回指针 A
/// 2. 再次调用 `url_info_get_filename` 缓存命中，返回指针 A
/// 3. 调用 `url_info_set_raw_name` 后，指针 A 指向的位置被释放，导致悬垂指针
pub unsafe extern "C" fn url_info_get_filename(handle: *const UrlInfo) -> *const c_char {
    unsafe { handle.as_ref().map_or(std::ptr::null(), UrlInfo::filename) }
}

#[unsafe(no_mangle)]
pub const unsafe extern "C" fn url_info_set_size(handle: *mut UrlInfo, size: u64) {
    if let Some(h) = unsafe { handle.as_mut() } {
        h.size = size;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn url_info_set_raw_name(handle: *mut UrlInfo, name: *const c_char) {
    if handle.is_null() || name.is_null() {
        return;
    }
    unsafe {
        (*handle).update_raw_name(CStr::from_ptr(name).to_owned());
    }
}

#[unsafe(no_mangle)]
pub const unsafe extern "C" fn url_info_set_supports_range(handle: *mut UrlInfo, value: bool) {
    if let Some(h) = unsafe { handle.as_mut() } {
        h.supports_range = value;
    }
}

#[unsafe(no_mangle)]
pub const unsafe extern "C" fn url_info_set_fast_download(handle: *mut UrlInfo, value: bool) {
    if let Some(h) = unsafe { handle.as_mut() } {
        h.fast_download = value;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn url_info_set_final_url(handle: *mut UrlInfo, url: *const c_char) {
    if handle.is_null() || url.is_null() {
        return;
    }
    unsafe { CStr::from_ptr(url).clone_into(&mut (*handle).final_url) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn url_info_set_etag(handle: *mut UrlInfo, etag: *const c_char) {
    if handle.is_null() {
        return;
    }
    let h = unsafe { &mut *handle };
    if etag.is_null() {
        h.etag = None;
    } else if let Ok(s) = unsafe { CStr::from_ptr(etag) }.to_str() {
        h.etag = Some(CString::new(s).unwrap_or_default());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn url_info_set_last_modified(
    handle: *mut UrlInfo,
    last_modified: *const c_char,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        (*handle).last_modified = last_modified.as_ref().map(|p| CStr::from_ptr(p).to_owned());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn url_info_set_content_type(
    handle: *mut UrlInfo,
    content_type: *const c_char,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        (*handle).content_type = content_type.as_ref().map(|p| CStr::from_ptr(p).to_owned());
    }
}
