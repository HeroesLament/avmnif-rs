#[macro_export]
macro_rules! nif_collection {
    (
        $moniker:ident,
        init = $init_fn:ident,
        nifs = [ $( ($name:literal, $arity:literal, $func:path) ),* $(,)? ]
    ) => {
        ::paste::paste! {
            // ── init & resolver ───────────────────────────────────────────────
            #[no_mangle]
            pub extern "C" fn [<$moniker _nif_init>](ctx: *mut $crate::Context) {
                unsafe { $init_fn(&mut *ctx) }
            }

            #[no_mangle]
            pub extern "C" fn [<$moniker _get_nif>](name: *const u8)
                -> *const core::ffi::c_void
            {
                let cstr = unsafe { core::ffi::CStr::from_ptr(name as *const _) };
                match cstr.to_str().unwrap_or("") {
                    $(
                        $name => $func as *const () as *const core::ffi::c_void,
                    )*
                    _ => core::ptr::null(),
                }
            }

            // ── registration blob ────────────────────────────────────────────
            #[used]
            #[cfg_attr(
                any(target_os = "macos", target_os = "ios"),
                link_section = "__DATA,.nif_collection"
            )]
            #[cfg_attr(
                not(any(target_os = "macos", target_os = "ios")),
                link_section = ".nif_collection"
            )]
            static _REGISTER: extern "C" fn() = {
                extern "C" fn register() {
                    // skip during `cargo test` so the host linker
                    // doesn’t look for AtomVM’s C symbol
                    #[cfg(not(test))]
                    unsafe {
                        extern "C" {
                            fn REGISTER_NIF_COLLECTION(
                                name: *const u8,
                                init: *const core::ffi::c_void,
                                destroy: *const core::ffi::c_void,
                                resolver: *const core::ffi::c_void,
                            );
                        }
                        REGISTER_NIF_COLLECTION(
                            concat!(stringify!($moniker), "\0").as_ptr(),
                            [<$moniker _nif_init>] as *const _,
                            core::ptr::null(),
                            [<$moniker _get_nif>] as *const _,
                        );
                    }
                }
                register
            };
        }
    };
}
