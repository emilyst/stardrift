// This test should fail to compile because tuple structs are not supported
use stardrift_macros::ConfigDefaults;

#[derive(ConfigDefaults)]
struct TupleStruct(i32, String);

fn main() {}
