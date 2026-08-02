// Deserialisation
use super::data::{base64, hex};
use super::{json_capnp, rust_json_capnp, DataEncoding, EncodingOptions};

enum ParseError {
  UnexpectedEndOfInput,
  InvalidToken(char),
  Other(String),
}

impl From<ParseError> for capnp::Error {
  fn from(err: ParseError) -> Self {
    match err {
      ParseError::UnexpectedEndOfInput => capnp::Error::failed(
        "Unexpected end of input while parsing JSON".into(),
      ),
      ParseError::InvalidToken(c) => {
        capnp::Error::failed(format!("Invalid token '{c}' while parsing JSON"))
      }
      // TODO: Use better values here?
      ParseError::Other(msg) => capnp::Error::failed(msg),
    }
  }
}

use std::collections::HashMap;

use super::JsonValue;

struct Parser<I>
where
  I: Iterator<Item = char>,
{
  // FIXME: By using an iter over char here, we restrict ourselves to not
  // being able to use string slices for must of the parsing. THis is piggy.
  // It would be better to just have a &str and an index probably.
  input_iter: std::iter::Peekable<std::iter::Fuse<I>>,
}

impl<I> Parser<I>
where
  I: Iterator<Item = char>,
{
  fn new(iter: I) -> Self {
    Self {
      input_iter: iter.fuse().peekable(),
    }
  }

  /// Advance past any whitespace and peek at next value
  fn peek_next(&mut self) -> Option<char> {
    self.discard_whitespace();
    self.peek()
  }

  /// Peek at the current value
  fn peek(&mut self) -> Option<char> {
    self.input_iter.peek().copied()
  }

  /// Consume the current value
  fn advance(&mut self) -> capnp::Result<char> {
    self
      .input_iter
      .next()
      .ok_or(ParseError::UnexpectedEndOfInput.into())
  }

  /// Consume the current value if it matches `c`, otherwise error
  fn consume(&mut self, c: char) -> capnp::Result<char> {
    match self.advance()? {
      p if p == c => Ok(p),
      p => Err(ParseError::InvalidToken(p).into()),
    }
  }

  /// Advance past any whitespace and consume the current value if it matches `c`, otherwise error
  fn consume_next(&mut self, c: char) -> capnp::Result<char> {
    self.discard_whitespace();
    match self.advance()? {
      p if p == c => Ok(p),
      p => Err(ParseError::InvalidToken(p).into()),
    }
  }

  fn discard_whitespace(&mut self) {
    while let Some(c) = self.peek() {
      if c.is_whitespace() {
        self.advance().ok();
      } else {
        break;
      }
    }
  }

  /// Parse one JSON value.
  ///
  /// `recursion_level` counts the arrays and objects already entered, not the
  /// values parsed, so that a scalar does not cost a level of its own. This
  /// matches the C++ codec, whose `nestingDepth` is incremented only by
  /// `parseArray` and `parseObject`; counting scalars too would make the same
  /// numeric limit one level stricter than C++'s.
  fn parse_value(
    &mut self,
    options: &crate::CodecOptions,
    recursion_level: usize,
  ) -> capnp::Result<JsonValue> {
    // Entering a container takes the depth to `recursion_level + 1`, so the
    // limit is reached when `recursion_level` has caught up with it.
    let check_container_depth = || {
      if recursion_level >= options.recursion_limit {
        return Err(capnp::Error::failed(
          "Recursion limit exceeded while parsing JSON".into(),
        ));
      }
      Ok(())
    };

    match self.peek_next() {
      None => Err(ParseError::UnexpectedEndOfInput.into()),
      Some('n') => {
        self.advance()?;
        self.consume('u')?;
        self.consume('l')?;
        self.consume('l')?;
        Ok(JsonValue::Null)
      }
      Some('t') => {
        self.advance()?;
        self.consume('r')?;
        self.consume('u')?;
        self.consume('e')?;
        Ok(JsonValue::Boolean(true))
      }
      Some('f') => {
        self.advance()?;
        self.consume('a')?;
        self.consume('l')?;
        self.consume('s')?;
        self.consume('e')?;
        Ok(JsonValue::Boolean(false))
      }
      Some('\"') => Ok(JsonValue::String(self.parse_string()?)),
      Some('0'..='9') | Some('-') => {
        let num_str = self.parse_number()?;
        let num = num_str.parse::<f64>().map_err(|e| {
          ParseError::Other(format!("Invalid number format: {e}"))
        })?;
        Ok(JsonValue::Number(num))
      }
      Some('[') => {
        check_container_depth()?;
        self.advance()?;
        let mut items = Vec::new();
        let mut require_comma = false;
        while self.peek_next().is_some_and(|c| c != ']') {
          if require_comma {
            self.consume(',')?;
          }
          require_comma = true;
          let item = self.parse_value(options, recursion_level + 1)?;
          items.push(item);
        }
        self.consume_next(']')?;
        Ok(JsonValue::Array(items))
      }
      Some('{') => {
        check_container_depth()?;
        self.advance()?;
        let mut members = HashMap::new();
        let mut require_comma = false;
        while self.peek_next().is_some_and(|c| c != '}') {
          if require_comma {
            self.consume(',')?;
          }
          require_comma = true;
          let key = self.parse_string()?;
          self.consume_next(':')?;
          let value = self.parse_value(options, recursion_level + 1)?;
          if members.insert(key.clone(), value).is_some() {
            return Err(
              ParseError::Other(format!("Duplicate key in object: {key}"))
                .into(),
            );
          }
        }
        self.consume_next('}')?;
        Ok(JsonValue::Object(members))
      }
      Some(c) => Err(ParseError::InvalidToken(c).into()),
    }
  }

  fn parse_string(&mut self) -> capnp::Result<String> {
    self.consume_next('\"')?;
    let mut result = String::new();
    loop {
      let c = self.advance()?;
      match c {
        '\"' => return Ok(result),
        '\\' => {
          let escaped = self.advance()?;
          match escaped {
            '\"' => result.push('\"'),
            '\\' => result.push('\\'),
            '/' => result.push('/'),
            'b' => result.push('\u{08}'),
            'f' => result.push('\u{0C}'),
            'n' => result.push('\n'),
            'r' => result.push('\r'),
            't' => result.push('\t'),
            'u' => result.push(self.parse_unicode_escape()?),
            other => {
              return Err(
                ParseError::Other(format!(
                  "Invalid escape character: \\{other}"
                ))
                .into(),
              );
            }
          }
        }
        other => result.push(other),
      }
    }
  }

  /// Read the four hex digits of a `\uXXXX` escape, the `\u` itself having
  /// already been consumed.
  fn parse_hex4(&mut self) -> capnp::Result<u16> {
    let mut hex = String::with_capacity(4);
    for _ in 0..4 {
      hex.push(self.advance()?);
    }
    u16::from_str_radix(&hex, 16).map_err(|_| {
      ParseError::Other(format!("Invalid unicode escape: \\u{hex}")).into()
    })
  }

  /// Decode a `\uXXXX` escape, combining a surrogate pair into the single
  /// character it stands for.
  ///
  /// A `\u` escape carries one UTF-16 code unit, which cannot reach beyond the
  /// BMP on its own. Anything above U+FFFF is therefore written as a *pair* of
  /// escapes — a high surrogate followed by a low one — which is how every
  /// JSON producer that escapes its output (`JSON.stringify` with non-ASCII
  /// escaping, Python's `json.dumps` by default) writes an emoji. Decoding the
  /// two halves independently yields two unpaired surrogates, which are not
  /// Unicode scalar values and so cannot appear in a Rust `String` or in
  /// Cap'n Proto text.
  ///
  /// The C++ codec does decode them independently and produces WTF-8 — the two
  /// surrogates encoded separately, which is not valid UTF-8 — and says as
  /// much in a TODO. Matching that is not an option here, and is not needed
  /// for interoperability either: the C++ *encoder* never emits `\u` escapes
  /// for non-BMP characters, writing them as literal UTF-8, which decodes
  /// through the ordinary path.
  ///
  /// An unpaired surrogate has no representation in UTF-8 at all, so it is
  /// rejected rather than quietly replaced with U+FFFD.
  fn parse_unicode_escape(&mut self) -> capnp::Result<char> {
    const HIGH: std::ops::RangeInclusive<u16> = 0xD800..=0xDBFF;
    const LOW: std::ops::RangeInclusive<u16> = 0xDC00..=0xDFFF;

    let unit = self.parse_hex4()?;

    if LOW.contains(&unit) {
      return Err(
        ParseError::Other(format!(
          "Invalid unicode escape: \\u{unit:04X} is a trailing surrogate with \
           no leading surrogate before it"
        ))
        .into(),
      );
    }

    if HIGH.contains(&unit) {
      // A leading surrogate is only half a character; the other half must be
      // the very next escape.
      if self.peek() != Some('\\') {
        return Err(
          ParseError::Other(format!(
            "Invalid unicode escape: \\u{unit:04X} is a leading surrogate and \
             must be followed by a \\u escape"
          ))
          .into(),
        );
      }
      self.advance()?;
      self.consume('u')?;

      let low = self.parse_hex4()?;
      if !LOW.contains(&low) {
        return Err(
          ParseError::Other(format!(
            "Invalid unicode escape: \\u{unit:04X} must be followed by a \
             trailing surrogate, found \\u{low:04X}"
          ))
          .into(),
        );
      }

      let code_point =
        0x10000 + (((unit as u32 - 0xD800) << 10) | (low as u32 - 0xDC00));
      return std::char::from_u32(code_point).ok_or_else(|| {
        ParseError::Other(format!(
          "Invalid unicode code point: \\u{unit:04X}\\u{low:04X}"
        ))
        .into()
      });
    }

    // Not a surrogate, so it is a scalar value and the conversion cannot fail.
    std::char::from_u32(unit as u32).ok_or_else(|| {
      ParseError::Other(format!("Invalid unicode code point: \\u{unit:04X}"))
        .into()
    })
  }

  fn parse_number(&mut self) -> capnp::Result<String> {
    let mut num_str = String::new();
    if self.peek_next().is_some_and(|c| c == '-') {
      num_str.push(self.advance()?);
    }
    while self.peek().is_some_and(|c| c.is_ascii_digit()) {
      num_str.push(self.advance()?);
    }
    if self.peek().is_some_and(|c| c == '.') {
      num_str.push(self.advance()?);
      while self.peek().is_some_and(|c| c.is_ascii_digit()) {
        num_str.push(self.advance()?);
      }
    }
    if self.peek().is_some_and(|c| c == 'e' || c == 'E') {
      num_str.push(self.advance()?);
      if self.peek().is_some_and(|c| c == '+' || c == '-') {
        num_str.push(self.advance()?);
      }
      while self.peek().is_some_and(|c| c.is_ascii_digit()) {
        num_str.push(self.advance()?);
      }
    }
    Ok(num_str)
  }
}

