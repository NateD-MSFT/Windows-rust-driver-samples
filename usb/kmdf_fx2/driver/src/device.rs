use core::ptr::{null, null_mut};

use wdk::{nt_success, paged_code};
use wdk_sys::{
    macros, ntddk::KeGetCurrentIrql, APC_LEVEL, DEVICE_REGISTRY_PROPERTY::{
        DevicePropertyDeviceDescription,
        DevicePropertyFriendlyName,
        DevicePropertyLocationInformation,
    }, PVOID, STATUS_SUCCESS, WDFDEVICE, WDFMEMORY, WDFOBJECT, WDF_NO_OBJECT_ATTRIBUTES, WDF_OBJECT_ATTRIBUTES, _POOL_TYPE::NonPagedPoolNx
};

use crate::osrusbfx2::device_get_context;

pub fn get_device_logging_names(device: WDFDEVICE) {
    let mut pDevContext = unsafe { device_get_context(device as WDFOBJECT) };

    let mut objectAttributes: WDF_OBJECT_ATTRIBUTES = Default::default();
    let mut deviceNameMemory: WDFMEMORY = null_mut();
    let mut locationMemory: WDFMEMORY = null_mut();

    let mut status = STATUS_SUCCESS;

    paged_code!();

    crate::wdf::util::wdf_object_attributes_init(&mut objectAttributes);
    objectAttributes.ParentObject = device as PVOID;

    status = unsafe {
        macros::call_unsafe_wdf_function_binding!(
            WdfDeviceAllocAndQueryProperty,
            device,
            DevicePropertyFriendlyName,
            NonPagedPoolNx,
            &mut objectAttributes,
            &mut deviceNameMemory
        )
    };

    if !nt_success(status) {
        status = unsafe {
            macros::call_unsafe_wdf_function_binding!(
                WdfDeviceAllocAndQueryProperty,
                device,
                DevicePropertyDeviceDescription,
                NonPagedPoolNx,
                &mut objectAttributes,
                &mut deviceNameMemory
            )
        };
    }

    if nt_success(status) {
        unsafe {
            (*pDevContext).DeviceNameMemory = deviceNameMemory;
            (*pDevContext).DeviceName = macros::call_unsafe_wdf_function_binding!(
                WdfMemoryGetBuffer,
                deviceNameMemory,
                null_mut(),
            ) as *const u16;
        }
    } else {
        unsafe {
            (*pDevContext).DeviceNameMemory = null_mut(); // Redundant given above initialization but matching OSR driver
            (*pDevContext).DeviceName = widestring::u16cstr!("(error retrieving name)").as_ptr(); // Concern: is this correctly allocated or is this stack memory?
        }
    }

    // Retrieve the device location string.
    //

    status = unsafe {
        macros::call_unsafe_wdf_function_binding!(
            WdfDeviceAllocAndQueryProperty,
            device,
            DevicePropertyLocationInformation,
            NonPagedPoolNx,
            WDF_NO_OBJECT_ATTRIBUTES,
            &mut locationMemory
        )
    };

    if nt_success(status) {
        unsafe {
            (*pDevContext).LocationMemory = deviceNameMemory;
            (*pDevContext).Location = macros::call_unsafe_wdf_function_binding!(
                WdfMemoryGetBuffer,
                locationMemory,
                null_mut(),
            ) as *const u16;
        }
    } else {
        unsafe {
            (*pDevContext).LocationMemory = null_mut(); // Redundant given above initialization but matching OSR driver
            (*pDevContext).Location = widestring::u16cstr!("(error retrieving location)").as_ptr(); // Concern: is this correctly allocated or is this stack memory?
        }
    }
}
