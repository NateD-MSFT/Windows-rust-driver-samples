use wdk_sys::{FILE_READ_ACCESS, FILE_WRITE_ACCESS, METHOD_BUFFERED, METHOD_OUT_DIRECT, UCHAR};

// TODO: I could do a macro to reduce code duplication here.  Probably not worth
// it though.

struct SwitchStateInternal {
    Switch1: UCHAR,
    Switch2: UCHAR,
    Switch3: UCHAR,
    Switch4: UCHAR,
    Switch5: UCHAR,
    Switch6: UCHAR,
    Switch7: UCHAR,
    Switch8: UCHAR,
}

enum SwitchState {
    IndividualSwitches(SwitchStateInternal),
    AllSwitches(UCHAR),
}

struct BarGraphStateInternal {
    Bar1: UCHAR,
    Bar2: UCHAR,
    Bar3: UCHAR,
    Bar4: UCHAR,
    Bar5: UCHAR,
    Bar6: UCHAR,
    Bar7: UCHAR,
    Bar8: UCHAR,
}

enum BarGraphState {
    IndividualBars(BarGraphStateInternal),
    AllBars(UCHAR),
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
