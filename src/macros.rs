/*
 * Rust BareBones OS
 * - By John Hodge (Mutabah/thePowersGang) 
 *
 * macros.rs
 * - Macros used by the kernel
 *
 * This code has been put into the public domain, there are no restrictions on
 * its use, and the author takes no liability.
 */

/// A very primitive logging macro
///
/// Obtaines a logger instance (locking the log channel) with the current module name passed
/// then passes the standard format! arguments to it
macro_rules! log {
    ( $($arg:tt)* ) => ({
        // Import the Writer trait (required by write!)
        use core::fmt::Write;
        let _ = write!(&mut crate::logging::Writer::get(module_path!()), $($arg)*);
    })
}

/// works exactly the same as log!, however requres debug_messages to be set at compile time
macro_rules! debug {
    ( $($arg:tt)* ) => ({
        #[cfg(debug_messages)]
        {
            // Import the Writer trait (required by write!)
            use core::fmt::Write;
            let _ = write!(&mut crate::logging::Writer::get(module_path!()), $($arg)*);
        }
    })
}
