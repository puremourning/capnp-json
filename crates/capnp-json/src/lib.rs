//! A [Cap'n Proto](https://capnproto.org) JSON codec, implementing the codec
//! defined in [`json.capnp`].
//!
//! The wire format is compatible with the C++ `capnp::JsonCodec` that ships
//! with Cap'n Proto: messages encoded by this crate can be decoded by the C++
//! codec, and vice-versa. Encoding and decoding are driven entirely by the
//! schema's runtime type information, so no per-type derive or code generation
//! beyond `capnpc` is required.
//!
//! # Quick start
//!
//! [`to_json`] turns any struct reader into a JSON string, and [`from_json`]
//! fills a struct builder from one:
//!
//! ```ignore
//! use capnp::message;
//! use capnp_json::{from_json, to_json};
//!
//! let mut builder = message::Builder::new_default();
//! let root: my_schema_capnp::my_struct::Builder<'_> = builder.init_root();
//! // ... populate `root` ...
//!
//! let json: String = to_json(root.reborrow_as_reader())?;
//!
//! let mut decoded = message::Builder::new_default();
//! let decoded_root: my_schema_capnp::my_struct::Builder<'_> =
//!   decoded.init_root();
//! from_json(&json, decoded_root)?;
//! ```
//!
//! Both are thin wrappers around [`Codec`], which is what you need as soon as
//! you want custom encodings for particular fields or types — see
//! [custom codecs](#custom-codecs) below.
//!
//! # Type mapping
//!
//! | Cap'n Proto | JSON |
//! | --- | --- |
//! | `Void` | `null` |
//! | `Bool` | `true` / `false` |
//! | `Int8`/`Int16`/`Int32`, `UInt8`/`UInt16`/`UInt32` | number |
//! | `Int64`, `UInt64` | **string** holding a decimal integer |
//! | `Float32`, `Float64` | number, except `NaN`, `Infinity` and `-Infinity`, which are strings |
//! | `Text` | string |
//! | `Data` | array of byte-valued numbers, unless `$Json.base64` or `$Json.hex` is applied |
//! | enum | string naming the enumerant, or a number if the ordinal is not in the schema |
//! | struct, group | object |
//! | `List(T)` | array |
//! | `AnyPointer`, interface | not representable; an error unless a custom codec is registered |
//!
//! 64-bit integers are strings because JSON numbers are IEEE-754 doubles and
//! cannot represent the full 64-bit range exactly. When decoding, both the
//! string and the number form are accepted for `Int64`/`UInt64`.
//!
//! On encode, a field is omitted entirely when it is unset — for pointer
//! fields (text, data, lists, structs) that means a null pointer. On decode, a
//! field absent from the JSON is left at its schema default, and a JSON field
//! that does not correspond to anything in the schema is ignored.
//!
//! # JSON annotations
//!
//! To use any of the JSON annotations defined in [`json.capnp`], tell `capnpc`
//! to resolve references to the annotation schema to this crate from your
//! `build.rs`:
//!
//! ```ignore
//! capnpc::CompilerCommand::new()
//!     .crate_provides("capnp_json", [0x8ef99297a43a5e34])
//!     .file("my_schema.capnp")
//!     .run()
//!     .expect("compiling schema");
//! ```
//!
//! `0x8ef99297a43a5e34` is the file ID of `json.capnp`. The supported
//! annotations are:
//!
//! - **`$Json.name("...")`** — use a different name for a field, enumerant,
//!   group or union member in the JSON representation.
//! - **`$Json.flatten()`** / **`$Json.flatten(prefix = "p.")`** — splice a
//!   struct, group or union's members directly into the parent object rather
//!   than nesting them, optionally prefixing each name.
//!
//!   Because a flattened field consumes no JSON nesting, flattening must
//!   terminate: a struct cannot flatten a field of its own type, directly or
//!   through a chain of other flattened fields and groups. [`validate_schema`]
//!   checks this and reports it the way the C++ codec does; encoding and
//!   decoding do not, since a schema is compile-time data and a cycle in one
//!   is a build-time mistake rather than a property of any input. Left
//!   unchecked, a cyclic schema is still rejected when decoded, but by the
//!   recursion limit and with a less pointed message.
//! - **`$Json.discriminator(name = "kind", valueName = "value")`** — encode
//!   which member of a union is active as a sibling string field rather than
//!   by the presence of the member's own key.
//! - **`$Json.base64`** / **`$Json.hex`** — encode a `Data` field (or the
//!   elements of a list of `Data`) as a Base64 or hex string instead of an
//!   array of byte values. Applying both to one field is an error, as is
//!   applying either to a field that is not `Data`.
//!
//! # Custom codecs
//!
//! Some things have no natural JSON form — `AnyPointer` and interface fields
//! most obviously, but also domain types such as timestamps that you would
//! rather see as an ISO-8601 string than as a struct. [`Codec`] lets you
//! supply a [`FieldCodec`] for these, bound in one of three ways:
//!
//! - [`Codec::with_field_override`] — for one specific field of one specific
//!   struct.
//! - [`Codec::with_type_override`] — for every field of a given type.
//! - [`Codec::with_named_codec`] — for every field or struct tagged
//!   `$Rust.codec("name")` in the schema, which keeps the choice next to the
//!   data it applies to.
//!
//! The `$Rust.codec` annotation is defined in this crate's own
//! `rust-json.capnp` (file ID `0xf955e504bf781ac6`); see
//! [`Codec::with_named_codec`] for the `build.rs` setup it needs.
//!
//! # Compatibility notes
//!
//! The following are known divergences from the C++ codec. They matter mostly
//! when decoding input from an untrusted or third-party producer:
//!
//! - Input after the top-level value is ignored rather than rejected.
//! - Numbers are range-checked only loosely: a value too large for the target
//!   integer type is saturated rather than rejected, where C++ raises an
//!   error.
//! - `\uXXXX` escapes are decoded individually, so a surrogate pair — the way
//!   any non-BMP character is written by an escaping JSON producer — is
//!   rejected. Non-BMP characters written literally as UTF-8 are fine.
//! - A JSON `null` for a pointer-typed field is an error; C++ treats it as an
//!   absent field.
//! - Only `Int64`/`UInt64` accept the string form of an integer on decode;
//!   C++ accepts it for every integer width.
//! - Decoding a struct that *has* an `AnyPointer` or interface field fails
//!   even when the JSON does not mention that field, unless a codec is
//!   registered for it.
//! - Duplicate keys within one JSON object are rejected.
//! - Floats are written in Rust's `Display` form, which never uses exponent
//!   notation: `1e300` is emitted as 301 digits rather than as `1e300`. Both
//!   parse back to the same value.
//!
//! Output is not pretty-printed, and the `Value` / `Call` / `raw` extensions
//! from `json.capnp` are not implemented.
//!
//! [`json.capnp`]: https://github.com/capnproto/capnproto/blob/master/c%2B%2B/src/capnp/compat/json.capnp

