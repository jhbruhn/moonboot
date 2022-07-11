use crate::{
    hardware::{processor::Processor, Config},
    state::{State, Update},
    Context,
};

use crate::log;

/// Instantiate this in your application to enable mutation of the State specified in this and jump
/// to the bootloader to apply any updates.
pub struct MoonbootManager<CONTEXT: Context, const INTERNAL_PAGE_SIZE: usize> {
    config: Config,
    storage: CONTEXT::Storage,
    state: CONTEXT::State,
    processor: CONTEXT::Processor,
}

pub struct InitError;

pub enum MarkError<E> {
    UpdateQueuedButNotInstalled,
    State(E),
}

impl<CONTEXT: Context, const INTERNAL_PAGE_SIZE: usize>
    MoonbootManager<CONTEXT, INTERNAL_PAGE_SIZE>
{
    pub fn new(
        config: Config,
        storage: CONTEXT::Storage,
        state: CONTEXT::State,
        processor: CONTEXT::Processor,
    ) -> Result<Self, InitError> {
        if config.update_bank.size > config.boot_bank.size {
            log::error!(
                "Requested update bank {:?} is larger than boot bank {:?}",
                config.update_bank,
                config.boot_bank
            );
            return Err(InitError);
        }

        if config.update_bank.size == 0 || config.boot_bank.size == 0 {
            log::error!("Requested banks are of zero size");
            return Err(InitError);
        }

        Ok(Self {
            config,
            storage,
            state,
            processor,
        })
    }

    /// Destroy this instance of the boot manager and return access to the hardware peripheral
    pub fn destroy(self) -> (CONTEXT::Storage, CONTEXT::State, CONTEXT::Processor) {
        (self.storage, self.state, self.processor)
    }

    /// Run this immediately after booting your new image successfully to mark the boot as
    /// succesful. If you do not do this, any reset will cause the bootloader to restore to the
    /// previous firmware image.
    pub fn mark_boot_successful(
        &mut self,
    ) -> Result<(), MarkError<<CONTEXT::State as State>::Error>> {
        let mut current_state = self.state.read().map_err(MarkError::State)?;

        log::info!(
            "Application running, marking boot as successful. Current state: {:?}",
            current_state
        );

        current_state.update = match current_state.update {
            Update::None => {
                log::info!("No Update was done.");
                Update::None
            }
            Update::Revert(_) => {
                log::info!("Software was updated, marking as successful.");
                Update::None
            }
            _ => {
                log::error!("There is an update queued, but it has not been installed yet. Did you skip the bootloader?");
                return Err(MarkError::UpdateQueuedButNotInstalled);
            }
        };

        log::trace!("New state: {:?}", current_state);

        self.state.write(&current_state).map_err(MarkError::State)
    }

    // Upgrade firmware verifiying the given signature over the size of size.
    // Can only return an error or diverge (!, represented by Void while ! is not a type yet)
    pub fn update(&mut self) -> Result<void::Void, <CONTEXT::State as State>::Error> {
        // Apply the update stored in the update bank
        let bank = self.config.update_bank;

        log::info!("Update requested on slot {:?}", bank);

        let mut current_state = self.state.read()?;

        if current_state.update != Update::None {
            log::warn!(
                "There is already an update in progress or queued: {:?}",
                current_state.update
            );
        }

        current_state.update = Update::Request(bank);

        self.state.write(&current_state)?;

        log::info!("Stored update request, jumping to bootloader! Geronimo!");

        let bootloader_address = self.config.bootloader_bank.location;

        log::info!("Executing pre jump handler.");
        extern "Rust" {
            fn _moonboots_pre_jump();
        }
        unsafe {
            _moonboots_pre_jump();
        }

        self.processor.do_jump(bootloader_address)
    }
}
