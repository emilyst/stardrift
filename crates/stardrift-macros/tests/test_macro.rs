use stardrift_macros::ConfigDefaults;

#[test]
fn test_derive_macro_compiles() {
    // This test verifies that the macro compiles correctly
    #[derive(ConfigDefaults, Clone, Debug, PartialEq)]
    struct TestConfig {
        #[default(42)]
        pub field1: i32,

        #[default("hello")]
        pub field2: String,

        #[default(None)]
        pub field3: Option<u64>,

        #[default(vec![1, 2, 3])]
        pub field4: Vec<i32>,
    }

    // Test that Default is implemented
    let config = TestConfig::default();
    assert_eq!(config.field1, 42);
    assert_eq!(config.field2, "hello");
    assert_eq!(config.field3, None);
    assert_eq!(config.field4, vec![1, 2, 3]);
}

#[test]
fn test_complex_expressions() {
    #[derive(ConfigDefaults, Clone, Debug, PartialEq)]
    struct MathConfig {
        #[default(1.0 / 3.0)]
        pub fraction: f64,

        #[default(10 * 10)]
        pub computed: i32,

        #[default(true && true)]
        pub boolean: bool,
    }

    let config = MathConfig::default();
    assert!((config.fraction - 0.3333333333333333).abs() < 0.0001);
    assert_eq!(config.computed, 100);
    assert_eq!(config.boolean, true);
}

#[test]
fn test_nested_types() {
    #[derive(ConfigDefaults, Clone, Debug, PartialEq)]
    struct InnerConfig {
        #[default(100)]
        pub value: i32,
    }

    #[derive(ConfigDefaults, Clone, Debug, PartialEq)]
    struct OuterConfig {
        #[default(InnerConfig::default())]
        pub inner: InnerConfig,

        #[default(200)]
        pub other: i32,
    }

    let config = OuterConfig::default();
    assert_eq!(config.inner.value, 100);
    assert_eq!(config.other, 200);
}

#[test]
fn test_generic_struct() {
    #[derive(ConfigDefaults, Clone, Debug, PartialEq)]
    struct GenericConfig<T: Clone + Default> {
        #[default(T::default())]
        pub value: T,

        #[default(42)]
        pub number: i32,
    }

    let config: GenericConfig<String> = GenericConfig::default();
    assert_eq!(config.value, String::default());
    assert_eq!(config.number, 42);
}

#[test]
fn test_with_serde() {
    // Test that it works with serde derives
    use serde::{Deserialize, Serialize};

    #[derive(ConfigDefaults, Serialize, Deserialize, Clone, Debug, PartialEq)]
    #[serde(default)]
    struct SerdeConfig {
        #[default(999)]
        pub value: i32,

        #[default("test")]
        pub name: String,
    }

    let config = SerdeConfig::default();
    assert_eq!(config.value, 999);
    assert_eq!(config.name, "test");

    // Test serialization round-trip
    let json = serde_json::to_string(&config).unwrap();
    let deserialized: SerdeConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(config, deserialized);

    // Test partial deserialization
    let partial_json = r#"{"value": 500}"#;
    let partial: SerdeConfig = serde_json::from_str(partial_json).unwrap();
    assert_eq!(partial.value, 500);
    assert_eq!(partial.name, "test"); // Uses default
}

#[test]
fn test_numeric_type_inference() {
    // Test that Rust's type inference handles numeric literals correctly
    #[derive(ConfigDefaults, Clone, Debug, PartialEq)]
    struct NumericConfig {
        #[default(100)] // Type inference makes this usize
        pub count: usize,

        #[default(42)] // Type inference makes this u32
        pub id: u32,

        #[default(255)] // Type inference makes this u8
        pub byte_value: u8,

        #[default(1000)] // Type inference makes this i64
        pub large_number: i64,

        #[default(3.14)] // Type inference makes this f64
        pub pi: f64,

        #[default(2.5)] // Type inference makes this f32
        pub ratio: f32,
    }

    let config = NumericConfig::default();
    assert_eq!(config.count, 100);
    assert_eq!(config.id, 42);
    assert_eq!(config.byte_value, 255);
    assert_eq!(config.large_number, 1000);
    assert!((config.pi - 3.14).abs() < 0.001);
    assert!((config.ratio - 2.5).abs() < 0.001);
}

#[test]
fn test_string_conversions() {
    // Test that String fields automatically convert from &str
    #[derive(ConfigDefaults, Clone, Debug, PartialEq)]
    struct StringConfig {
        #[default("hello")]
        pub greeting: String,

        #[default("world")]
        pub target: String,

        #[default(String::from("explicit"))]
        pub explicit: String,

        #[default(format!("formatted {}", 123))]
        pub formatted: String,
    }

    let config = StringConfig::default();
    assert_eq!(config.greeting, "hello");
    assert_eq!(config.target, "world");
    assert_eq!(config.explicit, "explicit");
    assert_eq!(config.formatted, "formatted 123");
}

#[test]
fn test_enum_defaults() {
    #[derive(Clone, Debug, PartialEq)]
    enum TestEnum {
        VariantA,
        #[allow(dead_code)]
        VariantB,
    }

    #[derive(ConfigDefaults, Clone, Debug, PartialEq)]
    struct EnumConfig {
        #[default(TestEnum::VariantA)]
        pub variant: TestEnum,
    }

    let config = EnumConfig::default();
    assert_eq!(config.variant, TestEnum::VariantA);
}