#![warn(missing_docs)]

use std::collections::HashMap;

mod data;
mod decode;
mod encode;
mod validate;

#[allow(missing_docs)]
mod schema {
  capnp::generated_code!(pub mod json_capnp);
}

// The generated code for schemas annotated with `$Rust.codec` refers to this
// module by path, so it has to be public even though it is not part of the
// hand-written API. See `Codec::with_named_codec`.
#[allow(missing_docs)]
mod rust_json_schema {
  capnp::generated_code!(pub mod rust_json_capnp);
}

#[doc(hidden)]
pub use rust_json_schema::rust_json_capnp;
#[doc(hidden)]
pub use schema::json_capnp;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum DataEncoding {
  #[default]
  Default,
  Base64,
  Hex,
}

struct EncodingOptions<'schema, 'prefix> {
  prefix:        &'prefix std::borrow::Cow<'schema, str>,
  name:          &'schema str,
  field:         Option<capnp::schema::Field>,
  flatten:       Option<json_capnp::flatten_options::Reader<'schema>>,
  discriminator: Option<json_capnp::discriminator_options::Reader<'schema>>,
  data_encoding: DataEncoding,
  codec:         Option<&'schema str>,
}

impl Default for EncodingOptions<'_, '_> {
  fn default() -> Self {
    Self {
      prefix:        &std::borrow::Cow::Borrowed(""),
      name:          "",
      field:         None,
      flatten:       None,
      discriminator: None,
      data_encoding: DataEncoding::Default,
      codec:         None,
    }
  }
}

impl<'schema, 'prefix> EncodingOptions<'schema, 'prefix> {
  fn from_field(
    prefix: &'prefix std::borrow::Cow<'schema, str>,
    field: capnp::schema::Field,
  ) -> capnp::Result<Self> {
    let mut options = Self {
      prefix,
      name: field.get_proto().get_name()?.to_str()?,
      field: Some(field),
      flatten: None,
      discriminator: None,
      data_encoding: DataEncoding::Default,
      codec: None,
    };

    for anno in field.get_annotations()?.iter() {
      match anno.get_id() {
        rust_json_capnp::codec::ID => {
          options.codec = Some(
            anno
              .get_value()?
              .downcast::<capnp::text::Reader>()
              .to_str()?,
          );
        }
        json_capnp::name::ID => {
          options.name = anno
            .get_value()?
            .downcast::<capnp::text::Reader>()
            .to_str()?;
        }
        json_capnp::base64::ID => {
          if options.data_encoding != DataEncoding::Default {
            return Err(capnp::Error::failed(
                            "Cannot specify both base64 and hex annotations on the same field"
                                .into(),
                        ));
          }
          options.data_encoding = DataEncoding::Base64;
        }
        json_capnp::hex::ID => {
          if options.data_encoding != DataEncoding::Default {
            return Err(capnp::Error::failed(
                            "Cannot specify both base64 and hex annotations on the same field"
                                .into(),
                        ));
          }
          options.data_encoding = DataEncoding::Hex;
        }
        json_capnp::flatten::ID => {
          options.flatten = Some(
            anno
              .get_value()?
              .downcast_struct::<json_capnp::flatten_options::Owned>(),
          );
        }
        json_capnp::discriminator::ID => {
          options.discriminator = Some(
            anno
              .get_value()?
              .downcast_struct::<json_capnp::discriminator_options::Owned>(),
          );
        }
        _ => {}
      }
    }
    if options.data_encoding != DataEncoding::Default {
      let mut element_type = field.get_type();
      while let capnp::introspect::TypeVariant::List(sub_element_type) =
        element_type.which()
      {
        element_type = sub_element_type;
      }
      if !matches!(element_type.which(), capnp::introspect::TypeVariant::Data) {
        return Err(capnp::Error::failed(
          "base64/hex annotation can only be applied to Data fields".into(),
        ));
      }
    }
    Ok(options)
  }
}

