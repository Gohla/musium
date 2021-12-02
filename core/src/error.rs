use std::error::Error;

/// An error with only static references or owned values, that can be sent to and referenced from different threads.
///
/// [`Send`], [`Sync`], and `'static` are required for conversion to [`anyhow::Error`].
///
/// [`Sync`] is also needed in order to use the error in an [`Arc`] for shared ownership across threads, because an
/// [`Arc`] is only [`Send`] if the wrapped type implements [`Send`] and [`Sync`]. Shared ownership across threads may
/// be needed when cloning errors in a multi-threaded setting, as errors typically do not implement [`Clone`].
pub trait SyncError: 'static + Error + Send + Sync {}

impl<T: 'static + Error + Send + Sync> SyncError for T {}
