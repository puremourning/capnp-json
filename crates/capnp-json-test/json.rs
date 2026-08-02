// Copyright (c) 2025 Ben Jackson [puremourning@gmail.com] and Cap'n Proto contributors
// Licensed under the MIT License:
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.

#[cfg(test)]
capnp::generated_code!(pub mod test_capnp);
#[cfg(test)]
capnp::generated_code!(pub mod json_test_capnp);
#[cfg(test)]
capnp::generated_code!(pub mod test_compat_capnp);

mod cppcompat;

#[cfg(test)]
mod tests {
  use capnp::message;
  use capnp_json as json;
  use json::JsonValue;

  use crate::json_test_capnp::test_json_annotations;
  use crate::test_capnp::{
    test_json_flatten_union,
    test_json_types,
    test_union,
    test_unnamed_union,
    TestEnum,
  };

  #[test]
  fn test_encode_json_types_default() {
    let mut builder = message::Builder::new_default();
    let root: test_json_types::Builder<'_> = builder.init_root();
    let expected = r#"{"voidField":null,"boolField":false,"int8Field":0,"int16Field":0,"int32Field":0,"int64Field":"0","uInt8Field":0,"uInt16Field":0,"uInt32Field":0,"uInt64Field":"0","float32Field":0,"float64Field":0,"enumField":"foo"}"#;
    assert_eq!(expected, json::to_json(root.reborrow_as_reader()).unwrap());
  }

  #[test]
  fn test_encode_all_json_types() {
    let mut builder = message::Builder::new_default();
    let mut root: test_json_types::Builder<'_> = builder.init_root();
    root.set_int8_field(-8);
    root.set_int16_field(-16);
    root.set_int32_field(-32);
    root.set_int64_field(-64);
    root.set_u_int8_field(8);
    root.set_u_int16_field(16);
    root.set_u_int32_field(32);
    root.set_u_int64_field(64);
    root.set_bool_field(true);
    root.set_void_field(());
    root.set_text_field("hello");
    root.set_float32_field(1.32);
    root.set_float64_field(1.64);
    root.set_data_field(&[0xde, 0xad, 0xbe, 0xef, 0xca, 0xfe, 0xba, 0xbe]);
    root.set_base64_field(&[0xde, 0xad, 0xbe, 0xef, 0xca, 0xfe, 0xba, 0xbe]);
    root.set_hex_field(&[0xde, 0xad, 0xbe, 0xef, 0xca, 0xfe, 0xba, 0xbe]);
    {
      let mut embedded = root.reborrow().init_struct_field();
      let mut text_list = embedded.reborrow().init_text_list(2);
      text_list.set(0, "frist");
      text_list.set(1, "segund");
      embedded.set_text_field("inner");
      let mut hex_list = embedded.reborrow().init_hex_list(2);
      hex_list.set(0, &[0xde, 0xad, 0xbe, 0xef]);
      hex_list.set(1, &[0xba, 0xdf, 0x00, 0xd0]);
      let mut based_list = embedded.reborrow().init_base64_list(2);
      based_list.set(0, &[0xde, 0xad, 0xbe, 0xef]);
      based_list.set(1, &[0xba, 0xdf, 0x00, 0xd0]);
    }
    root.set_enum_field(TestEnum::Quux);
    {
      let mut enum_list = root.reborrow().init_enum_list(3);
      enum_list.set(0, TestEnum::Foo);
      enum_list.set(1, TestEnum::Bar);
      enum_list.set(2, TestEnum::Garply);
    }
    {
      let mut floats = root.reborrow().init_float32_list(3);
      floats.set(0, f32::NAN);
      floats.set(1, f32::INFINITY);
      floats.set(2, f32::NEG_INFINITY);
    }
    {
      let mut floats = root.reborrow().init_float64_list(3);
      floats.set(0, f64::NAN);
      floats.set(1, f64::INFINITY);
      floats.set(2, f64::NEG_INFINITY);
    }

    let expected = concat!(
      "{",
      r#""voidField":null,"#,
      r#""boolField":true,"#,
      r#""int8Field":-8,"#,
      r#""int16Field":-16,"#,
      r#""int32Field":-32,"#,
      r#""int64Field":"-64","#,
      r#""uInt8Field":8,"#,
      r#""uInt16Field":16,"#,
      r#""uInt32Field":32,"#,
      r#""uInt64Field":"64","#,
      r#""float32Field":1.3200000524520874,"#,
      r#""float64Field":1.64,"#,
      r#""textField":"hello","#,
      r#""dataField":[222,173,190,239,202,254,186,190],"#,
      r#""base64Field":"3q2+78r+ur4=","#,
      r#""hexField":"deadbeefcafebabe","#,
      r#""structField":{"#,
      r#""voidField":null,"#,
      r#""boolField":false,"#,
      r#""int8Field":0,"#,
      r#""int16Field":0,"#,
      r#""int32Field":0,"#,
      r#""int64Field":"0","#,
      r#""uInt8Field":0,"#,
      r#""uInt16Field":0,"#,
      r#""uInt32Field":0,"#,
      r#""uInt64Field":"0","#,
      r#""float32Field":0,"#,
      r#""float64Field":0,"#,
      r#""textField":"inner","#,
      r#""enumField":"foo","#,
      r#""textList":["frist","segund"],"#,
      r#""base64List":["3q2+7w==","ut8A0A=="],"#,
      r#""hexList":["deadbeef","badf00d0"]"#,
      "},",
      r#""enumField":"quux","#,
      r#""float32List":["NaN","Infinity","-Infinity"],"#,
      r#""float64List":["NaN","Infinity","-Infinity"],"#,
      r#""enumList":["foo","bar","garply"]"#,
      "}"
    );
    assert_eq!(expected, json::to_json(root.reborrow_as_reader()).unwrap());
  }

  // Union encoding with flattening

  #[test]
  fn test_named_union_non_flattened() {
    let mut builder = message::Builder::new_default();
    let mut root: test_union::Builder<'_> = builder.init_root();
    root.set_bit0(true);
    root.set_bit2(false);
    root.set_bit3(true);
    root.set_bit4(false);
    root.set_bit5(true);
    root.set_bit6(false);
    root.set_bit7(true);
    root.set_byte0(0xAA);
    let mut union0 = root.reborrow().init_union0();
    union0.set_u0f0sp("not this one");
    union0.set_u0f0s16(-12345);

    let expected = concat!(
      "{",
      r#""union0":{"u0f0s16":-12345},"#,
      r#""union1":{"u1f0s0":null},"#,
      r#""union2":{"u2f0s1":false},"#,
      r#""union3":{"u3f0s1":false},"#,
      r#""bit0":true,"#,
      r#""bit2":false,"#,
      r#""bit3":true,"#,
      r#""bit4":false,"#,
      r#""bit5":true,"#,
      r#""bit6":false,"#,
      r#""bit7":true,"#,
      r#""byte0":170"#,
      "}",
    );

    assert_eq!(expected, json::to_json(root.reborrow_as_reader()).unwrap());
  }

  #[test]
  fn test_unnamed_union() {
    let mut builder = message::Builder::new_default();
    let mut root: test_unnamed_union::Builder<'_> = builder.init_root();
    root.set_before("before");
    root.set_middle(1234);
    root.set_after("after");
    root.set_foo(16);
    root.set_bar(32);
    let expected = concat!(
      "{",
      r#""before":"before","#,
      r#""middle":1234,"#,
      r#""after":"after","#,
      r#""bar":32"#,
      "}",
    );
    assert_eq!(expected, json::to_json(root.reborrow_as_reader()).unwrap());
  }

