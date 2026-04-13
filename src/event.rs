use std::ffi::CString;
use std::os::raw::{c_char, c_void};

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventType {
    PrefetchError = 0,
    Pulling,
    PullError,
    PullTimeout,
    PullProgress,
    Pushing,
    PushError,
    PushProgress,
    Flushing,
    FlushError,
    Finished,
}

/// 当下载过程中发生事件时，此回调会被调用。
///
/// # 参数
/// - `context`: 用户注册回调时提供的自定义指针，原样传回。
/// - `event_type`: 事件类型，见 `EventType` 枚举。
/// - `id`: 关联的线程 ID（仅部分事件有效，否则为 0）。
/// - `message`: 错误消息或描述（仅部分事件有效，否则为 NULL）。
/// - `range_start`: 进度范围的起始字节，包含（仅部分事件有效有效）。
/// - `range_end`: 进度范围的结束字节，不包含（仅部分事件有效有效）。
///
/// # 安全性
/// - `message` 指向的字符串仅在回调函数内有效，回调返回后可能被释放，调用者不应保存该指针或在其外部使用。
pub type EventCallback = Option<
    extern "C" fn(
        context: *mut c_void,
        event_type: EventType,
        id: usize,
        message: *const c_char,
        range_start: u64,
        range_end: u64,
    ),
>;

#[derive(Debug)]
pub struct CallbackContext {
    pub callback: EventCallback,
    pub context: *mut c_void,
}

impl CallbackContext {
    pub fn emit_c_event(&self, event: fast_down_ffi::Event) {
        let (event_type, id, message, range_start, range_end) = match event {
            fast_down_ffi::Event::PrefetchError(e) => {
                let msg = CString::new(e).unwrap_or_default();
                (EventType::PrefetchError, 0, Some(msg), 0, 0)
            }
            fast_down_ffi::Event::Pulling(id) => (EventType::Pulling, id, None, 0, 0),
            fast_down_ffi::Event::PullError(id, e) => {
                let msg = CString::new(e).unwrap_or_default();
                (EventType::PullError, id, Some(msg), 0, 0)
            }
            fast_down_ffi::Event::PullTimeout(id) => (EventType::PullTimeout, id, None, 0, 0),
            fast_down_ffi::Event::PullProgress(id, range) => {
                (EventType::PullProgress, id, None, range.start, range.end)
            }
            fast_down_ffi::Event::Pushing(id, range) => {
                (EventType::Pushing, id, None, range.start, range.end)
            }
            fast_down_ffi::Event::PushError(id, range, e) => {
                let msg = CString::new(e).unwrap_or_default();
                (EventType::PushError, id, Some(msg), range.start, range.end)
            }
            fast_down_ffi::Event::PushProgress(id, range) => {
                (EventType::PushProgress, id, None, range.start, range.end)
            }
            fast_down_ffi::Event::Flushing => (EventType::Flushing, 0, None, 0, 0),
            fast_down_ffi::Event::FlushError(e) => {
                let msg = CString::new(e).unwrap_or_default();
                (EventType::FlushError, 0, Some(msg), 0, 0)
            }
            fast_down_ffi::Event::Finished(id) => (EventType::Finished, id, None, 0, 0),
        };
        let msg_ptr = message.as_ref().map_or(std::ptr::null(), |m| m.as_ptr());
        if let Some(cb) = self.callback {
            (cb)(
                self.context,
                event_type,
                id,
                msg_ptr,
                range_start,
                range_end,
            );
        }
    }
}
