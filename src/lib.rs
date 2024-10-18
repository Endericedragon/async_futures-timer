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

mod native;
pub use self::native::Delay;
