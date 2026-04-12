use std::ops::Deref;

#[derive(Clone, Debug, Default)]
pub struct CancellationToken {
    pub token: tokio_util::sync::CancellationToken,
}

impl Deref for CancellationToken {
    type Target = tokio_util::sync::CancellationToken;
    fn deref(&self) -> &Self::Target {
        &self.token
    }
}

/// 创建一个新的 `CancellationToken`
#[unsafe(no_mangle)]
pub extern "C" fn cancellation_token_new() -> *mut CancellationToken {
    Box::into_raw(Box::new(CancellationToken::default()))
}

/// 增加原 `CancellationToken` 的引用计数
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cancellation_token_retain(
    ptr: *const CancellationToken,
) -> *mut CancellationToken {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let token = unsafe { &*ptr };
    Box::into_raw(Box::new(token.clone()))
}

/// 减少原 `CancellationToken` 的引用计数，若计数归零则销毁内部数据
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cancellation_token_release(ptr: *mut *mut CancellationToken) {
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

/// 创建子取消令牌
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cancellation_token_child(
    ptr: *const CancellationToken,
) -> *mut CancellationToken {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let token = unsafe { &*ptr };
    Box::into_raw(Box::new(CancellationToken {
        token: token.child_token(),
    }))
}

/// 触发取消
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cancellation_token_cancel(ptr: *const CancellationToken) {
    if let Some(token) = unsafe { ptr.as_ref() } {
        token.cancel();
    }
}

/// 查询是否已取消，空指针永远返回 true
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cancellation_token_is_cancelled(ptr: *const CancellationToken) -> bool {
    unsafe { ptr.as_ref().is_none_or(|t| t.is_cancelled()) }
}
