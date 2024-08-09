use wdk_sys::{SIZE_T, ULONG, WDFQUEUE, WDFREQUEST};

pub unsafe extern "C" fn osr_fx_evt_io_stop(
    queue: WDFQUEUE,
    request: WDFREQUEST,
    action_flags: ULONG,
) {
}

pub unsafe extern "C" fn osr_fx_evt_io_write(queue: WDFQUEUE, request: WDFREQUEST, length: usize) {}
