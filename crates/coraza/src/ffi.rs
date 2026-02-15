use std::os::raw::{c_char, c_int, c_void};

extern "C" {
    pub fn coraza_new_waf(directives: *const c_char) -> u64;
    pub fn coraza_new_transaction(waf_id: u64) -> u64;
    pub fn coraza_process_request_headers(
        tx_id: u64,
        method: *const c_char,
        uri: *const c_char,
        protocol: *const c_char,
        headers_json: *const c_char,
    ) -> c_int;
    pub fn coraza_process_request_body(tx_id: u64, body: *const c_void, body_len: c_int) -> c_int;
    pub fn coraza_process_response_headers(
        tx_id: u64,
        status_code: c_int,
        headers_json: *const c_char,
    ) -> c_int;
    pub fn coraza_process_response_body(
        tx_id: u64,
        body: *const c_void,
        body_len: c_int,
    ) -> c_int;
    pub fn coraza_intervention_status(tx_id: u64) -> c_int;
    pub fn coraza_intervention_url(tx_id: u64) -> *mut c_char;
    pub fn coraza_free_transaction(tx_id: u64);
    pub fn coraza_free_waf(waf_id: u64);
}
