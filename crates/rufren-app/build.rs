use std::{env, fs, path::PathBuf};
fn main() {
    println!("cargo:rerun-if-env-changed=RUFREN_DEV_CA_CERT");
    println!("cargo:rerun-if-env-changed=RUFREN_DEV_RELAY_ORIGINS");
    let certificate = match env::var("RUFREN_DEV_CA_CERT") {
        Ok(path) => {
            println!("cargo:rerun-if-changed={path}");
            let bytes =
                fs::read(path).expect("Cannot read explicit private development CA certificate");
            assert!(
                bytes.len() <= 65_536,
                "Development CA certificate is too large"
            );
            let text = std::str::from_utf8(&bytes).expect("Development CA must be PEM");
            assert!(
                text.contains("BEGIN CERTIFICATE") && !text.contains("PRIVATE KEY"),
                "Only the public CA certificate may be embedded"
            );
            assert!(
                !env::var("RUFREN_DEV_RELAY_ORIGINS")
                    .unwrap_or_default()
                    .is_empty(),
                "Explicit relay origins are required with a development CA"
            );
            bytes
        }
        Err(_) => Vec::new(),
    };
    fs::write(
        PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("dev-relay-ca.pem"),
        certificate,
    )
    .expect("Cannot prepare embedded development CA");
}
