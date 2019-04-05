use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, Condvar, PoisonError,
    },
    time::Duration,
};

use futures::{Future, Async};

#[derive(Debug)]
pub enum CompletionError {
    Deadlock,
    AlreadyComplete,
    TimedOut,
}

impl<T> From<PoisonError<T>> for CompletionError {
    fn from(_: PoisonError<T>) -> Self {
        CompletionError::Deadlock
    }
}

#[derive(Clone)]
pub struct CompletableFuture<T: Clone + Send + Sync> {
    inner: Arc<(AtomicBool, Mutex<Option<T>>, Condvar)>,
}

impl<T: Clone + Send + Sync> CompletableFuture<T> {
    pub fn new() -> Self {
        CompletableFuture {
            inner: Arc::new((AtomicBool::new(false), Mutex::new(None), Condvar::new())),
        }
    }

    pub fn completed(value: T) -> Self {
        CompletableFuture {
            inner: Arc::new((AtomicBool::new(true), Mutex::new(Some(value)), Condvar::new())),
        }
    }

    pub fn complete(&mut self, value: T) -> Result<(), CompletionError> {
        let &(ref complete, ref lock, ref cvar)  = &*self.inner;
        if Self::check_atomic(complete) {
            return Err(CompletionError::AlreadyComplete);
        }
        let mut option = lock.lock()?;
        if Self::check_atomic(complete) {
            return Err(CompletionError::AlreadyComplete);
        }
        complete.store(true, Ordering::Relaxed);
        *option = Some(value);
        cvar.notify_all();
        Ok(())
    }

    pub fn wait_timeout(&self, duration: Duration) -> Result<T, CompletionError> {
        let &(_, ref lock, ref cvar)  = &*self.inner;
        let mut option = lock.lock()?;
        while option.is_none() {
            let result = cvar.wait_timeout(option, duration)?;
            if result.1.timed_out() {
                return Err(CompletionError::TimedOut);
            }
            option = result.0;
        }
        Ok(option.clone().unwrap())
    }

    pub fn is_complete(&self) -> bool {
        let &(ref complete, _, _)  = &*self.inner;
        Self::check_atomic(complete)
    }

    fn check_atomic(complete: &AtomicBool) -> bool {
        complete.load(Ordering::Relaxed)
    }
}

impl<T: Clone + Send + Sync> Future for CompletableFuture<T> {
    type Item = T;
    type Error = CompletionError;

    fn poll(&mut self) -> Result<Async<Self::Item>, Self::Error> {
        let &(ref complete, ref lock, _) = &*self.inner;
        if !Self::check_atomic(complete) {
            return Ok(Async::NotReady)
        }
        let option = lock.lock()?;
        match option.clone() {
            Some(value) => Ok(Async::Ready(value)),
            None => Ok(Async::NotReady),
        }
    }
}