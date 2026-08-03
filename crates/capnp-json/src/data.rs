// We don't want to pull in base64 crate just for this. So hand-rolling a
// base64 codec.
pub(crate) mod base64 {
  const BASE64_CHARS: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

  /// Encode straight into a writer. Base64 output is pure ASCII, so callers
  /// writing it into a JSON string can emit it without an escaping pass and
  /// without an intermediate `String`.
  pub(crate) fn encode_to<W: std::io::Write>(
    writer: &mut W,
    data: &[u8],
  ) -> capnp::Result<()> {
    // 3 input bytes -> 4 output bytes; a fixed buffer keeps this to one write
    // per 48 input bytes rather than one per 3.
    let mut out = [0u8; 64];
    let mut len = 0;
    for chunk in data.chunks(3) {
      #[allow(clippy::get_first)]
      let b0 = chunk.get(0).copied().unwrap_or(0);
      let b1 = chunk.get(1).copied().unwrap_or(0);
      let b2 = chunk.get(2).copied().unwrap_or(0);
      let n = ((b0 as u32) << 16) | ((b1 as u32) << 8) | (b2 as u32);
      out[len] = BASE64_CHARS[((n >> 18) & 0x3F) as usize];
      out[len + 1] = BASE64_CHARS[((n >> 12) & 0x3F) as usize];
      out[len + 2] = if chunk.len() > 1 {
        BASE64_CHARS[((n >> 6) & 0x3F) as usize]
      } else {
        b'='
      };
      out[len + 3] = if chunk.len() > 2 {
        BASE64_CHARS[(n & 0x3F) as usize]
      } else {
        b'='
      };
      len += 4;
      if len == out.len() {
        writer.write_all(&out)?;
        len = 0;
      }
    }
    writer.write_all(&out[..len])?;
    Ok(())
  }

  pub(crate) fn decode(data: &str) -> capnp::Result<Vec<u8>> {
    let bytes = data.as_bytes();
    if bytes.len() % 4 != 0 {
      return Err(capnp::Error::failed(
        "Base64 string length must be a multiple of 4".into(),
      ));
    }
    let mut decoded = Vec::with_capacity(bytes.len() / 4 * 3);
    for chunk in bytes.chunks(4) {
      let mut n: u32 = 0;
      let mut padding = 0;
      for &c in chunk {
        n <<= 6;
        match c {
          b'A'..=b'Z' => n |= (c - b'A') as u32,
          b'a'..=b'z' => n |= (c - b'a' + 26) as u32,
          b'0'..=b'9' => n |= (c - b'0' + 52) as u32,
          b'+' => n |= 62,
          b'/' => n |= 63,
          b'=' => {
            n |= 0;
            padding += 1;
          }
          _ => {
            return Err(capnp::Error::failed(format!(
              "Invalid base64 character: {}",
              c as char
            )));
          }
        }
      }
      decoded.push(((n >> 16) & 0xFF) as u8);
      if padding < 2 {
        decoded.push(((n >> 8) & 0xFF) as u8);
      }
      if padding < 1 {
        decoded.push((n & 0xFF) as u8);
      }
    }
    Ok(decoded)
  }
}

// We don't want to pull in hex crate just for this. So hand-rolling a
// hex codec.
pub(crate) mod hex {
  const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";
  fn hex_char_to_value(c: u8) -> capnp::Result<u8> {
    match c {
      b'0'..=b'9' => Ok(c - b'0'),
      b'a'..=b'f' => Ok(c - b'a' + 10),
      b'A'..=b'F' => Ok(c - b'A' + 10),
      _ => Err(capnp::Error::failed(format!(
        "Invalid hex character: {}",
        c as char
      ))),
    }
  }

  /// Encode straight into a writer. Hex output is pure ASCII, so it needs no
  /// escaping and no intermediate `String`.
  pub(crate) fn encode_to<W: std::io::Write>(
    writer: &mut W,
    data: &[u8],
  ) -> capnp::Result<()> {
    let mut out = [0u8; 64];
    let mut len = 0;
    for &byte in data {
      out[len] = HEX_CHARS[(byte >> 4) as usize];
      out[len + 1] = HEX_CHARS[(byte & 0x0F) as usize];
      len += 2;
      if len == out.len() {
        writer.write_all(&out)?;
        len = 0;
      }
    }
    writer.write_all(&out[..len])?;
    Ok(())
  }

  pub(crate) fn decode(data: &str) -> capnp::Result<Vec<u8>> {
    if data.len() % 2 != 0 {
      return Err(capnp::Error::failed(
        "Hex string must have even length".into(),
      ));
    }
    let mut decoded = Vec::with_capacity(data.len() / 2);
    let bytes = data.as_bytes();
    for i in (0..data.len()).step_by(2) {
      let high = hex_char_to_value(bytes[i])?;
      let low = hex_char_to_value(bytes[i + 1])?;
      decoded.push((high << 4) | low);
    }
    Ok(decoded)
  }
}
