use chrono::Local;

fn main() {
    let build_date = Local::now().format("%Y-%m-%d").to_string();
    println!("cargo:rustc-env=BUILD_DATE={}", build_date);
    println!("cargo:rerun-if-changed=build.rs");
}
