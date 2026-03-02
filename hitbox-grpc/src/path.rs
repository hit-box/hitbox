//! gRPC path parsing.
//!
//! Parses gRPC request URI paths of the form `/{service}/{method}`.
//!
//! # Examples
//!
//! ```
//! use hitbox_grpc::path::GrpcPath;
//!
//! let path = GrpcPath::parse("/my.package.UserService/GetUser").unwrap();
//! assert_eq!(path.service(), "my.package.UserService");
//! assert_eq!(path.method(), "GetUser");
//! ```

/// A parsed gRPC request path.
///
/// gRPC paths follow the format `/{fully.qualified.ServiceName}/{MethodName}`.
/// The service name typically includes the protobuf package (e.g., `my.pkg.UserService`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrpcPath {
    full: String,
    service_end: usize,
}

impl GrpcPath {
    /// Parses a URI path into a `GrpcPath`.
    ///
    /// Expects the format `/{service}/{method}`. Returns `None` if:
    /// - The path doesn't start with `/`
    /// - The path doesn't contain exactly two segments
    /// - Either segment is empty
    pub fn parse(uri_path: &str) -> Option<Self> {
        let path = uri_path.strip_prefix('/')?;

        let slash_pos = path.find('/')?;
        let service = &path[..slash_pos];
        let method = &path[slash_pos + 1..];

        if service.is_empty() || method.is_empty() {
            return None;
        }

        // Reject trailing slashes (would mean method contains '/')
        if method.contains('/') {
            return None;
        }

        Some(GrpcPath {
            full: path.to_string(),
            service_end: slash_pos,
        })
    }

    /// Returns the full path without the leading `/`.
    pub fn full(&self) -> &str {
        &self.full
    }

    /// Returns the service name (e.g., `"my.package.UserService"`).
    pub fn service(&self) -> &str {
        &self.full[..self.service_end]
    }

    /// Returns the method name (e.g., `"GetUser"`).
    pub fn method(&self) -> &str {
        &self.full[self.service_end + 1..]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_standard_grpc_path() {
        let path = GrpcPath::parse("/my.package.UserService/GetUser").unwrap();
        assert_eq!(path.service(), "my.package.UserService");
        assert_eq!(path.method(), "GetUser");
        assert_eq!(path.full(), "my.package.UserService/GetUser");
    }

    #[test]
    fn parse_simple_service() {
        let path = GrpcPath::parse("/Greeter/SayHello").unwrap();
        assert_eq!(path.service(), "Greeter");
        assert_eq!(path.method(), "SayHello");
    }

    #[test]
    fn reject_no_leading_slash() {
        assert!(GrpcPath::parse("service/method").is_none());
    }

    #[test]
    fn reject_empty_service() {
        assert!(GrpcPath::parse("//method").is_none());
    }

    #[test]
    fn reject_empty_method() {
        assert!(GrpcPath::parse("/service/").is_none());
    }

    #[test]
    fn reject_single_segment() {
        assert!(GrpcPath::parse("/service").is_none());
    }

    #[test]
    fn reject_trailing_slash() {
        assert!(GrpcPath::parse("/service/method/extra").is_none());
    }

    #[test]
    fn reject_empty_path() {
        assert!(GrpcPath::parse("").is_none());
        assert!(GrpcPath::parse("/").is_none());
    }
}
