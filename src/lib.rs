#![no_std]

//!Moonboot is a framework to build bootloaders for embedded devices, or other kinds of no_std
//!Rust environments.
//!
//!This crate contains implementations, macros and build.rs helpers for:
//!* Partitioning of your memory into different sections
//!* Exchange of the contents of those partitions via the bootloader
//!* Signature/Checksum-checking of the partitions contents with an algorithm of your choice, because it is
//!done in firmware, not in bootloader
//!* Automatic Linker Script generation based on a Section/Parition Description in Rust Code

mod boot;

/// Implementations for use in the bootloader
pub use boot::MoonbootBoot;

mod manager;

/// Implementations for use in the firmware
pub use manager::MoonbootManager;

/// Common hardware abstractions and associated implementations
pub mod hardware;
/// Shared state management between firmware and bootloader
pub mod state;

pub use embedded_storage;

// Because most of the time, ...
//pub use boot::MoonbootBoot as LeftBoot;
// ... there's two boots involved.
//pub use manager::MoonbootManager as RightBoot;

/// Address type in RAM or ROM
pub type Address = u32;

/// Marker macro for a handler fn invoked shortly before jumping to a different image. Use this to
/// uninitialize your hardware.
pub use moonboot_macros::pre_jump_handler;

#[cfg(feature = "use-defmt")]
pub(crate) use defmt as log;

#[cfg(feature = "use-log")]
pub(crate) use logger_crate as log;

#[cfg(not(any(feature = "use-log", feature = "use-defmt")))]
pub(crate) mod log {
    macro_rules! info {
        ( $( $x:expr ),* ) => {};
    }
    pub(crate) use info;
    macro_rules! trace {
        ( $( $x:expr ),* ) => {};
    }
    pub(crate) use trace;
    macro_rules! error {
        ( $( $x:expr ),* ) => {};
    }
    pub(crate) use error;
    macro_rules! warner {
        ( $( $x:expr ),* ) => {};
    }
    pub(crate) use warner as warn;
}

#[export_name = "__moonboots_default_pre_jump"]
fn default_pre_jump() {}