/// Check that a schema can be represented as JSON at all.
///
/// At present this means checking that `$Json.flatten` terminates. A flattened
/// field splices its members into the parent's JSON object instead of nesting
/// them, so a struct that flattens a field of its own type — directly, or
/// through a chain of other flattened fields and groups — describes an object
/// of infinite width. The C++ codec rejects such a schema outright with
/// "cyclic JSON flattening detected"; this function is how you ask for the
/// same verdict.
///
/// `T` is the generated `Owned` type of the struct you encode or decode as the
/// root. Every struct reachable from it is checked too — including through
/// plain fields and list element types — so validating the root type covers
/// the whole message.
///
/// # When to call this
///
/// Once, at startup or from a test — not per message. A schema is compile-time
/// data: `capnpc` generates it and nothing can change it at runtime, so a
/// cyclic flatten is a mistake in your `.capnp` file rather than a property of
/// any particular input. Encoding and decoding therefore do *not* run this
/// check, and paying for it on every call would be a permanent tax to
/// re-discover a build-time bug.
///
/// ```ignore
/// #[test]
/// fn schema_is_json_encodable() {
///   capnp_json::validate_schema::<my_schema_capnp::my_struct::Owned>()
///     .expect("schema must be JSON-encodable");
/// }
/// ```
///
/// Skipping it is not dangerous, only less informative: a cyclic schema still
/// gets rejected when decoded, by the recursion limit
/// ([`CodecOptions::recursion_limit`]), just with a message that points at the
/// depth rather than at the cycle.
pub fn validate_schema<T: capnp::traits::OwnedStruct>() -> capnp::Result<()> {
  // `OwnedStruct` is only implemented for struct types, so the other variants
  // are unreachable; report rather than panic if that ever stops holding.
  let capnp::introspect::TypeVariant::Struct(raw) = T::introspect().which()
  else {
    return Err(capnp::Error::failed(
      "validate_schema requires a struct type".into(),
    ));
  };
  validate::check_flattening_terminates(capnp::schema::StructSchema::new(raw))
}

/// Encode a Cap'n Proto struct as a JSON string.
///
/// `reader` accepts anything that converts into a
/// [`capnp::dynamic_value::Reader`] — typically a struct reader obtained from
/// `message::Reader::get_root()` or `message::Builder::reborrow_as_reader()`.
/// The value must be a struct, since the top-level JSON value is an object.
///
/// The mapping from Cap'n Proto values to JSON, and the effect of the
/// `$Json.*` annotations, are described in the
/// [module documentation](crate#type-mapping); all of it matches the C++
/// `capnp::JsonCodec`. The output is compact, with no insignificant
/// whitespace.
///
/// This is [`Codec::new().encode(reader)`](Codec::encode). Use a [`Codec`]
/// directly to register custom [`FieldCodec`]s — which is required for
/// messages containing `AnyPointer` or interface fields.
///
/// ```ignore
/// let json: String = capnp_json::to_json(root.reborrow_as_reader())?;
/// ```
pub fn to_json<'msg>(
  reader: impl Into<capnp::dynamic_value::Reader<'msg>>,
) -> capnp::Result<String> {
  Codec::new().encode(reader)
}

/// Decode a JSON string into a Cap'n Proto struct builder.
///
/// `builder` accepts anything that converts into a
/// [`capnp::dynamic_value::Builder`]; it must be a struct builder, since JSON
/// objects map to Cap'n Proto structs. The value mapping and annotations are
/// the same as for [`to_json`].
///
/// Fields absent from the JSON keep whatever `builder` already holds, and
/// JSON fields with no counterpart in the schema are ignored.
///
/// Returns an error if `json` is malformed, if the top-level JSON value is
/// not an object, or if any field's value cannot be coerced to its declared
/// Cap'n Proto type. On error, `builder` may have been partially populated.
///
/// This is [`Codec::new().decode(json, builder)`](Codec::decode). Read
/// [the compatibility notes](crate#compatibility-notes) before decoding input
/// you do not control.
///
/// ```ignore
/// capnp_json::from_json(&json, root)?;
/// ```
pub fn from_json<'segments>(
  json: &str,
  builder: impl Into<capnp::dynamic_value::Builder<'segments>>,
) -> capnp::Result<()> {
  Codec::new().decode(json, builder)
}

