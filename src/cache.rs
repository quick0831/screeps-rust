use std::arch::wasm32::unreachable;
use std::ops::Deref;
use std::ops::DerefMut;
use std::sync::Mutex;
use std::sync::MutexGuard;

use log::error;

/// This type is meant to be used as a global static
///
/// This is a type of memory that store data between ticks, but not after code restart
pub struct Cache<T>(Mutex<Option<T>>);

pub struct CacheRef<'a, T>(MutexGuard<'a, Option<T>>);

impl<T> Cache<T> {
    pub const fn new() -> Self {
        Self(Mutex::new(None))
    }

    pub fn get_or_init<'a>(&'a self, init: impl FnOnce() -> T) -> CacheRef<'a, T> {
        let mut lock = match self.0.try_lock() {
            Ok(lock) => lock,
            Err(err) => {
                error!("Failed to lock Mutex: {err}");
                unreachable();
            }
        };
        lock.get_or_insert_with(init);
        CacheRef(lock)
    }
}

impl<T> Deref for CacheRef<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.0.deref().as_ref().unwrap_or_else(|| unreachable())
    }
}

impl<T> DerefMut for CacheRef<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.deref_mut().as_mut().unwrap_or_else(|| unreachable())
    }
}