pub(crate) fn parse(
  codec: &super::Codec,
  json: &str,
  builder: capnp::dynamic_struct::Builder<'_>,
) -> capnp::Result<()> {
  let mut parser = Parser::new(json.chars());
  let mut value = parser.parse_value(&codec.options, 0)?;
  parser.discard_whitespace();
  if parser.peek().is_some() {
    return Err(capnp::Error::failed(
      "Trailing characters after JSON value".into(),
    ));
  }
  let meta = EncodingOptions::default();
  decode_struct(0, codec, &mut value, builder, &meta)
}

/// Whether a JSON `null` here means "this field is absent".
///
/// A null pointer and an absent field are the same thing in Cap'n Proto, so
/// for the pointer types a JSON `null` says the field was not set rather than
/// that it holds an empty value. This mirrors `isPointerToJsonNull` in the C++
/// codec, and the type list is the same one: Text, Data, List and Struct.
///
/// `Void` is deliberately not in that list even though it encodes as `null`:
/// `null` is its *value*, and a void field decoded from `null` must be set,
/// not skipped. C++ excludes it for the same reason.
///
/// This is a property of the field, not of the value alone, so it applies only
/// where a field is being decoded. C++ likewise checks it in `decodeField` and
/// not in `decodeArray`, so `null` remains an error as a list element.
fn is_pointer_to_json_null(
  value: &JsonValue,
  field_type: &capnp::introspect::Type,
) -> bool {
  matches!(value, JsonValue::Null)
    && matches!(
      field_type.which(),
      capnp::introspect::TypeVariant::Text
        | capnp::introspect::TypeVariant::Data
        | capnp::introspect::TypeVariant::List(_)
        | capnp::introspect::TypeVariant::Struct(_)
    )
}

