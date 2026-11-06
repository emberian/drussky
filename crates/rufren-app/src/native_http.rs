//! Native relay client. A development CA is trusted only for explicit compiled HTTPS origins.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn client(
    endpoint: &str,
    timeout: std::time::Duration,
) -> Result<reqwest::Client, String> {
    let url = reqwest::Url::parse(endpoint).map_err(|_| "Invalid private relay URL")?;
    let mut builder = reqwest::Client::builder()
        .timeout(timeout)
        .connect_timeout(std::time::Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none());
    const CA: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/dev-relay-ca.pem"));
    let origins = option_env!("RUFREN_DEV_RELAY_ORIGINS").unwrap_or_default();
    if !CA.is_empty()
        && url.scheme() == "https"
        && origins
            .split(',')
            .any(|allowed| allowed.trim() == url.origin().ascii_serialization())
    {
        let certificate = reqwest::Certificate::from_pem(CA)
            .map_err(|_| "The embedded private relay certificate is invalid")?;
        // No certificate/hostname verification bypass. Other origins keep their normal roots.
        builder = builder
            .tls_built_in_root_certs(false)
            .add_root_certificate(certificate);
    }
    builder
        .build()
        .map_err(|_| "Could not initialize the private relay connection".into())
}
