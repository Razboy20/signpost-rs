#![warn(missing_docs)]

//! Signpost library for macOS and iOS
//!
//! See https://github.com/mhallin/signpost-rs

/// Macros for the `signpost` crate
pub mod macros;

mod sys {
    use std::{ffi::c_void, os::raw::c_char};

    #[allow(non_camel_case_types)]
    pub type os_log_t = usize;
    #[allow(non_camel_case_types)]
    pub type os_signpost_type_t = u8;
    #[allow(non_camel_case_types)]
    pub type os_signpost_id_t = u64;

    pub const SIGNPOST_TYPE_EVENT: os_signpost_type_t = 0;
    pub const SIGNPOST_TYPE_INTERVAL_BEGIN: os_signpost_type_t = 1;
    pub const SIGNPOST_TYPE_INTERVAL_END: os_signpost_type_t = 2;

    extern "C" {
        pub static mut __dso_handle: c_void;
        pub static mut _os_log_default: c_void;

        #[cfg(all(
            not(feature = "disable-signposts"),
            any(target_os = "macos", target_os = "ios")
        ))]
        pub fn os_log_create(subsystem: *const c_char, category: *const c_char) -> os_log_t;

        #[cfg(all(
            not(feature = "disable-signposts"),
            any(target_os = "macos", target_os = "ios")
        ))]
        pub fn os_signpost_enabled(log: os_log_t) -> bool;

        #[cfg(all(
            not(feature = "disable-signposts"),
            any(target_os = "macos", target_os = "ios")
        ))]
        pub fn _os_signpost_emit_with_name_impl(
            dso: *mut c_void,
            log: os_log_t,
            type_: os_signpost_type_t,
            spid: os_signpost_id_t,
            name: *const c_char,
            format: *const u8,
            buf: *mut u8,
            size: u32,
        );
    }
}

use std::{
    ffi::CStr,
    ptr::addr_of_mut,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Once,
    },
};

/// Signpost logger
pub struct OsLog {
    subsystem: &'static CStr,
    category: &'static CStr,
    handle: AtomicUsize,
    init: Once,
}

/// Scope guard for signpost intervals
pub struct SignpostInterval<'a> {
    log: &'a OsLog,
    id: u64,
    name: &'a CStr,
}

impl OsLog {
    /// Create a new signpost logger
    ///
    /// See <https://developer.apple.com/documentation/os/1643744-os_log_create>
    ///
    /// The recommendation is to use a reverse domain name as `subsystem`, and
    /// one of the predefined categories:
    ///
    /// * [OsLog::CATEGORY_POINTS_OF_INTEREST] - shows up by default in Instruments
    /// * [OsLog::CATEGORY_DYNAMIC_TRACING]
    /// * [OsLog::CATEGORY_DYNAMIC_STACK_TRACING]
    pub const fn new(subsystem: &'static CStr, category: &'static CStr) -> Self {
        OsLog {
            subsystem,
            category,
            handle: AtomicUsize::new(0),
            init: Once::new(),
        }
    }

    /// Log category that translates to "Points of Interest" in Instruments
    pub const CATEGORY_POINTS_OF_INTEREST: &'static CStr =
        unsafe { &*(b"PointsOfInterest\0" as *const [u8] as *const CStr) };

    /// Log category disabled by default, reducing logging overhead
    ///
    /// This category will be enabled when running the application under
    /// Instruments.
    pub const CATEGORY_DYNAMIC_TRACING: &'static CStr =
        unsafe { &*(b"PointsOfInterest\0" as *const [u8] as *const CStr) };

    /// Stack-trace capturing category disabled by default, reducing logging overhead
    ///
    /// Like [OsLog::CATEGORY_DYNAMIC_TRACING], this category will be enabled when
    /// running the application under Instruments.
    pub const CATEGORY_DYNAMIC_STACK_TRACING: &'static CStr =
        unsafe { &*(b"PointsOfInterest\0" as *const [u8] as *const CStr) };

    /// Change the category of a newly constructed logger
    ///
    /// ```
    /// use signpost::{OsLog, const_poi_logger};
    /// static LOGGER: OsLog = const_poi_logger!("com.yourapp")
    ///     .with_category(OsLog::CATEGORY_DYNAMIC_STACK_TRACING);
    /// ```
    pub const fn with_category(mut self, category: &'static CStr) -> Self {
        self.category = category;
        self
    }

