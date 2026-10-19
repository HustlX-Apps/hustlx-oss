//! AppKit Quit must ask the durable command lane before terminating the process.
//! Iced handles window close requests, but AppKit's Quit menu bypasses those.
#[cfg(target_os = "macos")]
mod mac {
    use std::{
        ffi::{c_char, c_void},
        sync::{
            OnceLock,
            atomic::{AtomicBool, Ordering},
        },
    };
    type Object = *mut c_void;
    static REQUESTED: AtomicBool = AtomicBool::new(false);
    static INSTALLED: OnceLock<bool> = OnceLock::new();
    #[link(name = "AppKit", kind = "framework")]
    unsafe extern "C" {
        static NSApp: Object;
    }
    #[link(name = "objc")]
    unsafe extern "C" {
        fn sel_registerName(name: *const c_char) -> Object;
        #[link_name = "objc_msgSend"]
        fn delegate(object: Object, selector: Object) -> Object;
        fn object_getClass(object: Object) -> Object;
        fn class_getInstanceMethod(class: Object, selector: Object) -> Object;
        fn class_addMethod(
            class: Object,
            selector: Object,
            implementation: unsafe extern "C" fn(),
            types: *const c_char,
        ) -> i8;
    }
    // NSApplicationTerminateReply is NSUInteger on supported 64-bit macOS.
    unsafe extern "C" fn should_terminate(_: Object, _: Object, _: Object) -> usize {
        REQUESTED.store(true, Ordering::Release);
        0 // NSTerminateCancel: the UI saves, then closes its window itself.
    }
    pub fn install() -> bool {
        *INSTALLED.get_or_init(|| {
            // Called only by Iced on the main thread after its AppKit delegate exists.
            // Add the public optional delegate method, preserving all existing handlers.
            // Refuse to replace a handler if a future Iced/winit supplies its own.
            unsafe {
                if NSApp.is_null() {
                    return false;
                }
                let target = delegate(NSApp, sel_registerName(c"delegate".as_ptr()));
                if target.is_null() {
                    return false;
                }
                let class = object_getClass(target);
                let selector = sel_registerName(c"applicationShouldTerminate:".as_ptr());
                if !class_getInstanceMethod(class, selector).is_null() {
                    return false;
                }
                let implementation = std::mem::transmute::<
                    unsafe extern "C" fn(Object, Object, Object) -> usize,
                    unsafe extern "C" fn(),
                >(should_terminate);
                class_addMethod(class, selector, implementation, c"Q@:@".as_ptr()) != 0
            }
        })
    }
    pub fn take_request() -> bool {
        REQUESTED.swap(false, Ordering::AcqRel)
    }
}
#[cfg(target_os = "macos")]
pub use mac::{install, take_request};
#[cfg(not(target_os = "macos"))]
pub fn install() -> bool {
    true
}
#[cfg(not(target_os = "macos"))]
pub fn take_request() -> bool {
    false
}
