#![no_std]
#![cfg_attr(feature = "nightly", feature(hint_must_use))]
#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![warn(clippy::cargo)]
#![allow(clippy::missing_safety_doc)]

#[cfg(not(test))]
extern crate wdk_panic;

mod ioctl;
mod osrusbfx2;
mod public;
mod trace;
mod wdf;
mod wdf_object_context;
use alloc::format;
use core::{borrow::BorrowMut, mem::size_of, ops::Deref, ptr::null_mut};

use wdf::util::{wdf_device_pnp_capabilities_init, wdf_io_queue_config_init_default_queue};
mod device;

use lazy_static::lazy_static;
use osrusbfx2::device_get_context;
use wdk::nt_success;
#[cfg(not(test))]
use wdk_alloc::WDKAllocator;
use wdk_sys::{
    macros,
    ntddk::{MmGetSystemRoutineAddress, RtlInitUnicodeString},
    BOOLEAN,
    DRIVER_OBJECT,
    LPGUID,
    NTSTATUS,
    PCUNICODE_STRING,
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
    WDFCMRESLIST,
    WDFDEVICE,
    WDFDEVICE__,
    WDFDRIVER,
    WDFOBJECT,
    WDFQUEUE,
    WDFQUEUE__,
    WDF_DEVICE_PNP_CAPABILITIES,
    WDF_IO_QUEUE_CONFIG,
    WDF_NO_HANDLE,
    WDF_NO_OBJECT_ATTRIBUTES,
    WDF_OBJECT_ATTRIBUTES,
    WDF_OBJECT_CONTEXT_TYPE_INFO,
    WDF_PNPPOWER_EVENT_CALLBACKS,
    WDF_POWER_DEVICE_STATE,
    _WDF_DEVICE_IO_TYPE,
    _WDF_EXECUTION_LEVEL::WdfExecutionLevelInheritFromParent,
    _WDF_IO_QUEUE_DISPATCH_TYPE::WdfIoQueueDispatchParallel,
    _WDF_OBJECT_ATTRIBUTES,
    _WDF_SYNCHRONIZATION_SCOPE::WdfSynchronizationScopeInheritFromParent,
    _WDF_TRI_STATE::WdfTrue,
};
use win_etw_provider::EventOptions;

use crate::{
    osrusbfx2::*,
    trace::{trace_events, EVENT_LOGGER},
    wdf_object_context::wdf_get_context_type_info,
};

extern crate alloc;