/// An in-memory JSON value.
///
/// This is the currency of the [`FieldCodec`] trait: a codec produces a
/// `JsonValue` when encoding and is handed one when decoding. The codec itself
/// never deals with JSON syntax — serialising and parsing are handled by this
/// crate.
///
/// ```
/// use capnp_json::JsonValue;
///
/// let value = JsonValue::Array(vec![
///   JsonValue::String("hello".into()),
///   JsonValue::Number(42.0),
///   JsonValue::Null,
/// ]);
/// assert_eq!(value, value.clone());
/// ```
///
/// # Numbers
///
/// [`Number`](JsonValue::Number) is an `f64`, matching JSON's own numeric
/// model. A codec that needs the full 64-bit integer range should encode to
/// [`String`](JsonValue::String), which is what this crate does for `Int64`
/// and `UInt64` fields.
///
/// # Object ordering
///
/// [`Object`](JsonValue::Object) is a [`HashMap`], so it neither preserves
/// insertion order nor sorts its keys, and the order in which a custom
/// codec's object members are written will differ from run to run. Consumers
/// that compare encoded output byte-for-byte (golden-file tests, signatures)
/// should not use object-valued custom codecs, or should sort the output
/// themselves. Members of *schema* structs are unaffected: those are written
/// in schema declaration order.
///
/// Duplicate keys are rejected when parsing, so an `Object` decoded by this
/// crate never loses a member.
// FIXME: The String valued below could be Cow<'input, str> as they only really
// need to be allocated if the input contains escaped characters. That would be
// a little more tricky lower down, but not by a lot.
#[derive(Debug, Clone, PartialEq)]
pub enum JsonValue {
  /// JSON `null`. Also the encoding of a Cap'n Proto `Void`.
  Null,
  /// JSON `true` or `false`.
  Boolean(bool),
  /// A JSON number. See [the note on numbers](JsonValue#numbers).
  Number(f64),
  /// A JSON string, already unescaped.
  String(String),
  /// A JSON array.
  Array(Vec<JsonValue>),
  /// A JSON object. See [the note on ordering](JsonValue#object-ordering).
  Object(HashMap<String, JsonValue>),

  /// Internal scratch space used while decoding `Data` fields; not part of
  /// the JSON data model.
  ///
  /// Decoding a `Data` field has to hand out a `capnp::data::Reader`
  /// borrowing from somewhere, and the decoded bytes have no other home, so
  /// they are parked in the [`JsonValue`] being decoded. A [`FieldCodec`]
  /// will never be given this variant and should never construct one; treat
  /// it as an unreachable case.
  // FIXME: Remove this from the public type and use a wrapper inside decode
  #[doc(hidden)]
  DataBuffer(Vec<u8>),
}

