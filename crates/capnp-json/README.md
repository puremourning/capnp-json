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
capnp = "0.25"
capnp-json = "0.1"
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
  JSON strings (matching the C++ codec).
- `Float32` / `Float64` `NaN`, `Infinity`, and `-Infinity` encoded as JSON
  strings.
- Structs, lists, lists of lists, and lists of structs.
- Enums, encoded by name (or by ordinal if the enumerant is missing).
- Named and unnamed unions.
- Annotations:
  - `$Json.name` &mdash; rename a field, enumerant, method, group, or union
    member in the JSON representation.
  - `$Json.flatten` &mdash; flatten a struct, group, or union into its parent.
  - `$Json.discriminator` &mdash; encode a union's variant as a sibling
    discriminator field.
  - `$Json.base64` / `$Json.hex` &mdash; encode `Data` fields as Base64 or
    hex strings instead of arrays of bytes.

## Not yet supported

- The `Value` / `Call` / `raw` extensions from `json.capnp`.
- `AnyPointer` and `Capability` fields (these are returned as errors).
- Custom encoder/decoder handlers for specific types &mdash; the C++ codec
  exposes a `Handler` API; this crate does not yet.
- Pretty-printed output.

## License

[MIT](LICENSE)
