pub mod ram;
pub mod scratch;

use embedded_storage::Storage;

use crate::state::{ExchangeProgress, State};

/// Error occured during memory access
#[cfg_attr(feature = "use-defmt", derive(Format))]
#[derive(Debug)]
pub enum MemoryError {
    BankSizeNotEqual,
    BankSizeZero,
    ReadFailure,
    WriteFailure,
}

pub trait Swap {
    fn exchange<InternalMemory: Storage, HardwareState: State, const INTERNAL_PAGE_SIZE: usize>(
        &mut self,
        internal_memory: &mut InternalMemory,
        state: &mut HardwareState,
        exchange: ExchangeProgress,
    ) -> Result<(), MemoryError>;
}
