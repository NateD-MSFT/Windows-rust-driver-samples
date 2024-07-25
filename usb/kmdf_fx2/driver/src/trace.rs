use lazy_static::lazy_static;
use win_etw_macros::trace_logging_provider_kernel;
lazy_static! {
    pub static ref EVENT_LOGGER: OsrUsbFxLogger = OsrUsbFxLogger::new();
}

#[trace_logging_provider_kernel(name = "OSRUSBFX2", guid = "D23A0C5A-D307-4f0e-AE8E-E2A355AD5DAB")]
pub trait OsrUsbFxLogger {
    fn send_u16_cstring(arg: &U16CStr);
    fn send_string(arg: &str);
}

macro_rules! trace_events {
    ($event_text:expr, $importance:expr) => {
        EVENT_LOGGER.send_string(
            Some(&EventOptions {
                level: Some($importance),
                activity_id: Default::default(),
                related_activity_id: Default::default(),
            }),
            $event_text,
        );
    };
}

pub(crate) use trace_events;