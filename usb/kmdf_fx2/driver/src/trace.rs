use win_etw_macros::trace_logging_provider;

#[trace_logging_provider(name = "OSRUSBFX2", guid = "D23A0C5A-D307-4f0e-AE8E-E2A355AD5DAB")]
pub trait OsrUsbFxLogger {
    fn send_u16_cstring(arg: &U16CStr);
    fn send_string(arg: &str);
}