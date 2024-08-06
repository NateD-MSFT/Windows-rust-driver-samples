use wdk_sys::{ULONG, WDFQUEUE, WDFREQUEST};

pub unsafe extern "C" fn osr_fx_evt_io_device_control(
    queue: WDFQUEUE,
    request: WDFREQUEST,
    output_buffer_length: usize,
    input_buffer_length: usize,
    io_control_code: ULONG,
) {
}

pub unsafe extern "C" fn osr_fx_evt_io_read(queue: WDFQUEUE, request: WDFREQUEST, length: usize) {}

pub unsafe extern "C" fn osr_fx_evt_io_stop(
    queue: WDFQUEUE,
    request: WDFREQUEST,
    action_flags: ULONG,
) {
}
