pub mod processor;

use crate::Address;

#[cfg(feature = "defmt")]
use defmt::Format;
#[cfg(feature = "ram-state")]
use desse::{Desse, DesseSized};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "defmt", derive(Format))]
#[cfg_attr(feature = "derive", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "ram-state", derive(Desse, DesseSized))]
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum MemoryUnit {
    Internal,
    // External(usize) // nth external unit
}

#[cfg_attr(feature = "defmt", derive(Format))]
#[cfg_attr(feature = "derive", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "ram-state", derive(Desse, DesseSized))]
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct Bank {
    // TODO: Hide members?
    /// Starting address of this Bank
    pub location: Address,
    /// Size of this Bank
    pub size: Address, // TODO: Use NonZeroU32 to remove checks?
    /// In which memory unit this bank is stored
    pub memory_unit: MemoryUnit,
}

#[cfg_attr(feature = "defmt", derive(Format))]
#[cfg_attr(feature = "derive", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "ram-state", derive(Desse, DesseSized))]
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct Config {
    /// bank this bootloader jumps to
    pub boot_bank: Bank,
    /// alternative bank we write the new firmware image to
    pub update_bank: Bank,
    /// bank the bootloader is contained in
    pub bootloader_bank: Bank,
    // Initial Image is stored to this bank after first update, restore on failure
    // pub golden_bank: Bank,
    /// section of RAM of this device
    pub ram_bank: Bank,
}

#[cfg_attr(feature = "defmt", derive(Format))]
#[cfg_attr(feature = "derive", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "ram-state", derive(Desse, DesseSized))]
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct LinkerConfig {
    pub flash_origin: Address,
    pub ram_origin: Address,
    // TODO enable via feature flag?
    pub has_ram_state: bool,
}
