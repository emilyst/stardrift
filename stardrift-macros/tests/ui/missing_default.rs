// This test should fail to compile because field2 is missing a default attribute
use stardrift_macros::ConfigDefaults;

#[derive(ConfigDefaults)]
struct MissingDefault {
    #[default(42)]
    pub field1: i32,

    // Missing #[default(...)]
    pub field2: String,
}

fn main() {}
