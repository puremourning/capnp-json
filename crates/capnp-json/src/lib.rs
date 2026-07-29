//! A [Cap'n Proto](https://capnproto.org) JSON codec, implementing the codec
//! defined in [`json.capnp`].
//!
//! The wire format is compatible with the C++ `capnp::JsonCodec` that ships
//! with Cap'n Proto: messages encoded by this crate can be decoded by the C++
//! codec, and vice-versa.
//!
//! # Quick start
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
//! # JSON annotations
//!
//! To use any of the JSON annotations defined in [`json.capnp`] (for example
//! `$Json.name`, `$Json.flatten`, `$Json.discriminator`, `$Json.base64`,
//! `$Json.hex`), tell `capnpc` to resolve references to the annotation schema
//! to this crate from your `build.rs`:
//!
//! ```ignore
//! capnpc::CompilerCommand::new()
//!     .crate_provides("capnp_json", [0x8ef99297a43a5e34])
//!     .file("my_schema.capnp")
//!     .run()
//!     .expect("compiling schema");
//! ```
//!
//! [`json.capnp`]: https://github.com/capnproto/capnproto/blob/master/c%2B%2B/src/capnp/compat/json.capnp

use std::collections::HashMap;

mod data;
mod decode;
mod encode;

mod schema {
  capnp::generated_code!(pub mod json_capnp);
}

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
    };

    for anno in field.get_annotations()?.iter() {
      match anno.get_id() {
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

/// Encode a Cap'n Proto value as a JSON string.
///
/// `reader` accepts anything that converts into a [`capnp::dynamic_value::Reader`]
/// — typically a struct reader obtained from `message::Reader::get_root()` or
/// `message::Builder::reborrow_as_reader()`.
///
/// `Int64`, `UInt64`, and non-finite floats are encoded as JSON strings, and
/// `Data` fields are encoded as JSON arrays of bytes by default. The
/// `$Json.base64` and `$Json.hex` annotations override the `Data` encoding,
/// and the `$Json.name`, `$Json.flatten`, and `$Json.discriminator`
/// annotations affect the layout of object fields and unions, all matching
/// the C++ `capnp::JsonCodec` behaviour.
pub fn to_json<'msg>(
  reader: impl Into<capnp::dynamic_value::Reader<'msg>>,
) -> capnp::Result<String> {
  Codec::new().encode(reader)
}

/// Decode a JSON string into a Cap'n Proto struct builder.
///
/// `builder` accepts anything that converts into a [`capnp::dynamic_value::Builder`];
/// the top-level value must be a struct builder, since JSON objects map to
/// Cap'n Proto structs. The fields and annotations supported are the same as
/// in [`to_json`].
///
/// Returns an error if `json` is malformed, if the top-level JSON value is
/// not an object, or if any field's value cannot be coerced to its declared
/// Cap'n Proto type.
pub fn from_json<'segments>(
  json: &str,
  builder: impl Into<capnp::dynamic_value::Builder<'segments>>,
) -> capnp::Result<()> {
  Codec::new().decode(json, builder)
}

// FIXME: The String valued below could be Cow<'input, str> as they only really
// need to be allocated if the input contains escaped characters. That would be
// a little more tricky lower down, but not by a lot.
pub enum JsonValue {
  Null,
  Boolean(bool),
  Number(f64),
  String(String),
  Array(Vec<JsonValue>),
  Object(HashMap<String, JsonValue>),

  // FIXME: Remove this from the public type and use a wrapper inside decode
  DataBuffer(Vec<u8>), // HACK: This is so we have somewhere to store the data
                       // temporarily when we are decoding data fields into
                       // Readers
}

/// A trait for encoding and decoding a single field of a Cap'n Proto struct
/// as JSON. This is used to implement custom encoding/decoding for specific
/// fields. Importantly, this is required for fields of type `AnyPointer`, which
/// cannot be automatically encoded/decoded.
pub trait FieldCodec {
  fn encode_value(
    &self,
    source: capnp::dynamic_value::Reader<'_>,
  ) -> capnp::Result<JsonValue>;
  fn decode_value(
    &self,
    source: &JsonValue,
    target: capnp::dynamic_value::Builder<'_>,
  ) -> capnp::Result<()>;
}

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

/// Create a FieldCodec from two functions - an encoder and a decoder.
/// See [`FieldCodec`] for the required function signatures.
pub fn make_field_codec<'env>(
  encode_fn: impl Fn(capnp::dynamic_value::Reader<'_>) -> capnp::Result<JsonValue>
    + 'env,
  decode_fn: impl Fn(&JsonValue, capnp::dynamic_value::Builder<'_>) -> capnp::Result<()>
    + 'env,
) -> impl FieldCodec + 'env {
  (encode_fn, decode_fn)
}

pub struct Codec<'env> {
  field_overrides: HashMap<capnp::schema::Field, Box<dyn FieldCodec + 'env>>,
}

/// A JSON codec for Cap'n Proto messages.
impl<'env> Codec<'env> {
  pub fn new() -> Self {
    Self {
      field_overrides: HashMap::new(),
    }
  }

  pub fn with_field_override(
    mut self,
    field: capnp::schema::Field,
    codec: impl FieldCodec + 'env,
  ) -> Self {
    self.field_overrides.insert(field, Box::new(codec));
    self
  }

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
