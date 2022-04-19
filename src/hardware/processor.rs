use crate::Address;

/// This trait executes an execution jump to the specified startign address.
/// The implementation is ISA-dependent.
pub trait Processor {
    /// Jump to the image stored at the specified address. Never returns because it either
    /// successfully switches to a new application image, or fails which leads to some
    /// Hardfault/Panic/Whatever the Processor does then
    fn do_jump(&mut self, address: Address) -> !;
    /// Setup the specified hardware config. Can be used to initialize an MPU for example.
    fn setup(&mut self, config: &crate::hardware::Config);
}

/// Implementation of a processor based on the cortex-m crate
#[cfg(feature = "cortex-m")]
mod cortex_m {
    use super::Processor;
    /// cortex-m based [Processor]
    pub struct CortexM {}

    impl CortexM {
        /// Instantiate a new processor
        pub fn new() -> Self {
            Self {}
        }
    }

    impl Processor for CortexM {
        fn do_jump(&mut self, address: super::Address) -> ! {
            unsafe {
                // Set Vector Table to new vector table (unsafe but okay here)
                (*cortex_m::peripheral::SCB::ptr()).vtor.write(address);

                cortex_m::asm::bootload(address as *const u32);
            }
        }

        fn setup(&mut self, config: &crate::hardware::Config) {
            // Nothing to do!
        }
    }
}

#[cfg(feature = "cortex-m")]
/// A Jumper implementation for use with cortex-m processors
pub use cortex_m::CortexM;