/// A custom JSON representation for a single Cap'n Proto value.
///
/// Implement this to override how a field, or every value of a given type, is
/// converted to and from JSON. It is *required* for `AnyPointer` and interface
/// fields, which carry no schema the codec could drive itself; it is merely
/// useful for everything else, when the default mapping is not the shape you
/// want on the wire.
///
/// A `FieldCodec` is attached to a [`Codec`] by one of
/// [`with_field_override`](Codec::with_field_override),
/// [`with_type_override`](Codec::with_type_override) or
/// [`with_named_codec`](Codec::with_named_codec).
///
/// # Implementing
///
/// The trait has two required methods and one that you will need to override
/// more often than its default suggests:
///
/// - [`encode_value`](FieldCodec::encode_value) is handed the Cap'n Proto
///   value and returns the [`JsonValue`] to write in its place.
/// - [`decode_value`](FieldCodec::decode_value) is handed a parsed
///   [`JsonValue`] and a builder already positioned at the target value, and
///   populates it.
/// - [`decode_member`](FieldCodec::decode_member) is handed the *parent*
///   struct builder plus the field to write, and so gets to decide how the
///   target is created.
///
/// Which of the two decode methods is called depends on how the codec is
/// bound — see [the note below](#which-decode-method-is-called).
///
/// For simple cases a pair of closures is easier than a named type; see
/// [`make_field_codec`].
///
/// ```
/// use capnp_json::{FieldCodec, JsonValue};
///
/// /// Encodes a struct with `seconds`/`nanos` fields as a single number.
/// struct Timestamp;
///
/// impl FieldCodec for Timestamp {
///   fn encode_value(
///     &self,
///     source: capnp::dynamic_value::Reader<'_>,
///   ) -> capnp::Result<JsonValue> {
///     let source: capnp::dynamic_struct::Reader<'_> = source.downcast();
///     let seconds: i64 = source.get_named("seconds")?.downcast();
///     let nanos: i64 = source.get_named("nanos")?.downcast();
///     Ok(JsonValue::Number(seconds as f64 + nanos as f64 / 1e9))
///   }
///
///   fn decode_value(
///     &self,
///     source: &JsonValue,
///     target: capnp::dynamic_value::Builder<'_>,
///   ) -> capnp::Result<()> {
///     let JsonValue::Number(value) = source else {
///       return Err(capnp::Error::failed("expected a number".into()));
///     };
///     let mut target: capnp::dynamic_struct::Builder<'_> = target.downcast();
///     target.set_named("seconds", (value.trunc() as i64).into())?;
///     target.set_named("nanos", ((value.fract() * 1e9) as i64).into())?;
///     Ok(())
///   }
/// }
/// ```
///
/// # Which decode method is called
///
/// When the codec is bound to a *field* — via `with_field_override`,
/// `with_type_override`, or `$Rust.codec` on a field —
/// [`decode_member`](FieldCodec::decode_member) is called. When it is bound to
/// a *struct type* via `$Rust.codec` on the struct declaration,
/// [`decode_value`](FieldCodec::decode_value) is called with a builder for
/// that struct.
///
/// The default `decode_member` initialises the field and delegates to
/// `decode_value`. That works for struct, list and `AnyPointer` fields, but
/// **fails for primitive, text, data and enum fields**, because those cannot
/// be `init`ialised. If your codec targets one of those, override
/// `decode_member` and use `set` on the parent builder instead:
///
/// ```
/// # use capnp_json::{FieldCodec, JsonValue};
/// # struct Celsius;
/// # impl FieldCodec for Celsius {
/// #   fn encode_value(&self, _: capnp::dynamic_value::Reader<'_>)
/// #     -> capnp::Result<JsonValue> { Ok(JsonValue::Null) }
/// #   fn decode_value(&self, _: &JsonValue, _: capnp::dynamic_value::Builder<'_>)
/// #     -> capnp::Result<()> { Ok(()) }
/// fn decode_member(
///   &self,
///   source: &JsonValue,
///   mut target: capnp::dynamic_struct::Builder<'_>,
///   field: capnp::schema::Field,
/// ) -> capnp::Result<()> {
///   let JsonValue::Number(value) = source else {
///     return Err(capnp::Error::failed("expected a number".into()));
///   };
///   target.set(field, (*value as i32).into())
/// }
/// # }
/// ```
pub trait FieldCodec {
  /// Convert a Cap'n Proto value into the JSON that should stand for it.
  ///
  /// `source` is the value being encoded: the field's value when the codec is
  /// bound to a field, or the struct itself when bound to a struct type. For
  /// a field of a list type this is called once per element, with `source`
  /// being the element.
  ///
  /// The returned [`JsonValue`] is serialised by this crate, so no escaping
  /// or quoting is needed. Returning [`JsonValue::DataBuffer`] is an error.
  fn encode_value(
    &self,
    source: capnp::dynamic_value::Reader<'_>,
  ) -> capnp::Result<JsonValue>;

  /// Populate an already-created Cap'n Proto value from JSON.
  ///
  /// `target` is a builder for the value itself, not for its parent; use
  /// [`decode_member`](FieldCodec::decode_member) if you need to create the
  /// value rather than fill it in.
  ///
  /// This is the method called for codecs bound to a struct type via
  /// `$Rust.codec`, and — through the default `decode_member` — for codecs
  /// bound to struct, list and `AnyPointer` fields.
  fn decode_value(
    &self,
    source: &JsonValue,
    target: capnp::dynamic_value::Builder<'_>,
  ) -> capnp::Result<()>;

  /// Write one field of a struct from JSON.
  ///
  /// Called when this codec is bound to a field. `target` is the *containing*
  /// struct's builder and `field` identifies the field to write, so an
  /// implementation controls how the value is created — by `init` for
  /// pointer-typed fields, or by `set` for everything else.
  ///
  /// This is only called when the field is actually present in the JSON
  /// object; an absent field is left at its default.
  ///
  /// The default implementation initialises the field and forwards to
  /// [`decode_value`](FieldCodec::decode_value), which is only valid for
  /// struct, list and `AnyPointer` fields — see
  /// [the note on the trait](FieldCodec#which-decode-method-is-called).
  fn decode_member(
    &self,
    source: &JsonValue,
    target: capnp::dynamic_struct::Builder<'_>,
    field: capnp::schema::Field,
  ) -> capnp::Result<()> {
    self.decode_value(source, target.init(field)?)
  }
}

/// Lets a `&T` be used wherever a [`FieldCodec`] is expected, so one codec
/// instance can be shared between several [`Codec`]s.
///
/// Note that this forwards only the two required methods: a `T` that
/// overrides [`decode_member`](FieldCodec::decode_member) will have that
/// override bypassed when used through a reference.
impl<T: FieldCodec + ?Sized> FieldCodec for &T {
  fn encode_value(
    &self,
    source: capnp::dynamic_value::Reader<'_>,
  ) -> capnp::Result<JsonValue> {
    (**self).encode_value(source)
  }
  fn decode_value(
    &self,
    source: &JsonValue,
    target: capnp::dynamic_value::Builder<'_>,
  ) -> capnp::Result<()> {
    (**self).decode_value(source, target)
  }
}

