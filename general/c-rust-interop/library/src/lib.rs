// Copyright (c) Microsoft Corporation
// License: MIT OR Apache-2.0

#![no_std]

//! A dummy Rust library to be linked and called from a Windows KMDF driver.

#[cfg(not(test))]
extern crate wdk_panic;

#[cfg(not(test))]
#[global_allocator]
static GLOBAL_ALLOCATOR: wdk_alloc::WdkAllocator = wdk_alloc::WdkAllocator;

#[no_mangle]
/// Simple Rust function that prints a Hello World function.
pub extern "C" fn rust_hello_world() {
    wdk::println!("[RUST] Hello, world!");
}

#[no_mangle]
/// Simple Rust function that adds two ULONGs and returns the result.
pub extern "C" fn rust_add_function(arg1: wdk_sys::ULONG, arg2: wdk_sys::ULONG) -> wdk_sys::ULONG {
    arg1 + arg2
}

#[no_mangle]
/// Simple Rust function that takes a PWDF_OBJECT_ATTRIBUTES from a C driver
/// context and reads out some data.  
/// # Safety
///
/// The caller must provide a valid WDF_OBJECT_ATTRIBUTES pointer.
pub unsafe extern "C" fn rust_read_attributes(device_init: wdk_sys::PWDF_OBJECT_ATTRIBUTES) {
    wdk::println!("[Rust] printing info from WDF_OBJECT_ATTRIBUTES.");
    unsafe {
        wdk::println!("[Rust] Size: {}", (*device_init).Size);
        wdk::println!("[Rust] Execution level: {}", (*device_init).ExecutionLevel);
    }
}
