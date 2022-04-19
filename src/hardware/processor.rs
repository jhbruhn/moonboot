use crate::Address;

// This trait executes an execution jump to the specified startign address.
// The implementation is ISA-dependent.
pub trait Processor {
    fn do_jump(&mut self, address: Address) -> !;
    fn setup(&mut self, config: &crate::hardware::Config);
}

#[cfg(feature = "cortex-m")]
mod cortex_m {
    use super::Processor;
    pub struct CortexM {}

    impl CortexM {
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
// A Jumper implementation for use with cortex-m processors
pub use cortex_m::CortexM;
