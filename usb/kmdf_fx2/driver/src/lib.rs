#![no_std]
#![cfg_attr(feature = "nightly", feature(hint_must_use))]
#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![warn(clippy::cargo)]
#![allow(clippy::missing_safety_doc)]

#[cfg(not(test))]
extern crate wdk_panic;

mod public;

use public::BarGraphState;
#[cfg(not(test))]
use wdk_alloc::WDKAllocator;
use wdk_sys::{DRIVER_OBJECT, NTSTATUS, PCUNICODE_STRING};

extern crate alloc;

#[cfg(not(test))]
#[global_allocator]
static GLOBAL_ALLOCATOR: WDKAllocator = WDKAllocator;

fn main() {}

#[link_section = "INIT"]
#[export_name = "DriverEntry"] // WDF expects a symbol with the name DriverEntry
extern "system" fn driver_entry(
    _driver: &mut DRIVER_OBJECT,
    _registry_path: PCUNICODE_STRING,
) -> NTSTATUS {
    let mut value = BarGraphState(2);
    value.set_bit(true, 2);
    // value.get_bit::<9>(); Does not compile
    i32::from(value.0)
}
