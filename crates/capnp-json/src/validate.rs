//! Schema validation, exposed as [`crate::validate_schema`].
//!
//! Currently this detects cyclic JSON flattening, matching the check the C++
//! codec performs when it builds its annotated handlers. It is not run by
//! encoding or decoding — see [`crate::validate_schema`] for why.

use std::collections::HashSet;

use super::json_capnp;

/// Reject schemas in which flattening does not terminate.
///
/// `$Json.flatten` splices a struct's members into its parent's JSON object
/// rather than nesting them, so a flattened field consumes no JSON nesting. A
/// cycle of such fields therefore describes an object of infinite width, and
/// walking it does not terminate. The C++ codec detects this when it loads the
/// handlers for a schema and fails with "cyclic JSON flattening detected"; we
/// do the same check here so that both implementations reject the same
/// schemas.
///
/// Edges are exactly the ones C++ follows when building handlers:
///
/// - a struct to each of its *group* members, whether or not the group is
///   flattened (a group's handler is always loaded so that its discriminator
///   can be passed down);
/// - a struct to the type of each struct-typed field annotated
///   `$Json.flatten`.
///
/// A plain struct-typed field without `$Json.flatten` is not an edge: it nests
/// in the JSON, so it terminates.
///
/// The check covers every struct reachable from `root` through any field —
/// including through list element types — not just those reachable by
/// flattening, again matching C++, which loads handlers for a schema's whole
/// dependency graph.
///
/// Cycles are keyed by schema node ID rather than by branded schema. For
/// generic types that is equivalent: the brand cannot add or remove flatten
/// edges, and a self-reference through flattening fails to terminate whatever
/// the type arguments are.
///
/// This walks the schema graph on every call and keeps no state of its own.
/// Callers are expected to invoke it once per root type rather than per
/// message; see [`crate::validate_schema`].
pub(crate) fn check_flattening_terminates(
  root: capnp::schema::StructSchema,
) -> capnp::Result<()> {
  // Structs whose flatten graph has been fully explored and found acyclic.
  let mut acyclic = HashSet::new();
  // Structs already enqueued by the reachability walk.
  let mut reached = HashSet::new();
  let mut worklist = vec![root];

  while let Some(schema) = worklist.pop() {
    if !reached.insert(schema.get_proto().get_id()) {
      continue;
    }

    let mut path = Vec::new();
    check_acyclic(schema, &mut acyclic, &mut path)?;

    for field in schema.get_fields()? {
      collect_struct_types(field.get_type(), &mut worklist);
    }
  }

  Ok(())
}

/// Depth-first search over flatten edges, colouring nodes as it goes: a node
/// on `path` is being explored, and a node in `acyclic` has been cleared.
/// Meeting a node that is already on `path` closes a cycle.
fn check_acyclic(
  schema: capnp::schema::StructSchema,
  acyclic: &mut HashSet<u64>,
  path: &mut Vec<u64>,
) -> capnp::Result<()> {
  let id = schema.get_proto().get_id();
  if acyclic.contains(&id) {
    return Ok(());
  }
  if path.contains(&id) {
    return Err(capnp::Error::failed(format!(
      "cyclic JSON flattening detected in {}",
      schema.get_proto().get_display_name()?.to_str()?
    )));
  }

  path.push(id);
  for field in schema.get_fields()? {
    if !is_flatten_edge(field)? {
      continue;
    }
    if let capnp::introspect::TypeVariant::Struct(raw) =
      field.get_type().which()
    {
      check_acyclic(capnp::schema::StructSchema::new(raw), acyclic, path)?;
    }
  }
  path.pop();

  acyclic.insert(id);
  Ok(())
}

/// Whether descending into `field` continues the enclosing JSON object rather
/// than starting a nested one.
fn is_flatten_edge(field: capnp::schema::Field) -> capnp::Result<bool> {
  // A group is always an edge: it shares its parent's JSON object unless it
  // is explicitly named, and C++ loads its handler either way.
  if matches!(
    field.get_proto().which()?,
    capnp::schema_capnp::field::Group(_)
  ) {
    return Ok(true);
  }

  Ok(
    field
      .get_annotations()?
      .iter()
      .any(|anno| anno.get_id() == json_capnp::flatten::ID),
  )
}

/// Add every struct type named by `typ` to `worklist`, looking through lists.
fn collect_struct_types(
  typ: capnp::introspect::Type,
  worklist: &mut Vec<capnp::schema::StructSchema>,
) {
  let mut typ = typ;
  loop {
    match typ.which() {
      capnp::introspect::TypeVariant::Struct(raw) => {
        worklist.push(capnp::schema::StructSchema::new(raw));
        return;
      }
      capnp::introspect::TypeVariant::List(element) => typ = element,
      _ => return,
    }
  }
}
