use super::data::{base64, hex};
use super::{
  json_capnp,
  rust_json_capnp,
  DataEncoding,
  EncodingOptions,
  JsonValue,
};

pub(crate) fn serialize_json_to<'reader, W>(
  codec: &super::Codec,
  writer: &mut W,
  reader: impl Into<capnp::dynamic_value::Reader<'reader>>,
) -> capnp::Result<()>
where
  W: std::io::Write,
{
  let meta = EncodingOptions::default();
  serialize_value_to(codec, writer, reader.into(), &meta, &mut true)
}

fn write_json_value<W: std::io::Write>(
  writer: &mut W,
  value: &JsonValue,
  meta: &EncodingOptions<'_, '_>,
  first: &mut bool,
) -> capnp::Result<()> {
  let (flatten, field_prefix) = if let Some(flatten_options) = &meta.flatten {
    (
      true,
      std::borrow::Cow::Owned(format!(
        "{}{}",
        meta.prefix,
        flatten_options.get_prefix()?.to_str()?
      )),
    )
  } else {
    (false, std::borrow::Cow::Borrowed(""))
  };

  if flatten && !matches!(value, JsonValue::Object(_)) {
    return Err(capnp::Error::failed(format!(
      "Canot encode {}: Flattening is only supported for objects, found {:?}",
      meta.name, value
    )));
  }

  match value {
    JsonValue::Null => writer.write_all(b"null").map_err(Into::into),
    JsonValue::Boolean(v) => writer
      .write_all(if *v { b"true" } else { b"false" })
      .map_err(Into::into),
    JsonValue::Number(v) => write_float(writer, *v),
    JsonValue::String(v) => write_string(writer, v.as_str()),
    JsonValue::Array(json_values) => {
      writer.write_all(b"[")?;
      let mut first = true;
      for item in json_values {
        if !first {
          writer.write_all(b",")?;
        }
        first = false;
        write_json_value(writer, item, &EncodingOptions::default(), &mut true)?;
      }
      writer.write_all(b"]")?;
      Ok(())
    }
    JsonValue::Object(hash_map) => {
      let mut my_first = true;
      let first = if !flatten {
        writer.write_all(b"{")?;
        &mut my_first
      } else {
        first
      };
      for (key, value) in hash_map {
        if !*first {
          writer.write_all(b",")?;
        }
        *first = false;
        write_key(writer, &field_prefix, key)?;
        write_json_value(writer, value, &EncodingOptions::default(), first)?;
      }
      if !flatten {
        writer.write_all(b"}")?;
      }
      Ok(())
    }
    JsonValue::DataBuffer(_items) => Err(capnp::Error::unimplemented(
      "DataBuffer is not a valid encoding target".into(),
    )),
  }
}