  #[test]
  fn test_named_union_flattened() {
    let mut builder = message::Builder::new_default();
    let mut root: test_json_flatten_union::Builder<'_> = builder.init_root();
    root.set_before("before");
    root.set_middle(1234);
    root.set_after("after");
    let mut maybe = root.reborrow().init_maybe();
    maybe.set_foo(16);
    maybe.set_bar(32);

    let expected = concat!(
      "{",
      r#""before":"before","#,
      r#""maybe_bar":32,"#,
      r#""middle":1234,"#,
      r#""after":"after","#,
      r#""foo":0,"#,
      r#""bar":0,"#,
      r#""nested_baz":0,"#,
      r#""baz":0"#,
      "}",
    );
    assert_eq!(expected, json::to_json(root.reborrow_as_reader()).unwrap());
  }

  #[test]
  fn test_discriminated_union() {
    let mut builder = message::Builder::new_default();
    let mut root: test_json_annotations::Builder<'_> = builder.init_root();

    let mut expected = String::from("{");

    root.set_some_field("Some Field");
    expected.push_str(r#""names-can_contain!anything Really":"Some Field","#);

    {
      let mut a_group = root.reborrow().init_a_group();
      // a_group is flattenned
      a_group.set_flat_foo(0xF00);
      expected.push_str(r#""flatFoo":3840,"#);

      a_group.set_flat_bar("0xBaa");
      expected.push_str(r#""flatBar":"0xBaa","#);

      a_group.reborrow().init_flat_baz().set_hello(true);
      expected.push_str(r#""renamed-flatBaz":{"hello":true},"#);

      a_group.reborrow().init_double_flat().set_flat_qux("Qux");
      expected.push_str(r#""flatQux":"Qux","#);
    }

    {
      let mut prefixed_group = root.reborrow().init_prefixed_group();
      prefixed_group.set_foo("Foo");
      expected.push_str(r#""pfx.foo":"Foo","#);

      prefixed_group.set_bar(0xBAA);
      expected.push_str(r#""pfx.renamed-bar":2986,"#);

      prefixed_group.reborrow().init_baz().set_hello(false);
      expected.push_str(r#""pfx.baz":{"hello":false},"#);

      prefixed_group.reborrow().init_more_prefix().set_qux("Qux");
      expected.push_str(r#""pfx.xfp.qux":"Qux","#);
    }

    {
      let mut a_union_bar = root.reborrow().init_a_union().init_bar();
      expected.push_str(r#""union-type":"renamed-bar","#);
      a_union_bar.set_bar_member(0xAAB);
      expected.push_str(r#""barMember":2731,"#);
      a_union_bar.set_multi_member("Member");
      expected.push_str(r#""multiMember":"Member","#);
    }

    {
      let mut dependency = root.reborrow().init_dependency();
      dependency.set_foo("dep-foo");
      expected.push_str(r#""dependency":{"renamed-foo":"dep-foo"},"#);
    }

    {
      let mut simple_group = root.reborrow().init_simple_group();
      simple_group.set_grault("grault");
      expected.push_str(r#""simpleGroup":{"renamed-grault":"grault"},"#);
    }

    {
      let mut e = root.reborrow().init_enums(4);
      e.set(0, crate::json_test_capnp::TestJsonAnnotatedEnum::Foo);
      e.set(1, crate::json_test_capnp::TestJsonAnnotatedEnum::Bar);
      e.set(2, crate::json_test_capnp::TestJsonAnnotatedEnum::Baz);
      e.set(3, crate::json_test_capnp::TestJsonAnnotatedEnum::Qux);
      expected
        .push_str(r#""enums":["foo","renamed-bar","renamed-baz","qux"],"#);
    }

    {
      let mut b_union = root.reborrow().init_b_union();
      expected.push_str(r#""bUnion":"renamed-bar","#);
      b_union.set_bar(100);
      expected.push_str(r#""bValue":100,"#);
    }

    {
      let mut external_union = root.reborrow().init_external_union();
      external_union.reborrow().init_bar().set_value("Value");
      expected.push_str(r#""externalUnion":{"type":"bar","value":"Value"},"#);
    }

    {
      let mut union_with_void = root.reborrow().init_union_with_void();
      union_with_void.set_void_value(());
      expected.push_str(r#""unionWithVoid":{"type":"voidValue"},"#);
    }

    expected.pop(); // Remove trailing comma
    expected.push('}');

    assert_eq!(expected, json::to_json(root.reborrow_as_reader()).unwrap());
  }

  #[test]
  fn test_base64_union() {
    let mut builder = message::Builder::new_default();
    let mut root: crate::json_test_capnp::test_base64_union::Builder<'_> =
      builder.init_root();

    root.set_foo(&[0xde, 0xad, 0xbe, 0xef]);
    assert_eq!(
      r#"{"foo":"3q2+7w=="}"#,
      json::to_json(root.reborrow_as_reader()).unwrap()
    );
  }

  #[test]
  fn test_string_encoding() {
    let mut builder = message::Builder::new_default();
    let mut root: crate::json_test_capnp::test_flattened_struct::Builder<'_> =
      builder.init_root();

    root.set_value("");
    assert_eq!(
      r#"{"value":""}"#,
      json::to_json(root.reborrow_as_reader()).unwrap()
    );

    root.set_value(
      "tab: \t, newline: \n, carriage return: \r, quote: \", backslash: \\",
    );
    assert_eq!(
      r#"{"value":"tab: \t, newline: \n, carriage return: \r, quote: \", backslash: \\"}"#,
      json::to_json(root.reborrow_as_reader()).unwrap()
    );

    root.set_value("unicode: †eśt");
    assert_eq!(
      r#"{"value":"unicode: †eśt"}"#,
      json::to_json(root.reborrow_as_reader()).unwrap()
    );

    root.set_value("backspace: \u{0008}, formfeed: \u{000C}");
    assert_eq!(
      r#"{"value":"backspace: \b, formfeed: \f"}"#,
      json::to_json(root.reborrow_as_reader()).unwrap()
    );

    root.set_value("bell: \u{0007}, SOH: \u{0001}");
    assert_eq!(
      r#"{"value":"bell: \u0007, SOH: \u0001"}"#,
      json::to_json(root.reborrow_as_reader()).unwrap()
    );
  }

  #[test]
  fn test_nested_data_list() -> capnp::Result<()> {
    let mut builder = message::Builder::new_default();
    let mut root =
      builder.init_root::<crate::json_test_capnp::nested_hex::Builder<'_>>();
    let mut awd = root.reborrow().init_data_all_the_way_down(2);
    let mut first = awd.reborrow().init(0, 2);
    first.set(0, &[0xde, 0xad, 0xbe, 0xef]);
    first.set(1, &[0xef, 0xbe, 0xad, 0xde]);
    let mut second = awd.reborrow().init(1, 1);
    second.set(0, &[0xba, 0xdf, 0x00, 0xd0]);

    assert_eq!(
      r#"{"dataAllTheWayDown":[["deadbeef","efbeadde"],["badf00d0"]]}"#,
      json::to_json(root.reborrow_as_reader())?
    );

    Ok(())
  }

  // Decode

  #[test]
  fn test_decode_simple() -> capnp::Result<()> {
    let mut builder = message::Builder::new_default();
    let mut root: test_json_types::Builder<'_> = builder.init_root();
    json::from_json(
      r#"
            {
              "voidField": null,
              "boolField": true,
              "int8Field": -8,
              "int16Field": -16,
              "int32Field": -32,
              "int64Field": "-64",
              "uInt8Field": 8,
              "uInt16Field": 16,
              "uInt32Field": 32,
              "uInt64Field": "64",
              "float32Field": 1.3200000524520874,
              "float64Field": 0.164e2,
              "textField": "hello",
              "dataField": [
                222,
                173

                ,

                190,
                239,
                202,
                254,
                186,
                190
              ],
              "base64Field": "3q2+78r+ur4=",
              "hexField": "deadbeefcafebabe",
              "structField": {
                "voidField": null,
                "boolField": false,
                "int8Field": 0,
                "int16Field": 0,
                "int32Field": 0,
                "int64Field": "0",
                "uInt8Field": 0,
                "uInt16Field"
                : 0,
                "uInt32Field": 0,
                "uInt64Field": "0",
                "float32Field": 0,
                "float64Field": 0,
                "textField": "inner",
                "enumField": "foo",
                "textList": [
                  "frist",
                  "segund"
                ],
                "base64List": [
                  "3q2+7w==",
                  "ut8A0A=="
                ],
                "hexList": [
                  "deadbeef",
                  "badf00d0"
                ]
              },
              "enumField": "quux",
              "float32List": [
                "NaN",
                "Infinity",
                "-Infinity"
              ],
              "float64List": [
                "NaN",
                "Infinity" ,
                "-Infinity"
              ],
              "enumList": [
                "foo",
                "bar",
                "garply"
              ],
              "int64List": [
                "1",
                "2",
                "4",
                "8"
              ]
            }
          "#,
      root.reborrow(),
    )?;

    let reader = root.into_reader();
    assert_eq!((), reader.get_void_field());
    assert!(reader.get_bool_field());
    assert_eq!(-8, reader.get_int8_field());
    assert_eq!(-16, reader.get_int16_field());
    assert_eq!(-32, reader.get_int32_field());
    assert_eq!(-64, reader.get_int64_field());
    assert_eq!(8, reader.get_u_int8_field());
    assert_eq!(16, reader.get_u_int16_field());
    assert_eq!(32, reader.get_u_int32_field());
    assert_eq!(64, reader.get_u_int64_field());
    assert_eq!(1.32, reader.get_float32_field());
    assert_eq!(16.4, reader.get_float64_field());
    assert_eq!("hello", reader.get_text_field()?.to_str()?);
    assert_eq!(
      [0xde, 0xad, 0xbe, 0xef, 0xca, 0xfe, 0xba, 0xbe],
      reader.get_data_field()?
    );
    assert_eq!(
      [0xde, 0xad, 0xbe, 0xef, 0xca, 0xfe, 0xba, 0xbe],
      reader.get_base64_field()?
    );
    assert_eq!(
      [0xde, 0xad, 0xbe, 0xef, 0xca, 0xfe, 0xba, 0xbe],
      reader.get_hex_field()?
    );

    for i in 0..4 {
      assert_eq!(1 << i, reader.get_int64_list()?.get(i as u32));
    }

    Ok(())
  }

  #[test]
  fn test_encode_with_empty_flattened() -> capnp::Result<()> {
    let mut builder = capnp::message::Builder::new_default();
    let root = builder
      .init_root::<crate::json_test_capnp::test_json_annotations::Builder<'_>>(
      );

    assert_eq!(
      r#"{"flatFoo":0,"renamed-flatBaz":{"hello":false},"pfx.renamed-bar":0,"pfx.baz":{"hello":false},"union-type":"foo","multiMember":0,"simpleGroup":{},"unionWithVoid":{"type":"intValue","intValue":0}}"#,
      json::to_json(root.reborrow_as_reader())?
    );

    Ok(())
  }

  #[test]
  fn test_decode_flattened() -> capnp::Result<()> {
    let j = r#"
        {
          "names-can_contain!anything Really": "Some Field",
          "flatFoo": 1234,
          "flatBar": "0xBaa",
          "renamed-flatBaz": {"hello": true},
          "flatQux": "Qux",
          "pfx.baz": {"hello": true},
          "union-type": "renamed-bar",
          "barMember": 2731,
          "multiMember": "Member",
          "bUnion": "renamed-bar",
          "bValue": 100
        }
      "#;
    let mut builder = capnp::message::Builder::new_default();
    let mut root = builder
      .init_root::<crate::json_test_capnp::test_json_annotations::Builder<'_>>(
    );
    json::from_json(j, root.reborrow())?;

    let reader = root.into_reader();
    assert_eq!("Some Field", reader.get_some_field()?.to_str()?);
    assert_eq!(1234, reader.get_a_group().get_flat_foo());
    assert_eq!("0xBaa", reader.get_a_group().get_flat_bar()?.to_str()?);
    assert!(reader.get_a_group().get_flat_baz().get_hello());
    assert_eq!(
      "Qux",
      reader
        .get_a_group()
        .get_double_flat()
        .get_flat_qux()?
        .to_str()?
    );
    assert!(reader.get_prefixed_group().get_baz().get_hello());
    assert!(matches!(
      reader.get_a_union().which()?,
      crate::json_test_capnp::test_json_annotations::a_union::Bar(_)
    ));
    {
      let bar = match reader.get_a_union().which()? {
        crate::json_test_capnp::test_json_annotations::a_union::Bar(b) => b,
        _ => panic!("Expected Bar"),
      };
      assert_eq!(2731, bar.get_bar_member());
      assert_eq!("Member", bar.get_multi_member()?.to_str()?);
    }
    assert!(matches!(
      reader.get_b_union().which()?,
      crate::json_test_capnp::test_json_annotations::b_union::Bar(_)
    ));
    {
      let bar = match reader.get_b_union().which()? {
        crate::json_test_capnp::test_json_annotations::b_union::Bar(b) => b,
        _ => panic!("Expected Bar"),
      };
      assert_eq!(100, bar);
    }

    Ok(())
  }

  #[test]
  fn test_decode_base64_union() -> capnp::Result<()> {
    {
      let j = r#"
            {
              "foo":"3q2+7w=="
            }
          "#;
      let mut builder = capnp::message::Builder::new_default();
      let mut root = builder
        .init_root::<crate::json_test_capnp::test_base64_union::Builder<'_>>();
      json::from_json(j, root.reborrow())?;

      let reader = root.into_reader();
      assert!(matches!(
        reader.which()?,
        crate::json_test_capnp::test_base64_union::Foo(_)
      ));
      {
        let foo = match reader.which()? {
          crate::json_test_capnp::test_base64_union::Foo(f) => f,
          _ => panic!("Expected Foo"),
        }?;
        assert_eq!(&[0xde, 0xad, 0xbe, 0xef], foo);
      }
    }

    {
      let j = r#"
            {
              "bar":"To the bar!"
            }
          "#;
      let mut builder = capnp::message::Builder::new_default();
      let mut root = builder
        .init_root::<crate::json_test_capnp::test_base64_union::Builder<'_>>();
      json::from_json(j, root.reborrow())?;

      let reader = root.into_reader();
      assert!(matches!(
        reader.which()?,
        crate::json_test_capnp::test_base64_union::Bar(_)
      ));
      {
        let bar = match reader.which()? {
          crate::json_test_capnp::test_base64_union::Bar(b) => b?,
          _ => panic!("Expected Foo"),
        };
        assert_eq!("To the bar!", bar.to_str()?);
      }
    }

    // When both variants are present, we pick the first one in the spec
    {
      let j = r#"
            {
              "bar":"To the bar!",
              "foo":"3q2+7w=="
            }
          "#;
      let mut builder = capnp::message::Builder::new_default();
      let mut root = builder
        .init_root::<crate::json_test_capnp::test_base64_union::Builder<'_>>();
      json::from_json(j, root.reborrow())?;

      let reader = root.into_reader();
      assert!(matches!(
        reader.which()?,
        crate::json_test_capnp::test_base64_union::Foo(_)
      ));
      {
        let foo = match reader.which()? {
          crate::json_test_capnp::test_base64_union::Foo(f) => f,
          _ => panic!("Expected Foo"),
        }?;
        assert_eq!(&[0xde, 0xad, 0xbe, 0xef], foo);
      }
    }

    {
      let j = r#"
            {
              "bar":"To the bar!",
              "foo":"3q2+7w=="
            }
          "#;
      let mut builder = capnp::message::Builder::new_default();
      let mut root = builder
        .init_root::<crate::json_test_capnp::test_renamed_anon_union::Builder<
        '_,
      >>();
      json::from_json(j, root.reborrow())?;

      let reader = root.into_reader();
      assert!(matches!(
        reader.which()?,
        crate::json_test_capnp::test_renamed_anon_union::Bar(_)
      ));
      {
        let bar = match reader.which()? {
          crate::json_test_capnp::test_renamed_anon_union::Bar(b) => b?,
          _ => panic!("Expected Foo"),
        };
        assert_eq!("To the bar!", bar.to_str()?);
      }
    }

    {
      let j = r#"
            {
              "bar":"To the bar!",
              "renamed-foo":"3q2+7w=="
            }
          "#;
      let mut builder = capnp::message::Builder::new_default();
      let mut root = builder
        .init_root::<crate::json_test_capnp::test_renamed_anon_union::Builder<
        '_,
      >>();
      json::from_json(j, root.reborrow())?;

      let reader = root.into_reader();
      assert!(matches!(
        reader.which()?,
        crate::json_test_capnp::test_renamed_anon_union::Foo(_)
      ));
      {
        let foo = match reader.which()? {
          crate::json_test_capnp::test_renamed_anon_union::Foo(f) => f,
          _ => panic!("Expected Foo"),
        }?;
        assert_eq!(&[0xde, 0xad, 0xbe, 0xef], foo);
      }
    }
    Ok(())
  }

  #[test]
  fn test_decode_nested_data_list() -> capnp::Result<()> {
    let json =
      r#"{"dataAllTheWayDown":[["deadbeef","efbeadde"],["badf00d0"]]}"#;
    let mut builder = message::Builder::new_default();
    let mut root =
      builder.init_root::<crate::json_test_capnp::nested_hex::Builder<'_>>();
    json::from_json(json, root.reborrow())?;

    let reader = root.into_reader();

    {
      let awd = reader.get_data_all_the_way_down()?;
      let first = awd.get(0)?;
      assert_eq!(2, first.len());
      assert_eq!(&[0xde, 0xad, 0xbe, 0xef], first.get(0)?);
      assert_eq!(&[0xef, 0xbe, 0xad, 0xde], first.get(1)?);
      let second = awd.get(1)?;
      assert_eq!(1, second.len());
      assert_eq!(&[0xba, 0xdf, 0x00, 0xd0], second.get(0)?);
    }

    Ok(())
  }

  #[test]
  fn test_decode_union_with_void() -> capnp::Result<()> {
    let json = r#"
        {
          "unionWithVoid": {
            "type": "voidValue"
          }
        }
      "#;

    let mut builder = capnp::message::Builder::new_default();
    let mut root = builder
      .init_root::<crate::json_test_capnp::test_json_annotations::Builder<'_>>(
    );
    json::from_json(json, root.reborrow())?;

    let reader = root.into_reader();
    assert!(matches!(
      reader.get_union_with_void().which()?,
      crate::json_test_capnp::test_json_annotations::union_with_void::VoidValue(
        _
      )
    ));

    Ok(())
  }

  #[test]
  fn test_encode_decode_no_name_discriminator() -> capnp::Result<()> {
    let mut builder = capnp::message::Builder::new_default();
    let mut root = builder
      .init_root::<crate::json_test_capnp::unnamed_discriminator::Builder<'_>>(
    );
    root.reborrow().init_baz().set_bar(100);
    root.reborrow().init_sbaz().set_sfoo("Hello");
    let json = json::to_json(root.reborrow_as_reader())?;
    assert_eq!(r#"{"baz":{"bar":100},"sbaz":"sfoo","sfoo":"Hello"}"#, json);

    let mut builder = capnp::message::Builder::new_default();
    let mut root = builder
      .init_root::<crate::json_test_capnp::unnamed_discriminator::Builder<'_>>(
    );
    json::from_json(&json, root.reborrow())?;
    let reader = root.into_reader();
    assert_eq!(
      100,
      match reader.get_baz().which()? {
        crate::json_test_capnp::unnamed_discriminator::baz::Bar(b) => b,
        _ => panic!("Expected Bar"),
      },
    );
    assert_eq!(
      "Hello",
      match reader.get_sbaz().which()? {
        crate::json_test_capnp::unnamed_discriminator::sbaz::Sfoo(s) =>
          s?.to_str()?,
        _ => panic!("Expected Sfoo"),
      }
    );

    Ok(())
  }

  #[test]
  fn any_pointer_is_not_encoded() -> capnp::Result<()> {
    let mut builder = capnp::message::Builder::new_default();
    let mut root = builder
      .init_root::<crate::json_test_capnp::test_any_pointer::Builder<'_>>();
    let mut v = root
      .reborrow()
      .init_any_pointer_field()
      .init_as::<crate::test_capnp::test_all_types::Builder<'_>>();

    v.set_text_field("Hello");

    let result = json::to_json(root.reborrow_as_reader());
    assert!(matches!(
      result,
      Err(capnp::Error {
        kind: capnp::ErrorKind::Unimplemented,
        ..
      })
    ));
    Ok(())
  }

  #[test]
  fn any_pointer_is_encoded_with_override() -> capnp::Result<()> {
    let mut builder = capnp::message::Builder::new_default();
    let mut root = builder
      .init_root::<crate::json_test_capnp::test_any_pointer::Builder<'_>>();
    let mut v = root
      .reborrow()
      .init_any_pointer_field()
      .init_as::<crate::test_capnp::test_all_types::Builder<'_>>();

    v.set_text_field("Hello");

    let any_pointer_field = {
      use capnp::introspect::Introspect;
      let capnp::introspect::TypeVariant::Struct(schema) =
        crate::json_test_capnp::test_any_pointer::Owned::introspect().which()
      else {
        panic!("Expected struct");
      };
      capnp::schema::StructSchema::new(schema)
        .get_field_by_name("anyPointerField")?
    };

    let s = "Any pointer you like".to_string();

    let codec = json::Codec::new().with_field_override(
      any_pointer_field,
      json::make_field_codec(
        |_reader| Ok(JsonValue::String(s.clone())),
        |_value, _builder| Ok(()),
      ),
    );

    let j = codec.encode(root.reborrow_as_reader())?;
    assert_eq!(r#"{"anyPointerField":"Any pointer you like"}"#, j);
    Ok(())
  }

  #[test]
  fn any_pointer_is_not_decoded() -> capnp::Result<()> {
    let mut builder = capnp::message::Builder::new_default();
    let mut root = builder
      .init_root::<crate::json_test_capnp::test_any_pointer::Builder<'_>>();

    let json = r#"{"anyPointerField":{"textField":"Hello"}}"#;

    let result = json::from_json(json, root.reborrow());
    assert!(matches!(
      result,
      Err(capnp::Error {
        kind: capnp::ErrorKind::Unimplemented,
        ..
      })
    ));
    Ok(())
  }

  #[test]
  fn any_pointer_is_decoded_with_override() -> capnp::Result<()> {
    let json = r#"{"anyPointerField":{"textField":"Hello"}}"#;

    let any_pointer_field = {
      use capnp::introspect::Introspect;
      let capnp::introspect::TypeVariant::Struct(schema) =
        crate::json_test_capnp::test_any_pointer::Owned::introspect().which()
      else {
        panic!("Expected struct");
      };
      capnp::schema::StructSchema::new(schema)
        .get_field_by_name("anyPointerField")?
    };

    struct MyFieldCodec;
    impl json::FieldCodec for MyFieldCodec {
      fn encode_value(
        &self,
        _source: capnp::dynamic_value::Reader<'_>,
      ) -> capnp::Result<JsonValue> {
        Err(capnp::Error::unimplemented("Fail".into()))
      }

      fn decode_value(
        &self,
        source: &JsonValue,
        target: capnp::dynamic_value::Builder<'_>,
      ) -> capnp::Result<()> {
        let JsonValue::Object(obj) = source else {
          return Err(capnp::Error::failed(
            "Expected object for any pointer field".into(),
          ));
        };

        let capnp::dynamic_value::Builder::AnyPointer(any_pointer) = target
        else {
          return Err(capnp::Error::failed(
            "Expected any pointer builder".into(),
          ));
        };

        let mut builder = any_pointer
          .init_as::<crate::test_capnp::test_all_types::Builder<'_>>();
        builder.set_text_field(
          obj
            .get("textField")
            .and_then(|v| match v {
              JsonValue::String(s) => Some(s.as_str()),
              _ => None,
            })
            .ok_or_else(|| {
              capnp::Error::failed("Expected textField to be a string".into())
            })?,
        );

        Ok(())
      }
    }

    let codec =
      json::Codec::new().with_field_override(any_pointer_field, MyFieldCodec);

    let mut builder = capnp::message::Builder::new_default();
    let mut root = builder
      .init_root::<crate::json_test_capnp::test_any_pointer::Builder<'_>>();

    codec.decode(json, root.reborrow())?;
    let reader = root.reborrow_as_reader();

    let all_types = reader
      .get_any_pointer_field()
      .get_as::<crate::test_capnp::test_all_types::Reader<'_>>(
    )?;
    assert_eq!("Hello", all_types.get_text_field()?.to_str()?);

    Ok(())
  }

  #[test]
  fn test_different_overrides_for_different_brands() -> capnp::Result<()> {
    let json_text =
      r#"{"anyPointerField":"Hello, text","genericField":"text"}"#;
    let json_struct = r#"{"anyPointerField":"Hello, struct","genericField":{"voidField":null,"boolField":false,"int8Field":0,"int16Field":0,"int32Field":0,"int64Field":"0","uInt8Field":0,"uInt16Field":0,"uInt32Field":0,"uInt64Field":"0","float32Field":0,"float64Field":0,"textField":"Hello to you too","enumField":"foo"}}"#;

    let any_pointer_field_text = {
      use capnp::introspect::Introspect;
      let capnp::introspect::TypeVariant::Struct(schema) =
        crate::json_test_capnp::test_generic::Owned::<capnp::text::Owned>::introspect().which()
      else {
        panic!("Expected struct");
      };
      capnp::schema::StructSchema::new(schema)
        .get_field_by_name("anyPointerField")?
    };
    let any_pointer_field_struct = {
      use capnp::introspect::Introspect;
      let capnp::introspect::TypeVariant::Struct(schema) =
        crate::json_test_capnp::test_generic::Owned::<
          crate::test_capnp::test_all_types::Owned,
        >::introspect()
        .which()
      else {
        panic!("Expected struct");
      };
      capnp::schema::StructSchema::new(schema)
        .get_field_by_name("anyPointerField")?
    };

    assert!(any_pointer_field_text != any_pointer_field_struct);

    let codec = json::Codec::new()
      .with_field_override(
        any_pointer_field_text,
        json::make_field_codec(
          |_reader| Ok(JsonValue::String("Hello, text".to_string())),
          |_value, _builder| Ok(()),
        ),
      )
      .with_field_override(
        any_pointer_field_struct,
        json::make_field_codec(
          |_reader| Ok(JsonValue::String("Hello, struct".to_string())),
          |_value, _builder| Ok(()),
        ),
      );

    let mut builder_text = capnp::message::Builder::new_default();
    let mut root_text = builder_text
      .init_root::<crate::json_test_capnp::test_generic::Builder<
      capnp::text::Owned,
    >>();
    root_text
      .reborrow()
      .init_any_pointer_field()
      .set_as::<capnp::text::Owned>("Hello, text")?;
    root_text.set_generic_field("text")?;
    assert_eq!(json_text, codec.encode(root_text.reborrow_as_reader())?);

    let mut builder_struct = capnp::message::Builder::new_default();
    let mut root_struct = builder_struct
      .init_root::<crate::json_test_capnp::test_generic::Builder<
      crate::test_capnp::test_all_types::Owned,
    >>();
    root_struct
      .reborrow()
      .init_any_pointer_field()
      .set_as::<capnp::text::Owned>("Hello, struct")?;
    root_struct
      .reborrow()
      .init_generic_field()
      .set_text_field("Hello to you too");
    assert_eq!(json_struct, codec.encode(root_struct.reborrow_as_reader())?);

    Ok(())
  }

  #[test]
  fn custom_codec_encode() -> capnp::Result<()> {
    let mut msg = capnp::message::Builder::new_default();
    let mut root = msg.init_root::<crate::json_test_capnp::struct_with_custom_codec::Builder<'_>>();

    root.reborrow().init_struct_level();
    root.set_field_level("Hello, field level!");

    let codec = json::Codec::new()
      .with_named_codec(
        "TestCodec",
        json::make_field_codec(
          |_value| {
            let mut v = std::collections::HashMap::new();
            v.insert("test".to_string(), JsonValue::String("Inside".into()));
            Ok(JsonValue::Object(v))
          },
          |_value, _builder| Ok(()),
        ),
      )
      .with_named_codec(
        "TestFieldCodec",
        json::make_field_codec(
          |value| {
            Ok(JsonValue::String(format!(
              "Hello, {}!",
              value.downcast::<capnp::text::Reader<'_>>().to_str()?
            )))
          },
          |_value, _builder| Ok(()),
        ),
      );

    let json = codec.encode(root.reborrow_as_reader())?;

    assert_eq!(
      r#"{"structLevel":{"test":"Inside"},"fieldLevel":"Hello, Hello, field level!!"}"#,
      json
    );

    Ok(())
  }

  #[test]
  fn custom_codec_decode() -> capnp::Result<()> {
    let json = r#"{"structLevel":{"test":"Inside"},"fieldLevel":{"prefix":"Hello, ","suffix":"!", "value": "World"}}"#;

    let mut msg = capnp::message::Builder::new_default();
    let mut root = msg.init_root::<crate::json_test_capnp::struct_with_custom_codec::Builder<'_>>();

    struct TestFieldCodec {}
    impl json::FieldCodec for TestFieldCodec {
      fn encode_value(
        &self,
        _: capnp::dynamic_value::Reader<'_>,
      ) -> capnp::Result<JsonValue> {
        todo!()
      }

      fn decode_value(
        &self,
        _source: &JsonValue,
        _target: capnp::dynamic_value::Builder<'_>,
      ) -> capnp::Result<()> {
        todo!()
      }

      fn decode_member(
        &self,
        value: &JsonValue,
        mut target: capnp::dynamic_struct::Builder<'_>,
        field: capnp::schema::Field,
      ) -> capnp::Result<()> {
        let JsonValue::Object(obj) = value else {
          return Err(capnp::Error::failed("Expected object".into()));
        };

        let (prefix, suffix, value) = (
          obj
            .get("prefix")
            .and_then(|v| match v {
              JsonValue::String(s) => Some(s.as_str()),
              _ => None,
            })
            .ok_or_else(|| {
              capnp::Error::failed("Expected prefix field".into())
            })?,
          obj
            .get("suffix")
            .and_then(|v| match v {
              JsonValue::String(s) => Some(s.as_str()),
              _ => None,
            })
            .ok_or_else(|| {
              capnp::Error::failed("Expected suffix field".into())
            })?,
          obj
            .get("value")
            .and_then(|v| match v {
              JsonValue::String(s) => Some(s.as_str()),
              _ => None,
            })
            .ok_or_else(|| {
              capnp::Error::failed("Expected value field".into())
            })?,
        );

        target.set(
          field,
          capnp::dynamic_value::Reader::Text(
            format!("{}{}{}", prefix, value, suffix).as_str().into(),
          ),
        )?;

        Ok(())
      }
    }

    let codec = json::Codec::new()
      .with_named_codec(
        "TestCodec",
        json::make_field_codec(
          |_value| {
            let mut v = std::collections::HashMap::new();
            v.insert("test".to_string(), JsonValue::String("Inside".into()));
            Ok(JsonValue::Object(v))
          },
          |value, builder| {
            let JsonValue::Object(obj) = value else {
              return Err(capnp::Error::failed("Expected object".into()));
            };
            let test_value = obj
              .get("test")
              .and_then(|v| match v {
                JsonValue::String(v) => Some(v),
                _ => None,
              })
              .ok_or_else(|| {
                capnp::Error::failed("Expected test field".into())
              })?;
            let builder = builder.downcast_struct::<crate::json_test_capnp::test_custom_codec::Owned>();
            builder.get_something().set_as::<capnp::text::Owned>(test_value)?;
            Ok(())
          },
        ),
      );
    let codec = codec.with_named_codec("TestFieldCodec", TestFieldCodec {});

    codec.decode(json, root.reborrow())?;

    let reader = root.reborrow_as_reader();
    assert_eq!(
      "Inside",
      reader
        .get_struct_level()?
        .get_something()
        .get_as::<capnp::text::Reader<'_>>()?
        .to_str()?
    );
    assert_eq!("Hello, World!", reader.get_field_level()?);

    Ok(())
  }

  #[test]
  fn errors_in_custom_codecs_propagate() -> capnp::Result<()> {
    let json = r#"{"structLevel":{"test":"Inside"},"fieldLevel":{"prefix":"Hello, ","suffix":"!", "value": "World"}}"#;

    let mut msg = capnp::message::Builder::new_default();
    let mut root = msg.init_root::<crate::json_test_capnp::struct_with_custom_codec::Builder<'_>>();

    struct FailingCodec {}
    impl json::FieldCodec for FailingCodec {
      fn encode_value(
        &self,
        _: capnp::dynamic_value::Reader<'_>,
      ) -> capnp::Result<JsonValue> {
        Err(capnp::Error::failed("FailingCodec encode".into()))
      }

      fn decode_value(
        &self,
        _source: &JsonValue,
        _target: capnp::dynamic_value::Builder<'_>,
      ) -> capnp::Result<()> {
        Err(capnp::Error::failed("FailingCodec decode".into()))
      }
    }

    let codec = json::Codec::new()
      .with_named_codec("TestCodec", FailingCodec {})
      .with_named_codec("TestFieldCodec", FailingCodec {});

    let result = codec.decode(json, root.reborrow());
    let Err(e) = result else {
      panic!("Expected error");
    };
    assert_eq!(e.kind, capnp::ErrorKind::Failed);
    assert_eq!(e.extra, "FailingCodec decode");

    let result = codec.encode(root.reborrow_as_reader());
    let Err(e) = result else {
      panic!("Expected error");
    };
    assert_eq!(e.kind, capnp::ErrorKind::Failed);
    assert_eq!(e.extra, "FailingCodec encode");

    Ok(())
  }

  #[test]
  fn type_overrides_encode() -> capnp::Result<()> {
    let mut builder = capnp::message::Builder::new_default();
    let mut root = builder
      .init_root::<crate::json_test_capnp::test_json_annotations::Builder<'_>>(
    );
    root.reborrow().init_a_group().set_flat_foo(1234);

    let a_group_field = {
      use capnp::introspect::Introspect;
      let capnp::introspect::TypeVariant::Struct(schema) =
        crate::json_test_capnp::test_json_annotations::Owned::introspect()
          .which()
      else {
        panic!("Expected struct");
      };
      capnp::schema::StructSchema::new(schema).get_field_by_name("aGroup")?
    };

    let codec = json::Codec::new().with_type_override(
      a_group_field.get_type(),
      json::make_field_codec(
        |_reader| {
          let v = std::collections::HashMap::from([(
            "aGroup".to_string(),
            JsonValue::String("Overridden".into()),
          )]);
          Ok(JsonValue::Object(v))
        },
        |_value, _builder| Ok(()),
      ),
    );

    let json = codec.encode(root.reborrow_as_reader())?;
    assert_eq!(
      r#"{"aGroup":"Overridden","pfx.renamed-bar":0,"pfx.baz":{"hello":false},"union-type":"foo","multiMember":0,"simpleGroup":{},"unionWithVoid":{"type":"intValue","intValue":0}}"#,
      json
    );
    Ok(())
  }

  #[test]
  fn type_overrides_decode() -> capnp::Result<()> {
    let json = r#"{"aGroup":"Overridden","pfx.renamed-bar":0,"pfx.baz":{"hello":false},"union-type":"foo","multiMember":0,"simpleGroup":{},"unionWithVoid":{"type":"intValue","intValue":0}}"#;

    let mut builder = capnp::message::Builder::new_default();
    let mut root = builder
      .init_root::<crate::json_test_capnp::test_json_annotations::Builder<'_>>(
    );

    let a_group_field = {
      use capnp::introspect::Introspect;
      let capnp::introspect::TypeVariant::Struct(schema) =
        crate::json_test_capnp::test_json_annotations::Owned::introspect()
          .which()
      else {
        panic!("Expected struct");
      };
      capnp::schema::StructSchema::new(schema).get_field_by_name("aGroup")?
    };

    let codec = json::Codec::new().with_type_override(
      a_group_field.get_type(),
      json::make_field_codec(
        |_reader| {
          let v = std::collections::HashMap::from([(
            "aGroup".to_string(),
            JsonValue::String("Overridden".into()),
          )]);
          Ok(JsonValue::Object(v))
        },
        |value, builder| {
          assert_eq!(
            JsonValue::String("Overridden".into()),
            value.clone()
          );
          let mut builder = builder.downcast_struct::<crate::json_test_capnp::test_json_annotations::a_group::Owned>();
          builder.reborrow().set_flat_bar("Overridden");
          Ok(())
        }
      ),
    );

    codec.decode(json, root.reborrow())?;
    let reader = root.reborrow_as_reader();
    assert_eq!("Overridden", reader.get_a_group().get_flat_bar()?.to_str()?);

    Ok(())
  }

  #[test]
  fn recursion_level_limit() -> capnp::Result<()> {
    let codec = json::Codec::new_with_options(json::CodecOptions {
      recursion_limit: 2,
      ..Default::default()
    });

    let mut builder = capnp::message::Builder::new_default();
    let mut root = builder
      .init_root::<crate::json_test_capnp::test_any_pointer::Builder<'_>>();

    let json = r#"{"anyPointerField":[[["too deep"]]]}"#;
    let result = codec.decode(json, root.reborrow());
    let Err(e) = result else {
      panic!("Expected error");
    };
    assert_eq!(e.kind, capnp::ErrorKind::Failed);
    assert_eq!(e.extra, "Recursion limit exceeded while parsing JSON");

    let json = r#"{"anyPointerField":{"anyPointerField":{"anyPointerField":"too deep"}}}"#;
    let result = codec.decode(json, root.reborrow());
    let Err(e) = result else {
      panic!("Expected error");
    };
    assert_eq!(e.kind, capnp::ErrorKind::Failed);
    assert_eq!(e.extra, "Recursion limit exceeded while parsing JSON");

    let json = r#"{"anyPointerField":[{"anyPointerField":{"anyPointerField":"too deep"}}]}"#;
    let result = codec.decode(json, root.reborrow());
    let Err(e) = result else {
      panic!("Expected error");
    };
    assert_eq!(e.kind, capnp::ErrorKind::Failed);
    assert_eq!(e.extra, "Recursion limit exceeded while parsing JSON");

    Ok(())
  }

  // ---------------------------------------------------------------------
  // Recursion limits
  //
  // Two independent limits guard against unbounded recursion while decoding:
  // the parser's, which bounds the depth of the `JsonValue` tree it builds,
  // and `decode_struct`'s, which bounds how deep the schema walk descends.
  // The second is not redundant: a struct that flattens a field of its own
  // type recurses on the schema without descending into the JSON at all, so
  // the parser's limit can never fire for it.
  // ---------------------------------------------------------------------

  /// The default matches the C++ codec's `maxNestingDepth`.
  #[test]
  fn recursion_limit_default_matches_cpp() {
    assert_eq!(json::CodecOptions::default().recursion_limit, 64);
  }

  /// Nesting one JSON object per schema level: the boundary is exact, and
  /// input one level inside it still decodes correctly.
  #[test]
  fn recursion_limit_object_nesting_boundary() -> capnp::Result<()> {
    use crate::json_test_capnp::self_struct;

    fn nested(depth: usize) -> String {
      let mut s = String::new();
      for _ in 0..depth {
        s.push_str(r#"{"inner":"#);
      }
      s.push_str(r#"{"value":7}"#);
      for _ in 0..depth {
        s.push('}');
      }
      s
    }

    let codec = json::Codec::new();
    let limit = json::CodecOptions::default().recursion_limit;

    // `nested(d)` writes d `{"inner":` wrappers around a `{"value":7}` leaf,
    // so it contains d + 1 objects in total. A `recursion_limit` of N admits
    // N nested containers, which is exactly what `capnp convert` accepts at
    // the same setting: 64 objects through, 65 rejected.
    let deepest_accepted = limit - 1;

    // At the limit: accepted, and the value at the bottom survives.
    let mut builder = capnp::message::Builder::new_default();
    let mut root = builder.init_root::<self_struct::Builder<'_>>();
    codec.decode(&nested(deepest_accepted), root.reborrow())?;
    let mut cursor = root.reborrow_as_reader();
    for _ in 0..deepest_accepted {
      cursor = cursor.get_inner()?;
    }
    assert_eq!(cursor.get_value(), 7, "value at the bottom must survive");

    // One past it: rejected rather than overflowing the stack.
    let mut builder = capnp::message::Builder::new_default();
    let root = builder.init_root::<self_struct::Builder<'_>>();
    let Err(e) = codec.decode(&nested(deepest_accepted + 1), root) else {
      panic!("expected the recursion limit to reject this");
    };
    assert_eq!(e.kind, capnp::ErrorKind::Failed);
    assert!(
      e.extra.contains("Recursion limit exceeded"),
      "unexpected error: {}",
      e.extra
    );
    Ok(())
  }

  /// `validate_schema` reports cyclic flattening the way the C++ codec does.
  /// `$Json.flatten` splices members into the parent object rather than
  /// nesting them, so a cycle of flattened fields describes an object of
  /// infinite width.
  ///
  /// It is a property of the schema, not of the data, so it holds for a type
  /// nothing has been written to.
  #[test]
  fn validate_schema_reports_cyclic_flattening() {
    use crate::json_test_capnp::cyclic_flatten;

    let Err(e) = json::validate_schema::<cyclic_flatten::Owned>() else {
      panic!("expected cyclic flattening to be reported");
    };
    assert_eq!(e.kind, capnp::ErrorKind::Failed);
    assert!(
      e.extra.starts_with("cyclic JSON flattening detected"),
      "unexpected error: {}",
      e.extra
    );
  }

  /// Encoding and decoding do not run the schema check, so a cyclic schema is
  /// not reported as such there. It is still rejected on decode -- by the
  /// recursion limit, which is what guarantees decoding terminates -- just
  /// with a message about depth rather than about the cycle.
  #[test]
  fn cyclic_flatten_is_not_checked_by_decode() {
    use crate::json_test_capnp::cyclic_flatten;

    for json in [r#"{}"#, r#"{"value":1}"#, r#"{"i.value":1}"#] {
      let mut builder = capnp::message::Builder::new_default();
      let root = builder.init_root::<cyclic_flatten::Builder<'_>>();
      let Err(e) = json::Codec::new().decode(json, root) else {
        panic!("expected {json} to be rejected");
      };
      assert_eq!(e.kind, capnp::ErrorKind::Failed);
      assert_eq!(e.extra, "Recursion limit exceeded while decoding JSON");
    }
  }

  /// Encoding a cyclic schema terminates on its own: an unset pointer field is
  /// skipped rather than descended into, so there is nothing for the check to
  /// save us from here.
  #[test]
  fn cyclic_flatten_encodes() -> capnp::Result<()> {
    use crate::json_test_capnp::cyclic_flatten;

    let mut builder = capnp::message::Builder::new_default();
    let mut root = builder.init_root::<cyclic_flatten::Builder<'_>>();
    root.set_value(1);
    assert_eq!(json::to_json(root.reborrow_as_reader())?, r#"{"value":1}"#);

    root.reborrow().init_inner().set_value(2);
    assert_eq!(
      json::to_json(root.reborrow_as_reader())?,
      r#"{"i.value":2,"value":1}"#
    );
    Ok(())
  }

  /// Which schemas count as cyclic, checked against the verdict `capnp
  /// convert` gives for each shape.
  #[test]
  fn cyclic_flatten_matches_cpp_verdicts() -> capnp::Result<()> {
    use crate::json_test_capnp::{
      cyclic_flatten,
      flatten_through_group,
      mutual_flatten_a,
      mutual_one_flat_a,
      references_cyclic,
      references_cyclic_via_list,
      self_struct,
    };

    // Cyclic: a struct flattening its own type.
    assert!(json::validate_schema::<cyclic_flatten::Owned>().is_err());
    // Cyclic: a group is an edge even when it is not itself flattened.
    assert!(json::validate_schema::<flatten_through_group::Owned>().is_err());
    // Cyclic: A -> B -> A, both flattened.
    assert!(json::validate_schema::<mutual_flatten_a::Owned>().is_err());
    // Cyclic: reachable through a plain field, and through a list element
    // type. C++ loads handlers for the whole dependency graph, so these are
    // rejected even though the root itself flattens nothing.
    assert!(json::validate_schema::<references_cyclic::Owned>().is_err());
    assert!(
      json::validate_schema::<references_cyclic_via_list::Owned>().is_err()
    );

    // Not cyclic: the return edge is a plain field, so it nests and
    // flattening terminates.
    json::validate_schema::<mutual_one_flat_a::Owned>()?;
    // Not cyclic: plain self-reference without any flattening.
    json::validate_schema::<self_struct::Owned>()?;
    Ok(())
  }

  /// The check must not be over-eager: the schemas this crate exercises
  /// everywhere else flatten legitimately and must all pass.
  #[test]
  fn validate_schema_accepts_ordinary_schemas() -> capnp::Result<()> {
    use crate::test_capnp::{
      test_json_flatten_union,
      test_json_types,
      test_union,
      test_unnamed_union,
    };

    json::validate_schema::<test_json_types::Owned>()?;
    json::validate_schema::<test_json_flatten_union::Owned>()?;
    json::validate_schema::<test_union::Owned>()?;
    json::validate_schema::<test_unnamed_union::Owned>()?;
    json::validate_schema::<test_json_annotations::Owned>()?;
    Ok(())
  }

  /// A schema that flattens without cycling still works, so the check has not
  /// simply banned flattening.
  #[test]
  fn non_cyclic_flatten_still_round_trips() -> capnp::Result<()> {
    use crate::json_test_capnp::mutual_one_flat_a;

    let mut builder = capnp::message::Builder::new_default();
    let mut root = builder.init_root::<mutual_one_flat_a::Builder<'_>>();
    root.set_value(9);
    {
      let mut b = root.reborrow().init_b();
      b.set_other(4);
      b.reborrow().init_a().set_value(3);
    }
    let encoded = json::to_json(root.reborrow_as_reader())?;
    assert_eq!(encoded, r#"{"a":{"value":3},"other":4,"value":9}"#);

    let mut rt = capnp::message::Builder::new_default();
    let mut rt_root = rt.init_root::<mutual_one_flat_a::Builder<'_>>();
    json::Codec::new().decode(&encoded, rt_root.reborrow())?;

    let r = rt_root.reborrow_as_reader();
    assert_eq!(r.get_value(), 9);
    assert_eq!(r.get_b()?.get_other(), 4);
    assert_eq!(r.get_b()?.get_a()?.get_value(), 3);

    // Re-encoding does not reproduce the input byte-for-byte: decoding always
    // initialises a flattened struct field, even when no member of it was
    // present, so the inner `a`'s own flattened `b` now exists and its
    // members appear on the way back out. That is the pre-existing
    // `decode_struct` FIXME, not an effect of the cycle check -- recorded
    // here so the day it is fixed this test says so.
    assert_eq!(
      json::to_json(rt_root.reborrow_as_reader())?,
      r#"{"a":{"other":0,"value":3},"other":4,"value":9}"#,
      "flattened struct fields are still eagerly initialised on decode"
    );
    Ok(())
  }

  /// Self-reference through `List(Struct)`, which alternates arrays and
  /// objects. Bounded, and legal input inside the limit still round-trips.
  #[test]
  fn recursion_limit_list_of_structs() -> capnp::Result<()> {
    use crate::json_test_capnp::self_list;

    fn nested(depth: usize) -> String {
      let mut s = String::new();
      for _ in 0..depth {
        s.push_str(r#"{"children":["#);
      }
      s.push_str(r#"{"value":7}"#);
      for _ in 0..depth {
        s.push_str("]}");
      }
      s
    }

    let codec = json::Codec::new();

    // Comfortably inside the limit: decodes, and the leaf survives.
    let mut builder = capnp::message::Builder::new_default();
    let mut root = builder.init_root::<self_list::Builder<'_>>();
    codec.decode(&nested(20), root.reborrow())?;
    let mut cursor = root.reborrow_as_reader();
    for _ in 0..20 {
      cursor = cursor.get_children()?.get(0);
    }
    assert_eq!(cursor.get_value(), 7);

    // Far outside it: an error, not a stack overflow.
    let mut builder = capnp::message::Builder::new_default();
    let root = builder.init_root::<self_list::Builder<'_>>();
    // 100 levels is 200 JSON containers, comfortably past the limit of 64,
    // and cheap enough for miri.
    let far_past_limit = if cfg!(miri) { 100 } else { 500 };
    let Err(e) = codec.decode(&nested(far_past_limit), root) else {
      panic!("expected the recursion limit to reject this");
    };
    assert!(
      e.extra.contains("Recursion limit exceeded"),
      "unexpected error: {}",
      e.extra
    );
    Ok(())
  }

  /// Nested lists, which recurse through `decode_list` rather than
  /// `decode_struct`.
  #[test]
  fn recursion_limit_nested_lists() -> capnp::Result<()> {
    use crate::test_capnp::test_complex_list;

    let depth = if cfg!(miri) { 100 } else { 500 };
    let mut json = String::from(r#"{"primListListList":"#);
    for _ in 0..depth {
      json.push('[');
    }
    for _ in 0..depth {
      json.push(']');
    }
    json.push('}');

    let mut builder = capnp::message::Builder::new_default();
    let root = builder.init_root::<test_complex_list::Builder<'_>>();
    let Err(e) = json::Codec::new().decode(&json, root) else {
      panic!("expected the recursion limit to reject this");
    };
    assert_eq!(e.extra, "Recursion limit exceeded while parsing JSON");
    Ok(())
  }

  /// A hostile payload that is nothing but nesting must be rejected quickly
  /// rather than aborting the process. This is the case that motivated the
  /// limit; before it existed this test crashed the whole test binary.
  #[test]
  fn deeply_nested_hostile_input_does_not_abort() -> capnp::Result<()> {
    use crate::json_test_capnp::self_struct;

    // Miri interprets every character of the parse, so the huge sizes take
    // far too long there. The limit fires at 64, so 1_000 already proves the
    // point; the larger sizes exist to show the cost stays bounded.
    const DEPTHS: &[usize] = if cfg!(miri) {
      &[1_000]
    } else {
      &[1_000, 100_000, 1_000_000]
    };

    for &depth in DEPTHS {
      let mut json = String::from(r#"{"inner":"#);
      for _ in 0..depth {
        json.push('[');
      }
      let mut builder = capnp::message::Builder::new_default();
      let root = builder.init_root::<self_struct::Builder<'_>>();
      let Err(e) = json::Codec::new().decode(&json, root) else {
        panic!("expected depth {depth} to be rejected");
      };
      assert_eq!(e.extra, "Recursion limit exceeded while parsing JSON");
    }
    Ok(())
  }

  /// The limit is configurable in both directions.
  #[test]
  fn recursion_limit_is_configurable() -> capnp::Result<()> {
    use crate::json_test_capnp::self_struct;

    let json = r#"{"inner":{"inner":{"value":7}}}"#;

    // Three levels of object nesting; a limit of 2 rejects it.
    let codec = json::Codec::new_with_options(json::CodecOptions {
      recursion_limit: 2,
      ..Default::default()
    });
    let mut builder = capnp::message::Builder::new_default();
    let root = builder.init_root::<self_struct::Builder<'_>>();
    assert!(codec.decode(json, root).is_err());

    // A limit of 8 accepts it.
    let codec = json::Codec::new_with_options(json::CodecOptions {
      recursion_limit: 8,
      ..Default::default()
    });
    let mut builder = capnp::message::Builder::new_default();
    let mut root = builder.init_root::<self_struct::Builder<'_>>();
    codec.decode(json, root.reborrow())?;
    assert_eq!(
      root
        .reborrow_as_reader()
        .get_inner()?
        .get_inner()?
        .get_value(),
      7
    );
    Ok(())
  }

  /// `to_json` and `from_json` share one cached codec per thread rather than
  /// building one per call, so exercise them from several threads at once:
  /// each thread must warm its own cache and produce the same answer.
  #[test]
  fn convenience_api_is_usable_from_many_threads() {
    use crate::test_capnp::test_json_types;

    let expected = {
      let mut builder = message::Builder::new_default();
      let mut root: test_json_types::Builder<'_> = builder.init_root();
      root.set_int8_field(-8);
      root.set_text_field("hello");
      json::to_json(root.reborrow_as_reader()).unwrap()
    };

    let (workers, iterations) = if cfg!(miri) { (2, 5) } else { (8, 200) };
    let threads: Vec<_> = (0..workers)
      .map(|_| {
        let expected = expected.clone();
        std::thread::spawn(move || {
          for _ in 0..iterations {
            let mut builder = message::Builder::new_default();
            let mut root: test_json_types::Builder<'_> = builder.init_root();
            root.set_int8_field(-8);
            root.set_text_field("hello");
            assert_eq!(
              json::to_json(root.reborrow_as_reader()).unwrap(),
              expected
            );

            let mut rt = message::Builder::new_default();
            let mut rt_root: test_json_types::Builder<'_> = rt.init_root();
            json::from_json(&expected, rt_root.reborrow()).unwrap();
            assert_eq!(rt_root.reborrow_as_reader().get_int8_field(), -8);
          }
        })
      })
      .collect();

    for t in threads {
      t.join().expect("worker thread panicked");
    }
  }
}
