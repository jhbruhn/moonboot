pub mod ram;

pub mod scratch;

use crate::{
    hardware::Config,
    state::{ExchangeProgress, State},
};
use embedded_storage::Storage;

/// Abstraction for the exchange operation of the current state.
pub trait Exchange<STORAGE: Storage, STATE: State> {
    type Error: core::fmt::Debug;

    fn exchange<const INTERNAL_PAGE_SIZE: usize>(
        &mut self,
        config: &Config,
        storage: &mut STORAGE,
        state: &mut STATE,
        progress: ExchangeProgress,
    ) -> Result<(), Self::Error>;
}