fn serialize_value_to<W>(
  codec: &super::Codec,
  writer: &mut W,
  reader: capnp::dynamic_value::Reader<'_>,
  meta: &EncodingOptions<'_, '_>,
  first: &mut bool,
) -> capnp::Result<()>
where
  W: std::io::Write,
{
  if let Some(field_codec) =
    meta.codec.and_then(|c| codec.registry.get(c)).or_else(|| {
      // Consulting the override maps means hashing the field, and this runs
      // for every value encoded. Most codecs register no overrides at all, so
      // rule that out first.
      if codec.field_overrides.is_empty() && codec.type_overrides.is_empty() {
        return None;
      }
      meta.field.and_then(|f| {
        codec
          .field_overrides
          .get(&f)
          .or_else(|| codec.type_overrides.get(&f.get_type()))
      })
    })
  {
    write_json_value(writer, &field_codec.encode_value(reader)?, meta, first)
  } else {
    // No codec claimed this value, so a declared field type may reinterpret
    // it. Same reasoning as the override maps above: skip the hash when none
    // are registered.
    //
    // Only the read is needed here. Once the AnyPointer is resolved to a
    // concrete `dynamic_value::Reader` the match below handles it like any
    // other value, so declaring a type costs nothing further on this side —
    // unlike decoding, which has to dispatch on the type rather than the
    // value. See `decode::decode_typed_member`.
    let reader = if codec.field_types.is_empty() {
      reader
    } else {
      match meta.field.and_then(|f| codec.field_types.get(&f)) {
        Some(field_type) => {
          match reader {
            capnp::dynamic_value::Reader::AnyPointer(any) => {
              field_type.read(any)?
            }
            // The elements of a list carry their containing field's `meta`, so
            // this runs again for each of them. The declaration applies to the
            // field, not to its elements, and was already applied when the
            // list itself came through — so leave an already-resolved value
            // alone. (`with_type_override` has the same shape and the same
            // caveat: it matches a field, and a list field's elements are not
            // fields.)
            resolved => resolved,
          }
        }
        None => reader,
      }
    };

    match reader {
      capnp::dynamic_value::Reader::Void => {
        writer.write_all(b"null").map_err(Into::into)
      }
      capnp::dynamic_value::Reader::Bool(value) => writer
        .write_all(if value { b"true" } else { b"false" })
        .map_err(Into::into),
      capnp::dynamic_value::Reader::Int8(value) => write_int(writer, value),
      capnp::dynamic_value::Reader::Int16(value) => write_int(writer, value),
      capnp::dynamic_value::Reader::Int32(value) => write_int(writer, value),
      // 64-bit integers go out as strings, but digits never need escaping, so
      // they are written straight between the quotes.
      capnp::dynamic_value::Reader::Int64(value) => {
        writer.write_all(b"\"")?;
        write_int(writer, value)?;
        writer.write_all(b"\"").map_err(Into::into)
      }
      capnp::dynamic_value::Reader::UInt8(value) => write_int(writer, value),
      capnp::dynamic_value::Reader::UInt16(value) => write_int(writer, value),
      capnp::dynamic_value::Reader::UInt32(value) => write_int(writer, value),
      capnp::dynamic_value::Reader::UInt64(value) => {
        writer.write_all(b"\"")?;
        write_int(writer, value)?;
        writer.write_all(b"\"").map_err(Into::into)
      }
      capnp::dynamic_value::Reader::Float32(value) => {
        write_float(writer, value)
      }
      capnp::dynamic_value::Reader::Float64(value) => {
        write_float(writer, value)
      }
      capnp::dynamic_value::Reader::Enum(value) => {
        if let Some(enumerant) = value.get_enumerant()? {
          let value = enumerant
            .get_annotations()?
            .iter()
            .find(|a| a.get_id() == json_capnp::name::ID)
            .and_then(|a| {
              a.get_value()
                .ok()
                .map(|v| v.downcast::<capnp::text::Reader>().to_str())
            })
            .unwrap_or(enumerant.get_proto().get_name()?.to_str());
          write_string(writer, value?)
        } else {
          write_int(writer, value.get_value())
        }
      }
      capnp::dynamic_value::Reader::Text(reader) => {
        write_string(writer, reader.to_str()?)
      }
      capnp::dynamic_value::Reader::Data(data) => {
        write_data(writer, data, meta.data_encoding)
      }
      capnp::dynamic_value::Reader::Struct(reader) => {
        if let Some(field_codec) = reader
          .get_schema()
          .get_annotations()?
          .iter()
          .find(|a| a.get_id() == rust_json_capnp::codec::ID)
        {
          let field_codec = field_codec
            .get_value()?
            .downcast::<capnp::text::Reader<'_>>()
            .to_str()?;
          if let Some(field_codec) = codec.registry.get(field_codec) {
            return write_json_value(
              writer,
              &field_codec.encode_value(reader.into())?,
              meta,
              first,
            );
          }
        }

        write_object(codec, writer, reader, meta, first)
      }
      capnp::dynamic_value::Reader::List(reader) => {
        write_array(codec, writer, reader.iter(), meta)
      }
      // A *null* AnyPointer never reaches here. `write_object` skips any field
      // for which `has()` is false, and `has()` is false for a null pointer
      // (`Type::is_pointer_type` counts AnyPointer). Lists cannot supply one
      // either: `capnp compile` rejects `List(AnyPointer)` outright, including
      // via an unbound generic parameter. So reaching this arm means a
      // non-null pointer, which is not representable.
      //
      // Do not be tempted to return `Ok(())` for the null case: callers have
      // already written the field name and colon, or the separating comma,
      // and a branch that writes nothing would produce malformed JSON.
      capnp::dynamic_value::Reader::AnyPointer(_) => {
        Err(capnp::Error::unimplemented(
          "AnyPointer cannot be represented in JSON".into(),
        ))
      }
      capnp::dynamic_value::Reader::Capability(_) => {
        Err(capnp::Error::unimplemented(
          "Capability cannot be represented in JSON".into(),
        ))
      }
    }
  }
}

/// Write an integer. Integers go out as JSON numbers verbatim; they are never
/// non-finite and never need quoting or escaping, so this is just the digits.
///
/// Kept separate from [`write_float`] because routing an integer through `f64`
/// formatting, as this used to, is both slower and harder to read than
/// formatting it as what it is.
fn write_int<W: std::io::Write>(
  writer: &mut W,
  value: impl std::fmt::Display,
) -> capnp::Result<()> {
  write!(writer, "{value}").map_err(Into::into)
}

