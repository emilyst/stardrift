// This test should fail to compile because unit structs are not supported
use stardrift_macros::ConfigDefaults;

#[derive(ConfigDefaults)]
struct UnitStruct;

fn main() {}
