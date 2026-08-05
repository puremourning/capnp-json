// Copyright (c) 2025 Ben Jackson [puremourning@gmail.com] and Cap'n Proto contributors
// Licensed under the MIT License.

//! Encode/decode benchmarks for `capnp-json`.
//!
//! Three payload shapes, each exercising a different part of the codec:
//!
//! - **text** — a long list of short strings. Dominated by string escaping on
//!   the way out and the character-at-a-time parser on the way in.
//! - **scalars** — every primitive type, a nested struct and some lists.
//!   Dominated by per-field work: name construction, annotation lookup and the
//!   value conversions.
//! - **flattened** — groups, prefixed groups, flattened unions and
//!   discriminators. This is the shape that exercises struct creation during
//!   decode, so it is the one to watch when that changes.
//!
//! `small` uses the least interesting possible message, where per-call
//! overhead dominates and the throughput figures would hide it.
//! `decode_merge` decodes twice into one builder, which is where creating (or
//! clearing) a flattened field unconditionally shows up.
//!
//! The decode benchmarks allocate a fresh `message::Builder` inside the timed
//! loop, because a decode needs somewhere to put its output; that allocation
//! is part of what is being measured.
//!
//! Run with `cargo bench -p capnp-json-test`, or one at a time with
//! `cargo bench -p capnp-json-test -- decode/flattened`.

use std::hint::black_box;

use capnp::message;
use capnp_json as json;
use capnp_json_test::json_test_capnp::{
  test_json_annotations,
  TestJsonAnnotatedEnum,
};
use capnp_json_test::test_capnp::{test_json_types, TestEnum};
use criterion::{criterion_group, criterion_main, Criterion, Throughput};

type Msg = message::Builder<message::HeapAllocator>;

/// A long list of short strings.
fn build_text(builder: &mut Msg) {
  let mut root: test_json_types::Builder<'_> = builder.init_root();
  let mut list = root.reborrow().init_text_list(2000);
  for i in 0..2000 {
    list.set(i, "the quick brown fox jumps over the lazy dog 0123456789");
  }
}

/// Every primitive type, a nested struct, and a few lists.
fn build_scalars(builder: &mut Msg) {
  let mut root: test_json_types::Builder<'_> = builder.init_root();
  root.set_void_field(());
  root.set_bool_field(true);
  root.set_int8_field(-8);
  root.set_int16_field(-16);
  root.set_int32_field(-32);
  root.set_int64_field(-64);
  root.set_u_int8_field(8);
  root.set_u_int16_field(16);
  root.set_u_int32_field(32);
  root.set_u_int64_field(64);
  root.set_float32_field(1.5);
  root.set_float64_field(2.5);
  root.set_text_field("hello, world");
  root.set_data_field(&[0xde, 0xad, 0xbe, 0xef]);
  root.set_base64_field(&[0xde, 0xad, 0xbe, 0xef]);
  root.set_hex_field(&[0xde, 0xad, 0xbe, 0xef]);
  root.set_enum_field(TestEnum::Quux);
  {
    let mut inner = root.reborrow().init_struct_field();
    inner.set_text_field("nested");
    inner.set_int32_field(1234);
    inner.set_float64_field(0.5);
  }
  {
    let mut ints = root.reborrow().init_int32_list(64);
    for i in 0..64 {
      ints.set(i, i as i32);
    }
  }
  {
    let mut enums = root.reborrow().init_enum_list(8);
    for i in 0..8 {
      enums.set(i, TestEnum::Bar);
    }
  }
}

/// Groups, prefixed groups, flattened unions and discriminators.
fn build_flattened(builder: &mut Msg) {
  let mut root: test_json_annotations::Builder<'_> = builder.init_root();
  root.set_some_field("Some Field");
  {
    let mut a_group = root.reborrow().init_a_group();
    a_group.set_flat_foo(0xF00);
    a_group.set_flat_bar("0xBaa");
    a_group.reborrow().init_flat_baz().set_hello(true);
    a_group.reborrow().init_double_flat().set_flat_qux("Qux");
  }
  {
    let mut prefixed = root.reborrow().init_prefixed_group();
    prefixed.set_foo("Foo");
    prefixed.set_bar(0xBAA);
    prefixed.reborrow().init_baz().set_hello(false);
    prefixed.reborrow().init_more_prefix().set_qux("Qux");
  }
  {
    let mut bar = root.reborrow().init_a_union().init_bar();
    bar.set_bar_member(0xAAB);
    bar.set_multi_member("Member");
  }
  root.reborrow().init_dependency().set_foo("dep-foo");
  root.reborrow().init_simple_group().set_grault("grault");
  {
    let mut enums = root.reborrow().init_enums(4);
    enums.set(0, TestJsonAnnotatedEnum::Foo);
    enums.set(1, TestJsonAnnotatedEnum::Bar);
    enums.set(2, TestJsonAnnotatedEnum::Baz);
    enums.set(3, TestJsonAnnotatedEnum::Qux);
  }
  root.reborrow().init_b_union().set_bar(100);
  root
    .reborrow()
    .init_external_union()
    .init_bar()
    .set_value("Value");
  root.reborrow().init_union_with_void().set_void_value(());
}

