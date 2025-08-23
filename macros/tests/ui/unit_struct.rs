// This test should fail to compile because unit structs are not supported
use macros::ConfigDefaults;

#[derive(ConfigDefaults)]
struct UnitStruct;

fn main() {}