/// Convert a JSON number to an integer, rejecting anything the target type
/// cannot hold exactly.
///
/// JSON numbers are `f64`, so `300` is a perfectly well-formed JSON number to
/// find in an `Int8` field, and `1.9` in an `Int32`. Converting with `as`
/// would silently store 127 and 1 respectively and report success, turning
/// malformed input into plausible-looking data.
///
/// The C++ codec rejects both, via three checks in `capnp/dynamic.c++`:
/// `value >= MIN`, `value <= MAX`, and `T(value) == value`. Converting and
/// converting back tests all three at once: Rust's float-to-integer `as`
/// saturates, so anything outside the range comes back as the clamped bound,
/// and any fractional part is lost — either way the round trip differs from
/// the input. `NaN` and the infinities fail too, since neither survives the
/// round trip.
///
/// This deliberately does not apply to floats or to enum ordinals, neither of
/// which C++ range-checks: `1e300` into a `Float32` gives `inf` there and
/// here.
macro_rules! checked_int {
  ($value:expr, $rust_ty:ty, $capnp_ty:literal, $field:expr) => {{
    let value: f64 = $value;
    let converted = value as $rust_ty;
    if converted as f64 == value {
      Ok(converted)
    } else if value.trunc() == value {
      Err(capnp::Error::failed(format!(
        "Value {value} is out of range for {} field {}",
        $capnp_ty, $field
      )))
    } else {
      Err(capnp::Error::failed(format!(
        "Value {value} is not an integer, required for {} field {}",
        $capnp_ty, $field
      )))
    }
  }};
}

