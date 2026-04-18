use bytes::Bytes;
use fast_down_ffi::{ProgressEntry, Pusher};
use std::ffi::c_void;
use std::os::raw::c_int;

/// 推送数据回调
///
/// - `context`: 用户自定义指针
/// - `offset`: 数据在文件中的起始偏移量
/// - `data`: 数据指针
/// - `len`: 数据长度
///
/// 返回值：0 成功，非 0 失败
pub type PushCallback =
    Option<extern "C" fn(context: *mut c_void, offset: u64, data: *const u8, len: usize) -> c_int>;

/// 刷新回调
///
/// - `context`: 用户自定义指针
///
/// 返回值：0 成功，非 0 失败
pub type FlushCallback = Option<extern "C" fn(context: *mut c_void) -> c_int>;

pub struct CPusher {
    push_cb: PushCallback,
    flush_cb: FlushCallback,
    context: *mut c_void,
}

// 显式标记 Send，因为调用者保证 context 及其回调线程安全
unsafe impl Send for CPusher {}

impl CPusher {
    pub fn new(push_cb: PushCallback, flush_cb: FlushCallback, context: *mut c_void) -> Self {
        Self {
            push_cb,
            flush_cb,
            context,
        }
    }

    /// 发送数据到 C 回调
    fn send_to_c(&self, offset: u64, data: &[u8]) -> Result<(), String> {
        let Some(push_cb) = self.push_cb else {
            return Err("Push callback is none".to_string());
        };
        let ret = push_cb(self.context, offset, data.as_ptr(), data.len());
        if ret == 0 {
            Ok(())
        } else {
            Err(format!("Push callback returned error code: {ret}"))
        }
    }
}

impl Pusher for CPusher {
    type Error = String;

    fn push(&mut self, range: &ProgressEntry, content: Bytes) -> Result<(), (Self::Error, Bytes)> {
        self.send_to_c(range.start, &content)
            .map_err(|e| (e, content))
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        if let Some(flush_cb) = &self.flush_cb {
            let ret = flush_cb(self.context);
            if ret != 0 {
                return Err(format!("Flush callback returned error code: {ret}"));
            }
        }
        Ok(())
    }
}
