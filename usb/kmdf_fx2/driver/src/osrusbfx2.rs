#![feature(const_option_ext)]

use wdk_sys::*;

use crate::wdf_object_context::wdf_declare_context_type_with_name;

const _DRIVER_NAME_: &str = "OSRUSBFX2";

const TEST_BOARD_TRANSFER_BUFFER_SIZE: u32 = 64 * 1024;
const DEVICE_DESC_LENGTH: u16 = 256;

pub const DEVPROP_TRUE: DEVPROP_BOOLEAN = -1;
pub const DEVPROP_FALSE: DEVPROP_BOOLEAN = 0;

const DEFAULT_CONTROL_TRANSFER_TIMEOUT: LONGLONG = wdf_rel_timeout_in_sec(5);

macro_rules! DEFINE_GUID {
    (
        $name:ident,
        $data1:expr,
        $data2:expr,
        $data3:expr,
        $data4:expr,
        $data5:expr,
        $data6:expr,
        $data7:expr,
        $data8:expr,
        $data9:expr,
        $data10:expr,
        $data11:expr
    ) => {
        pub const $name: GUID = make_guid(
            $data1, $data2, $data3, $data4, $data5, $data6, $data7, $data8, $data9, $data10,
            $data11,
        );
    };
}

pub const fn make_guid(
    data1: u32,
    data2: u16,
    data3: u16,
    data4: u8,
    data5: u8,
    data6: u8,
    data7: u8,
    data8: u8,
    data9: u8,
    data10: u8,
    data11: u8,
) -> GUID {
    GUID {
        Data1: data1,
        Data2: data2,
        Data3: data3,
        Data4: [data4, data5, data6, data7, data8, data9, data10, data11],
    }
}

DEFINE_GUID!(
    GUID_DEVINTERFACE_OSRUSBFX2,
    0x573E8C73,
    0xCB4,
    0x4471,
    0xA1,
    0xBF,
    0xFA,
    0xB2,
    0x6C,
    0x31,
    0xD3,
    0x84
);

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
/// ID. This is ridiculous in Rust terms, but is what the C driver does.
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
    pub UsbDevice: WDFUSBDEVICE,

    pub UsbInterface: WDFUSBINTERFACE,
    pub BulkReadPipe: WDFUSBPIPE,
    pub BulkWritePipe: WDFUSBPIPE,
    pub InterruptPipe: WDFUSBPIPE,
    pub ResetDeviceWaitLock: WDFWAITLOCK,
    pub CurrentSwitchState: UCHAR,
    pub InterruptMsgQueue: WDFQUEUE,
    pub UsbDeviceTraits: ULONG,

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
