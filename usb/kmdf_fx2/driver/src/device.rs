use alloc::{format, string::ToString};
use core::{borrow::BorrowMut, ffi::c_void, ptr::null_mut};
use ntddk::DbgBreakPointWithStatus;

use wdk::{nt_success, paged_code};
use wdk_sys::{
    macros,
    ntddk::KeGetCurrentIrql,
    APC_LEVEL,
    DEVICE_REGISTRY_PROPERTY::{
        DevicePropertyDeviceDescription, DevicePropertyFriendlyName,
        DevicePropertyLocationInformation,
    },
    PVOID, STATUS_SUCCESS, WDFDEVICE, WDFMEMORY, WDFOBJECT, WDF_NO_OBJECT_ATTRIBUTES,
    WDF_OBJECT_ATTRIBUTES,
    _POOL_TYPE::NonPagedPoolNx,
    *,
};
use widestring::u16cstr;
use win_etw_provider::EventOptions;
use windows_sys::Win32::Devices::Properties::{
    DEVPKEY_DeviceInterface_Restricted, DEVPKEY_DeviceInterface_UnrestrictedAppCapabilities,
};
use _WDF_EXECUTION_LEVEL::WdfExecutionLevelInheritFromParent;
use _WDF_IO_QUEUE_DISPATCH_TYPE::{
    WdfIoQueueDispatchManual, WdfIoQueueDispatchParallel, WdfIoQueueDispatchSequential,
};
use _WDF_REQUEST_TYPE::WdfRequestTypeWrite;
use _WDF_SYNCHRONIZATION_SCOPE::WdfSynchronizationScopeInheritFromParent;
use _WDF_TRI_STATE::{WdfFalse, WdfTrue};

use crate::{
    device_to_activity_id,
    ioctl::{osr_fx_evt_io_read, osr_fx_evt_io_stop},
    osrusbfx2::device_get_context,
    trace::{trace_events, EVENT_LOGGER},
    wdf::util::{
        wdf_device_pnp_capabilities_init, wdf_driver_config_init, wdf_io_queue_config_init,
        wdf_io_queue_config_init_default_queue, wdf_object_attributes_init,
    },
    wdf_object_context::wdf_get_context_type_info,
    DEVPROP_FALSE, IO_SET_DEVICE_INTERFACE_PROPERTY_DATA, WDF_DEVICE_CONTEXT_TYPE_INFO,
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
            (*pDevContext).DeviceName = widestring::u16cstr!("(error retrieving name)").as_ptr();
            // Concern: is this correctly allocated or is this stack memory?
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
            (*pDevContext).Location = widestring::u16cstr!("(error retrieving location)").as_ptr();
            // Concern: is this correctly allocated or is this stack memory?
        }
    }
}

