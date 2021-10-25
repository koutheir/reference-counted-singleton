#![doc(html_root_url = "https://docs.rs/reference-counted-singleton/0.1.1")]

/*!
[`RefCountedSingleton`] is a reference-counted singleton whose protected data
can be recreated as needed.

The protected data is created when [`RefCountedSingleton::get_or_init`]
is called.
That functions returns an [`RCSRef`] reference to the singleton.

[`RCSRef`] instances can be cloned as needed.
The last [`RCSRef`] reference drops the data.
Calling [`RefCountedSingleton::get_or_init`] again recreates the data.
*/

#[cfg(test)]
mod tests;

use std::error::Error;
use std::hash::{Hash, Hasher};
use std::mem::ManuallyDrop;
use std::ops::Deref;
use std::sync::{Arc, Mutex};

/// A reference-counted singleton whose protected data can be recreated
/// as needed.
///
/// The protected data is created when [`RefCountedSingleton::get_or_init`]
/// is called.
/// That functions returns an [`RCSRef`] reference to the singleton.
///
/// [`RCSRef`] instances can be cloned as needed.
/// The last [`RCSRef`] reference drops the data.
/// Calling [`RefCountedSingleton::get_or_init`] again recreates the data.
#[derive(Debug)]
pub struct RefCountedSingleton<T>(Mutex<Option<Arc<T>>>);

impl<T> Default for RefCountedSingleton<T> {
    fn default() -> Self {
        Self(Mutex::new(None))
    }
}

impl<T> RefCountedSingleton<T> {
    /// Return a counted reference to the protected data if such data exists,
    /// otherwise creates a new instance of the data by calling `creator()`.
    ///
    /// If the lock is poisoned, then this returns `Err(None)`.
    /// If `creator()` returns an error `err`, then this returns
    /// `Err(Some(err))`.
    pub fn get_or_init<E: Error>(
        &'_ self,
        creator: impl FnOnce() -> Result<T, E>,
    ) -> Result<RCSRef<'_, T>, Option<E>> {
        if let Ok(mut value) = self.0.lock() {
            match *value {
                // Data is not created.
                None => match creator() {
                    Ok(data) => {
                        // We created a new instance.
                        let data = Arc::new(data);
                        let data_ref = Arc::clone(&data);

                        *value = Some(data);

                        Ok(RCSRef {
                            data: ManuallyDrop::new(data_ref),
                            rcs: self,
                        })
                    }

                    // Failed to create a new instance of the data.
                    Err(err) => Err(Some(err)),
                },

                // Data is already created. Return a new reference.
                Some(ref data) => Ok(RCSRef {
                    data: ManuallyDrop::new(Arc::clone(data)),
                    rcs: self,
                }),
            }
        } else {
            Err(None) // The mutex was poisoned.
        }
    }

    /// Return a counted reference to the protected data if such data exists.
    ///
    /// If such data is not instantiated, or the lock is poisoned, then this
    /// returns `None`.
    pub fn get(&'_ self) -> Option<RCSRef<'_, T>> {
        self.0.lock().ok().and_then(|value| {
            value.as_ref().map(|data| RCSRef {
                data: ManuallyDrop::new(Arc::clone(data)),
                rcs: self,
            })
        })
    }
}

/// Read-only counted reference to an instance of [`RefCountedSingleton`].
#[derive(Debug)]
pub struct RCSRef<'t, T> {
    data: ManuallyDrop<Arc<T>>,
    rcs: &'t RefCountedSingleton<T>,
}

impl<'t, T: PartialEq> PartialEq for RCSRef<'t, T> {
    fn eq(&self, other: &Self) -> bool {
        self.data.as_ref().deref().eq(other.data.as_ref().deref())
    }
}

impl<'t, T: Eq> Eq for RCSRef<'t, T> {}

impl<'t, T: PartialOrd> PartialOrd for RCSRef<'t, T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.data
            .as_ref()
            .deref()
            .partial_cmp(other.data.as_ref().deref())
    }
}

impl<'t, T: Ord> Ord for RCSRef<'t, T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.data.as_ref().deref().cmp(other.data.as_ref().deref())
    }
}

impl<'t, T: Hash> Hash for RCSRef<'t, T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.data.as_ref().deref().hash(state)
    }
}

impl<'t, T> Deref for RCSRef<'t, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.data.deref().deref()
    }
}

impl<'t, T> Clone for RCSRef<'t, T> {
    fn clone(&self) -> Self {
        Self {
            data: ManuallyDrop::new(Arc::clone(&self.data)),
            rcs: self.rcs,
        }
    }
}

impl<'t, T> Drop for RCSRef<'t, T> {
    fn drop(&mut self) {
        // Drop our own counted reference.
        // SAFETY: `self.data` is not used after this.
        unsafe { ManuallyDrop::drop(&mut self.data) };

        if let Ok(mut value) = self.rcs.0.lock() {
            if let Some(data) = value.take() {
                match Arc::try_unwrap(data) {
                    // Singleton locked, and there are no more counted references to it.
                    // Destroy the singleton.
                    Ok(data) => drop(data),

                    // Singleton locked, but there are other counted references to it.
                    // Put the singleton data back.
                    Err(data) => *value = Some(data),
                }
            }
        }
    }
}
