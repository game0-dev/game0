//! Thread-safe wrapper around UnsafeCell
//! Provides Send and Sync implementations for UnsafeCell

use std::cell::UnsafeCell;

/// A wrapper around UnsafeCell that implements Send and Sync
/// # Safety
/// This struct bypasses Rust's Send/Sync checks and should be used with extreme caution.
/// Ensure that the contained data is actually safe to be accessed concurrently before using this.
#[derive(Debug)]
pub struct SendSyncUnsafeCell<T>(UnsafeCell<T>);

// Safety: Explicitly implement Send and Sync
unsafe impl<T> Send for SendSyncUnsafeCell<T> {}
unsafe impl<T> Sync for SendSyncUnsafeCell<T> {}

impl<T> SendSyncUnsafeCell<T> {
    /// Create a new SendSyncUnsafeCell containing the given value
    pub fn new(value: T) -> Self {
        SendSyncUnsafeCell(UnsafeCell::new(value))
    }

    /// Get an immutable reference to the contained value
    pub fn get(&self) -> &T {
        unsafe { &*self.0.get() }
    }

    /// Get a mutable reference to the contained value
    pub fn get_mut(&self) -> &mut T {
        unsafe { &mut *self.0.get() }
    }

    /// Get the underlying UnsafeCell
    pub fn into_inner(self) -> UnsafeCell<T> {
        self.0
    }
}
