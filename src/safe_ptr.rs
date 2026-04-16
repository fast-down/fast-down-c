use std::ops::{Deref, DerefMut};

pub struct SafePtr<T>(pub T);
#[allow(clippy::non_send_fields_in_send_ty)]
unsafe impl<T> Send for SafePtr<T> {}
#[allow(clippy::non_send_fields_in_send_ty)]
unsafe impl<T> Sync for SafePtr<T> {}

impl<T> Deref for SafePtr<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for SafePtr<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> SafePtr<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}
