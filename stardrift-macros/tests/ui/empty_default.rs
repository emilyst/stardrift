// This test should fail to compile because the default attribute is empty
use stardrift_macros::ConfigDefaults;

#[derive(ConfigDefaults)]
struct EmptyDefault {
    #[default()] // Empty!
    pub field1: i32,
}

fn main() {}
