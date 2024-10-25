# C Driver for Statically Linked Rust Libraries

This driver is a modified version of the [KMDF Echo Sample Driver](https://github.com/microsoft/Windows-driver-samples/tree/main/general/echo/kmdf/driver/DriverSync) from the [C Windows Driver Samples repository](https://github.com/microsoft/Windows-driver-samples).  It should not be used for general C driver development learning.

This sample has been modified to link to the windows-driver-library sample in this repo.  During EchoDeviceCreate, it calls the three functions currently present in that library.  To see this output, build and install this sample and attach a debugger or debug message viewer.