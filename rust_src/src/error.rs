#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum ErrorCode {
    Success = 0,
    InvalidParameter = 1,
    WorkspaceNotSet = 2,
    PathNotFound = 3,
    IoError = 4,
    NetworkError = 5,
    JsonError = 6,
    SessionError = 7,
    RequestError = 8,
    ResponseError = 9,
    LogError = 10,
    Unknown = 99,
}

impl ErrorCode {
    pub fn as_i32(self) -> i32 {
        self as i32
    }
    
    pub fn from_i32(code: i32) -> Self {
        match code {
            0 => ErrorCode::Success,
            1 => ErrorCode::InvalidParameter,
            2 => ErrorCode::WorkspaceNotSet,
            3 => ErrorCode::PathNotFound,
            4 => ErrorCode::IoError,
            5 => ErrorCode::NetworkError,
            6 => ErrorCode::JsonError,
            7 => ErrorCode::SessionError,
            8 => ErrorCode::RequestError,
            9 => ErrorCode::ResponseError,
            10 => ErrorCode::LogError,
            _ => ErrorCode::Unknown,
        }
    }
}

#[derive(Debug, Clone)]
pub struct VfsError {
    pub code: ErrorCode,
    pub message: String,
}

impl VfsError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
    
    pub fn success() -> Self {
        Self::new(ErrorCode::Success, "Success")
    }
    
    pub fn is_success(&self) -> bool {
        self.code == ErrorCode::Success
    }
}

impl std::fmt::Display for VfsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "VfsError(code={}, message={})", self.code.as_i32(), self.message)
    }
}

impl std::error::Error for VfsError {}

pub type VfsResult<T> = Result<T, VfsError>;