fn decode_primitive<'json, 'meta>(
  field_value: &'json mut JsonValue,
  field_type: &'meta capnp::introspect::Type,
  field_meta: &'meta EncodingOptions,
) -> capnp::Result<capnp::dynamic_value::Reader<'json>> {
  match field_type.which() {
    capnp::introspect::TypeVariant::Void => {
      if !matches!(field_value, JsonValue::Null) {
        Err(capnp::Error::failed(format!(
          "Expected null for void field {}",
          field_meta.name
        )))
      } else {
        Ok(capnp::dynamic_value::Reader::Void)
      }
    }
    capnp::introspect::TypeVariant::Bool => {
      let JsonValue::Boolean(field_value) = field_value else {
        return Err(capnp::Error::failed(format!(
          "Expected boolean for field {}",
          field_meta.name
        )));
      };
      Ok((*field_value).into())
    }
    capnp::introspect::TypeVariant::Int8 => {
      let JsonValue::Number(field_value) = field_value else {
        return Err(capnp::Error::failed(format!(
          "Expected number for field {}",
          field_meta.name
        )));
      };
      Ok(checked_int!(*field_value, i8, "Int8", field_meta.name)?.into())
    }
    capnp::introspect::TypeVariant::Int16 => {
      let JsonValue::Number(field_value) = field_value else {
        return Err(capnp::Error::failed(format!(
          "Expected number for field {}",
          field_meta.name
        )));
      };
      Ok(checked_int!(*field_value, i16, "Int16", field_meta.name)?.into())
    }
    capnp::introspect::TypeVariant::Int32 => {
      let JsonValue::Number(field_value) = field_value else {
        return Err(capnp::Error::failed(format!(
          "Expected number for field {}",
          field_meta.name
        )));
      };
      Ok(checked_int!(*field_value, i32, "Int32", field_meta.name)?.into())
    }
    capnp::introspect::TypeVariant::Int64 => match field_value {
      JsonValue::Number(field_value) => {
        Ok(checked_int!(*field_value, i64, "Int64", field_meta.name)?.into())
      }
      JsonValue::String(field_value) => Ok(
        (field_value.parse::<i64>().map_err(|_| {
          capnp::Error::failed(format!(
            "Invalid numeric value '{}' for field {}",
            field_value, field_meta.name
          ))
        })?)
        .into(),
      ),
      _ => Err(capnp::Error::failed(format!(
        "Expected number or string number for field {}",
        field_meta.name
      ))),
    },
    capnp::introspect::TypeVariant::UInt8 => {
      let JsonValue::Number(field_value) = field_value else {
        return Err(capnp::Error::failed(format!(
          "Expected number for field {}",
          field_meta.name
        )));
      };
      Ok(checked_int!(*field_value, u8, "UInt8", field_meta.name)?.into())
    }
    capnp::introspect::TypeVariant::UInt16 => {
      let JsonValue::Number(field_value) = field_value else {
        return Err(capnp::Error::failed(format!(
          "Expected number for field {}",
          field_meta.name
        )));
      };
      Ok(checked_int!(*field_value, u16, "UInt16", field_meta.name)?.into())
    }
    capnp::introspect::TypeVariant::UInt32 => {
      let JsonValue::Number(field_value) = field_value else {
        return Err(capnp::Error::failed(format!(
          "Expected number for field {}",
          field_meta.name
        )));
      };
      Ok(checked_int!(*field_value, u32, "UInt32", field_meta.name)?.into())
    }
    capnp::introspect::TypeVariant::UInt64 => match field_value {
      JsonValue::Number(field_value) => {
        Ok(checked_int!(*field_value, u64, "UInt64", field_meta.name)?.into())
      }
      JsonValue::String(field_value) => Ok(
        (field_value.parse::<u64>().map_err(|_| {
          capnp::Error::failed(format!(
            "Invalid numeric value '{}' for field {}",
            field_value, field_meta.name
          ))
        })?)
        .into(),
      ),
      _ => Err(capnp::Error::failed(format!(
        "Expected string number for field {}",
        field_meta.name
      ))),
    },
    capnp::introspect::TypeVariant::Float32 => {
      let field_value = match field_value {
        // C++ decodes a JSON null into a float as NaN.
        JsonValue::Null => f32::NAN,
        JsonValue::Number(field_value) => *field_value as f32,
        JsonValue::String(field_value) => match field_value.as_str() {
          "NaN" => f32::NAN,
          "Infinity" => f32::INFINITY,
          "-Infinity" => f32::NEG_INFINITY,
          _ => {
            return Err(capnp::Error::failed(format!(
              "Expected number for field {}",
              field_meta.name
            )));
          }
        },
        _ => {
          return Err(capnp::Error::failed(format!(
            "Expected number for field {}",
            field_meta.name
          )));
        }
      };
      Ok(field_value.into())
    }
    capnp::introspect::TypeVariant::Float64 => {
      let field_value = match field_value {
        // C++ decodes a JSON null into a float as NaN.
        JsonValue::Null => f64::NAN,
        JsonValue::Number(field_value) => *field_value,
        JsonValue::String(field_value) => match field_value.as_str() {
          "NaN" => f64::NAN,
          "Infinity" => f64::INFINITY,
          "-Infinity" => f64::NEG_INFINITY,
          _ => {
            return Err(capnp::Error::failed(format!(
              "Expected number for field {}",
              field_meta.name
            )));
          }
        },
        _ => {
          return Err(capnp::Error::failed(format!(
            "Expected number for field {}",
            field_meta.name
          )));
        }
      };
      Ok(field_value.into())
    }
    capnp::introspect::TypeVariant::Text => {
      let JsonValue::String(field_value) = field_value else {
        return Err(capnp::Error::failed(format!(
          "Expected string for field {}",
          field_meta.name
        )));
      };
      Ok((*field_value.as_str()).into())
    }
    capnp::introspect::TypeVariant::Enum(enum_schema) => match field_value {
      JsonValue::String(field_value) => {
        let enum_schema = capnp::schema::EnumSchema::new(enum_schema);
        let Some(enum_value) = enum_schema.get_enumerants()?.iter().find(|e| {
          // FIXME: this is naive, enum values can be renamed using
          // $Json.name so we need to handle that

          let annotations = e.get_annotations().ok();
          let value = annotations
            .and_then(|anns| {
              anns
                .iter()
                .find(|a| a.get_id() == json_capnp::name::ID)
                .and_then(|a| {
                  a.get_value()
                    .ok()
                    .map(|v| v.downcast::<capnp::text::Reader>().to_str().ok())
                })
            })
            .unwrap_or(
              e.get_proto().get_name().ok().and_then(|n| n.to_str().ok()),
            );
          value.is_some_and(|s| s == field_value)
        }) else {
          return Err(capnp::Error::failed(format!(
            "Invalid enum value '{}' for field {}",
            field_value, field_meta.name
          )));
        };

        Ok(capnp::dynamic_value::Reader::Enum(
          capnp::dynamic_value::Enum::new(
            enum_value.get_ordinal(),
            enum_value.get_containing_enum(),
          ),
        ))
      }
      JsonValue::Number(enum_value) => {
        let enum_schema = capnp::schema::EnumSchema::new(enum_schema);
        Ok(capnp::dynamic_value::Reader::Enum(
          capnp::dynamic_value::Enum::new(*enum_value as u16, enum_schema),
        ))
      }
      _ => Err(capnp::Error::failed(format!(
        "Expected string or number for enum field {}",
        field_meta.name
      ))),
    },
    capnp::introspect::TypeVariant::Data => match field_meta.data_encoding {
      // The reason we have this ugly DataBuffer hack is to ensure that we
      // can return a Reader from this function whose lifetime is tied to
      // the field_value, as there is no other buffer we can use. We don't
      // currently support Orphans, but if we did, most of this Reader
      // dance could probably be avoided.
      DataEncoding::Default => {
        let JsonValue::Array(data_value) = field_value else {
          return Err(capnp::Error::failed(format!(
            "Expected array for data field {}",
            field_meta.name
          )));
        };
        let mut data = Vec::with_capacity(data_value.len());
        for byte_value in data_value.drain(..) {
          let JsonValue::Number(byte_value) = byte_value else {
            return Err(capnp::Error::failed(format!(
              "Expected number for data byte in field {}",
              field_meta.name
            )));
          };
          // C++: "Number in byte array is not an integer in [0, 255]".
          data.push(checked_int!(
            byte_value,
            u8,
            "Data byte in",
            field_meta.name
          )?);
        }
        *field_value = JsonValue::DataBuffer(data);
        Ok(capnp::dynamic_value::Reader::Data(match field_value {
          JsonValue::DataBuffer(data) => data.as_slice(),
          _ => unreachable!(),
        }))
      }
      DataEncoding::Base64 => {
        let JsonValue::String(data_value) = field_value else {
          return Err(capnp::Error::failed(format!(
            "Expected string for base64 data field {}",
            field_meta.name
          )));
        };
        *field_value = JsonValue::DataBuffer(base64::decode(data_value)?);
        Ok(capnp::dynamic_value::Reader::Data(match field_value {
          JsonValue::DataBuffer(data) => data.as_slice(),
          _ => unreachable!(),
        }))
      }
      DataEncoding::Hex => {
        let JsonValue::String(data_value) = field_value else {
          return Err(capnp::Error::failed(format!(
            "Expected string for hex data field {}",
            field_meta.name
          )));
        };
        *field_value = JsonValue::DataBuffer(hex::decode(data_value)?);
        Ok(capnp::dynamic_value::Reader::Data(match field_value {
          JsonValue::DataBuffer(data) => data.as_slice(),
          _ => unreachable!(),
        }))
      }
    },
    _ => Err(capnp::Error::failed(format!(
      "Unsupported primitive type for field {}",
      field_meta.name
    ))),
  }
}