/// A pair of closures `(encode, decode)` is a [`FieldCodec`]. Usually reached
/// through [`make_field_codec`] rather than written out.
// implement FieldCodec for any (fn, fn) pair that matches the signature
impl<F, G> FieldCodec for (F, G)
where
  F: Fn(capnp::dynamic_value::Reader<'_>) -> capnp::Result<JsonValue>,
  G: Fn(&JsonValue, capnp::dynamic_value::Builder<'_>) -> capnp::Result<()>,
{
  fn encode_value(
    &self,
    source: capnp::dynamic_value::Reader<'_>,
  ) -> capnp::Result<JsonValue> {
    (self.0)(source)
  }
  fn decode_value(
    &self,
    source: &JsonValue,
    target: capnp::dynamic_value::Builder<'_>,
  ) -> capnp::Result<()> {
    (self.1)(source, target)
  }
}

/// Build a [`FieldCodec`] from an encoder and a decoder closure.
///
/// A shorthand for the cases that do not need a named type — the two closures
/// correspond to [`FieldCodec::encode_value`] and
/// [`FieldCodec::decode_value`]. The resulting codec uses the default
/// [`decode_member`](FieldCodec::decode_member), so it is only suitable for
/// struct, list and `AnyPointer` fields; for a primitive, text, data or enum
/// field, implement [`FieldCodec`] directly and override `decode_member`.
///
/// ```
/// use capnp_json::{make_field_codec, JsonValue};
///
/// // Represent a struct's `text` field as the whole JSON value.
/// let codec = make_field_codec(
///   |source: capnp::dynamic_value::Reader<'_>| {
///     let source: capnp::dynamic_struct::Reader<'_> = source.downcast();
///     let text: capnp::text::Reader<'_> = source.get_named("text")?.downcast();
///     Ok(JsonValue::String(text.to_str()?.to_owned()))
///   },
///   |source: &JsonValue, target: capnp::dynamic_value::Builder<'_>| {
///     let JsonValue::String(text) = source else {
///       return Err(capnp::Error::failed("expected a string".into()));
///     };
///     let mut target: capnp::dynamic_struct::Builder<'_> = target.downcast();
///     target.set_named("text", text.as_str().into())
///   },
/// );
/// # let _ = codec;
/// ```
pub fn make_field_codec<'env>(
  encode_fn: impl Fn(capnp::dynamic_value::Reader<'_>) -> capnp::Result<JsonValue>
    + 'env,
  decode_fn: impl Fn(&JsonValue, capnp::dynamic_value::Builder<'_>) -> capnp::Result<()>
    + 'env,
) -> impl FieldCodec + 'env {
  (encode_fn, decode_fn)
}

/// Encoding and decoding options for a [`Codec`].
///
/// Construct with [`Default`] and adjust what you need, so that options added
/// in future versions keep their defaults:
///
/// ```
/// use capnp_json::{Codec, CodecOptions};
///
/// let codec = Codec::new_with_options(CodecOptions {
///   recursion_limit: 32,
///   ..Default::default()
/// });
/// # let _ = codec;
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodecOptions {
  /// How deeply decoding will recurse before giving up. Defaults to 64,
  /// matching the C++ codec's `maxNestingDepth`.
  ///
  /// Decoding is recursive, so without a bound, deeply nested input exhausts
  /// the stack and aborts the process — which is not a catchable error in
  /// Rust. The limit turns that into an ordinary `Err`.
  ///
  /// It bounds two things: the nesting depth of the JSON itself, and the
  /// depth of the walk over the schema. The second is not implied by the
  /// first, because a struct that flattens a field of its own type recurses
  /// on the schema without descending into the JSON at all.
  ///
  /// A limit of `N` admits `N` nested arrays or objects. Scalars do not count
  /// against it, so the boundary is the same one the C++ codec applies at the
  /// same numeric setting.
  ///
  /// **Raising this reintroduces the crash it exists to prevent.** The safe
  /// ceiling depends on your build profile and the stack size of the thread
  /// doing the decoding; measured on a 1 MiB stack, decoding survived depth
  /// 1600 in release but overflowed at depth 200 in debug. The default is
  /// comfortably safe in both. Lowering it is always safe.
  pub recursion_limit: usize,
}

impl Default for CodecOptions {
  fn default() -> Self {
    Self {
      recursion_limit: 64,
    }
  }
}

/// A JSON codec for Cap'n Proto messages.
///
/// A `Codec` holds the custom [`FieldCodec`]s to apply while encoding and
/// decoding. If you need none, [`to_json`] and [`from_json`] are equivalent to
/// `Codec::new().encode(..)` and `Codec::new().decode(..)` and are more
/// convenient.
///
/// A `Codec` is built up with the `with_*` methods, which consume and return
/// it:
///
/// ```
/// use capnp_json::{make_field_codec, Codec, JsonValue};
///
/// let codec = Codec::new().with_named_codec(
///   "shouty",
///   make_field_codec(
///     |source: capnp::dynamic_value::Reader<'_>| {
///       let text: capnp::text::Reader<'_> = source.downcast();
///       Ok(JsonValue::String(text.to_str()?.to_uppercase()))
///     },
///     |_source: &JsonValue, _target: capnp::dynamic_value::Builder<'_>| Ok(()),
///   ),
/// );
/// # let _ = codec;
/// ```
///
/// The `'env` lifetime is that of the data borrowed by the registered codecs;
/// for codecs that own everything they use, it is `'static`.
///
/// # Reuse and threads
///
/// Encoding and decoding take `&self` and keep no state between calls, so one
/// `Codec` serves any number of messages and building one is cheap. Reuse it
/// if it is convenient; nothing is lost by not doing so.
///
/// A `Codec` is neither `Send` nor `Sync`, because the [`FieldCodec`]s it
/// holds are trait objects carrying no thread-safety bound. Give each thread
/// its own.
///
/// # Which codec wins
///
/// At most one [`FieldCodec`] applies to any given value. When encoding or
/// decoding a struct field, the first match of the following is used:
///
/// 1. a `$Rust.codec("name")` annotation on the field, resolved against the
///    names registered with [`with_named_codec`](Codec::with_named_codec);
/// 2. a [`with_field_override`](Codec::with_field_override) registered for
///    exactly that field;
/// 3. a [`with_type_override`](Codec::with_type_override) registered for the
///    field's declared type.
///
/// If none matches and the value is a struct, a `$Rust.codec("name")`
/// annotation on the *struct's own declaration* is used if one is present.
///
/// A `$Rust.codec` name that is not registered is silently ignored and the
/// default encoding applies.
///
/// # Scope of overrides
///
/// Both overrides are matched against a *field*, which has two consequences
/// worth knowing:
///
/// - Neither applies to the root value passed to [`encode`](Codec::encode) or
///   [`decode`](Codec::decode), since that is not reached through a field.
///   Use a `$Rust.codec` annotation on the struct declaration for that.
/// - For a field of type `List(T)`, a type override must be registered for
///   `List(T)` rather than for `T` — one registered for `T` will not fire for
///   the elements. A `$Rust.codec` annotation on a list field behaves the
///   other way round: it is applied to each element in turn.
pub struct Codec<'env> {
  field_overrides: HashMap<capnp::schema::Field, Box<dyn FieldCodec + 'env>>,
  type_overrides:  HashMap<capnp::introspect::Type, Box<dyn FieldCodec + 'env>>,
  registry:        HashMap<String, Box<dyn FieldCodec + 'env>>,

  options: CodecOptions,
}

impl<'env> Codec<'env> {
  /// Create a codec with no custom [`FieldCodec`]s registered.
  ///
  /// The result encodes and decodes exactly as [`to_json`] and [`from_json`]
  /// do.
  pub fn new() -> Self {
    Self::new_with_options(CodecOptions::default())
  }

  /// Create a codec with no custom [`FieldCodec`]s registered, and the given
  /// [`CodecOptions`].
  ///
  /// Equivalent to [`new`](Codec::new) other than the options; read
  /// [`CodecOptions::recursion_limit`] before raising the recursion limit.
  pub fn new_with_options(options: CodecOptions) -> Self {
    Self {
      field_overrides: HashMap::new(),
      type_overrides: HashMap::new(),
      registry: HashMap::new(),
      options,
    }
  }

  /// Use `codec` for one specific field of one specific struct type.
  ///
  /// `field` is obtained from the field's containing
  /// [`StructSchema`](capnp::schema::StructSchema), which in turn comes from
  /// the generated `Owned` type:
  ///
  /// ```ignore
  /// use capnp::introspect::Introspect;
  ///
  /// let capnp::introspect::TypeVariant::Struct(schema) =
  ///   my_schema_capnp::my_struct::Owned::introspect().which()
  /// else {
  ///   unreachable!("my_struct is a struct");
  /// };
  /// let field = capnp::schema::StructSchema::new(schema)
  ///   .get_field_by_name("myField")?;
  ///
  /// let codec = Codec::new().with_field_override(field, MyFieldCodec);
  /// ```
  ///
  /// This is the most specific binding and takes precedence over
  /// [`with_type_override`](Codec::with_type_override); see
  /// [which codec wins](Codec#which-codec-wins). Registering a second codec
  /// for the same field replaces the first.
  pub fn with_field_override(
    mut self,
    field: capnp::schema::Field,
    codec: impl FieldCodec + 'env,
  ) -> Self {
    self.field_overrides.insert(field, Box::new(codec));
    self
  }

  /// Use `codec` for every field whose declared type is `typ`.
  ///
  /// The type comes from the generated `Owned` type of whatever you want to
  /// override:
  ///
  /// ```ignore
  /// use capnp::introspect::Introspect;
  ///
  /// let codec = Codec::new()
  ///   .with_type_override(my_schema_capnp::timestamp::Owned::introspect(), Timestamp);
  /// ```
  ///
  /// Matching is on the field's *declared* type, so read
  /// [the note on scope](Codec#scope-of-overrides) before using this for
  /// list element types. Registering a second codec for the same type
  /// replaces the first.
  pub fn with_type_override(
    mut self,
    typ: capnp::introspect::Type,
    codec: impl FieldCodec + 'env,
  ) -> Self {
    self.type_overrides.insert(typ, Box::new(codec));
    self
  }

  /// Register `codec` under `name`, for use by `$Rust.codec` annotations.
  ///
  /// This puts the choice of representation in the schema, next to the data
  /// it describes, rather than in the code that builds the [`Codec`]:
  ///
  /// ```capnp
  /// using Rust = import "/rust-json.capnp";
  ///
  /// struct Reading {
  ///   takenAt @0 :Int64 $Rust.codec("iso8601");
  /// }
  ///
  /// struct Duration $Rust.codec("iso8601-duration") {
  ///   seconds @0 :Int64;
  /// }
  /// ```
  ///
  /// ```ignore
  /// let codec = Codec::new().with_named_codec("iso8601", Iso8601);
  /// ```
  ///
  /// The annotation may be applied to a field or to a struct declaration. On
  /// a field it takes precedence over both override maps; on a struct it
  /// applies wherever a value of that struct type is encoded, including at
  /// the root. A name with no matching registration is ignored and the
  /// default encoding is used, so a codec registered under the wrong name
  /// fails silently rather than loudly.
  ///
  /// The annotation is declared in this crate's `rust-json.capnp`. Copy that
  /// file somewhere on your schema import path, and point `capnpc` at this
  /// crate for its file ID:
  ///
  /// ```ignore
  /// capnpc::CompilerCommand::new()
  ///     .crate_provides("capnp_json", [0xf955e504bf781ac6])
  ///     .file("my_schema.capnp")
  ///     .run()
  ///     .expect("compiling schema");
  /// ```
  ///
  /// If the schema also uses the `$Json.*` annotations, list both file IDs:
  /// `[0x8ef99297a43a5e34, 0xf955e504bf781ac6]`.
  pub fn with_named_codec(
    mut self,
    name: impl Into<String>,
    codec: impl FieldCodec + 'env,
  ) -> Self {
    self.registry.insert(name.into(), Box::new(codec));
    self
  }

  /// Encode a Cap'n Proto struct as a JSON string.
  ///
  /// The value mapping is described in the
  /// [module documentation](crate#type-mapping). Returns an error if `reader`
  /// is not a struct, if the message contains an `AnyPointer` or interface
  /// field with no codec registered for it, or if a registered
  /// [`FieldCodec`] fails.
  ///
  /// Use [`encode_to`](Codec::encode_to) to write into an existing buffer or
  /// stream instead of allocating a `String`.
  pub fn encode<'msg>(
    &self,
    reader: impl Into<capnp::dynamic_value::Reader<'msg>>,
  ) -> capnp::Result<String> {
    let mut writer = std::io::Cursor::new(Vec::with_capacity(4096));
    self.encode_to(&mut writer, reader)?;
    String::from_utf8(writer.into_inner()).map_err(|e| {
      capnp::Error::failed(format!(
        "Failed to convert JSON bytes to string: {}",
        e
      ))
    })
  }

  /// Encode a Cap'n Proto struct as JSON, writing it to `writer`.
  ///
  /// Behaves as [`encode`](Codec::encode) but streams the output, so it does
  /// not hold the whole document in memory. The JSON is written as UTF-8.
  ///
  /// Output is written in many small pieces, so wrap unbuffered destinations
  /// such as `File` or `TcpStream` in a [`BufWriter`](std::io::BufWriter).
  ///
  /// If an error occurs part-way through, a partial document will already
  /// have been written.
  pub fn encode_to<'msg, W: std::io::Write>(
    &self,
    writer: &mut W,
    reader: impl Into<capnp::dynamic_value::Reader<'msg>>,
  ) -> capnp::Result<()> {
    let capnp::dynamic_value::Reader::Struct(reader) = reader.into() else {
      return Err(capnp::Error::failed(
        "Top-level value must be a struct".into(),
      ));
    };
    encode::serialize_json_to(self, writer, reader)
  }

  /// Decode a JSON string into a Cap'n Proto struct builder.
  ///
  /// The top-level JSON value must be an object, and `builder` must therefore
  /// be a struct builder. Fields absent from the JSON keep the values already
  /// in `builder`; JSON fields with no counterpart in the schema are ignored.
  /// `builder` is not cleared first, so decoding into a builder that has
  /// already been populated merges into it.
  ///
  /// Returns an error if `json` is malformed, if a value cannot be coerced to
  /// its field's declared type, or if a registered [`FieldCodec`] fails. On
  /// error, `builder` may have been partially populated.
  ///
  /// Read [the compatibility notes](crate#compatibility-notes) before
  /// decoding input you do not control — in particular, deeply nested JSON
  /// can exhaust the stack.
  pub fn decode<'segments>(
    &self,
    json: &str,
    builder: impl Into<capnp::dynamic_value::Builder<'segments>>,
  ) -> capnp::Result<()> {
    let capnp::dynamic_value::Builder::Struct(builder) = builder.into() else {
      return Err(capnp::Error::failed(
        "Top-level JSON value must be an object".into(),
      ));
    };
    decode::parse(self, json, builder)
  }
}

impl Default for Codec<'_> {
  fn default() -> Self {
    Self::new()
  }
}
