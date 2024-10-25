# Rust Driver Library

This folder contains a Rust static library that builds using the WDK and refers to KMDF driver structures, as well as a C-based Visual Studio driver that references and includes this library to demonstrate C-Rust interop.

## How to build

Manual builds can be performed by carrying out the following steps from a Windows Driver Kit environment:

1. Run cargo build from the root of the repo or from the "library" subfolder.
2. Run "msbuild /p:Configuration=Release /p:Platform=x64" from the "driver" subfolder.  The driver has been configured to point to the .lib file and .h file generated in step 1.

A cleaner build script is under development.