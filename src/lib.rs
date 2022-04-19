#![no_std]

pub mod boot;
pub mod hardware;
pub mod manager;
pub mod state;

pub use embedded_storage; // TODO

/// Because most of the time, ...
pub use boot as left_boot;
/// ... there's two boots involved.
pub use manager as right_boot;

// Address type in RAM or ROM
pub type Address = u32;

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
