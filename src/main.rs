#![crate_name="ockernel"]

#![no_std]
#![no_main]

#![feature(panic_info_message)]
#![feature(abi_x86_interrupt)]

#![feature(custom_test_frameworks)]
#![test_runner(crate::test::test_runner)]
#![reexport_test_harness_main = "test_main"]

#![feature(alloc_error_handler)]

#![allow(clippy::missing_safety_doc)] // dont really want to write safety docs yet

/// Macros, need to be loaded before everything else due to how rust parses
#[macro_use]
mod macros;

// architecture specific modules
#[path="arch/i586/mod.rs"]
#[cfg(target_arch = "i586")]
pub mod arch;

// platform specific modules
#[path="platform/ibmpc/mod.rs"]
#[cfg(target_platform = "ibmpc")]
pub mod platform;

/// Exception handling (panic)
pub mod unwind;

/// Logging code
mod logging;

/// text mode console
mod console;

/// memory management
pub mod mm;

/// various utility things
pub mod util;

/// tests
#[cfg(test)]
pub mod test;

// we need this to effectively use our heap
extern crate alloc;

pub const NAME: &str = env!("CARGO_PKG_NAME");
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
use platform::debug::exit_success;

// kernel entrypoint (called by arch/<foo>/boot.S)
#[no_mangle]
pub extern fn kmain() -> ! {
    // initialize kernel
    arch::init(); // platform specific initialization

    mm::init(); // init memory management/heap/etc

    console::init(); // init console

    log!("{} v{}", NAME, VERSION);

    #[cfg(test)]
    {
        test_main();
        exit_success();
    }

    #[cfg(not(test))]
    {
        log!("UwU");

        /*unsafe {
            *(0xdeadbeef as *mut u32) = 3621; // page fault lmao
        }*/
    }

    arch::halt();
}