#[link_section = "PAGE"]
pub unsafe extern "C" fn osr_fx_evt_device_add(
    driver: WDFDRIVER,
    mut device_init: PWDFDEVICE_INIT,
) -> NTSTATUS {

    wdk::dbg_break();

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
            format!("WdfIoQueueCreate failed with Status code {status:x}\n").as_str(),
            win_etw_provider::Level::ERROR
        );
        return status;
    }

    wdf_io_queue_config_init(&mut io_queue_config, WdfIoQueueDispatchSequential);
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
            format!("WdfIoQueueCreate failed with Status code {status:x}\n").as_str(),
            win_etw_provider::Level::ERROR
        );
        return status;
    }

    let status = unsafe {
        macros::call_unsafe_wdf_function_binding!(
            WdfDeviceConfigureRequestDispatching,
            device,
            sequential_queue,
            WdfRequestTypeWrite
        )
    };

    if !nt_success(status) {
        trace_events!(
            format!("WdfDeviceConfigureRequestDispatching failed with Status code {status:x}\n")
                .as_str(),
            win_etw_provider::Level::ERROR
        );
        return status;
    }

    // We will create another sequential queue and configure it
    // to receive write requests.
    //
    wdf_io_queue_config_init(&mut io_queue_config, WdfIoQueueDispatchSequential);

    io_queue_config.EvtIoWrite = Some(crate::bulkrwr::osr_fx_evt_io_write);
    io_queue_config.EvtIoStop = Some(crate::bulkrwr::osr_fx_evt_io_stop);
    let mut sequential_queue_write: WDFQUEUE = null_mut();
    let status = unsafe {
        macros::call_unsafe_wdf_function_binding!(
            WdfIoQueueCreate,
            device,
            &mut io_queue_config,
            WDF_NO_OBJECT_ATTRIBUTES,
            &mut sequential_queue_write
        )
    };

    if !nt_success(status) {
        trace_events!(
            format!("WdfIoQueueCreate failed with Status code {status:x}\n").as_str(),
            win_etw_provider::Level::ERROR
        );
        return status;
    }

    // Register a manual I/O queue for handling Interrupt Message Read Requests.
    // This queue will be used for storing Requests that need to wait for an
    // interrupt to occur before they can be completed.
    //
    wdf_io_queue_config_init(&mut io_queue_config, WdfIoQueueDispatchManual);

    // This queue is used for requests that dont directly access the device. The
    // requests in this queue are serviced only when the device is in a fully
    // powered state and sends an interrupt. So we can use a non-power managed
    // queue to park the requests since we dont care whether the device is idle
    // or fully powered up.
    //
    io_queue_config.PowerManaged = WdfFalse;
    let mut manual_queue: WDFQUEUE = null_mut();
    let status = unsafe {
        macros::call_unsafe_wdf_function_binding!(
            WdfIoQueueCreate,
            device,
            &mut io_queue_config,
            WDF_NO_OBJECT_ATTRIBUTES,
            &mut manual_queue
        )
    };

    if !nt_success(status) {
        trace_events!(
            format!("WdfIoQueueCreate failed with Status code {status:x}\n").as_str(),
            win_etw_provider::Level::ERROR
        );
        return status;
    }

    // Register a device interface so that app can find our device and talk to it.
    //
    let status = unsafe {
        macros::call_unsafe_wdf_function_binding!(
            WdfDeviceCreateDeviceInterface,
            device,
            &crate::osrusbfx2::GUID_DEVINTERFACE_OSRUSBFX2,
            null_mut()
        )
    };

    if !nt_success(status) {
        trace_events!(
            format!("WdfDeviceCreateDeviceInterface failed with Status code {status:x}\n").as_str(),
            win_etw_provider::Level::ERROR
        );
        return status;
    }

    // Create the lock that we use to serialize calls to ResetDevice(). As an
    // alternative to using a WDFWAITLOCK to serialize the calls, a sequential
    // WDFQUEUE can be created and reset IOCTLs would be forwarded to it.
    //
    wdf_object_attributes_init(attributes);
    attributes.ParentObject = unsafe { core::mem::transmute(device) }; // Hm.  Better way to do this?

    let status = unsafe {
        macros::call_unsafe_wdf_function_binding!(
            WdfWaitLockCreate,
            attributes,
            &mut (*pDevContext).ResetDeviceWaitLock
        )
    };

    if !nt_success(status) {
        trace_events!(
            format!("WdfWaitLockCreate failed with Status code {status:x}\n").as_str(),
            win_etw_provider::Level::ERROR
        );
        return status;
    }

    // Get the string for the device interface and set the restricted
    // property on it to allow applications bound with device metadata
    // to access the interface.
    //
    match IO_SET_DEVICE_INTERFACE_PROPERTY_DATA.as_ref() {
        Some(func) => {
            let mut symbolic_link_string: WDFSTRING = null_mut();
            let status = unsafe {
                macros::call_unsafe_wdf_function_binding!(
                    WdfStringCreate,
                    null_mut(),
                    WDF_NO_OBJECT_ATTRIBUTES,
                    &mut symbolic_link_string
                )
            };

            if !nt_success(status) {
                trace_events!(
                    format!("WdfStringCreate failed with Status code {status:x}\n").as_str(),
                    win_etw_provider::Level::ERROR
                );
                return status;
            }

            let status = unsafe {
                macros::call_unsafe_wdf_function_binding!(
                    WdfDeviceRetrieveDeviceInterfaceString,
                    device,
                    &crate::GUID_DEVINTERFACE_OSRUSBFX2,
                    null_mut(),
                    symbolic_link_string
                )
            };

            if !nt_success(status) {
                trace_events!(
                    format!(
                        "WdfDeviceRetrieveDeviceInterfaceString failed with Status code \
                         {status:x}\n"
                    )
                    .as_str(),
                    win_etw_provider::Level::ERROR
                );
                return status;
            }

            let mut symbolic_link_name: UNICODE_STRING = UNICODE_STRING {
                ..Default::default()
            };

            unsafe {
                macros::call_unsafe_wdf_function_binding!(
                    WdfStringGetUnicodeString,
                    symbolic_link_string,
                    &mut symbolic_link_name
                )
            }

            let mut is_restricted: DEVPROP_BOOLEAN = DEVPROP_FALSE;

            let status = unsafe {
                func(
                    &mut symbolic_link_name,
                    &DEVPKEY_DeviceInterface_Restricted,
                    0,
                    0,
                    DEVPROP_TYPE_BOOLEAN,
                    u32::try_from(core::mem::size_of::<DEVPROP_BOOLEAN>())
                        .expect("There's no way the size of a boolean doesn't fit in a u32."),
                    &mut is_restricted as *mut _ as *mut c_void,
                )
            };

            if !nt_success(status) {
                trace_events!(
                    format!(
                        "IoSetDeviceInterfacePropertyData failed to set restricted property \
                         {status:x}\n"
                    )
                    .as_str(),
                    win_etw_provider::Level::ERROR
                );
                return status;
            }

            // Adding Custom Capability:
            //
            // Adds a custom capability to device interface instance that allows a Windows
            // Store device app to access this interface using Windows.Devices.Custom
            // namespace. This capability can be defined either in INF or here
            // as shown below. In order to define it from the INF, uncomment the
            // section "OsrUsb Interface installation" from the INF and remove
            // the block of code below.
            //

            let custom_capabilities = u16cstr!("microsoft.hsaTestCustomCapability_q536wpkpf5cy2");

            let status = unsafe {
                func(
                    &mut symbolic_link_name,
                    &DEVPKEY_DeviceInterface_UnrestrictedAppCapabilities,
                    0,
                    0,
                    DEVPROP_TYPE_STRING_LIST,
                    u32::try_from(
                        (custom_capabilities.as_slice_with_nul().len())
                            * core::mem::size_of::<u16>(),
                    )
                    .expect("Custom capability string too long!"),
                    &mut is_restricted as *mut _ as *mut c_void,
                )
            };

            if !nt_success(status) {
                trace_events!(
                    format!(
                        "IoSetDeviceInterfacePropertyData failed to set restricted property \
                         {status:x}\n"
                    )
                    .as_str(),
                    win_etw_provider::Level::ERROR
                );
                return status;
            }

            unsafe {
                macros::call_unsafe_wdf_function_binding!(
                    WdfObjectDelete,
                    symbolic_link_string as *mut _ as *mut c_void
                )
            }
        }
        None => (),
    }

    trace_events!(
        "<-- OsrFxEvtDeviceAdd routine\n",
        win_etw_provider::Level::INFO
    );
    status
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