#[cfg(not(test))]
#[global_allocator]
static GLOBAL_ALLOCATOR: WDKAllocator = WDKAllocator;
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
    wdf::util::wdf_object_attributes_init(wdf_attributes);
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
    driver: WDFDRIVER,
    mut device_init: PWDFDEVICE_INIT,
) -> NTSTATUS {
    EVENT_LOGGER.send_string(
        Some(&EventOptions {
            level: Some(win_etw_provider::Level::INFO),
            activity_id: Default::default(),
            related_activity_id: Default::default(),
        }),
        "--> OsrFxEvtDeviceAdd routine\n",
    );

    // Initialize the pnpPowerCallbacks structure.  Callback events for PNP
    // and Power are specified here.  If you don't supply any callbacks,
    // the Framework will take appropriate default actions based on whether
    // DeviceInit is initialized to be an FDO, a PDO or a filter device
    // object.
    //

    let pnp_power_callbacks = &mut WDF_PNPPOWER_EVENT_CALLBACKS {
        Size: core::mem::size_of::<WDF_PNPPOWER_EVENT_CALLBACKS>() as u32,
        EvtDeviceD0Entry: Some(OsrFxEvtDeviceD0Entry),
        EvtDevicePrepareHardware: Some(OsrFxEvtDevicePrepareHardware),
        EvtDeviceD0Exit: Some(OsrFxEvtDeviceD0Exit),
        ..Default::default()
    };

    macros::call_unsafe_wdf_function_binding!(
        WdfDeviceInitSetPnpPowerEventCallbacks,
        device_init,
        pnp_power_callbacks
    );

    macros::call_unsafe_wdf_function_binding!(
        WdfDeviceInitSetIoType,
        device_init,
        _WDF_DEVICE_IO_TYPE::WdfDeviceIoBuffered
    );

    // Now specify the size of device extension where we track per device
    // context.DeviceInit is completely initialized. So call the framework
    // to create the device and attach it to the lower stack.
    //
    let attributes = &mut WDF_OBJECT_ATTRIBUTES {
        ContextTypeInfo: wdf_get_context_type_info!(DeviceContext),
        Size: core::mem::size_of::<_WDF_OBJECT_ATTRIBUTES>() as u32,
        ExecutionLevel: WdfExecutionLevelInheritFromParent,
        SynchronizationScope: WdfSynchronizationScopeInheritFromParent,
        ..Default::default()
    };

    let mut device: WDFDEVICE = WDFDEVICE__ {
        ..Default::default()
    }
    .borrow_mut();

    let status = macros::call_unsafe_wdf_function_binding!(
        WdfDeviceCreate,
        &mut device_init,
        attributes,
        &mut device
    );

    if !nt_success(status) {
        EVENT_LOGGER.send_string(
            Some(&EventOptions {
                level: Some(win_etw_provider::Level::ERROR),
                ..Default::default()
            }),
            format!("WdfDeviceCreate failed with Status code {status:x}\n").as_str(),
        );
        return status;
    }

    let activity = device_to_activity_id(&device);
    let pDevContext = unsafe { device_get_context(device as WDFOBJECT) };

    // Get the device's friendly name and location so that we can use it in
    // error logging.  If this fails then it will setup dummy strings.
    //

    device::get_device_logging_names(device);

    // Tell the framework to set the SurpriseRemovalOK in the DeviceCaps so
    // that you don't get the popup in usermode when you surprise remove the device.

    let mut caps: WDF_DEVICE_PNP_CAPABILITIES = Default::default();
    wdf_device_pnp_capabilities_init(&mut caps);
    caps.SurpriseRemovalOK = WdfTrue;

    unsafe {
        macros::call_unsafe_wdf_function_binding!(WdfDeviceSetPnpCapabilities, device, &mut caps);
    }

    // Create a parallel default queue and register an event callback to
    // receive ioctl requests. We will create separate queues for
    // handling read and write requests. All other requests will be
    // completed with error status automatically by the framework.

    let mut io_queue_config: WDF_IO_QUEUE_CONFIG = Default::default();
    wdf_io_queue_config_init_default_queue(&mut io_queue_config, WdfIoQueueDispatchParallel);

    io_queue_config.EvtIoDeviceControl = Some(crate::ioctl::osr_fx_evt_io_device_control);
    let mut queue: WDFQUEUE = null_mut();
    unsafe {
        macros::call_unsafe_wdf_function_binding!(
            WdfIoQueueCreate,
            device,
            &mut io_queue_config,
            WDF_NO_OBJECT_ATTRIBUTES,
            &mut queue //TODO: Using null_mut here feels off.
        );
    }

    trace_events!(
        "<-- OsrFxEvtDeviceAdd routine\n",
        win_etw_provider::Level::INFO
    );
    STATUS_SUCCESS
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

unsafe extern "C" fn OsrFxEvtDevicePrepareHardware(
    Device: WDFDEVICE,
    ResourceList: WDFCMRESLIST,
    ResourceListTranslated: WDFCMRESLIST,
) -> NTSTATUS {
    STATUS_SUCCESS
}

unsafe extern "C" fn OsrFxEvtDeviceD0Entry(
    Device: WDFDEVICE,
    PreviousState: WDF_POWER_DEVICE_STATE,
) -> NTSTATUS {
    STATUS_SUCCESS
}

unsafe extern "C" fn OsrFxEvtDeviceD0Exit(
    Device: WDFDEVICE,
    TargetState: WDF_POWER_DEVICE_STATE,
) -> NTSTATUS {
    STATUS_SUCCESS
}

unsafe extern "C" fn OsrFxEvtDeviceSelfManagedIoFlush(Device: WDFDEVICE) {}
