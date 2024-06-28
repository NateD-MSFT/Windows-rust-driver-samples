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
mod wdf;

use once_cell::race::Lazy;
use public::{BarGraphState, OsrUsbFxLogger};
#[cfg(not(test))]
use wdk_alloc::WDKAllocator;
use alloc::boxed::Box;
use wdk_sys::{
    DRIVER_OBJECT,
    NTSTATUS,
    PCUNICODE_STRING,
    PFN_WDF_DRIVER_DEVICE_ADD,
    PUNICODE_STRING,
    PWDFDEVICE_INIT,
    PWDF_DRIVER_CONFIG,
    PWDF_OBJECT_ATTRIBUTES,
    UNICODE_STRING,
    WDFDRIVER,
    WDF_DRIVER_CONFIG,
};

extern crate alloc;

#[cfg(not(test))]
#[global_allocator]
static GLOBAL_ALLOCATOR: WDKAllocator = WDKAllocator;

static EVENT_LOGGER : Lazy<OsrUsbFxLogger> = Lazy::new(||OsrUsbFxLogger::new());

fn main() {}

#[link_section = "INIT"]
#[export_name = "DriverEntry"] // WDF expects a symbol with the name DriverEntry
extern "system" fn driver_entry(
    _driver: &mut DRIVER_OBJECT,
    _registry_path: PCUNICODE_STRING,
) -> NTSTATUS {
    let mut func_name: UNICODE_STRING = Default::default();

    let bar = BarGraphState(0);
    bar.get_bit::<9>();

    // TODO: how to get the func name call into global-ish state?

    let wdf_config: PWDF_DRIVER_CONFIG = &mut Default::default();
    let wdf_attributes: PWDF_OBJECT_ATTRIBUTES = &mut Default::default();
    let device_add: PFN_WDF_DRIVER_DEVICE_ADD = Some(osr_fx_evt_device_add);
    wdf::util::wdf_driver_config_init(wdf_config, device_add);
    let status = match (wdf::util::wdf_object_attributes_init(wdf_attributes)) {
        Ok(_) => 0,
        Err(_) => todo!(),
    };
    status
}

#[link_section = "PAGE"]
unsafe extern "C" fn osr_fx_evt_device_add(
    _driver: WDFDRIVER,
    device_init: PWDFDEVICE_INIT,
) -> NTSTATUS {
    0
}
