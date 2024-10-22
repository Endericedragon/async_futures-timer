//! A general purpose crate for working with timeouts and delays with futures.
//!
//! # Examples
//!
//! ```no_run
//! # #[async_std::main]
//! # async fn main() {
//! use std::time::Duration;
//! use futures_timer::Delay;
//!
//! let now = Delay::new(Duration::from_secs(3)).await;
//! println!("waited for 3 secs");
//! # }
//! ```

#![no_std]
#![deny(missing_docs)]
#![warn(missing_debug_implementations)]

// mod native;
// pub use self::native::Delay;

use async_std::{self, time::Instant};
use core::{cell::RefCell, future::Future, task::Poll, time::Duration};

/// A future representing the notification that an elapsed duration has
/// occurred.
///
/// This is created through the `Delay::new` method indicating when the future should fire.
/// Note that these futures are not intended for high resolution timers, but rather they will
/// likely fire some granularity after the exact instant that they're otherwise indicated to fire
/// at.
#[derive(Debug)]
pub struct Delay {
    state: RefCell<DelayState>,
}

#[derive(Debug)]
struct DelayState {
    duration: Duration,
    time_start: Option<Instant>,
}

impl Delay {
    /// Creates a new future which will fire at `dur` time into the future.
    ///
    /// The returned object will be bound to the default timer for this thread.
    /// The default timer will be spun up in a helper thread on first use.
    #[inline]
    pub fn new(duration: Duration) -> Self {
        Self {
            state: RefCell::new(DelayState {
                duration: duration,
                time_start: None,
            }),
        }
    }

    /// Resets this timeout to an new timeout which will fire at the time
    /// specified by `at`.
    #[inline]
    pub fn reset(&mut self, duration: Duration) {
        let mut state = self.state.borrow_mut();
        state.duration = duration;
        state.time_start = None;
    }
}

impl Future for Delay {
    type Output = ();

    fn poll(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        let mut state = self.state.borrow_mut();
        if state.time_start.is_none() {
            state.time_start = Some(Instant::now());
        }

        let now = Instant::now();
        let deadline = Instant::from(*state.time_start.as_ref().unwrap()) + state.duration;
        if now >= deadline {
            Poll::Ready(())
        } else {
            // println_sync!("{:?}, {:?}", now, self.0);
            // 向执行器继续注册任务，保留该任务的Waker，防止其被丢弃
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}
