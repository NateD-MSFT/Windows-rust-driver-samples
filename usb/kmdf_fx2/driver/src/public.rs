use wdk_sys::{FILE_READ_ACCESS, FILE_WRITE_ACCESS, LPGUID, METHOD_BUFFERED, METHOD_OUT_DIRECT, NTSTATUS, PIRP, PVOID, UCHAR};

// TODO: I could do a macro to reduce code duplication here.  Probably not worth
// it though.

enum OsrUSBFxError {
    BitAccessError,
}

struct SwitchState(UCHAR);

impl SwitchState {
    /// Sets the bit at ``index`` for this [`SwitchState`].
    /// Must be in the range [0..7], or a compilation failure will occur.
    fn set_bit<const index: u8>(&mut self, toggle: bool) {
        const {
            assert!(index < 7);
        }
        match toggle {
            true => self.0 |= 1 << index,
            false => {
                let mask = !(1 << index);
                self.0 &= mask
            }
        }
    }

    /// Gets the bit at ``index`` for this [`SwitchState`].
    /// Must be in the range [0..7], or a compilation failure will occur.
    const fn get_bit_const<const index: u8>(&self) -> bool {
        const {
            assert!(index < 7);
        }
        (self.0 & (1u8 << index)) > 0
    }

    /// Get the bit at ``index``for this [`SwitchState`].
    ///
    /// Errors:
    ///
    /// If ``index`` falls outside of the range [0..7], returns an
    /// [`OsrUSBFxError::BitAccessError`].
    fn get_bit(&self, index: u8) -> Result<bool, OsrUSBFxError> {
        match index {
            0..=7 => Ok((self.0 & (1u8 << index)) > 0),
            _ => Err(OsrUSBFxError::BitAccessError),
        }
    }
}

pub struct BarGraphState(pub UCHAR);

#[allow(dead_code)]
impl BarGraphState {
    pub fn set_bit<const index: u8>(&mut self, toggle: bool) {
        const {
            assert!(index < 7);
        }
        match toggle {
            true => self.0 |= 1 << index,
            false => {
                let mask = !(1 << index);
                self.0 &= mask
            }
        }
    }

    pub const fn get_bit<const index: u8>(&self) -> bool {
        const {
            assert!(index < 7);
        }
        (self.0 & (1u8 << index)) > 0
    }
}

const fn ctl_code(device_type: u32, function: u32, method: u32, access: u32) -> u32 {
    ((device_type) << 16) | ((access) << 14) | ((function) << 2) | (method)
}

const IOCTL_INDEX: u32 = 0x800;
const FILE_DEVICE_OSRUSBFX2: u32 = 65550;
const IOCTL_OSRUSBFX2_GET_CONFIG_DESCRIPTOR: u32 = ctl_code(
    FILE_DEVICE_OSRUSBFX2,
    IOCTL_INDEX,
    METHOD_BUFFERED,
    FILE_READ_ACCESS,
);

const IOCTL_OSRUSBFX2_RESET_DEVICE: u32 = ctl_code(
    FILE_DEVICE_OSRUSBFX2,
    IOCTL_INDEX + 1,
    METHOD_BUFFERED,
    FILE_WRITE_ACCESS,
);

const IOCTL_OSRUSBFX2_REENUMERATE_DEVICE: u32 = ctl_code(
    FILE_DEVICE_OSRUSBFX2,
    IOCTL_INDEX + 3,
    METHOD_BUFFERED,
    FILE_WRITE_ACCESS,
);

const IOCTL_OSRUSBFX2_GET_BAR_GRAPH_DISPLAY: u32 = ctl_code(
    FILE_DEVICE_OSRUSBFX2,
    IOCTL_INDEX + 4,
    METHOD_BUFFERED,
    FILE_READ_ACCESS,
);

const IOCTL_OSRUSBFX2_SET_BAR_GRAPH_DISPLAY: u32 = ctl_code(
    FILE_DEVICE_OSRUSBFX2,
    IOCTL_INDEX + 5,
    METHOD_BUFFERED,
    FILE_WRITE_ACCESS,
);

const IOCTL_OSRUSBFX2_READ_SWITCHES: u32 = ctl_code(
    FILE_DEVICE_OSRUSBFX2,
    IOCTL_INDEX + 6,
    METHOD_BUFFERED,
    FILE_READ_ACCESS,
);

const IOCTL_OSRUSBFX2_GET_7_SEGMENT_DISPLAY: u32 = ctl_code(
    FILE_DEVICE_OSRUSBFX2,
    IOCTL_INDEX + 7,
    METHOD_BUFFERED,
    FILE_READ_ACCESS,
);

const IOCTL_OSRUSBFX2_SET_7_SEGMENT_DISPLAY: u32 = ctl_code(
    FILE_DEVICE_OSRUSBFX2,
    IOCTL_INDEX + 8,
    METHOD_BUFFERED,
    FILE_WRITE_ACCESS,
);

const IOCTL_OSRUSBFX2_GET_INTERRUPT_MESSAGE: u32 = ctl_code(
    FILE_DEVICE_OSRUSBFX2,
    IOCTL_INDEX + 9,
    METHOD_OUT_DIRECT,
    FILE_READ_ACCESS,
);
