use alloc::ffi::CString;

extern "C" {
    fn avmnif_log(msg: *const i8);
}

pub fn log_info(msg: &str) {
    let cstr = CString::new(msg).expect("log message contained null byte");
    unsafe {
        avmnif_log(cstr.as_ptr());
    }
}

#[macro_export]
macro_rules! nif_log {
    ($msg:expr) => {
        $crate::log::log_info($msg)
    };
    ($($arg:tt)*) => {{
        use alloc::fmt::Write;
        let mut buf = heapless::String::<256>::new();
        let _ = write!(buf, $($arg)*);
        $crate::log::log_info(&buf);
    }};
}