fn write_float<W: std::io::Write>(
  writer: &mut W,
  value: impl Into<f64>,
) -> capnp::Result<()> {
  let value: f64 = value.into();

  // From the C++ codec comments:
  // Inf, -inf and NaN are not allowed in the JSON spec. Storing into string.

  if value.is_finite() {
    write!(writer, "{value}")?;
  } else if value.is_nan() {
    writer.write_all(b"\"NaN\"")?;
  } else if value.is_sign_positive() {
    writer.write_all(b"\"Infinity\"")?;
  } else {
    writer.write_all(b"\"-Infinity\"")?;
  }
  Ok(())
}

/// Write a string's contents, escaped, without the surrounding quotes.
///
/// Characters needing no escape are written in runs rather than one at a time:
/// the common case is a whole string with nothing to escape, which becomes a
/// single `write_all` of the original bytes.
fn write_escaped<W: std::io::Write>(
  writer: &mut W,
  value: &str,
) -> capnp::Result<()> {
  let bytes = value.as_bytes();
  // Start of the run of bytes that can be written as-is.
  let mut run = 0;
  let mut i = 0;

  // Scanning bytes rather than characters is safe here, and much cheaper than
  // decoding UTF-8 for every character. Everything needing an escape is
  // ASCII, except the C1 controls U+0080-U+009F, which are always the two
  // bytes `C2 80`..`C2 9F`. No byte of a multi-byte sequence can be confused
  // with one of those cases: continuation bytes are 0x80..=0xBF and `C2`
  // never appears as one, so a match is always at a character boundary.
  while i < bytes.len() {
    let byte = bytes[i];
    if byte >= 0x20
      && byte != b'\"'
      && byte != b'\\'
      && byte != 0x7F
      && byte != 0xC2
    {
      i += 1;
      continue;
    }

    let escape: &[u8] = match byte {
      b'\"' => b"\\\"",
      b'\\' => b"\\\\",
      b'\n' => b"\\n",
      b'\r' => b"\\r",
      b'\t' => b"\\t",
      0x08 => b"\\b",
      0x0C => b"\\f",
      0xC2 => {
        match bytes.get(i + 1) {
          // A C1 control; its code point is the second byte's value.
          Some(&next) if (0x80..=0x9F).contains(&next) => {
            writer.write_all(&bytes[run..i])?;
            write!(writer, "\\u{next:04x}")?;
            i += 2;
            run = i;
          }
          // Any other character that happens to start with `C2`.
          _ => i += 1,
        }
        continue;
      }
      // The remaining C0 controls and DEL have no short form. Escaping DEL
      // and the C1 range is wider than JSON demands, but is what this crate
      // has always emitted.
      other => {
        writer.write_all(&bytes[run..i])?;
        write!(writer, "\\u{other:04x}")?;
        i += 1;
        run = i;
        continue;
      }
    };

    writer.write_all(&bytes[run..i])?;
    writer.write_all(escape)?;
    i += 1;
    run = i;
  }

  writer.write_all(&bytes[run..])?;
  Ok(())
}

fn write_string<W: std::io::Write>(
  writer: &mut W,
  value: &str,
) -> capnp::Result<()> {
  writer.write_all(b"\"")?;
  write_escaped(writer, value)?;
  writer.write_all(b"\"")?;
  Ok(())
}

/// Write `"<prefix><name>":`, the two halves of the key going into the same
/// pair of quotes without being joined into a temporary first. Field names are
/// written once per field, so the allocation this avoids is per-field.
fn write_key<W: std::io::Write>(
  writer: &mut W,
  prefix: &str,
  name: &str,
) -> capnp::Result<()> {
  writer.write_all(b"\"")?;
  write_escaped(writer, prefix)?;
  write_escaped(writer, name)?;
  writer.write_all(b"\":")?;
  Ok(())
}

fn write_array<'reader, W: std::io::Write, I>(
  codec: &super::Codec,
  writer: &mut W,
  items: I,
  meta: &EncodingOptions,
) -> capnp::Result<()>
where
  I: Iterator<Item = capnp::Result<capnp::dynamic_value::Reader<'reader>>>,
{
  writer.write_all(b"[")?;
  let mut first = true;
  for item in items {
    if !first {
      writer.write_all(b",")?;
    }
    first = false;
    serialize_value_to(codec, writer, item?, meta, &mut true)?;
  }
  writer.write_all(b"]")?;
  Ok(())
}

