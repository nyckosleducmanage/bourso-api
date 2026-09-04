use std::fmt;

#[derive(Debug)]
pub enum ClientError {
    InvalidCredentials,
    MfaRequired,
    QRCodeRequired(String),
    InvalidMfa,
    /// The password step succeeded but the login gives access to several
    /// identities: one must be selected before the session becomes usable.
    IdentitySelectionRequired,
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ClientError::InvalidCredentials => write!(f, "Invalid credentials"),
            ClientError::MfaRequired => write!(f, "MFA required"),
            ClientError::QRCodeRequired(msg) => write!(f, "{}", msg),
            ClientError::InvalidMfa => write!(f, "Invalid MFA"),
            ClientError::IdentitySelectionRequired => {
                write!(f, "This login gives access to several identities, one must be selected")
            }
        }
    }
}
