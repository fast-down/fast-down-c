use bytes::{Bytes, BytesMut};
use fast_down_ffi::{ProgressEntry, Pusher};
use std::collections::BTreeMap;
use std::ffi::c_void;
use std::os::raw::c_int;

/// 推送数据回调
/// - `context`: 用户自定义指针
/// - `offset`: 数据在文件中的起始偏移量
/// - `data`: 数据指针
/// - `len`: 数据长度
/// 返回值：0 成功，非 0 失败
pub type PushCallback =
    extern "C" fn(context: *mut c_void, offset: u64, data: *const u8, len: usize) -> c_int;

/// 刷新回调
/// - `context`: 用户自定义指针
/// 返回值：0 成功，非 0 失败
pub type FlushCallback = extern "C" fn(context: *mut c_void) -> c_int;

pub struct CPusher {
    push_cb: PushCallback,
    flush_cb: Option<FlushCallback>,
    cache: BTreeMap<u64, Bytes>,
    buffer_size: usize,
    cache_size: usize,
    context: *mut c_void,
}

// 显式标记 Send，因为调用者保证 context 及其回调线程安全
unsafe impl Send for CPusher {}

impl CPusher {
    pub fn new(
        push_cb: PushCallback,
        flush_cb: Option<FlushCallback>,
        buffer_size: usize,
        context: *mut c_void,
    ) -> Self {
        Self {
            push_cb,
            flush_cb,
            context,
            buffer_size,
            cache: BTreeMap::new(),
            cache_size: 0,
        }
    }

    /// 发送数据到 C 回调
    fn send_to_c(&self, offset: u64, data: &[u8]) -> Result<(), String> {
        let ret = (self.push_cb)(self.context, offset, data.as_ptr(), data.len());
        if ret == 0 {
            Ok(())
        } else {
            Err(format!("Push callback returned error code: {}", ret))
        }
    }

    /// 内部刷新：合并连续小块并发送
    fn flush_buffer(&mut self) -> Result<(), String> {
        let mut curr_start: Option<u64> = None;
        let mut curr_end: u64 = 0;
        let mut buf = BytesMut::new();
        while let Some((start, chunk)) = self.cache.pop_first() {
            let len = chunk.len();
            self.cache_size -= len;
            if let Some(c_start) = curr_start {
                if start <= curr_end {
                    let overlap = curr_end - start;
                    if overlap < (len as u64) {
                        #[allow(clippy::cast_possible_truncation)]
                        let new_data = &chunk[(overlap as usize)..];
                        buf.extend_from_slice(new_data);
                        curr_end += new_data.len() as u64;
                    }
                    continue;
                }
                let data_to_send = buf.split().freeze();
                if let Err(e) = self.send_to_c(c_start, &data_to_send) {
                    self.cache_size += data_to_send.len() + len;
                    self.cache.insert(c_start, data_to_send);
                    self.cache.insert(start, chunk);
                    return Err(e);
                }
            }
            curr_start = Some(start);
            curr_end = start + len as u64;
            buf.extend_from_slice(&chunk);
        }
        if let Some(c_start) = curr_start {
            if !buf.is_empty() {
                let data_to_send = buf.freeze();
                if let Err(e) = self.send_to_c(c_start, &data_to_send) {
                    self.cache_size += data_to_send.len();
                    self.cache.insert(c_start, data_to_send);
                    return Err(e);
                }
            }
        }
        Ok(())
    }
}

impl Pusher for CPusher {
    type Error = String;

    fn push(&mut self, range: &ProgressEntry, content: Bytes) -> Result<(), (Self::Error, Bytes)> {
        let start = range.start;
        let new_len = content.len();
        match self.cache.get(&start) {
            Some(old) if new_len <= old.len() => return Ok(()),
            Some(old) => self.cache_size -= old.len(),
            None => {}
        }
        self.cache.insert(start, content);
        self.cache_size += new_len;
        if self.cache_size >= self.buffer_size {
            self.flush_buffer().map_err(|e| {
                let failed_bytes = self.cache.remove(&range.start).unwrap_or_default();
                self.cache_size -= failed_bytes.len();
                (e, failed_bytes)
            })?;
        }
        Ok(())
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        self.flush_buffer()?;
        if let Some(flush_cb) = &self.flush_cb {
            let ret = (flush_cb)(self.context);
            if ret != 0 {
                return Err(format!("Flush callback returned error code: {}", ret));
            }
        }
        Ok(())
    }
}
