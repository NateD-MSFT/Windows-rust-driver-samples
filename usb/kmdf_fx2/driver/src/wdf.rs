/// Additional WDF wrapper functions/etc.
/// To eventually be merged into/replaced by windows-drivers-rs.
pub mod util {
    use core::mem;

    use wdk_sys::{
        PFN_WDF_DRIVER_DEVICE_ADD,
        PWDF_DEVICE_PNP_CAPABILITIES,
        PWDF_DRIVER_CONFIG,
        PWDF_IO_QUEUE_CONFIG,
        PWDF_OBJECT_ATTRIBUTES,
        ULONG,
        WDF_DEVICE_PNP_CAPABILITIES,
        WDF_DRIVER_CONFIG,
        WDF_IO_QUEUE_CONFIG,
        WDF_IO_QUEUE_DISPATCH_TYPE,
        WDF_OBJECT_ATTRIBUTES,
        _WDF_EXECUTION_LEVEL::WdfExecutionLevelInheritFromParent,
        _WDF_IO_QUEUE_DISPATCH_TYPE::{self, WdfIoQueueDispatchParallel},
        _WDF_SYNCHRONIZATION_SCOPE::WdfSynchronizationScopeInheritFromParent,
        _WDF_TRI_STATE::WdfUseDefault,
    };

    pub enum WDFError {
        UsizeMismatchError,
    }

    /// Initialize a given [`PWDF_DRIVER_CONFIG`] and assign the given AddDevice
    /// callback.
    pub fn wdf_driver_config_init(
        config: PWDF_DRIVER_CONFIG,
        device_add: PFN_WDF_DRIVER_DEVICE_ADD,
    ) {
        // Safety:
        //
        // All zeroes is a valid representation for a WDF_DRIVER_CONFIG structure.
        unsafe {
            (*config) = core::mem::zeroed::<WDF_DRIVER_CONFIG>();
            (*config).Size = u32::try_from(core::mem::size_of::<WDF_DRIVER_CONFIG>())
                .expect("Size of WDF_DRIVER_CONFIG was more than u32 capacity!");
            (*config).EvtDriverDeviceAdd = device_add;
        };
    }

    /// Initialize a given [`PWDF_OBJECT_ATTRIBUTES`].
    pub fn wdf_object_attributes_init(attributes: PWDF_OBJECT_ATTRIBUTES) {
        let attribute_size = u32::try_from(core::mem::size_of::<WDF_OBJECT_ATTRIBUTES>())
            .expect("WDF_OBJECT_ATTRIBUTES should always fit in a u32!");

        // Safety:
        // All zeroes is a valid representation for a WDF_OBJECT_ATTRIBUTES structure,
        // and we immediately fill the structure's fields.
        unsafe {
            (*attributes) = core::mem::zeroed::<WDF_OBJECT_ATTRIBUTES>();
            (*attributes).Size = attribute_size;
            (*attributes).ExecutionLevel = WdfExecutionLevelInheritFromParent;
            (*attributes).SynchronizationScope = WdfSynchronizationScopeInheritFromParent;
        }
    }

    pub fn wdf_device_pnp_capabilities_init(caps: PWDF_DEVICE_PNP_CAPABILITIES) {
        const { assert!(mem::size_of::<WDF_DEVICE_PNP_CAPABILITIES>() < u32::MAX as usize) }
        // SAFETY: We are setting defaults for a struct we have an exclusive reference
        // to.
        unsafe {
            *caps = mem::zeroed::<WDF_DEVICE_PNP_CAPABILITIES>();
            (*caps).Size = u32::try_from(mem::size_of::<WDF_DEVICE_PNP_CAPABILITIES>()).expect(
                "sizeof WDF_DEVICE_PNP_CAPABILITIES should fit in a u32! We even asserted this at \
                 compile time!",
            );
            (*caps).LockSupported = WdfUseDefault;
            (*caps).EjectSupported = WdfUseDefault;
            (*caps).Removable = WdfUseDefault;
            (*caps).DockDevice = WdfUseDefault;
            (*caps).UniqueID = WdfUseDefault;
            (*caps).SilentInstall = WdfUseDefault;
            (*caps).SurpriseRemovalOK = WdfUseDefault;
            (*caps).HardwareDisabled = WdfUseDefault;
            (*caps).NoDisplayInUI = WdfUseDefault;

            (*caps).Address = ULONG::MAX;
            (*caps).UINumber = ULONG::MAX;
        }
    }
    
    pub fn wdf_io_queue_config_init_default_queue(
        config: PWDF_IO_QUEUE_CONFIG,
        dispatch_type: WDF_IO_QUEUE_DISPATCH_TYPE,
    ) {
        // SAFETY: We are setting defaults for a struct we have an exclusive reference
        // to.
        unsafe {
            *config = mem::zeroed::<WDF_IO_QUEUE_CONFIG>();
            (*config).Size = wdf_structure_size::<WDF_IO_QUEUE_CONFIG>();
            (*config).PowerManaged = WdfUseDefault;
            (*config).DefaultQueue = true as u8;
            (*config).DispatchType = dispatch_type;
            if (*config).DispatchType == WdfIoQueueDispatchParallel {
                (*config).Settings.Parallel.NumberOfPresentedRequests = ULONG::MAX;
            }
        }
    }

    // TODO: represent WDF version logic?
    pub const fn wdf_structure_size<T>() -> ULONG {
        core::mem::size_of::<T>() as ULONG
    }
}
