pub mod ram;

pub mod scratch;

use crate::state::{ExchangeProgress, State};
use embedded_storage::Storage;

/// Abstraction for the exchange operation of the current state.
pub trait Exchange<STORAGE: Storage, STATE: State> {
    type Error: core::fmt::Debug;

    fn exchange<const INTERNAL_PAGE_SIZE: usize>(
        &mut self,
        storage: &mut STORAGE,
        state: &mut STATE,
        progress: ExchangeProgress,
    ) -> Result<(), Self::Error>;
}
