# Copyright (c) 2018 Cloudflare, Inc. and contributors
# Licensed under the MIT License:
#
# Permission is hereby granted, free of charge, to any person obtaining a copy
# of this software and associated documentation files (the "Software"), to deal
# in the Software without restriction, including without limitation the rights
# to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
# copies of the Software, and to permit persons to whom the Software is
# furnished to do so, subject to the following conditions:
#
# The above copyright notice and this permission notice shall be included in
# all copies or substantial portions of the Software.
#
# THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
# IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
# FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
# AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
# LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
# OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
# THE SOFTWARE.

@0xc9d405cf4333e4c9;

using Json = import "/capnp/compat/json.capnp";
using RustJson = import "/rust-json.capnp";

$import "/capnp/c++.capnp".namespace("capnp");

struct TestJsonAnnotations {
  someField @0 :Text $Json.name("names-can_contain!anything Really");

  aGroup :group $Json.flatten() {
    flatFoo @1 :UInt32;
    flatBar @2 :Text;
    flatBaz :group $Json.name("renamed-flatBaz") {
      hello @3 :Bool;
    }
    doubleFlat :group $Json.flatten() {
      flatQux @4 :Text;
    }
  }

  prefixedGroup :group $Json.flatten(prefix="pfx.") {
    foo @5 :Text;
    bar @6 :UInt32 $Json.name("renamed-bar");
    baz :group {
      hello @7 :Bool;
    }
    morePrefix :group $Json.flatten(prefix="xfp.") {
      qux @8 :Text;
    }
  }

  aUnion :union $Json.flatten() $Json.discriminator(name="union-type") {
    foo :group $Json.flatten() {
      fooMember @9 :Text;
      multiMember @10 :UInt32;
    }
    bar :group $Json.flatten() $Json.name("renamed-bar") {
      barMember @11 :UInt32;
      multiMember @12 :Text;
    }
  }

  dependency @13 :TestJsonAnnotations2;
  # To test that dependencies are loaded even if not flattened.

  simpleGroup :group {
    # To test that group types are loaded even if not flattened.
    grault @14 :Text $Json.name("renamed-grault");
  }

  enums @15 :List(TestJsonAnnotatedEnum);

  innerJson @16 :Json.Value;

  customFieldHandler @17 :Text;

  testBase64 @18 :Data $Json.base64;
  testHex @19 :Data $Json.hex;

  bUnion :union $Json.flatten() $Json.discriminator(valueName="bValue") {
    foo @20 :Text;
    bar @21 :UInt32 $Json.name("renamed-bar");
  }

  externalUnion @22 :TestJsonAnnotations3;

  unionWithVoid :union $Json.discriminator(name="type") {
    intValue @23 :UInt32;
    voidValue @24 :Void;
    textValue @25 :Text;
  }
}

struct TestJsonAnnotations2 {
  foo @0 :Text $Json.name("renamed-foo");
  cycle @1 :TestJsonAnnotations;
}

struct TestJsonAnnotations3 $Json.discriminator(name="type") {
  union {
    foo @0 :UInt32;
    bar @1 :TestFlattenedStruct $Json.flatten();
  }
}

struct TestFlattenedStruct {
  value @0 :Text;
}

enum TestJsonAnnotatedEnum {
  foo @0;
  bar @1 $Json.name("renamed-bar");
  baz @2 $Json.name("renamed-baz");
  qux @3;
}

struct TestBase64Union {
  union {
    foo @0 :Data $Json.base64;
    bar @1 :Text;
  }
}

struct TestRenamedAnonUnion {
  union {
    foo @0 :Data $Json.base64 $Json.name("renamed-foo");
    bar @1 :Text;
  }
}

struct NestedHex {
  dataAllTheWayDown @0 :List(List(Data)) $Json.hex;
}

struct UnnamedDiscriminator {
  baz :union $Json.discriminator() {
    foo @0 :Text;
    bar @1 :UInt32;
  }

  sbaz :union $Json.discriminator() $Json.flatten() {
    sfoo @2 :Text;
    sbar @3 :UInt32;
  }
}

struct NamedDiscriminator {
  baz :union $Json.discriminator(name="baz_kind") {
    foo @0 :Text;
    bar @1 :UInt32;
  }

  sbaz :union $Json.discriminator(name="sbaz_kind") $Json.flatten() {
    sfoo @2 :Text;
    sbar @3 :UInt32;
  }
}

struct TestAnyPointer {
  anyPointerField @0 :AnyPointer;
}

struct TestGeneric(T) {
  anyPointerField @0 :AnyPointer;
  genericField @1 :T;
}

struct TestCustomCodec $RustJson.codec("TestCodec") {
  something @0 :AnyPointer;
  # No codec is registered for this AnyPointer so it should fail, but 'TestCodec' being registered
  # overrides the entire struct codec
}

struct StructWithCustomCodec {
  structLevel @0 :TestCustomCodec;
  fieldLevel @1 :Text $RustJson.codec("TestFieldCodec");
}

struct CyclicFlatten {
  # A struct that flattens a field of its own type. The schema is legal, but
  # decoding recurses on the *schema* while staying at the same depth in the
  # JSON, so the parser's nesting limit cannot bound it. The C++ codec rejects
  # this outright ("cyclic JSON flattening detected").
  inner @0 :CyclicFlatten $Json.flatten(prefix="i.");
  value @1 :UInt16;
}

struct SelfList {
  # Self-reference through a list. Each level of nesting costs two levels of
  # JSON nesting (object + array), so the parser limit bounds this one.
  children @0 :List(SelfList);
  value @1 :UInt16;
}

struct SelfStruct {
  # Plain self-reference; one level of JSON nesting per level of schema.
  inner @0 :SelfStruct;
  value @1 :UInt16;
}

# Cyclic-flattening cases. The expected verdict against each is the one the
# C++ codec gives (checked with `capnp convert`), since the point of the check
# is that both implementations reject the same schemas.

struct FlattenThroughGroup {
  # Cyclic: C++ always loads a group's handler, flattened or not, so the group
  # is an edge even though it nests in the JSON.
  g :group {
    a @0 :FlattenThroughGroup $Json.flatten();
  }
}

struct MutualFlattenA {
  # Cyclic: A -> B -> A, both flattened.
  b @0 :MutualFlattenB $Json.flatten();
}

struct MutualFlattenB {
  a @0 :MutualFlattenA $Json.flatten();
}

struct MutualOneFlatA {
  # Not cyclic: the return edge is a plain struct field, which nests.
  b @0 :MutualOneFlatB $Json.flatten();
  value @1 :UInt16;
}

struct MutualOneFlatB {
  a @0 :MutualOneFlatA;
  other @1 :UInt16;
}

struct ReferencesCyclic {
  # Not itself cyclic, but reaches a cyclic type through a plain field. C++
  # loads handlers for the whole dependency graph, so this is rejected too.
  x @0 :CyclicFlatten;
}

struct ReferencesCyclicViaList {
  x @0 :List(CyclicFlatten);
}
