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
mod trace;
mod wdf;

use alloc::format;
use core::{borrow::BorrowMut, mem::size_of, ptr::null};

use lazy_static::lazy_static;
use public::BarGraphState;
use trace::OsrUsbFxLogger;
use wdk::nt_success;
#[cfg(not(test))]
use wdk_alloc::WDKAllocator;
use wdk_sys::{
    macros,
    ntddk::{IoGetActivityIdIrp, MmGetSystemRoutineAddress, RtlInitUnicodeString},
    BOOLEAN,
    DRIVER_OBJECT,
    LPGUID,
    NTSTATUS,
    PCUNICODE_STRING,
    PCWSTR,
    PDRIVER_OBJECT,
    PFN_WDF_DRIVER_DEVICE_ADD,
    PIRP,
    PUNICODE_STRING,
    PVOID,
    PWDFDEVICE_INIT,
    PWDF_DRIVER_CONFIG,
    PWDF_OBJECT_ATTRIBUTES,
    STATUS_SUCCESS,
    UNICODE_STRING,
    WCHAR,
    WDFDRIVER,
    WDFOBJECT,
    WDF_DRIVER_CONFIG,
    WDF_NO_HANDLE,
    WDF_NO_OBJECT_ATTRIBUTES,
};
use widestring::WideCString;
use win_etw_provider::EventOptions;

extern crate alloc;

#[cfg(not(test))]
#[global_allocator]
static GLOBAL_ALLOCATOR: WDKAllocator = WDKAllocator;
lazy_static! {
    static ref EVENT_LOGGER: OsrUsbFxLogger = { OsrUsbFxLogger::new() };
}
type FnIoGetActivityIdIrp = unsafe extern "C" fn(PIRP, LPGUID) -> NTSTATUS;
lazy_static! {
    static ref IO_GET_ACTIVITY_ID_IRP: Option<FnIoGetActivityIdIrp> = unsafe {
        // Safety: IoGetActivityIdIrp has the appropriate signature.
        get_system_routine_address_from_str::<FnIoGetActivityIdIrp>("IoGetActivityIdIrp")
    };
}
type FnIoSetDeviceInterfacePropertyData =
    unsafe extern "C" fn(PUNICODE_STRING, BOOLEAN) -> NTSTATUS;
lazy_static! {
    static ref IO_SET_DEVICE_INTERFACE_PROPERTY_DATA: Option<FnIoSetDeviceInterfacePropertyData> = unsafe {
        // Safety: IoSetDeviceInterfacePropertyData has the appropriate signature.
        get_system_routine_address_from_str::<FnIoSetDeviceInterfacePropertyData>("IoSetDeviceInterfacePropertyData")
    };
}

/// Looks up a system routine address for a function with signature `T`.  
///
/// Attempting to call with `T` not a pointer type will result in a compilation
/// failure.
///
/// Returns None if no system routine of that name can be found.
///
/// Safety:
///
/// The caller must provide the appropriate function signature as `T` for the
/// system routine they intend to use.  Providing the wrong signature will
/// result in returning a function pointer with the wrong signature, leading to
/// undefined behavior if used.
unsafe fn get_system_routine_address_from_str<T>(routine_name: &str) -> Option<T>
where
    T: Clone,
{
    // First, assert that we are casting from a pointer-sized type.
    const { assert!(size_of::<T>() == size_of::<PVOID>()) };
    let mut result: Option<T> = None;
    let mut io_get_string = UNICODE_STRING::default();
    if let Ok(io_get_activity_string) = widestring::WideCString::from_str(routine_name) {
        // SAFETY: If we get this far, we have a valid wide string for the function name
        // we're looking for, as well as a valid UNICODE_STRING to store the
        // results in.
        unsafe {
            RtlInitUnicodeString(io_get_string.borrow_mut(), io_get_activity_string.as_ptr());
            let function_address = MmGetSystemRoutineAddress(io_get_string.borrow_mut());

            if !PVOID::is_null(function_address) {
                // SAFETY: We asserted at the top that we are passing in a pointer the size of
                // PVOID.  We use transmute_copy because transmute cannot work on generic types.
                // It is the user's responsibility to ensure they are using the correct function
                // signature.
                result = Some(core::mem::transmute_copy::<PVOID, T>(&function_address))
            }
        }
    }
    result.clone()
}

fn main() {}

#[link_section = "INIT"]
#[inline(never)]
#[export_name = "DriverEntry"] // WDF expects a symbol with the name DriverEntry
extern "system" fn driver_entry(
    driver: &mut DRIVER_OBJECT,
    registry_path: PCUNICODE_STRING,
) -> NTSTATUS {
    EVENT_LOGGER.send_string(
        Some(&EventOptions {
            level: Some(win_etw_provider::Level::INFO),
            activity_id: Default::default(),
            related_activity_id: Default::default(),
        }),
        "OSRUSBFX2 Driver Sample - Driver Framework Edition.\n",
    );

    let wdf_config: PWDF_DRIVER_CONFIG = &mut Default::default();
    let wdf_attributes: PWDF_OBJECT_ATTRIBUTES = &mut Default::default();
    let device_add: PFN_WDF_DRIVER_DEVICE_ADD = Some(osr_fx_evt_device_add);
    wdf::util::wdf_driver_config_init(wdf_config, device_add);
    let status = match (wdf::util::wdf_object_attributes_init(wdf_attributes)) {
        Ok(_) => STATUS_SUCCESS,
        Err(_) => todo!(),
    };
    unsafe {
        // Safety: This is a direct assignment to an initialized structure which
        // is not shared with any other threads or references.
        (*wdf_attributes).EvtCleanupCallback = Some(osr_fx_evt_driver_context_cleanup);
    }
    let status = unsafe {
        macros::call_unsafe_wdf_function_binding!(
            WdfDriverCreate,
            driver as PDRIVER_OBJECT,
            registry_path,
            wdf_attributes,
            wdf_config,
            WDF_NO_HANDLE as *mut WDFDRIVER
        )
    };

    if !nt_success(status) {
        EVENT_LOGGER.send_string(
            Some(&EventOptions {
                level: Some(win_etw_provider::Level::ERROR),
                activity_id: Default::default(),
                related_activity_id: Default::default(),
            }),
            format!("WdfDriverCreate failed with status 0x{status}.\n").as_str(),
        );
    }

    status
}

#[link_section = "PAGE"]
unsafe extern "C" fn osr_fx_evt_device_add(
    _driver: WDFDRIVER,
    device_init: PWDFDEVICE_INIT,
) -> NTSTATUS {
    0
}

#[link_section = "PAGE"]
unsafe extern "C" fn osr_fx_evt_driver_context_cleanup(driver: WDFOBJECT) {
    EVENT_LOGGER.send_string(
        Some(&EventOptions {
            level: Some(win_etw_provider::Level::INFO),
            activity_id: Default::default(),
            related_activity_id: Default::default(),
        }),
        "--> OsrFxEvtDriverContextCleanup\n",
    );
}
