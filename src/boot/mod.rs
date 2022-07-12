use crate::{
    exchange::Exchange,
    hardware::processor::Processor,
    hardware::{Bank, Config},
    state::{ExchangeProgress, ExchangeStep, State, Update, UpdateError},
    Context,
};

use crate::log;

#[cfg(feature = "defmt")]
use defmt::Format;

/// Use this from your bootloader application and call boot() to do the magic, reading the current
/// state via the State type and then jumping to the new image using the Jumper specified
pub struct MoonbootBoot<CONTEXT: Context, const INTERNAL_PAGE_SIZE: usize> {
    config: Config,
    storage: CONTEXT::Storage,
    state: CONTEXT::State,
    processor: CONTEXT::Processor,
    exchange: CONTEXT::Exchange,
}

impl<CONTEXT: Context, const INTERNAL_PAGE_SIZE: usize> MoonbootBoot<CONTEXT, INTERNAL_PAGE_SIZE> {
    /// create a new instance of the bootloader
    pub fn new(
        config: Config,
        storage: CONTEXT::Storage,
        state: CONTEXT::State,
        processor: CONTEXT::Processor,
        exchange: CONTEXT::Exchange,
    ) -> Self {
        Self {
            config,
            storage,
            state,
            processor,
            exchange,
        }
    }

    /// Destroy this instance of the bootloader and return access to the hardware peripheral
    pub fn destroy(self) -> (CONTEXT::Storage, CONTEXT::State, CONTEXT::Processor) {
        (self.storage, self.state, self.processor)
    }

    /// Execute the update and boot logic of the bootloader
    pub fn boot(&mut self) -> Result<void::Void, <CONTEXT::State as State>::Error> {
        // TODO: consider error handling
        log::info!("Booting with moonboot!");

        self.processor.setup(&self.config);

        let mut state = self.state.read()?;

        log::info!("Old State: {:?}", state);

        // Step 1: Do things according to update state
        state.update = match state.update {
            Update::None => self.handle_none(),
            Update::Request(bank) => self.handle_request(bank),
            Update::Revert(bank) => self.handle_revert(bank),
            Update::Exchanging(progress) => self.handle_exchanging(progress)?,
            Update::Error(err) => Update::Error(err),
        };

        log::info!("New State: {:?}", state);

        // Step 2: Update state of Bootloader
        self.state.write(&state)?;

        // Step 3: Jump to new or unchanged firmware
        self.jump_to_firmware();
    }

    // Handle a Update::None Request (no op effectively)
    fn handle_none(&mut self) -> Update {
        // No update requested -> Nothing to do
        log::info!("Nothing to do, jumping straight to firmware!");
        Update::None
    }

    // Handle an Update::Request state, replacing the old firmware with the new one
    fn handle_request(&mut self, new_firmware: Bank) -> Update {
        log::info!("Update to firmware image {:?} requested.", new_firmware);

        self.exchange_firmwares(new_firmware, true)
    }

    // Handle a revert request because booting of the new firmware failed
    fn handle_revert(&mut self, revert_firmware: Bank) -> Update {
        // Exchange failed update firmware with old firmware image, on success return None so
        // firmware and bootloader functions as usual
        log::warn!(
            "Firmware did not reset state from Revert to None, something went wrong after update!"
        );
        log::info!(
            "Reverting to previous firmware image {:?}.",
            revert_firmware
        );
        self.exchange_firmwares(revert_firmware, false)
    }

    // Handle a case of power interruption or similar, which lead to a exchange_banks being
    // interrupted.
    fn handle_exchanging(
        &mut self,
        progress: ExchangeProgress,
    ) -> Result<Update, <CONTEXT::State as State>::Error> {
        log::error!(
            "Firmware Update was interrupted! Trying to recover with exchange operation: {:?}",
            progress
        );

        let exchange_result = self.exchange.exchange::<INTERNAL_PAGE_SIZE>(
            &mut self.storage,
            &mut self.state,
            progress,
        );

        Ok(if exchange_result.is_ok() {
            let state = self.state.read()?.update;
            match state {
                Update::Exchanging(progress) => {
                    if progress.recovering {
                        Update::None
                    } else {
                        Update::Revert(progress.a)
                    }
                }
                _ => Update::Error(UpdateError::InvalidState),
            }
        } else {
            log::error!(
                "Could not recover from failed update, Error: {:?}",
                exchange_result
            );
            Update::Error(UpdateError::ImageExchangeFailed)
        })
    }

    // Revert the bootable image with the image in index new_firmware. Returns Revert on success if
    // with_failsafe_revert is true, returns None if with_failsafe_revert ist false
    fn exchange_firmwares(&mut self, new: Bank, with_failsafe_revert: bool) -> Update {
        let old = self.config.boot_bank;
        // An update from new_firmware to the bootable firmware was requested.
        if new != old
        // TODO: sanity check firmware is not updated with itself && new_firmware != self.hardware.bootable_image
        {
            log::info!(
                "Exchanging bootable firmware image slot (address: 0x{:x}, size: {}K) with image (address: 0x{:x}, size: {}K).",
                old.location,
                old.size / 1024,
                new.location,
                new.size / 1024
            );

            // Try to exchange the firmware images
            let exchange_result = self.exchange.exchange::<INTERNAL_PAGE_SIZE>(
                &mut self.storage,
                &mut self.state,
                ExchangeProgress {
                    a: new,
                    b: old,
                    page_index: 0,
                    recovering: false,
                    step: ExchangeStep::AToScratch,
                },
            );
            if exchange_result.is_ok() {
                if with_failsafe_revert {
                    // Update Firmware Update State to revert. The Application will set this to
                    // None on successful boot. If we go into the bootloader again and this is still
                    // set, something is wrong with the new application, so we will revert!
                    Update::Revert(new)
                } else {
                    // Reverting to the new firmware, boot as usual to let the firmware try an
                    // update again
                    Update::None
                }
            } else {
                log::error!(
                    "Failed to exchange firmware images due to a hardware error: {:?}",
                    exchange_result
                );
                Update::Error(UpdateError::ImageExchangeFailed)
            }
        } else {
            log::error!("An invalid image index has been specified during update or revert!");
            Update::Error(UpdateError::InvalidImageIndex)
        }
    }

    // Jump to the firmware image marked as bootable
    fn jump_to_firmware(&mut self) -> ! {
        let app_exec_image = self.config.boot_bank;
        let app_address = app_exec_image.location;
        log::info!("Jumping to firmware at {:x}", app_address);

        log::info!("Executing pre jump handler.");
        extern "Rust" {
            fn _moonboots_pre_jump();
        }
        unsafe {
            _moonboots_pre_jump();
        }

        self.processor.do_jump(app_address)
    }
}
