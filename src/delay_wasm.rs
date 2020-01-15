//! A version of `Delay` that works on wasm.

use gloo_timers::future::TimeoutFuture;
use send_wrapper::SendWrapper;
use std::{time::Duration, pin::Pin, task::{Context, Poll}, future::Future};

/// A version of `Delay` that works on wasm.
#[derive(Debug)]
pub struct Delay(SendWrapper<TimeoutFuture>);

impl Delay {
	/// Creates a new future which will fire at `dur` time into the future.
	#[inline]
	pub fn new(dur: Duration) -> Delay {
		Self(
			SendWrapper::new(TimeoutFuture::new(dur.as_millis() as u32))
		)
	}
}

impl Future for Delay {
	type Output = ();

	fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
		Pin::new(&mut *Pin::into_inner(self).0).poll(cx)
	}
}
