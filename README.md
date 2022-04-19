# moonboot

Moonboot is a framework to build bootloaders for embedded devices, or other kinds of no_std
Rust environments.

This crate contains implementations, macros and build.rs helpers for:
* Partitioning of your memory into different sections
* Exchange of the contents of those partitions via the bootloader
* Signature/Checksum-checking of the partitions contents with an algorithm of your choice, because it is
done in firmware, not in bootloader
* Automatic Linker Script generation based on a Section/Parition Description in Rust Code

## License

Licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or https://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or https://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.