fn decode_list(
  recursion_level: usize,
  codec: &super::Codec,
  mut field_values: Vec<JsonValue>,
  mut list_builder: capnp::dynamic_list::Builder,
  field_meta: &EncodingOptions,
) -> capnp::Result<()> {
  match list_builder.element_type().which() {
    capnp::introspect::TypeVariant::Struct(_sub_element_schema) => {
      for (i, mut item_value) in field_values.drain(..).enumerate() {
        let struct_builder = list_builder
          .reborrow()
          .get(i as u32)?
          .downcast::<capnp::dynamic_struct::Builder>();
        decode_struct(
          recursion_level + 1,
          codec,
          &mut item_value,
          struct_builder,
          field_meta,
        )?;
      }
      Ok(())
    }
    capnp::introspect::TypeVariant::List(_sub_element_type) => {
      for (i, item_value) in field_values.drain(..).enumerate() {
        let JsonValue::Array(item_value) = item_value else {
          return Err(capnp::Error::failed(format!(
            "Expected array for list field {}",
            field_meta.name
          )));
        };
        let sub_element_builder = list_builder
          .reborrow()
          .init(i as u32, item_value.len() as u32)?
          .downcast::<capnp::dynamic_list::Builder>();
        decode_list(
          recursion_level + 1,
          codec,
          item_value,
          sub_element_builder,
          field_meta,
        )?;
      }
      Ok(())
    }
    _ => {
      for (i, mut item_value) in field_values.drain(..).enumerate() {
        list_builder.set(
          i as u32,
          decode_primitive(
            &mut item_value,
            &list_builder.element_type(),
            field_meta,
          )?,
        )?;
      }
      Ok(())
    }
  }
}

