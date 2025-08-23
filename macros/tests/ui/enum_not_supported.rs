// This test should fail to compile because enums are not supported
use macros::ConfigDefaults;

#[derive(ConfigDefaults)]
enum NotSupported {
    VariantA,
    VariantB,
}

fn main() {}
