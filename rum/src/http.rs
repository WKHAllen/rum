//! General HTTP-related types.

use hyper::Method;

/// An HTTP status code.
#[allow(missing_docs)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum StatusCode {
    // Informational responses (100 – 199)
    Continue = 100,
    SwitchingProtocols = 101,
    Processing = 102,
    EarlyHints = 103,

    // Successful responses (200 – 299)
    #[default]
    Ok = 200,
    Created = 201,
    Accepted = 202,
    NonAuthoritativeInformation = 203,
    NoContent = 204,
    ResetContent = 205,
    PartialContent = 206,
    MultiStatus = 207,
    AlreadyReported = 208,
    IMUsed = 226,

    // Redirection messages (300 – 399)
    MultipleChoices = 300,
    MovedPermanently = 301,
    Found = 302,
    SeeOther = 303,
    NotModified = 304,
    TemporaryRedirect = 307,
    PermanentRedirect = 308,

    // Client error responses (400 – 499)
    BadRequest = 400,
    Unauthorized = 401,
    PaymentRequired = 402,
    Forbidden = 403,
    NotFound = 404,
    MethodNotAllowed = 405,
    NotAcceptable = 406,
    ProxyAuthenticationRequired = 407,
    RequestTimeout = 408,
    Conflict = 409,
    Gone = 410,
    LengthRequired = 411,
    PreconditionFailed = 412,
    PayloadTooLarge = 413,
    URITooLong = 414,
    UnsupportedMediaType = 415,
    RangeNotSatisfiable = 416,
    ExpectationFailed = 417,
    ImATeapot = 418,
    MisdirectedRequest = 421,
    UnprocessableContent = 422,
    Locked = 423,
    FailedDependency = 424,
    TooEarly = 425,
    UpgradeRequired = 426,
    PreconditionRequired = 428,
    TooManyRequests = 429,
    RequestHeaderFieldsTooLarge = 431,
    UnavailableForLegalReasons = 451,

    // Server error responses (500 – 599)
    InternalServerError = 500,
    NotImplemented = 501,
    BadGateway = 502,
    ServiceUnavailable = 503,
    GatewayTimeout = 504,
    HTTPVersionNotSupported = 505,
    VariantAlsoNegotiates = 506,
    InsufficientStorage = 507,
    LoopDetected = 508,
    NotExtended = 510,
    NetworkAuthenticationRequired = 511,
}

impl StatusCode {
    /// Returns the status code number.
    pub fn code(&self) -> u16 {
        *self as u16
    }

    /// Returns the status code name.
    pub fn message(&self) -> &'static str {
        match self {
            Self::Continue => "Continue",
            Self::SwitchingProtocols => "Switching Protocols",
            Self::Processing => "Processing",
            Self::EarlyHints => "Early Hints",
            Self::Ok => "OK",
            Self::Created => "Created",
            Self::Accepted => "Accepted",
            Self::NonAuthoritativeInformation => "Non-Authoritative Information",
            Self::NoContent => "No Content",
            Self::ResetContent => "Reset Content",
            Self::PartialContent => "Partial Content",
            Self::MultiStatus => "Multi-Status",
            Self::AlreadyReported => "Already Reported",
            Self::IMUsed => "IM Used",
            Self::MultipleChoices => "Multiple Choices",
            Self::MovedPermanently => "Moved Permanently",
            Self::Found => "Found",
            Self::SeeOther => "See Other",
            Self::NotModified => "Not Modified",
            Self::TemporaryRedirect => "Temporary Redirect",
            Self::PermanentRedirect => "Permanent Redirect",
            Self::BadRequest => "Bad Request",
            Self::Unauthorized => "Unauthorized",
            Self::PaymentRequired => "Payment Required",
            Self::Forbidden => "Forbidden",
            Self::NotFound => "Not Found",
            Self::MethodNotAllowed => "Method Not Allowed",
            Self::NotAcceptable => "Not Acceptable",
            Self::ProxyAuthenticationRequired => "Proxy Authentication Required",
            Self::RequestTimeout => "Request Timeout",
            Self::Conflict => "Conflict",
            Self::Gone => "Gone",
            Self::LengthRequired => "Length Required",
            Self::PreconditionFailed => "Precondition Failed",
            Self::PayloadTooLarge => "Payload Too Large",
            Self::URITooLong => "URI Too Long",
            Self::UnsupportedMediaType => "Unsupported Media Type",
            Self::RangeNotSatisfiable => "Range Not Satisfiable",
            Self::ExpectationFailed => "Expectation Failed",
            Self::ImATeapot => "I'm a teapot",
            Self::MisdirectedRequest => "Misdirected Request",
            Self::UnprocessableContent => "Unprocessable Content",
            Self::Locked => "Locked",
            Self::FailedDependency => "Failed Dependency",
            Self::TooEarly => "Too Early",
            Self::UpgradeRequired => "Upgrade Required",
            Self::PreconditionRequired => "Precondition Required",
            Self::TooManyRequests => "Too Many Requests",
            Self::RequestHeaderFieldsTooLarge => "Request Header Fields Too Large",
            Self::UnavailableForLegalReasons => "Unavailable For Legal Reasons",
            Self::InternalServerError => "Internal Server Error",
            Self::NotImplemented => "Not Implemented",
            Self::BadGateway => "Bad Gateway",
            Self::ServiceUnavailable => "Service Unavailable",
            Self::GatewayTimeout => "Gateway Timeout",
            Self::HTTPVersionNotSupported => "HTTP Version Not Supported",
            Self::VariantAlsoNegotiates => "Variant Also Negotiates",
            Self::InsufficientStorage => "Insufficient Storage",
            Self::LoopDetected => "Loop Detected",
            Self::NotExtended => "Not Extended",
            Self::NetworkAuthenticationRequired => "Network Authentication Required",
        }
    }
}

/// An HTTP request method.
#[allow(missing_docs)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HttpMethod {
    Get,
    Head,
    Post,
    Put,
    Delete,
    Connect,
    Options,
    Trace,
    Patch,
}

impl From<&Method> for HttpMethod {
    fn from(value: &Method) -> Self {
        match *value {
            Method::GET => Self::Get,
            Method::HEAD => Self::Head,
            Method::POST => Self::Post,
            Method::PUT => Self::Put,
            Method::DELETE => Self::Delete,
            Method::CONNECT => Self::Connect,
            Method::OPTIONS => Self::Options,
            Method::TRACE => Self::Trace,
            Method::PATCH => Self::Patch,
            _ => unreachable!(),
        }
    }
}