fn decode_struct(
  recursion_level: usize,
  codec: &super::Codec,
  value: &mut JsonValue,
  mut builder: capnp::dynamic_struct::Builder<'_>,
  meta: &EncodingOptions,
) -> capnp::Result<()> {
  if recursion_level > codec.options.recursion_limit {
    return Err(capnp::Error::failed(
      "Recursion limit exceeded while decoding JSON".into(),
    ));
  }

  let field_prefix = if let Some(flatten_options) = &meta.flatten {
    std::borrow::Cow::Owned(format!(
      "{}{}",
      meta.prefix,
      flatten_options.get_prefix()?.to_str()?
    ))
  } else {
    std::borrow::Cow::Borrowed("")
  };

  if let Some(field_codec) = builder
    .get_schema()
    .get_annotations()?
    .iter()
    .find(|a| a.get_id() == rust_json_capnp::codec::ID)
  {
    let field_codec = field_codec
      .get_value()?
      .downcast::<capnp::text::Reader>()
      .to_str()?;
    if let Some(field_codec) = codec.registry.get(field_codec) {
      return field_codec.decode_value(value, builder.reborrow().into());
    }
  }

  fn decode_member(
    recursion_level: usize,
    codec: &super::Codec,
    mut builder: capnp::dynamic_struct::Builder<'_>,
    field: capnp::schema::Field,
    field_meta: &EncodingOptions,
    value: &mut JsonValue,
    value_name: &str,
  ) -> capnp::Result<()> {
    let JsonValue::Object(obj) = value else {
      return Err(capnp::Error::failed(
        "Expected object for struct field".into(),
      ));
    };

    if let Some(field_codec) = field_meta
      .codec
      .and_then(|c| codec.registry.get(c))
      .or_else(|| {
        field_meta.field.and_then(|f| {
          codec
            .field_overrides
            .get(&f)
            .or_else(|| codec.type_overrides.get(&f.get_type()))
        })
      })
    {
      let field_value = match obj.remove(value_name) {
        Some(v) => v,
        None => return Ok(()),
      };
      return field_codec.decode_member(
        &field_value,
        builder.reborrow(),
        field,
      );
    }

    match field.get_type().which() {
      capnp::introspect::TypeVariant::Struct(_struct_schema) => {
        if field_meta.flatten.is_none() {
          let mut field_value = match obj.remove(value_name) {
            Some(v) => v,
            None => return Ok(()),
          };
          if is_pointer_to_json_null(&field_value, &field.get_type()) {
            return Ok(());
          }

          let struct_builder = builder
            .reborrow()
            .init(field)?
            .downcast::<capnp::dynamic_struct::Builder>();

          decode_struct(
            recursion_level + 1,
            codec,
            &mut field_value,
            struct_builder,
            field_meta,
          )?;
        } else {
          //
          // FIXME: We should only init this struct if any field is
          // found in decode_struct. For now, we always init it.
          // To do that we would need to get decode_struct to actually
          // take the builder+field, or a callback to init it.
          //
          // The current implementation results in has_<field>()
          // returning true even if all fields are missing in the
          // JSON.
          //
          let struct_builder = builder
            .reborrow()
            .init(field)?
            .downcast::<capnp::dynamic_struct::Builder>();

          // Flattened struct; pass the JsonValue at this level down
          decode_struct(
            recursion_level + 1,
            codec,
            value,
            struct_builder,
            field_meta,
          )?;
        }
      }
      capnp::introspect::TypeVariant::List(_element_type) => {
        let Some(field_value) = obj.remove(value_name) else {
          return Ok(());
        };
        if is_pointer_to_json_null(&field_value, &field.get_type()) {
          return Ok(());
        }

        let JsonValue::Array(field_value) = field_value else {
          return Err(capnp::Error::failed(format!(
            "Expected array for field {}",
            field_meta.name
          )));
        };
        let list_builder = builder
          .reborrow()
          .initn(field, field_value.len() as u32)?
          .downcast::<capnp::dynamic_list::Builder>();
        decode_list(
          recursion_level,
          codec,
          field_value,
          list_builder,
          field_meta,
        )?;
      }

      capnp::introspect::TypeVariant::AnyPointer => {
        if obj.remove(value_name).is_some() {
          return Err(capnp::Error::unimplemented(
            "AnyPointer cannot be represented in JSON".into(),
          ));
        }
      }
      capnp::introspect::TypeVariant::Capability => {
        if obj.remove(value_name).is_some() {
          return Err(capnp::Error::unimplemented(
            "Capability cannot be represented in JSON".into(),
          ));
        }
      }

      _ => {
        let Some(mut field_value) = obj.remove(value_name) else {
          return Ok(());
        };
        if is_pointer_to_json_null(&field_value, &field.get_type()) {
          return Ok(());
        }

        builder.set(
          field,
          decode_primitive(&mut field_value, &field.get_type(), field_meta)?,
        )?;
      }
    }
    Ok(())
  }

  for field in builder.get_schema().get_non_union_fields()? {
    let field_meta = EncodingOptions::from_field(&field_prefix, field)?;
    let field_name = format!("{}{}", field_prefix, field_meta.name);

    decode_member(
      recursion_level,
      codec,
      builder.reborrow(),
      field,
      &field_meta,
      value,
      &field_name,
    )?;
  }

  let JsonValue::Object(obj) = value else {
    return Err(capnp::Error::failed(
      "Expected object for struct field".into(),
    ));
  };

  let struct_discriminator = builder
    .get_schema()
    .get_annotations()?
    .iter()
    .find(|a| a.get_id() == json_capnp::discriminator::ID)
    .and_then(|annotation| {
      annotation.get_value().ok().map(|v| {
        v.downcast_struct::<json_capnp::discriminator_options::Owned>()
      })
    });
  let discriminator = meta.discriminator.or(struct_discriminator);

  // FIXME: refactor this to only loop through union memberes once; each
  // iteration check if it matches the discriminant, *or* the requisite
  // named field is present, then decode and break;
  let discriminant = match discriminator {
    Some(discriminator) => {
      let discriminator_name = if discriminator.has_name() {
        discriminator.get_name()?.to_str()?
      } else {
        meta.name
      };
      let field_name = format!("{field_prefix}{discriminator_name}");
      if let Some(JsonValue::String(discriminant)) = obj.remove(&field_name) {
        Some(discriminant)
      } else {
        None
      }
    }
    None => None,
  };

  let discriminant = match discriminant {
    Some(discriminant) => Some(discriminant),
    None => {
      // find the first field that exists matching a union field?
      let mut discriminant = None;
      for field in builder.get_schema().get_union_fields()? {
        let field_meta = EncodingOptions::from_field(meta.prefix, field)?;
        let field_name = format!("{}{}", field_prefix, field_meta.name);
        if obj.contains_key(&field_name) {
          discriminant = Some(field_meta.name.to_string());
          break;
        }
      }
      discriminant
    }
  };

  if let Some(discriminant) = discriminant {
    for field in builder.get_schema().get_union_fields()? {
      let field_meta = EncodingOptions::from_field(meta.prefix, field)?;
      if field_meta.name != discriminant {
        continue;
      }
      let value_name = if let Some(discriminator) = discriminator {
        if discriminator.has_value_name() {
          discriminator.get_value_name()?.to_str()?
        } else {
          field_meta.name
        }
      } else {
        field_meta.name
      };
      if matches!(
        field.get_type().which(),
        capnp::introspect::TypeVariant::Void
      ) {
        // Void union member; just set the discriminant
        builder
          .reborrow()
          .set(field, capnp::dynamic_value::Reader::Void)?;
        break;
      }
      decode_member(
        recursion_level,
        codec,
        builder.reborrow(),
        field,
        &field_meta,
        value,
        value_name,
      )?;
      break;
    }
  }

  Ok(())
}

