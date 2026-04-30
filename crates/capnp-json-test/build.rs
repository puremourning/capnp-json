fn main() {
  // 0x8ef99297a43a5e34 is the file id of json.capnp.
  capnpc::CompilerCommand::new()
    .crate_provides("capnp_json", [0x8ef99297a43a5e34])
    .file("test.capnp")
    .file("json-test.capnp")
    .file("test-compat.capnp")
    .run()
    .expect("compiling schema");
}
