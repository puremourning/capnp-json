# capnp-json

A [Cap'n Proto](https://capnproto.org) JSON codec for [capnp-rust](https://github.com/capnproto/capnproto-rust),
implementing the codec defined in
[`json.capnp`](https://github.com/capnproto/capnproto/blob/master/c%2B%2B/src/capnp/compat/json.capnp).

It encodes a Cap'n Proto message to JSON, and decodes JSON into a Cap'n Proto
message, using the schema's runtime type information. The wire format is
compatible with the [C++ JSON codec](https://github.com/capnproto/capnproto/blob/master/c%2B%2B/src/capnp/compat/json.h)
that ships with Cap'n Proto.

## Usage

Add the dependency:

```toml
[dependencies]
capnp = "0.27"
capnp-json = "0.2.0"
```

Encoding a message reader to a JSON string, and decoding JSON back into a
message builder:

```rust
use capnp::message;
use capnp_json::{from_json, to_json};

# mod my_schema_capnp { capnp::generated_code!(pub mod my_schema_capnp); }
# fn run() -> capnp::Result<()> {
let mut builder = message::Builder::new_default();
let root: my_schema_capnp::my_struct::Builder<'_> = builder.init_root();
// ... populate `root` ...

let json: String = to_json(root.reborrow_as_reader())?;

let mut decoded = message::Builder::new_default();
let decoded_root: my_schema_capnp::my_struct::Builder<'_> = decoded.init_root();
from_json(&json, decoded_root)?;
# Ok(()) }
```

If your schema uses any of the JSON annotations (`$Json.name`, `$Json.flatten`,
`$Json.discriminator`, `$Json.base64`, `$Json.hex`), import them by adding the
following to your `build.rs`, so that the generated code links against the
annotations defined in this crate:

```rust
fn main() {
    capnpc::CompilerCommand::new()
        .crate_provides("capnp_json", [0x8ef99297a43a5e34])
        .file("my_schema.capnp")
        .run()
        .expect("compiling schema");
}
```

And in your schema:

```capnp
using Json = import "/capnp/compat/json.capnp";

struct MyStruct {
    myField @0 :Text $Json.name("my_field");
}
```

## Supported features

- All primitive Cap'n Proto types, including `Int64` / `UInt64` encoded as
  JSON strings (matching the C++ codec). Integer fields and `Data` bytes are
  range-checked on decode, so an out-of-range or fractional number is an error
  rather than being silently clamped; floats and enum ordinals are not
  checked, matching C++.
- `Float32` / `Float64` `NaN`, `Infinity`, and `-Infinity` encoded as JSON
  strings.
- Structs, lists, lists of lists, and lists of structs.
- Enums, encoded by name (or by ordinal if the enumerant is missing).
- Named and unnamed unions.
- Annotations:
  - `$Json.name` &mdash; rename a field, enumerant, method, group, or union
    member in the JSON representation.
  - `$Json.flatten` &mdash; flatten a struct, group, or union into its parent.
    Flattening must terminate: a schema that flattens a field of its own type,
    directly or through a chain, cannot be represented as JSON. Call
    `capnp_json::validate_schema::<MyStruct::Owned>()` once (from a test, or at
    startup) to get the same "cyclic JSON flattening detected" verdict the C++
    codec gives. Encoding and decoding skip the check, since a schema is
    compile-time data; a cyclic schema is still rejected when decoded, just via
    the recursion limit.
  - `$Json.discriminator` &mdash; encode a union's variant as a sibling
    discriminator field.
  - `$Json.base64` / `$Json.hex` &mdash; encode `Data` fields as Base64 or
    hex strings instead of arrays of bytes.
- Custom per-field and per-type encodings, via the `FieldCodec` trait and
  `Codec::with_field_override` / `with_type_override`. This is the equivalent
  of the C++ codec's `Handler` API, and is what makes `AnyPointer` and
  interface fields encodable.
- Named codecs selected from the schema with this crate's own
  `$Rust.codec("name")` annotation (see `rust-json.capnp`), registered with
  `Codec::with_named_codec`.

## Not yet supported

- The `Value` / `Call` / `raw` extensions from `json.capnp`.
- `AnyPointer` and interface fields, unless a `FieldCodec` is registered for
  them.
- Pretty-printed output.

## Known divergences from the C++ codec

These matter mainly when decoding input you do not control; see the crate
documentation for the full list.

- Input after the top-level value is ignored rather than rejected.
- `\uXXXX` surrogate pairs are combined into the character they denote, and
  unpaired surrogates are rejected. C++ decodes each escape separately and
  produces WTF-8, which is not valid UTF-8. Round-tripping is unaffected: the
  C++ encoder writes non-BMP characters as literal UTF-8, never as escapes.
- Only `Int64` / `UInt64` accept the string form of an integer when decoding;
  C++ accepts it for every integer width.

Note that a JSON `null` for a pointer-typed field is accepted as "field
absent", matching the C++ *main branch* (`isPointerToJsonNull`). That is not in
any C++ release yet, so released versions reject it — this crate is the more
permissive of the two.

## License

[MIT](LICENSE)
