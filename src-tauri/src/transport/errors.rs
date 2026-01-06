use std::fmt;

#[derive(Debug)]
pub enum TransportError {
    Disconnected,
    Protocol(String),
    InvalidConfig(String),
    Io(String),
}

impl fmt::Display for TransportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransportError::Disconnected => write!(f, "transport disconnected"),
            TransportError::Protocol(message) => write!(f, "protocol error: {message}"),
            TransportError::InvalidConfig(message) => write!(f, "invalid config: {message}"),
            TransportError::Io(message) => write!(f, "io error: {message}"),
        }
    }
}

impl std::error::Error for TransportError {}

impl From<std::io::Error> for TransportError {
    fn from(error: std::io::Error) -> Self {
        TransportError::Io(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::TransportError;
    use std::io;

    /// Each error variant renders the expected display text.
    #[test]
    fn display_messages_are_stable() {
        // Arrange
        // Act
        assert_eq!(
            TransportError::Disconnected.to_string(),
            "transport disconnected"
        );
        assert_eq!(
            TransportError::Protocol("oops".to_string()).to_string(),
            "protocol error: oops"
        );
        assert_eq!(
            TransportError::InvalidConfig("missing".to_string()).to_string(),
            "invalid config: missing"
        );
        assert_eq!(
            TransportError::Io("disk".to_string()).to_string(),
            "io error: disk"
        );
        // Assert
    }

    /// Mapping an IO error yields the `Io` transport variant.
    #[test]
    fn from_io_error_maps_to_io_variant() {
        // Arrange
        let error = io::Error::new(io::ErrorKind::Other, "broken");
        // Act
        let mapped = TransportError::from(error);
        // Assert
        assert_eq!(mapped.to_string(), "io error: broken");
    }
}