#[cfg(test)]
mod test {
  use super::*;
  #[test]
  fn test_parse_string() -> capnp::Result<()> {
    let json = r#""Hello, World!""#;

    let mut parser = Parser::new(json.chars());
    let value = parser.parse_value(&crate::CodecOptions::default(), 0)?;

    assert!(matches!(value, JsonValue::String(s) if s == "Hello, World!"));
    Ok(())
  }

  #[test]
  fn test_parse_string_with_special_chars() -> capnp::Result<()> {
    let json = r#""Hełło,\nWorld!\"†ęś†: \u0007""#;

    let mut parser = Parser::new(json.chars());
    let value = parser.parse_value(&crate::CodecOptions::default(), 0)?;

    assert!(
      matches!(value, JsonValue::String(s) if s == "Hełło,\nWorld!\"†ęś†: \u{0007}")
    );

    let json = r#"{"value":"tab: \t, newline: \n, carriage return: \r, quote: \", backslash: \\"}"#;
    let mut parser = Parser::new(json.chars());
    let value = parser.parse_value(&crate::CodecOptions::default(), 0)?;
    let JsonValue::Object(map) = value else {
      panic!("Expected object at top level");
    };
    let Some(JsonValue::String(s)) = map.get("value") else {
      panic!("Expected string value for 'value' key");
    };
    assert_eq!(
      s,
      "tab: \t, newline: \n, carriage return: \r, quote: \", backslash: \\"
    );
    Ok(())
  }
}
