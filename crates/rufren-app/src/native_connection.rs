//! First-launch private relay bootstrap; Apple Keychain retains it across app relaunches.
use serde::{Deserialize, Serialize};

const SERVICE: &str = "org.ember.rufren.private-relay";
const ACCOUNT: &str = "connection-v1";
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NativeConnection {
    version: u32,
    pub(crate) endpoint: String,
    pub(crate) token: String,
}
impl NativeConnection {
    fn validate(&self) -> Result<(), String> {
        let url = reqwest::Url::parse(&self.endpoint)
            .map_err(|_| "The saved private relay URL is invalid")?;
        if self.version != 1
            || url.scheme() != "https"
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || self.token.len() < 32
            || self.token.len() > 4096
            || self.token.chars().any(char::is_control)
        {
            return Err(
                "Private connection setup requires a full HTTPS URL and a valid private sync key"
                    .into(),
            );
        }
        Ok(())
    }
}
#[cfg(any(target_os = "ios", target_os = "macos"))]
fn save(connection: &NativeConnection) -> Result<(), String> {
    use security_framework::{
        access_control::{ProtectionMode, SecAccessControl},
        passwords::{self, PasswordOptions},
    };
    let bytes =
        serde_json::to_vec(connection).map_err(|_| "Could not prepare private connection")?;
    // Existing items retain their accessibility policy when their value is updated.
    match passwords::get_generic_password(SERVICE, ACCOUNT) {
        Ok(_) => passwords::set_generic_password(SERVICE, ACCOUNT, &bytes),
        Err(error) if error.code() == -25300 => {
            let protection = SecAccessControl::create_with_protection(
                Some(ProtectionMode::AccessibleWhenUnlockedThisDeviceOnly),
                0,
            )
            .map_err(|_| "Could not protect the private connection in Keychain")?;
            let mut options = PasswordOptions::new_generic_password(SERVICE, ACCOUNT);
            options.set_access_synchronized(Some(false));
            options.set_access_control(protection);
            passwords::set_generic_password_options(&bytes, options)
        }
        Err(_) => return Err("Unlock this device to save its private relay connection".into()),
    }
    .map_err(|_| "Could not save the private relay connection in this device's Keychain".into())
}
/// Called only by production AppState::open; isolated repository/model tests never read Keychain.
pub(crate) fn load() -> Result<Option<NativeConnection>, String> {
    let endpoint = std::env::var("RUFREN_RELAY_URL").ok();
    let token = std::env::var("RUFREN_SYNC_TOKEN").ok();
    if endpoint.is_some() || token.is_some() {
        let connection = NativeConnection {
            version: 1,
            endpoint: endpoint
                .ok_or("First-launch setup is missing its private relay URL")?
                .trim()
                .trim_end_matches('/')
                .into(),
            token: token.ok_or("First-launch setup is missing its private relay key")?,
        };
        connection.validate()?;
        #[cfg(any(target_os = "ios", target_os = "macos"))]
        save(&connection)?;
        #[cfg(not(any(target_os = "ios", target_os = "macos")))]
        return Err("Secure relay bootstrap is available in the Apple native app".into());
        #[cfg(any(target_os = "ios", target_os = "macos"))]
        return Ok(Some(connection));
    }
    #[cfg(any(target_os = "ios", target_os = "macos"))]
    {
        match security_framework::passwords::get_generic_password(SERVICE, ACCOUNT) {
            Ok(bytes) => {
                let connection: NativeConnection = serde_json::from_slice(&bytes)
                    .map_err(|_| "The saved private connection has an unsupported format")?;
                connection.validate()?;
                Ok(Some(connection))
            }
            Err(error) if error.code() == -25300 => Ok(None),
            Err(_) => Err("Unlock this device to restore its private relay connection".into()),
        }
    }
    #[cfg(not(any(target_os = "ios", target_os = "macos")))]
    {
        Ok(None)
    }
}

/// Explicit Disconnect removes the installed credential as well as in-memory state.
pub(crate) fn forget() -> Result<(), String> {
    #[cfg(any(target_os = "ios", target_os = "macos"))]
    {
        match security_framework::passwords::delete_generic_password(SERVICE, ACCOUNT) {
            Ok(()) => Ok(()),
            Err(error) if error.code() == -25300 => Ok(()),
            Err(_) => Err("Unlock this device to forget its private relay connection".into()),
        }
    }
    #[cfg(not(any(target_os = "ios", target_os = "macos")))]
    {
        Ok(())
    }
}
