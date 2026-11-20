//! Checkpoint 2: `postage`, parcels and the carriers that price them.
//! Reference solution.
//!
//! The crate has three modules, and each one knows only the ones above it
//! in this list:
//!
//! - [`parcel`]: what is being sent. Validated values, no prices.
//! - [`carrier`]: who can send it, and for how much.
//! - [`quote`]: comparing carriers.
//!
//! The names that most callers need are re-exported here, so `postage::Parcel`
//! and `postage::parcel::Parcel` are the same type.

pub mod carrier;
pub mod parcel;
pub mod quote;

pub use carrier::{Carrier, Dash, Discounted, Region, Snail, Zone};
pub use parcel::{Contents, Dimensions, Grams, Parcel, ParcelError, SizeClass};
pub use quote::{Quote, cheapest, total};
