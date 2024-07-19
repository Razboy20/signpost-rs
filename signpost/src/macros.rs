#[doc(hidden)]
#[macro_export]
macro_rules! c_str {
    ($lit:literal) => {
        unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(concat!($lit, "\0").as_bytes()) }
    };
}

/// Compile-time construct a logger for points of interest.
///
/// ```ignore
/// use signpost::{OsLog, poi_logger};
/// static LOGGER: OsLog = poi_logger!("Subsystem name");
/// ```
#[macro_export]
macro_rules! const_poi_logger {
    ($name:expr) => {
        signpost::OsLog::new(
            $crate::c_str!($name),
            signpost::OsLog::CATEGORY_POINTS_OF_INTEREST,
        )
    };
}

/// Emit an event on a logger.
///
/// The arguments are `logger`, `id`, `name`:
///
/// * `id` needs to be a non-zero positive integer, preferably unique
///   per type of event logged
/// * `name` is a string literal that will identify the event in Instruments.
///
/// ```ignore
/// use signpost::{OsLog, const_poi_logger};
/// static LOGGER: OsLog = const_poi_logger!("Subsystem name")
///
/// fn myfunc() {
///     signpost::emit_event!(LOGGER, 1, "My event");
/// }
/// ```
#[macro_export]
macro_rules! emit_event {
    ($log:expr, $id:expr, $name:literal) => {
        $log.emit_event($id, $crate::c_str!($name))
    };
}

/// Start a signpost interval on a logger
///
/// Similar to `emit_event` but this function stars an interval with a scope
/// guard that automatically ends the interval
///
/// ```ignore
/// use signpost::{OsLog, const_poi_logger};
/// static LOGGER: OsLog = const_poi_logger!("Subsystem name")
///
/// fn myfunc() {
///     let _interval = signpost::begin_interval!(LOGGER, 2, "Compute result");
///     // do work
///     // `_interval` will end the interval when it is dropped
/// }
/// ```
#[macro_export]
macro_rules! begin_interval {
    ($log:expr, $id:expr, $name:literal) => {
        $log.begin_interval($id, $crate::c_str!($name))
    };
}
