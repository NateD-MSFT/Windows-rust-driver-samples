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
    driver: &mut DRIVER_OBJECT,
    registry_path: PCUNICODE_STRING,
) -> NTSTATUS {
    0
}
