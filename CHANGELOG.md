# Changelog

This file describes notable changes in major and minor releases with a focus
solely on incompatible or API changes. For the full changelog see the git log.

## [Unreleased - newtype]

### Added

- `Codec::with_newtype_override(newtype_id, codec)` - override the encoding of every field declared with a `type` newtype

## [0.4]

capnp compatiblility: 0.28

### Changed

- [breaking] upgraded capnp to 0.28

## [0.3]

capnp compatiblility: 0.27

### Added

- `Codec` type with recursion limit option. `to_json`/`from_json` now use `Codec`
- `Codec::with_field_override` to override the encoding of existing fields
- `Codec::with_type_override` to override the encoding of specific types
- `Codec::with_named_coded` to provide a dictionary of codecs that can be refernced by annotation
- `Codec::with_anypointer_field_as::<T: Owned>(f: Field)` - specify a concrete type for AnyPointer fields

### Changed

- [breaking] internal `encode`, `decode`, `data` modules are no longer public
- performance improvements
- separated parsing and decoding to avoid users having to parse multiple times

### Fixed

- `null` is correctly interpreted for pointer fields
- trailing characters in JSON return `Err`
- flattened fields with no members are omitted
- matched c++ encoding of flattened, named unions with a name override

## [0.2]

capnp compatiblility: 0.26

### Changed

- compatible with capnp 0.26 (only)
