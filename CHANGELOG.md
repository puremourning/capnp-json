# Changelog

This file describes notable changes in major and minor releases with a focus
solely on incompatible or API changes. For the full changelog see the git log.

## [Unreleased]

capnp compatiblility: 0.27

### Added

- `Codec::with_anypointer_field_as::<T: Owned>(f: Field)` - specify a concrete type for AnyPointer fields

## [0.3]

capnp compatiblility: 0.27

### Added

- `Codec` type with recursion limit option. `to_json`/`from_json` now use `Codec`
- `Codec::with_field_override` to override the encoding of existing fields
- `Codec::with_type_override` to override the encoding of specific types
- `Codec::with_named_coded` to provide a dictionary of codecs that can be refernced by annotation

### Changed

- [breaking] internal `encode`, `decode`, `data` modules are no longer public
- performance improvements

### Fixed

- `null` is correctly interpreted for pointer fields
- trailing characters in JSON return `Err`
- flattened fields with no members are omitted

## [0.2]

capnp compatiblility: 0.26

### Changed

- compatible with capnp 0.26 (only)