/// Register one shape's encode benchmark. `$reader` names the root type, which
/// differs per schema, so this stays a macro rather than a generic function.
macro_rules! bench_encode_shape {
  ($group:expr, $name:literal, $reader:ty, $build:ident) => {{
    let mut msg = message::Builder::new_default();
    $build(&mut msg);
    let encoded =
      json::to_json(msg.get_root_as_reader::<$reader>().unwrap()).unwrap();

    $group.throughput(Throughput::Bytes(encoded.len() as u64));
    $group.bench_function($name, |b| {
      b.iter(|| {
        let root = msg.get_root_as_reader::<$reader>().unwrap();
        black_box(json::to_json(root).unwrap())
      })
    });
  }};
}

/// Register one shape's decode benchmark.
macro_rules! bench_decode_shape {
  ($group:expr, $name:literal, $reader:ty, $builder:ty, $build:ident) => {{
    let mut msg = message::Builder::new_default();
    $build(&mut msg);
    let encoded =
      json::to_json(msg.get_root_as_reader::<$reader>().unwrap()).unwrap();

    $group.throughput(Throughput::Bytes(encoded.len() as u64));
    $group.bench_function($name, |b| {
      b.iter(|| {
        let mut out = message::Builder::new_default();
        let root: $builder = out.init_root();
        json::from_json(&encoded, root).unwrap();
        black_box(())
      })
    });
  }};
}

fn bench_encode(c: &mut Criterion) {
  let mut group = c.benchmark_group("encode");
  bench_encode_shape!(group, "text", test_json_types::Reader<'_>, build_text);
  bench_encode_shape!(
    group,
    "scalars",
    test_json_types::Reader<'_>,
    build_scalars
  );
  bench_encode_shape!(
    group,
    "flattened",
    test_json_annotations::Reader<'_>,
    build_flattened
  );
  group.finish();
}

fn bench_decode(c: &mut Criterion) {
  let mut group = c.benchmark_group("decode");
  bench_decode_shape!(
    group,
    "text",
    test_json_types::Reader<'_>,
    test_json_types::Builder<'_>,
    build_text
  );
  bench_decode_shape!(
    group,
    "scalars",
    test_json_types::Reader<'_>,
    test_json_types::Builder<'_>,
    build_scalars
  );
  bench_decode_shape!(
    group,
    "flattened",
    test_json_annotations::Reader<'_>,
    test_json_annotations::Builder<'_>,
    build_flattened
  );
  group.finish();
}

/// The smallest interesting message, where per-call overhead dominates.
fn bench_small(c: &mut Criterion) {
  let mut group = c.benchmark_group("small");

  let mut msg = message::Builder::new_default();
  {
    let mut root: test_json_types::Builder<'_> = msg.init_root();
    root.set_int8_field(-8);
    root.set_text_field("hello");
  }
  let encoded = json::to_json(
    msg
      .get_root_as_reader::<test_json_types::Reader<'_>>()
      .unwrap(),
  )
  .unwrap();

  group.bench_function("encode", |b| {
    b.iter(|| {
      let root = msg
        .get_root_as_reader::<test_json_types::Reader<'_>>()
        .unwrap();
      black_box(json::to_json(root).unwrap())
    })
  });

  group.bench_function("decode", |b| {
    b.iter(|| {
      let mut out = message::Builder::new_default();
      let root: test_json_types::Builder<'_> = out.init_root();
      json::from_json(&encoded, root).unwrap();
      black_box(())
    })
  });

  // Decoding `{}` into a 36-field struct: no values at all, so what is left
  // is the walk over the schema's fields. Isolates per-schema-field cost from
  // per-value cost, which matters because decoding visits every field of the
  // schema whether or not the JSON mentions it.
  group.bench_function("decode_empty", |b| {
    b.iter(|| {
      let mut out = message::Builder::new_default();
      let root: test_json_types::Builder<'_> = out.init_root();
      json::from_json("{}", root).unwrap();
      black_box(())
    })
  });

  group.finish();
}

/// Decoding twice into the same builder. Flattened fields that get created or
/// cleared unconditionally show up here.
fn bench_decode_merge(c: &mut Criterion) {
  let mut msg = message::Builder::new_default();
  build_flattened(&mut msg);
  let encoded = json::to_json(
    msg
      .get_root_as_reader::<test_json_annotations::Reader<'_>>()
      .unwrap(),
  )
  .unwrap();

  c.bench_function("decode_merge/flattened", |b| {
    b.iter(|| {
      let mut out = message::Builder::new_default();
      {
        let root: test_json_annotations::Builder<'_> = out.init_root();
        json::from_json(&encoded, root).unwrap();
      }
      {
        let root: test_json_annotations::Builder<'_> = out.get_root().unwrap();
        json::from_json(&encoded, root).unwrap();
      }
      black_box(())
    })
  });
}

criterion_group!(
  benches,
  bench_encode,
  bench_decode,
  bench_small,
  bench_decode_merge
);
criterion_main!(benches);
