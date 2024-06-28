/// Additional WDF wrapper functions/etc.
/// To eventually be merged into/replaced by windows-drivers-rs.
pub mod util {
    use wdk_sys::{
        NTSTATUS,
        PFN_WDF_DRIVER_DEVICE_ADD,
        PWDFDEVICE_INIT,
        PWDF_DRIVER_CONFIG,
        PWDF_OBJECT_ATTRIBUTES,
        WDFDRIVER,
        WDF_DRIVER_CONFIG,
        WDF_OBJECT_ATTRIBUTES,
        _WDF_DRIVER_CONFIG,
        _WDF_EXECUTION_LEVEL::WdfExecutionLevelInheritFromParent,
        _WDF_SYNCHRONIZATION_SCOPE::WdfSynchronizationScopeInheritFromParent,
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
            (*config).EvtDriverDeviceAdd = device_add;
        };
    }

    /// Initialize a given [`PWDF_OBJECT_ATTRIBUTES`].
    ///
    /// Errors:
    ///
    /// Returns a [`WDFError::UsizeMismatchError`] if the size of
    /// [`WDF_OBJECT_ATTRIBUTES`] cannot fit in a u32.
    pub fn wdf_object_attributes_init(attributes: PWDF_OBJECT_ATTRIBUTES) -> Result<(), WDFError> {
        if let Ok(attribute_size) = u32::try_from(core::mem::size_of::<WDF_OBJECT_ATTRIBUTES>()) {
            // Safety:
            // All zeroes is a valid representation for a WDF_OBJECT_ATTRIBUTES structure.
            unsafe {
                (*attributes) = core::mem::zeroed::<WDF_OBJECT_ATTRIBUTES>();
                (*attributes).Size = attribute_size;
                (*attributes).ExecutionLevel = WdfExecutionLevelInheritFromParent;
                (*attributes).SynchronizationScope = WdfSynchronizationScopeInheritFromParent;
            }
            Ok(())
        } else {
            Err(WDFError::UsizeMismatchError)
        }
    }
}
