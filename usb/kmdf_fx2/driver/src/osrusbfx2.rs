#![feature(const_option_ext)]
use core::borrow::BorrowMut;

use wdk_sys::*;

use crate::wdf_object_context::wdf_declare_context_type_with_name;

const _DRIVER_NAME_: &str = "OSRUSBFX2";

const TEST_BOARD_TRANSFER_BUFFER_SIZE: u32 = 64 * 1024;
const DEVICE_DESC_LENGTH: u16 = 256;

const DEFAULT_CONTROL_TRANSFER_TIMEOUT: LONGLONG = wdf_rel_timeout_in_sec(5);

pub const WDF_TIMEOUT_TO_SEC: ULONGLONG = {
    //   to    to     to
    //   us    ms     sec
    1 * 10 * 1000 * 1000
};

pub const fn wdf_rel_timeout_in_sec(time: ULONGLONG) -> LONGLONG {
    match (time as i64).checked_mul(-1 * WDF_TIMEOUT_TO_SEC as i64) {
        Some(result) => result,
        None => LONGLONG::MAX,
    }
}

/// Copy the bytes of the WDFDEVICE object into a GUID to serve as an activity
/// ID. This is wildly unsafe in Rust terms, but is what the C driver does.
pub fn device_to_activity_id(device: &WDFDEVICE) -> GUID {
    const { assert!(core::mem::size_of::<WDFDEVICE>() < core::mem::size_of::<GUID>()) }
    let mut guid: GUID = Default::default();
    unsafe {
        // SAFETY: Any sequence of bytes is valid for a GUID,
        // and we const assert above that the size of a GUID > the size of a WDFDEVICE.
        guid = core::mem::zeroed::<GUID>();
        let dev_pointer: *const GUID = &guid;
        let dev_pointer: *mut WDFDEVICE = dev_pointer as *mut WDFDEVICE;
        core::ptr::copy(device, dev_pointer, 1);
    }
    guid
}

// A structure representing the instance information associated with
// this particular device.

#[repr(C)]
pub struct _DEVICE_CONTEXT {
    UsbDevice: WDFUSBDEVICE,

    UsbInterface: WDFUSBINTERFACE,
    BulkReadPipe: WDFUSBPIPE,
    BulkWritePipe: WDFUSBPIPE,
    InterruptPipe: WDFUSBPIPE,
    ResetDeviceWaitLock: WDFWAITLOCK,
    CurrentSwitchState: UCHAR,
    InterruptMsgQueue: WDFQUEUE,
    UsbDeviceTraits: ULONG,

    // The following fields are used during event logging to
    // report the events relative to this specific instance
    // of the device.
    pub DeviceNameMemory: WDFMEMORY,
    pub DeviceName: PCWSTR,
    pub LocationMemory: WDFMEMORY,
    pub Location: PCWSTR,
}

type DeviceContext = _DEVICE_CONTEXT;
type PDEVICE_CONTEXT<'a> = &'a mut _DEVICE_CONTEXT;
wdf_declare_context_type_with_name!(DeviceContext, device_get_context);
