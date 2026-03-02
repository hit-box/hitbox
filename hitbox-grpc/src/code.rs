//! gRPC status codes.
//!
//! The 17 standard gRPC status codes as defined in the
//! [gRPC specification](https://grpc.github.io/grpc/core/md_doc_statuscodes.html).

/// gRPC status code.
///
/// Represents the 17 standard gRPC status codes. These are transmitted in the
/// `grpc-status` trailer (or header for Trailers-Only responses).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum GrpcCode {
    /// The operation completed successfully.
    Ok = 0,
    /// The operation was cancelled.
    Cancelled = 1,
    /// Unknown error.
    Unknown = 2,
    /// Client specified an invalid argument.
    InvalidArgument = 3,
    /// Deadline expired before operation could complete.
    DeadlineExceeded = 4,
    /// Some requested entity was not found.
    NotFound = 5,
    /// The entity that a client attempted to create already exists.
    AlreadyExists = 6,
    /// The caller does not have permission.
    PermissionDenied = 7,
    /// Some resource has been exhausted.
    ResourceExhausted = 8,
    /// Operation was rejected because the system is not in a state required for execution.
    FailedPrecondition = 9,
    /// The operation was aborted.
    Aborted = 10,
    /// Operation was attempted past the valid range.
    OutOfRange = 11,
    /// Operation is not implemented.
    Unimplemented = 12,
    /// Internal error.
    Internal = 13,
    /// The service is currently unavailable.
    Unavailable = 14,
    /// Unrecoverable data loss or corruption.
    DataLoss = 15,
    /// The request does not have valid authentication credentials.
    Unauthenticated = 16,
}

impl GrpcCode {
    /// Parses a gRPC status code from its byte representation in the `grpc-status` header.
    ///
    /// The value should be an ASCII decimal string like `"0"` through `"16"`.
    ///
    /// Returns `None` for invalid or out-of-range values.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        match bytes {
            b"0" => Some(GrpcCode::Ok),
            b"1" => Some(GrpcCode::Cancelled),
            b"2" => Some(GrpcCode::Unknown),
            b"3" => Some(GrpcCode::InvalidArgument),
            b"4" => Some(GrpcCode::DeadlineExceeded),
            b"5" => Some(GrpcCode::NotFound),
            b"6" => Some(GrpcCode::AlreadyExists),
            b"7" => Some(GrpcCode::PermissionDenied),
            b"8" => Some(GrpcCode::ResourceExhausted),
            b"9" => Some(GrpcCode::FailedPrecondition),
            b"10" => Some(GrpcCode::Aborted),
            b"11" => Some(GrpcCode::OutOfRange),
            b"12" => Some(GrpcCode::Unimplemented),
            b"13" => Some(GrpcCode::Internal),
            b"14" => Some(GrpcCode::Unavailable),
            b"15" => Some(GrpcCode::DataLoss),
            b"16" => Some(GrpcCode::Unauthenticated),
            _ => None,
        }
    }

    /// Returns `true` if this is `GrpcCode::Ok`.
    pub fn is_ok(self) -> bool {
        self == GrpcCode::Ok
    }

    /// Returns the numeric value of this status code.
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

impl std::fmt::Display for GrpcCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            GrpcCode::Ok => "OK",
            GrpcCode::Cancelled => "CANCELLED",
            GrpcCode::Unknown => "UNKNOWN",
            GrpcCode::InvalidArgument => "INVALID_ARGUMENT",
            GrpcCode::DeadlineExceeded => "DEADLINE_EXCEEDED",
            GrpcCode::NotFound => "NOT_FOUND",
            GrpcCode::AlreadyExists => "ALREADY_EXISTS",
            GrpcCode::PermissionDenied => "PERMISSION_DENIED",
            GrpcCode::ResourceExhausted => "RESOURCE_EXHAUSTED",
            GrpcCode::FailedPrecondition => "FAILED_PRECONDITION",
            GrpcCode::Aborted => "ABORTED",
            GrpcCode::OutOfRange => "OUT_OF_RANGE",
            GrpcCode::Unimplemented => "UNIMPLEMENTED",
            GrpcCode::Internal => "INTERNAL",
            GrpcCode::Unavailable => "UNAVAILABLE",
            GrpcCode::DataLoss => "DATA_LOSS",
            GrpcCode::Unauthenticated => "UNAUTHENTICATED",
        };
        write!(f, "{name}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_codes() {
        assert_eq!(GrpcCode::from_bytes(b"0"), Some(GrpcCode::Ok));
        assert_eq!(GrpcCode::from_bytes(b"1"), Some(GrpcCode::Cancelled));
        assert_eq!(GrpcCode::from_bytes(b"16"), Some(GrpcCode::Unauthenticated));
    }

    #[test]
    fn parse_invalid_codes() {
        assert_eq!(GrpcCode::from_bytes(b"17"), None);
        assert_eq!(GrpcCode::from_bytes(b""), None);
        assert_eq!(GrpcCode::from_bytes(b"-1"), None);
        assert_eq!(GrpcCode::from_bytes(b"abc"), None);
        assert_eq!(GrpcCode::from_bytes(b"00"), None);
    }

    #[test]
    fn is_ok() {
        assert!(GrpcCode::Ok.is_ok());
        assert!(!GrpcCode::Internal.is_ok());
        assert!(!GrpcCode::Unauthenticated.is_ok());
    }

    #[test]
    fn display() {
        assert_eq!(format!("{}", GrpcCode::Ok), "OK");
        assert_eq!(format!("{}", GrpcCode::NotFound), "NOT_FOUND");
    }

    #[test]
    fn as_u8() {
        assert_eq!(GrpcCode::Ok.as_u8(), 0);
        assert_eq!(GrpcCode::Unauthenticated.as_u8(), 16);
    }
}
