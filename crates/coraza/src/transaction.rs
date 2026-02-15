use std::ffi::{CStr, CString};
use std::os::raw::{c_int, c_void};

use crate::ffi;

/// Represents the WAF engine decision for a given processing phase.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WafAction {
    /// The request/response is allowed to proceed.
    Pass,
    /// The request/response should be blocked with the given HTTP status code.
    Block { status: u16 },
    /// The request should be redirected to the given URL with the given status code.
    Redirect { status: u16, url: String },
}

/// A Coraza WAF engine instance. Wraps a Go-side WAF created from SecLang directives.
pub struct WafEngine {
    waf_id: u64,
}

impl WafEngine {
    /// Create a new WAF engine with the given SecLang directives string.
    ///
    /// Returns an error if the Go side fails to parse the directives.
    pub fn new(directives: &str) -> Result<Self, String> {
        let c_directives = CString::new(directives)
            .map_err(|e| format!("directives string contains interior NUL byte: {e}"))?;
        let waf_id = unsafe { ffi::coraza_new_waf(c_directives.as_ptr()) };
        if waf_id == 0 {
            return Err("coraza_new_waf failed: check directives".to_string());
        }
        Ok(Self { waf_id })
    }
}

// SAFETY: WafEngine holds an opaque ID into Go-side sync.Map, which is goroutine-safe.
unsafe impl Send for WafEngine {}
unsafe impl Sync for WafEngine {}

impl Drop for WafEngine {
    fn drop(&mut self) {
        unsafe {
            ffi::coraza_free_waf(self.waf_id);
        }
    }
}

/// A single WAF transaction, corresponding to one HTTP request/response cycle.
pub struct WafTransaction {
    tx_id: u64,
}

impl WafTransaction {
    /// Create a new transaction bound to the given WAF engine.
    ///
    /// # Panics
    /// Panics if the Go side returns 0.
    pub fn new(engine: &WafEngine) -> Self {
        let tx_id = unsafe { ffi::coraza_new_transaction(engine.waf_id) };
        assert!(tx_id != 0, "coraza_new_transaction failed");
        Self { tx_id }
    }

    /// Process request headers through the WAF.
    ///
    /// `headers` is a slice of `(name, value)` pairs.
    pub fn process_request_headers(
        &self,
        method: &str,
        uri: &str,
        protocol: &str,
        headers: &[(String, String)],
    ) -> WafAction {
        let c_method = CString::new(method).unwrap();
        let c_uri = CString::new(uri).unwrap();
        let c_protocol = CString::new(protocol).unwrap();

        let headers_vec: Vec<[&str; 2]> = headers.iter().map(|(k, v)| [k.as_str(), v.as_str()]).collect();
        let headers_json = serde_json::to_string(&headers_vec).unwrap();
        let c_headers = CString::new(headers_json).unwrap();

        let rc = unsafe {
            ffi::coraza_process_request_headers(
                self.tx_id,
                c_method.as_ptr(),
                c_uri.as_ptr(),
                c_protocol.as_ptr(),
                c_headers.as_ptr(),
            )
        };

        self.interpret_status(rc)
    }

    /// Process request body bytes through the WAF.
    pub fn process_request_body(&self, body: &[u8]) -> WafAction {
        let rc = unsafe {
            ffi::coraza_process_request_body(
                self.tx_id,
                body.as_ptr() as *const c_void,
                body.len() as c_int,
            )
        };
        self.interpret_status(rc)
    }

    /// Process response headers through the WAF.
    ///
    /// `headers` is a slice of `(name, value)` pairs.
    pub fn process_response_headers(
        &self,
        status: u16,
        headers: &[(String, String)],
    ) -> WafAction {
        let headers_vec: Vec<[&str; 2]> = headers.iter().map(|(k, v)| [k.as_str(), v.as_str()]).collect();
        let headers_json = serde_json::to_string(&headers_vec).unwrap();
        let c_headers = CString::new(headers_json).unwrap();

        let rc = unsafe {
            ffi::coraza_process_response_headers(
                self.tx_id,
                status as c_int,
                c_headers.as_ptr(),
            )
        };

        self.interpret_status(rc)
    }

    /// Process response body bytes through the WAF.
    pub fn process_response_body(&self, body: &[u8]) -> WafAction {
        let rc = unsafe {
            ffi::coraza_process_response_body(
                self.tx_id,
                body.as_ptr() as *const c_void,
                body.len() as c_int,
            )
        };
        self.interpret_status(rc)
    }

    /// Check whether the WAF has flagged an intervention on this transaction.
    pub fn check_intervention(&self) -> WafAction {
        let rc = unsafe { ffi::coraza_intervention_status(self.tx_id) };
        self.interpret_status(rc)
    }

    /// Convert a C return code into a `WafAction`, checking for redirects.
    fn interpret_status(&self, rc: c_int) -> WafAction {
        if rc <= 0 {
            return WafAction::Pass;
        }

        // Check if there is a redirect URL set on the intervention.
        let url_ptr = unsafe { ffi::coraza_intervention_url(self.tx_id) };
        if !url_ptr.is_null() {
            let url = unsafe { CStr::from_ptr(url_ptr) }
                .to_string_lossy()
                .into_owned();
            // The Go side allocated with C.CString; we must free it.
            unsafe {
                libc_free(url_ptr as *mut c_void);
            }
            WafAction::Redirect {
                status: rc as u16,
                url,
            }
        } else {
            WafAction::Block { status: rc as u16 }
        }
    }
}

// SAFETY: WafTransaction holds an opaque ID into Go-side sync.Map, which is goroutine-safe.
unsafe impl Send for WafTransaction {}
unsafe impl Sync for WafTransaction {}

impl Drop for WafTransaction {
    fn drop(&mut self) {
        unsafe {
            ffi::coraza_free_transaction(self.tx_id);
        }
    }
}

// We need to free C strings allocated by the Go side via C.CString (which uses C malloc).
extern "C" {
    #[link_name = "free"]
    fn libc_free(ptr: *mut c_void);
}
