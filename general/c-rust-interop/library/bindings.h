// Copyright (c) Microsoft Corporation
// License: MIT OR Apache-2.0

/* Generated with cbindgen:0.27.0 */

/**
 * Simple Rust function that prints a Hello World function.
 */
void rust_hello_world(void);

/**
 * Simple Rust function that adds two ULONGs and returns the result.
 */
ULONG rust_add_function(ULONG arg1, ULONG arg2);

/**
 * Simple Rust function that takes a PWDF_OBJECT_ATTRIBUTES from a C driver
 * context and reads out some data.
 * # Safety
 *
 * The caller must provide a valid WDF_OBJECT_ATTRIBUTES pointer.
 */
void rust_read_attributes(PWDF_OBJECT_ATTRIBUTES device_init);
