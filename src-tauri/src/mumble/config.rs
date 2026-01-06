#[derive(Clone, Debug)]
pub struct MumbleConfig {
    pub server: String,
    pub port: u16,
    pub username: String,
    pub password: Option<String>,
    pub cert_pem: Option<String>,
}

pub const DEFAULT_PORT: u16 = 64738;

impl MumbleConfig {
    pub fn new(server: String, port: u16, username: String) -> Self {
        Self {
            server,
            port,
            username,
            password: None,
            cert_pem: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::MumbleConfig;

    /// `new` populates required fields and leaves optional values empty.
    #[test]
    fn new_sets_defaults() {
        // Arrange
        // Act
        let config = MumbleConfig::new("example.org".to_string(), 64738, "alice".to_string());
        // Assert
        assert_eq!(config.server, "example.org");
        assert_eq!(config.port, 64738);
        assert_eq!(config.username, "alice");
        assert!(config.password.is_none());
        assert!(config.cert_pem.is_none());
    }
}