fn write_object<'reader, W: std::io::Write>(
  codec: &super::Codec,
  writer: &mut W,
  reader: capnp::dynamic_struct::Reader<'reader>,
  meta: &EncodingOptions<'_, '_>,
  first: &mut bool,
) -> capnp::Result<()> {
  let (flatten, field_prefix) = if let Some(flatten_options) = &meta.flatten {
    (
      true,
      std::borrow::Cow::Owned(format!(
        "{}{}",
        meta.prefix,
        flatten_options.get_prefix()?.to_str()?
      )),
    )
  } else {
    (false, std::borrow::Cow::Borrowed(""))
  };

  let mut my_first = true;

  let first = if !flatten {
    writer.write_all(b"{")?;
    &mut my_first
  } else {
    first
  };

  for field in reader.get_schema().get_non_union_fields()? {
    if !reader.has(field)? {
      continue;
    }
    let field_meta = EncodingOptions::from_field(&field_prefix, field)?;
    if field_meta.flatten.is_none() {
      if !*first {
        writer.write_all(b",")?;
      }
      *first = false;
      write_key(writer, &field_prefix, field_meta.name)?;
    }
    let field_value = reader.get(field)?;
    serialize_value_to(codec, writer, field_value, &field_meta, first)?;
  }

  // Comment copied verbatim from the Cap'n Proto C++ implementation:
  // There are two cases of unions:
  // * Named unions, which are special cases of named groups. In this case, the union may be
  //   annotated by annotating the field. In this case, we receive a non-null `discriminator`
  //   as a constructor parameter, and schemaProto.getAnnotations() must be empty because
  //   it's not possible to annotate a group's type (because the type is anonymous).
  // * Unnamed unions, of which there can only be one in any particular scope. In this case,
  //   the parent struct type itself is annotated.
  // So if we received `null` as the constructor parameter, check for annotations on the struct
  // type.
  let struct_discriminator = reader
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

  if let Some(active_union_member) = reader.which()? {
    let active_union_member_meta =
      EncodingOptions::from_field(&field_prefix, active_union_member)?;
    if reader.has(active_union_member)? {
      let mut value_name = active_union_member_meta.name;
      let mut suppress_void = false;
      if let Some(discriminator) = discriminator {
        let discriminator_name = if discriminator.has_name() {
          Some(discriminator.get_name()?.to_str()?)
        } else if flatten {
          Some(meta.name)
        } else {
          // https://github.com/capnproto/capnproto/issues/2461
          // The discriminator is not output even if the annoyation is
          // present if:
          //  - it doesn't have an explicit name, and
          //  - the group is _not_ being flattened.
          None
        };
        if discriminator.has_value_name() {
          value_name = discriminator.get_value_name()?.to_str()?;
        }

        if let Some(discriminator_name) = discriminator_name {
          if !*first {
            writer.write_all(b",")?;
          }
          *first = false;
          suppress_void = true;
          write_key(writer, &field_prefix, discriminator_name)?;
          write_string(writer, active_union_member_meta.name)?;
        }
      }
      let field_value = reader.get(active_union_member)?;
      if !suppress_void
        || !matches!(field_value, capnp::dynamic_value::Reader::Void)
      {
        if active_union_member_meta.flatten.is_none() {
          if !*first {
            writer.write_all(b",")?;
          }
          *first = false;
          write_key(writer, &field_prefix, value_name)?;
        }
        serialize_value_to(
          codec,
          writer,
          field_value,
          &active_union_member_meta,
          first,
        )?;
      }
    }
  }
  if !flatten {
    writer.write_all(b"}")?;
  }
  Ok(())
}

fn write_data<W: std::io::Write>(
  writer: &mut W,
  data: capnp::data::Reader<'_>,
  encoding: DataEncoding,
) -> capnp::Result<()> {
  match encoding {
    DataEncoding::Default => {
      writer.write_all(b"[")?;
      let mut first = true;
      for byte in data.iter() {
        if !first {
          writer.write_all(b",")?;
        }
        first = false;
        write_int(writer, *byte)?;
      }
      writer.write_all(b"]")?;
      Ok(())
    }
    // Base64 and hex output is ASCII by construction, so it needs no escaping
    // and can be written straight into the writer.
    DataEncoding::Base64 => {
      writer.write_all(b"\"")?;
      base64::encode_to(writer, data)?;
      writer.write_all(b"\"").map_err(Into::into)
    }
    DataEncoding::Hex => {
      writer.write_all(b"\"")?;
      hex::encode_to(writer, data)?;
      writer.write_all(b"\"").map_err(Into::into)
    }
  }
}
