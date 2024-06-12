pub trait Deleter {
    unsafe fn delete(ptr: *mut Self) -> i32;
}

pub struct Wrapper<T: 'static + Deleter + ?Sized>(*mut T);

impl<T: 'static + Deleter + ?Sized> Wrapper<T> {
    pub fn new(v: *mut T) -> Self {
        Self(v)
    }

    #[inline(always)]
    pub fn as_mut_ptr(&self) -> *mut T {
        self.0
    }
}

impl<T: 'static + Deleter + ?Sized> Drop for Wrapper<T> {
    fn drop(&mut self) {
        match unsafe { Deleter::delete(self.0) } {
            0 => {
                #[cfg(feature = "debug")]
                tracing::debug!("release {} succ", std::any::type_name::<T>());
            }
            r => tracing::error!("release {} failed: {}", std::any::type_name::<T>(), r),
        }
    }
}

impl<T: 'static + Deleter + ?Sized> std::ops::Deref for Wrapper<T> {
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0 }
    }
}

unsafe impl<T: 'static + Deleter + ?Sized> Send for Wrapper<T> {}
unsafe impl<T: 'static + Deleter + ?Sized> Sync for Wrapper<T> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrapper() {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::WARN)
            .init();

        struct ExpectedReleaseError;
        impl Deleter for ExpectedReleaseError {
            unsafe fn delete(_ptr: *mut Self) -> i32 {
                2333
            }
        }

        type Dummy = Wrapper<ExpectedReleaseError>;
        let _ = Dummy::new(std::ptr::null_mut());
    }
}
