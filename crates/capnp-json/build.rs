fn main() {
  println!("cargo:rerun-if-changed=json.capnp");
  let root_dir =
    std::env::var("CARGO_MANIFEST_DIR").expect("getting manifest dir");

  // Compile the built-in json schema. `src_prefix` strips the manifest dir
  // from the generated module names.
  capnpc::CompilerCommand::new()
    .file(root_dir.clone() + "/json.capnp")
    .src_prefix(root_dir)
    .run()
    .expect("compiling json.capnp");
}
