use alloc::format;
use core::{borrow::BorrowMut, ptr::null_mut};

use wdk::{nt_success, paged_code};
use wdk_sys::{
    macros,
    ntddk::KeGetCurrentIrql,
    APC_LEVEL,
    DEVICE_REGISTRY_PROPERTY::{
        DevicePropertyDeviceDescription,
        DevicePropertyFriendlyName,
        DevicePropertyLocationInformation,
    },
    PVOID,
    STATUS_SUCCESS,
    WDFDEVICE,
    WDFMEMORY,
    WDFOBJECT,
    WDF_NO_OBJECT_ATTRIBUTES,
    WDF_OBJECT_ATTRIBUTES,
    _POOL_TYPE::NonPagedPoolNx,
    *,
};
use win_etw_provider::EventOptions;
use _WDF_EXECUTION_LEVEL::WdfExecutionLevelInheritFromParent;
use _WDF_IO_QUEUE_DISPATCH_TYPE::{WdfIoQueueDispatchParallel, WdfIoQueueDispatchSequential};
use _WDF_SYNCHRONIZATION_SCOPE::WdfSynchronizationScopeInheritFromParent;
use _WDF_TRI_STATE::WdfTrue;

use crate::{
    device_to_activity_id,
    ioctl::{osr_fx_evt_io_read, osr_fx_evt_io_stop},
    osrusbfx2::device_get_context,
    trace::{trace_events, EVENT_LOGGER},
    wdf::util::{wdf_device_pnp_capabilities_init, wdf_io_queue_config_init_default_queue},
    wdf_object_context::wdf_get_context_type_info,
    WDF_DEVICE_CONTEXT_TYPE_INFO,
};

pub fn get_device_logging_names(device: WDFDEVICE) {
    let pDevContext = unsafe { device_get_context(device as WDFOBJECT) };

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

#[link_section = "PAGE"]
pub unsafe extern "C" fn osr_fx_evt_device_add(
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
        EvtDeviceD0Entry: Some(osr_fx_evt_device_d0_entry),
        EvtDevicePrepareHardware: Some(osr_fx_evt_device_prepare_hardware),
        EvtDeviceD0Exit: Some(osr_fx_evt_device_d0_exit),
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
        trace_events!(
            format!("WdfDeviceCreate failed with Status code {status:x}\n").as_str(),
            win_etw_provider::Level::ERROR
        );
        return status;
    }

    let activity = device_to_activity_id(&device);
    let pDevContext = unsafe { device_get_context(device as WDFOBJECT) };

    // Get the device's friendly name and location so that we can use it in
    // error logging.  If this fails then it will setup dummy strings.
    //

    get_device_logging_names(device);

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
    let status = unsafe {
        macros::call_unsafe_wdf_function_binding!(
            WdfIoQueueCreate,
            device,
            &mut io_queue_config,
            WDF_NO_OBJECT_ATTRIBUTES,
            &mut queue // TODO: Using null_mut here feels off.
        )
    };

    if !nt_success(status) {
        trace_events!(
            format!("WdfDeviceCreate failed with Status code {status:x}\n").as_str(),
            win_etw_provider::Level::ERROR
        );
        return status;
    }

    wdf_io_queue_config_init_default_queue(&mut io_queue_config, WdfIoQueueDispatchSequential);
    io_queue_config.EvtIoRead = Some(osr_fx_evt_io_read);
    io_queue_config.EvtIoStop = Some(osr_fx_evt_io_stop);
    let mut sequential_queue: WDFQUEUE = null_mut();
    let status = unsafe {
        macros::call_unsafe_wdf_function_binding!(
            WdfIoQueueCreate,
            device,
            &mut io_queue_config,
            WDF_NO_OBJECT_ATTRIBUTES,
            &mut sequential_queue // TODO: Using null_mut here feels off.
        )
    };

    if !nt_success(status) {
        trace_events!(
            format!("WdfDeviceCreate failed with Status code {status:x}\n").as_str(),
            win_etw_provider::Level::ERROR
        );
        return status;
    }

    trace_events!(
        "<-- OsrFxEvtDeviceAdd routine\n",
        win_etw_provider::Level::INFO
    );
    STATUS_SUCCESS
}

#[link_section = "PAGE"]
pub unsafe extern "C" fn osr_fx_evt_driver_context_cleanup(driver: WDFOBJECT) {
    EVENT_LOGGER.send_string(
        Some(&EventOptions {
            level: Some(win_etw_provider::Level::INFO),
            activity_id: Default::default(),
            related_activity_id: Default::default(),
        }),
        "--> OsrFxEvtDriverContextCleanup\n",
    );
}

unsafe extern "C" fn osr_fx_evt_device_prepare_hardware(
    Device: WDFDEVICE,
    ResourceList: WDFCMRESLIST,
    ResourceListTranslated: WDFCMRESLIST,
) -> NTSTATUS {
    STATUS_SUCCESS
}

unsafe extern "C" fn osr_fx_evt_device_d0_entry(
    Device: WDFDEVICE,
    PreviousState: WDF_POWER_DEVICE_STATE,
) -> NTSTATUS {
    STATUS_SUCCESS
}

unsafe extern "C" fn osr_fx_evt_device_d0_exit(
    Device: WDFDEVICE,
    TargetState: WDF_POWER_DEVICE_STATE,
) -> NTSTATUS {
    STATUS_SUCCESS
}

unsafe extern "C" fn OsrFxEvtDeviceSelfManagedIoFlush(Device: WDFDEVICE) {}
