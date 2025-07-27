use chrono::Local;

fn main() {
    // Set build date for version attribution
    let build_date = Local::now().format("%Y-%m-%d").to_string();
    println!("cargo:rustc-env=BUILD_DATE={build_date}");

    // Note: Release binaries built through GitHub Actions include
    // cryptographic build provenance attestations for supply chain security.
    // See README.md for verification instructions.

    println!("cargo:rerun-if-changed=build.rs");
}