    /// Emit an event to the logger
    ///
    /// Use this to add a single point in time to the "Points of Interest"
    /// in Instruments.
    ///
    /// The ID is arbitrary but must *not* be one of the built-in sentinel
    /// values: zero or u64::MAX.
    ///
    /// Avoid creating event names at runtime, prefer using the
    /// [emit_event] macro instead.
    #[cfg(all(
        not(feature = "disable-signposts"),
        any(target_os = "macos", target_os = "ios")
    ))]
    pub fn emit_event(&self, id: u64, name: &CStr) {
        let log = self.get();
        let mut buf = [0u8; 64];

        unsafe {
            if sys::os_signpost_enabled(log) {
                sys::_os_signpost_emit_with_name_impl(
                    addr_of_mut!(sys::__dso_handle),
                    log,
                    sys::SIGNPOST_TYPE_EVENT,
                    id,
                    name.as_ptr(),
                    std::ptr::null(),
                    buf.as_mut_ptr(),
                    buf.len() as u32,
                )
            }
        }
    }

    #[cfg(not(all(
        not(feature = "disable-signposts"),
        any(target_os = "macos", target_os = "ios")
    )))]
    pub fn emit_event(&self, _id: u64, _name: &CStr) {}

    /// Start a timed event
    ///
    /// The ID is used to disambiguate overlapping events, so make sure that
    /// it's unique among events that can overlap in time.
    ///
    /// Avoid create interval names at runtime, prefer using the
    /// [begin_interval] macro instead.
    #[cfg(all(
        not(feature = "disable-signposts"),
        any(target_os = "macos", target_os = "ios")
    ))]
    pub fn begin_interval<'a>(&'a self, id: u64, name: &'a CStr) -> SignpostInterval<'a> {
        let log_handle = self.get();
        let mut buf = [0u8; 64];

        unsafe {
            if sys::os_signpost_enabled(log_handle) {
                sys::_os_signpost_emit_with_name_impl(
                    addr_of_mut!(sys::__dso_handle),
                    log_handle,
                    sys::SIGNPOST_TYPE_INTERVAL_BEGIN,
                    id,
                    name.as_ptr(),
                    std::ptr::null(),
                    buf.as_mut_ptr(),
                    buf.len() as u32,
                )
            }
        }

        SignpostInterval {
            log: self,
            id,
            name,
        }
    }

    #[cfg(not(all(
        not(feature = "disable-signposts"),
        any(target_os = "macos", target_os = "ios")
    )))]
    pub fn begin_interval<'a>(&'a self, id: u64, name: &'a CStr) -> SignpostInterval<'a> {
        SignpostInterval {
            log: self,
            id,
            name,
        }
    }

    #[cfg(all(
        not(feature = "disable-signposts"),
        any(target_os = "macos", target_os = "ios")
    ))]
    fn get(&self) -> sys::os_log_t {
        unsafe {
            self.init.call_once(|| {
                self.handle.store(
                    sys::os_log_create(self.subsystem.as_ptr(), self.category.as_ptr()),
                    Ordering::SeqCst,
                );
            });

            self.handle.load(Ordering::SeqCst)
        }
    }

    #[cfg(not(all(
        not(feature = "disable-signposts"),
        any(target_os = "macos", target_os = "ios")
    )))]
    fn get(&self) -> sys::os_log_t {
        0
    }
}

impl<'a> Drop for SignpostInterval<'a> {
    #[cfg(all(
        not(feature = "disable-signposts"),
        any(target_os = "macos", target_os = "ios")
    ))]
    fn drop(&mut self) {
        let mut buf = [0u8; 4];
        let log_handle = self.log.get();

        unsafe {
            if sys::os_signpost_enabled(log_handle) {
                sys::_os_signpost_emit_with_name_impl(
                    addr_of_mut!(sys::__dso_handle),
                    log_handle,
                    sys::SIGNPOST_TYPE_INTERVAL_END,
                    self.id,
                    self.name.as_ptr(),
                    std::ptr::null(),
                    buf.as_mut_ptr(),
                    buf.len() as u32,
                )
            }
        }
    }

    #[cfg(not(all(
        not(feature = "disable-signposts"),
        any(target_os = "macos", target_os = "ios")
    )))]
    fn drop(&mut self) {}
}
