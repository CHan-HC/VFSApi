use crate::error::{ErrorCode, VfsError, VfsResult};
use crate::hilog::log_info;
use std::ffi::CString;
use std::os::raw::c_void;
use std::ptr;
use std::sync::{Arc, Mutex};

const TAG: &str = "[VFS_RCP]";

pub mod rcp_sys {
    #![allow(non_upper_case_globals)]
    #![allow(non_camel_case_types)]
    #![allow(non_snake_case)]
    #![allow(dead_code)]

    use std::os::raw::{c_char, c_void};

    pub const RCP_METHOD_GET: &[u8] = b"GET\0";
    pub const RCP_METHOD_POST: &[u8] = b"POST\0";
    pub const RCP_METHOD_PUT: &[u8] = b"PUT\0";
    pub const RCP_METHOD_DELETE: &[u8] = b"DELETE\0";

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    pub struct Rcp_Timeout {
        pub connectMs: u32,
        pub transferMs: u32,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    pub struct Rcp_DnsOverHttps {
        pub url: *const c_char,
        pub skipCertificatesValidation: bool,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    pub struct Rcp_TransferConfiguration {
        pub autoRedirect: bool,
        pub timeout: Rcp_Timeout,
        pub assumesHTTP3Capable: bool,
        pub pathPreference: u32,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    pub struct Rcp_InfoToCollect {
        pub textual: bool,
        pub incomingHeader: bool,
        pub outgoingHeader: bool,
        pub incomingData: bool,
        pub outgoingData: bool,
        pub incomingSslData: bool,
        pub outgoingSslData: bool,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    pub struct Rcp_OnDataReceiveCallback {
        pub callback: *mut c_void,
        pub usrObject: *mut c_void,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    pub struct Rcp_OnProgressCallback {
        pub callback: *mut c_void,
        pub usrObject: *mut c_void,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    pub struct Rcp_OnHeaderReceiveCallback {
        pub callback: *mut c_void,
        pub usrObject: *mut c_void,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    pub struct Rcp_OnVoidCallback {
        pub callback: *mut c_void,
        pub usrObject: *mut c_void,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    pub struct Rcp_EventsHandler {
        pub onDataReceive: Rcp_OnDataReceiveCallback,
        pub onUploadProgress: Rcp_OnProgressCallback,
        pub onDownloadProgress: Rcp_OnProgressCallback,
        pub onHeaderReceive: Rcp_OnHeaderReceiveCallback,
        pub onDataEnd: Rcp_OnVoidCallback,
        pub onCanceled: Rcp_OnVoidCallback,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    pub struct Rcp_TracingConfiguration {
        pub verbose: bool,
        pub infoToCollect: Rcp_InfoToCollect,
        pub collectTimeInfo: bool,
        pub httpEventsHandler: Rcp_EventsHandler,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    pub struct Rcp_Credential {
        pub username: *mut c_char,
        pub password: *mut c_char,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    pub struct Rcp_ServerAuthentication {
        pub credential: Rcp_Credential,
        pub authenticationType: u32,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    pub struct Rcp_Urls {
        pub url: *const c_char,
        pub next: *mut Rcp_Urls,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    pub union Rcp_ExclusionsData {
        pub urls: *mut Rcp_Urls,
        pub exclusionFunction: *mut c_void,
    }

    impl Default for Rcp_ExclusionsData {
        fn default() -> Self {
            Self { urls: std::ptr::null_mut() }
        }
    }

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    pub struct Rcp_Exclusions {
        pub type_: u32,
        pub data: Rcp_ExclusionsData,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    pub struct Rcp_CertificateAuthority {
        pub content: *mut c_char,
        pub filePath: *mut c_char,
        pub folderPath: *mut c_char,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    pub struct Rcp_ClientCertificate {
        pub content: *mut c_char,
        pub filePath: *mut c_char,
        pub key: *mut c_char,
        pub keyPassword: *mut c_char,
        pub type_: u32,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    pub struct Rcp_SecurityConfiguration {
        pub remoteValidationType: u32,
        pub certificateAuthority: Rcp_CertificateAuthority,
        pub certificate: Rcp_ClientCertificate,
        pub serverAuthentication: Rcp_ServerAuthentication,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    pub struct Rcp_WebProxy {
        pub url: *const c_char,
        pub createTunnel: u32,
        pub exclusions: Rcp_Exclusions,
        pub securityConfiguration: Rcp_SecurityConfiguration,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    pub struct Rcp_ProxyConfiguration {
        pub proxyType: u32,
        pub customProxy: Rcp_WebProxy,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    pub struct Rcp_DnsConfiguration {
        pub dnsRules: *mut c_void,
        pub dnsOverHttps: Rcp_DnsOverHttps,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    pub struct Rcp_Configuration {
        pub transferConfiguration: Rcp_TransferConfiguration,
        pub tracingConfiguration: Rcp_TracingConfiguration,
        pub proxyConfiguration: Rcp_ProxyConfiguration,
        pub dnsConfiguration: Rcp_DnsConfiguration,
        pub securityConfiguration: Rcp_SecurityConfiguration,
        pub configurationPrivate: *mut c_void,
    }

    #[repr(C)]
    pub struct Rcp_Session {
        _private: [u8; 0],
    }

    #[repr(C)]
    pub struct Rcp_Request {
        pub id: [c_char; 32],
        pub url: *mut c_char,
        pub method: *const c_char,
        pub headers: *mut Rcp_Headers,
        pub content: *mut Rcp_RequestContent,
        pub configuration: *mut Rcp_Configuration,
        pub transferRange: *mut c_void,
        pub cookies: *mut c_void,
        pub requestPrivate: *mut c_void,
    }

    #[repr(C)]
    pub struct Rcp_RequestContent {
        pub type_: u32,
        pub data: Rcp_RequestContentData,
    }

    #[repr(C)]
    pub union Rcp_RequestContentData {
        pub contentStr: Rcp_Buffer,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct Rcp_Buffer {
        pub buffer: *const c_char,
        pub length: u32,
    }

    #[repr(C)]
    pub struct Rcp_Headers {
        _private: [u8; 0],
    }

    #[repr(C)]
    pub struct Rcp_Response {
        pub request: *const Rcp_Request,
        pub effectiveUrl: *mut c_char,
        pub statusCode: u32,
        pub headers: *mut Rcp_Headers,
        pub body: Rcp_Buffer,
        pub destroyResponse: Option<unsafe extern "C" fn(*mut Rcp_Response)>,
    }

    #[repr(C)]
    pub struct Rcp_ResponseCallbackObject {
        pub callback: Option<unsafe extern "C" fn(*mut c_void, *mut Rcp_Response, u32)>,
        pub usrCtx: *mut c_void,
    }

    extern "C" {
        pub fn HMS_Rcp_CreateSession(config: *const c_void, errCode: *mut u32) -> *mut Rcp_Session;
        pub fn HMS_Rcp_CloseSession(session: *mut *mut Rcp_Session);
        pub fn HMS_Rcp_CancelSession(session: *mut Rcp_Session) -> u32;
        
        pub fn HMS_Rcp_CreateRequest(url: *const c_char) -> *mut Rcp_Request;
        pub fn HMS_Rcp_DestroyRequest(request: *mut Rcp_Request);
        
        pub fn HMS_Rcp_CreateHeaders() -> *mut Rcp_Headers;
        pub fn HMS_Rcp_DestroyHeaders(headers: *mut Rcp_Headers);
        pub fn HMS_Rcp_SetHeaderValue(headers: *mut Rcp_Headers, name: *const c_char, value: *const c_char) -> u32;
        
        pub fn HMS_Rcp_Fetch(
            session: *mut Rcp_Session,
            request: *mut Rcp_Request,
            callback: *const Rcp_ResponseCallbackObject,
        ) -> u32;
    }
}

pub struct HttpClient {
    session: *mut rcp_sys::Rcp_Session,
}

unsafe impl Send for HttpClient {}
unsafe impl Sync for HttpClient {}

impl HttpClient {
    pub async fn new() -> VfsResult<Self> {
        log_info(&format!("{} Creating HTTP client session...", TAG));
        let session = unsafe {
            let mut err_code: u32 = 0;
            let session = rcp_sys::HMS_Rcp_CreateSession(ptr::null_mut(), &mut err_code);
            if session.is_null() || err_code != 0 {
                log_info(&format!("{} Failed to create RCP session, error code: {}", TAG, err_code));
                return Err(VfsError::new(
                    ErrorCode::SessionError,
                    format!("Failed to create RCP session, error code: {}", err_code),
                ));
            }
            log_info(&format!("{} RCP session created successfully", TAG));
            session
        };
        
        Ok(Self { session })
    }
    
    pub async fn get(&self, url: &str) -> VfsResult<HttpResponse> {
        log_info(&format!("{} HTTP GET: {}", TAG, url));
        self.request(url, rcp_sys::RCP_METHOD_GET.as_ptr() as *const u8, None).await
    }
    
    pub async fn get_with_headers(&self, url: &str, headers: std::collections::HashMap<String, String>) -> VfsResult<HttpResponse> {
        log_info(&format!("{} HTTP GET with headers: {}", TAG, url));
        for (k, v) in &headers {
            log_info(&format!("{}   Header: {} = {}...", TAG, k, &v[..v.len().min(30)]));
        }
        self.request_with_headers(url, rcp_sys::RCP_METHOD_GET.as_ptr() as *const u8, None, headers).await
    }
    
    #[allow(dead_code)]
    pub async fn post(&self, url: &str, body: Option<&[u8]>, _content_type: Option<&str>) -> VfsResult<HttpResponse> {
        log_info(&format!("{} HTTP POST: {}", TAG, url));
        self.request(url, rcp_sys::RCP_METHOD_POST.as_ptr() as *const u8, body).await
    }
    
    #[allow(dead_code)]
    pub async fn post_with_headers(&self, url: &str, body: Option<&[u8]>, _content_type: Option<&str>, headers: std::collections::HashMap<String, String>) -> VfsResult<HttpResponse> {
        log_info(&format!("{} HTTP POST with headers: {}", TAG, url));
        for (k, v) in &headers {
            log_info(&format!("{}   Header: {} = {}...", TAG, k, &v[..v.len().min(30)]));
        }
        self.request_with_headers(url, rcp_sys::RCP_METHOD_POST.as_ptr() as *const u8, body, headers).await
    }
    
    #[allow(dead_code)]
    pub async fn delete_with_headers(&self, url: &str, headers: std::collections::HashMap<String, String>) -> VfsResult<HttpResponse> {
        log_info(&format!("{} HTTP DELETE with headers: {}", TAG, url));
        for (k, v) in &headers {
            log_info(&format!("{}   Header: {} = {}...", TAG, k, &v[..v.len().min(30)]));
        }
        self.request_with_headers(url, rcp_sys::RCP_METHOD_DELETE.as_ptr() as *const u8, None, headers).await
    }
    
    async fn request_with_headers(
        &self,
        url: &str,
        method: *const u8,
        body: Option<&[u8]>,
        headers: std::collections::HashMap<String, String>,
    ) -> VfsResult<HttpResponse> {
        log_info(&format!("{} request_with_headers: Creating request for {}", TAG, url));
        
        let url_c = CString::new(url).map_err(|_| {
            VfsError::new(ErrorCode::InvalidParameter, "Invalid URL")
        })?;
        
        let request = unsafe { rcp_sys::HMS_Rcp_CreateRequest(url_c.as_ptr()) };
        if request.is_null() {
            return Err(VfsError::new(ErrorCode::RequestError, "Failed to create request"));
        }
        
        let config = unsafe {
            libc::calloc(1, std::mem::size_of::<rcp_sys::Rcp_Configuration>()) 
                as *mut rcp_sys::Rcp_Configuration
        };
        if config.is_null() {
            unsafe {
                rcp_sys::HMS_Rcp_DestroyRequest(request);
            }
            return Err(VfsError::new(ErrorCode::RequestError, "Failed to allocate config"));
        }
        
        unsafe {
            (*request).method = method;
            (*config).transferConfiguration.autoRedirect = true;
            (*config).transferConfiguration.timeout.connectMs = 30000;
            (*config).transferConfiguration.timeout.transferMs = 30000;
            (*request).configuration = config;
        }
        
        log_info(&format!("{} request_with_headers: Setting {} headers", TAG, headers.len()));
        
        let rcp_headers = unsafe { rcp_sys::HMS_Rcp_CreateHeaders() };
        if rcp_headers.is_null() {
            log_info(&format!("{} Failed to create headers", TAG));
        } else {
            for (key, value) in &headers {
                let key_c = CString::new(key.clone()).unwrap();
                let value_c = CString::new(value.clone()).unwrap();
                unsafe {
                    let ret = rcp_sys::HMS_Rcp_SetHeaderValue(rcp_headers, key_c.as_ptr(), value_c.as_ptr());
                    log_info(&format!("{} Set header: {} = {}, ret = {}", TAG, key, value, ret));
                }
            }
            unsafe {
                (*request).headers = rcp_headers;
            }
            log_info(&format!("{} Headers set to request", TAG));
        }
        
        if let Some(data) = body {
            let content = unsafe {
                libc::calloc(1, std::mem::size_of::<rcp_sys::Rcp_RequestContent>()) 
                    as *mut rcp_sys::Rcp_RequestContent
            };
            if content.is_null() {
                unsafe {
                    let config = (*request).configuration;
                    if !config.is_null() {
                        libc::free(config as *mut libc::c_void);
                    }
                    rcp_sys::HMS_Rcp_DestroyRequest(request);
                }
                return Err(VfsError::new(ErrorCode::RequestError, "Failed to allocate content"));
            }
            
            let data_vec = data.to_vec();
            let data_ptr = data_vec.as_ptr() as *const u8;
            let data_len = data_vec.len() as u32;
            std::mem::forget(data_vec);
            
            unsafe {
                (*content).type_ = 0;
                (*content).data.contentStr.buffer = data_ptr;
                (*content).data.contentStr.length = data_len;
                (*request).content = content;
            }
        }
        
        let result = Arc::new(Mutex::new(None));
        let result_clone = Arc::clone(&result);
        let callback_data = Box::new(ResponseCallbackData { result: result_clone });
        let callback_data_ptr = Box::into_raw(callback_data) as *mut c_void;
        
        extern "C" fn response_callback_with_headers(
            usr_ctx: *mut c_void,
            response: *mut rcp_sys::Rcp_Response,
            err_code: u32,
        ) {
            unsafe {
                let data = &*(usr_ctx as *const ResponseCallbackData);
                let mut result = data.result.lock().unwrap();
                
                if response.is_null() {
                    crate::hilog::log_info(&format!("[VFS_RCP] Response is null, error code: {}", err_code));
                    *result = Some(Err(VfsError::new(
                        ErrorCode::ResponseError,
                        format!("Response is null, error code: {}", err_code),
                    )));
                } else {
                    let status_code = (*response).statusCode;
                    crate::hilog::log_info(&format!("[VFS_RCP] Response received, status: {}", status_code));
                    
                    let body = if !(*response).body.buffer.is_null() && (*response).body.length > 0 {
                        let slice = std::slice::from_raw_parts(
                            (*response).body.buffer as *const u8,
                            (*response).body.length as usize,
                        );
                        let body_vec = slice.to_vec();
                        if let Ok(body_str) = std::str::from_utf8(&body_vec) {
                            crate::hilog::log_info(&format!("[VFS_RCP] Response body ({} bytes):", body_vec.len()));
                            for line in body_str.lines() {
                                crate::hilog::log_info(&format!("[VFS_RCP] [BODY] {}", line));
                            }
                        }
                        Some(body_vec)
                    } else {
                        crate::hilog::log_info("[VFS_RCP] Response body: empty");
                        None
                    };
                    
                    if let Some(destroy) = (*response).destroyResponse {
                        destroy(response);
                    }
                    
                    *result = Some(Ok(HttpResponse {
                        status_code: status_code as i32,
                        body,
                        headers: std::collections::HashMap::new(),
                    }));
                }
                
                let _ = Box::from_raw(usr_ctx as *mut ResponseCallbackData);
            }
        }
        
        unsafe {
            let callback_obj = rcp_sys::Rcp_ResponseCallbackObject {
                callback: Some(response_callback_with_headers),
                usrCtx: callback_data_ptr,
            };
            
            log_info(&format!("{} Calling HMS_Rcp_Fetch...", TAG));
            let err_code = rcp_sys::HMS_Rcp_Fetch(self.session, request, &callback_obj);
            if err_code != 0 {
                log_info(&format!("{} HMS_Rcp_Fetch failed with error code: {}", TAG, err_code));
                let config = (*request).configuration;
                if !config.is_null() {
                    libc::free(config as *mut libc::c_void);
                }
                let headers = (*request).headers;
                if !headers.is_null() {
                    rcp_sys::HMS_Rcp_DestroyHeaders(headers);
                }
                rcp_sys::HMS_Rcp_DestroyRequest(request);
                let _ = Box::from_raw(callback_data_ptr as *mut ResponseCallbackData);
                return Err(VfsError::new(
                    ErrorCode::NetworkError,
                    format!("Failed to fetch, error code: {}", err_code),
                ));
            }
            log_info(&format!("{} HMS_Rcp_Fetch called successfully, waiting for response...", TAG));
        }
        
        for i in 0..100 {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            
            let result_guard = result.lock().unwrap();
            if result_guard.is_some() {
                log_info(&format!("{} Got response after {} iterations", TAG, i + 1));
                drop(result_guard);
                break;
            }
        }
        
        let mut result_guard = result.lock().unwrap();
        let result_opt = result_guard.take();
        drop(result_guard);
        
        unsafe {
            rcp_sys::HMS_Rcp_CancelSession(self.session);
            let config = (*request).configuration;
            if !config.is_null() {
                libc::free(config as *mut libc::c_void);
            }
            let headers = (*request).headers;
            if !headers.is_null() {
                rcp_sys::HMS_Rcp_DestroyHeaders(headers);
            }
            rcp_sys::HMS_Rcp_DestroyRequest(request);
        }
        
        match result_opt {
            Some(res) => res,
            None => {
                log_info(&format!("{} No response received after timeout", TAG));
                Err(VfsError::new(ErrorCode::ResponseError, "No response received"))
            },
        }
    }
    
    async fn request(
        &self,
        url: &str,
        method: *const u8,
        body: Option<&[u8]>,
    ) -> VfsResult<HttpResponse> {
        let url_c = CString::new(url).map_err(|_| {
            VfsError::new(ErrorCode::InvalidParameter, "Invalid URL")
        })?;
        
        let request = unsafe { rcp_sys::HMS_Rcp_CreateRequest(url_c.as_ptr()) };
        if request.is_null() {
            return Err(VfsError::new(ErrorCode::RequestError, "Failed to create request"));
        }
        
        let config = unsafe {
            libc::calloc(1, std::mem::size_of::<rcp_sys::Rcp_Configuration>()) 
                as *mut rcp_sys::Rcp_Configuration
        };
        if config.is_null() {
            unsafe {
                rcp_sys::HMS_Rcp_DestroyRequest(request);
            }
            return Err(VfsError::new(ErrorCode::RequestError, "Failed to allocate config"));
        }
        
        unsafe {
            (*request).method = method;
            (*config).transferConfiguration.autoRedirect = true;
            (*config).transferConfiguration.timeout.connectMs = 10000;
            (*config).transferConfiguration.timeout.transferMs = 10000;
            (*request).configuration = config;
        }
        
        if let Some(data) = body {
            let content = unsafe {
                libc::calloc(1, std::mem::size_of::<rcp_sys::Rcp_RequestContent>()) 
                    as *mut rcp_sys::Rcp_RequestContent
            };
            if content.is_null() {
                unsafe {
                    let config = (*request).configuration;
                    if !config.is_null() {
                        libc::free(config as *mut libc::c_void);
                    }
                    rcp_sys::HMS_Rcp_DestroyRequest(request);
                }
                return Err(VfsError::new(ErrorCode::RequestError, "Failed to allocate content"));
            }
            
            let data_vec = data.to_vec();
            let data_ptr = data_vec.as_ptr() as *const u8;
            let data_len = data_vec.len() as u32;
            std::mem::forget(data_vec);
            
            unsafe {
                (*content).type_ = 0;
                (*content).data.contentStr.buffer = data_ptr;
                (*content).data.contentStr.length = data_len;
                (*request).content = content;
            }
        }
        
        let result = Arc::new(Mutex::new(None));
        let result_clone = Arc::clone(&result);
        let callback_data = Box::new(ResponseCallbackData { result: result_clone });
        let callback_data_ptr = Box::into_raw(callback_data) as *mut c_void;
        
        extern "C" fn response_callback(
            usr_ctx: *mut c_void,
            response: *mut rcp_sys::Rcp_Response,
            err_code: u32,
        ) {
            unsafe {
                let data = &*(usr_ctx as *const ResponseCallbackData);
                let mut result = data.result.lock().unwrap();
                
                if response.is_null() {
                    *result = Some(Err(VfsError::new(
                        ErrorCode::ResponseError,
                        format!("Response is null, error code: {}", err_code),
                    )));
                } else {
                    let status_code = (*response).statusCode;
                    let body = if !(*response).body.buffer.is_null() && (*response).body.length > 0 {
                        let slice = std::slice::from_raw_parts(
                            (*response).body.buffer as *const u8,
                            (*response).body.length as usize,
                        );
                        let body_vec = slice.to_vec();
                        if let Ok(body_str) = std::str::from_utf8(&body_vec) {
                            crate::hilog::log_info(&format!("[VFS_RCP] HTTP Response Body ({} bytes):", body_vec.len()));
                            for line in body_str.lines() {
                                crate::hilog::log_info(&format!("[VFS_RCP] [BODY] {}", line));
                            }
                        }
                        Some(body_vec)
                    } else {
                        crate::hilog::log_info("[VFS_RCP] HTTP Response Body: empty");
                        None
                    };
                    
                    if let Some(destroy) = (*response).destroyResponse {
                        destroy(response);
                    }
                    
                    *result = Some(Ok(HttpResponse {
                        status_code: status_code as i32,
                        body,
                        headers: std::collections::HashMap::new(),
                    }));
                }
                
                let _ = Box::from_raw(usr_ctx as *mut ResponseCallbackData);
            }
        }
        
        unsafe {
            let callback_obj = rcp_sys::Rcp_ResponseCallbackObject {
                callback: Some(response_callback),
                usrCtx: callback_data_ptr,
            };
            
            let err_code = rcp_sys::HMS_Rcp_Fetch(self.session, request, &callback_obj);
            if err_code != 0 {
                let config = (*request).configuration;
                if !config.is_null() {
                    libc::free(config as *mut libc::c_void);
                }
                rcp_sys::HMS_Rcp_DestroyRequest(request);
                let _ = Box::from_raw(callback_data_ptr as *mut ResponseCallbackData);
                return Err(VfsError::new(
                    ErrorCode::NetworkError,
                    format!("Failed to fetch, error code: {}", err_code),
                ));
            }
        }
        
        for _ in 0..100 {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            
            let result_guard = result.lock().unwrap();
            if result_guard.is_some() {
                drop(result_guard);
                break;
            }
        }
        
        let mut result_guard = result.lock().unwrap();
        let result_opt = result_guard.take();
        drop(result_guard);
        
        unsafe {
            rcp_sys::HMS_Rcp_CancelSession(self.session);
            let config = (*request).configuration;
            if !config.is_null() {
                libc::free(config as *mut libc::c_void);
            }
            rcp_sys::HMS_Rcp_DestroyRequest(request);
        }
        
        match result_opt {
            Some(res) => res,
            None => Err(VfsError::new(ErrorCode::ResponseError, "No response received")),
        }
    }
}

impl Drop for HttpClient {
    fn drop(&mut self) {
        unsafe {
            if !self.session.is_null() {
                rcp_sys::HMS_Rcp_CloseSession(&mut self.session);
            }
        }
    }
}

struct ResponseCallbackData {
    result: Arc<Mutex<Option<VfsResult<HttpResponse>>>>,
}

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status_code: i32,
    pub body: Option<Vec<u8>>,
    #[allow(dead_code)]
    pub headers: std::collections::HashMap<String, String>,
}

impl HttpResponse {
    pub fn body_as_string(&self) -> Option<String> {
        self.body.as_ref().map(|b| String::from_utf8_lossy(b).into_owned())
    }
}

pub async fn http_get(url: &str) -> VfsResult<HttpResponse> {
    log_info(&format!("HTTP GET: {}", url));
    let client = HttpClient::new().await?;
    client.get(url).await
}

#[allow(dead_code)]
pub async fn http_post(url: &str, body: &[u8], content_type: &str) -> VfsResult<HttpResponse> {
    let client = HttpClient::new().await?;
    client.post(url, Some(body), Some(content_type)).await
}

#[allow(dead_code)]
pub async fn http_put(_url: &str, _body: &[u8], _content_type: &str) -> VfsResult<HttpResponse> {
    Err(VfsError::new(ErrorCode::InvalidParameter, "HTTP PUT not implemented"))
}

#[allow(dead_code)]
pub async fn http_delete(_url: &str) -> VfsResult<HttpResponse> {
    Err(VfsError::new(ErrorCode::InvalidParameter, "HTTP DELETE not implemented"))
}
