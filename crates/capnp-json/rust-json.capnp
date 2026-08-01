# This file contains annotations recognised by the rust capnp-json codec.

# To use this file you will need to make sure that it is included in the directories searched by
# `capnp compile`. An easy way to do this is to simply copy it to your project alongside your own
# schema files.

@0xf955e504bf781ac6;

annotation codec @0xabc0b6598cbf14e8 (field, struct) :Text;
# A named `FieldCodec` which is used to encode/decode the annotated field or struct. The name must
# be one of the names registered in the `capnp_json::Codec` used to encode/decode the message. 
