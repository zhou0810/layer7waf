package main

/*
#include <stdlib.h>
#include <stdint.h>
*/
import "C"

import (
	"encoding/json"
	"sync"
	"sync/atomic"
	"unsafe"

	"github.com/corazawaf/coraza/v3"
	"github.com/corazawaf/coraza/v3/types"
)

var (
	wafCounter uint64
	txCounter  uint64

	wafInstances sync.Map // map[uint64]coraza.WAF
	txInstances  sync.Map // map[uint64]types.Transaction
)

//export coraza_new_waf
func coraza_new_waf(directives *C.char) C.uint64_t {
	directivesStr := C.GoString(directives)

	cfg := coraza.NewWAFConfig().WithDirectives(directivesStr)
	waf, err := coraza.NewWAF(cfg)
	if err != nil {
		return 0
	}

	id := atomic.AddUint64(&wafCounter, 1)
	wafInstances.Store(id, waf)
	return C.uint64_t(id)
}

//export coraza_new_transaction
func coraza_new_transaction(wafID C.uint64_t) C.uint64_t {
	val, ok := wafInstances.Load(uint64(wafID))
	if !ok {
		return 0
	}
	waf := val.(coraza.WAF)

	tx := waf.NewTransaction()
	id := atomic.AddUint64(&txCounter, 1)
	txInstances.Store(id, tx)
	return C.uint64_t(id)
}

//export coraza_process_request_headers
func coraza_process_request_headers(txID C.uint64_t, method, uri, protocol, headersJSON *C.char) C.int {
	val, ok := txInstances.Load(uint64(txID))
	if !ok {
		return -1
	}
	tx := val.(types.Transaction)

	methodStr := C.GoString(method)
	uriStr := C.GoString(uri)
	protocolStr := C.GoString(protocol)

	tx.ProcessURI(uriStr, methodStr, protocolStr)

	headersStr := C.GoString(headersJSON)
	var headers [][2]string
	if err := json.Unmarshal([]byte(headersStr), &headers); err == nil {
		for _, h := range headers {
			tx.AddRequestHeader(h[0], h[1])
		}
	}

	tx.ProcessRequestHeaders()

	if it := tx.Interruption(); it != nil {
		return C.int(it.Status)
	}
	return 0
}

//export coraza_process_request_body
func coraza_process_request_body(txID C.uint64_t, body unsafe.Pointer, bodyLen C.int) C.int {
	val, ok := txInstances.Load(uint64(txID))
	if !ok {
		return -1
	}
	tx := val.(types.Transaction)

	if bodyLen > 0 && body != nil {
		buf := C.GoBytes(body, bodyLen)
		if it, _, err := tx.WriteRequestBody(buf); it != nil {
			return C.int(it.Status)
		} else if err != nil {
			return -1
		}
	}

	if it, err := tx.ProcessRequestBody(); it != nil {
		return C.int(it.Status)
	} else if err != nil {
		return -1
	}

	return 0
}

//export coraza_process_response_headers
func coraza_process_response_headers(txID C.uint64_t, statusCode C.int, headersJSON *C.char) C.int {
	val, ok := txInstances.Load(uint64(txID))
	if !ok {
		return -1
	}
	tx := val.(types.Transaction)

	headersStr := C.GoString(headersJSON)
	var headers [][2]string
	if err := json.Unmarshal([]byte(headersStr), &headers); err == nil {
		for _, h := range headers {
			tx.AddResponseHeader(h[0], h[1])
		}
	}

	tx.ProcessResponseHeaders(int(statusCode), "HTTP/1.1")

	if it := tx.Interruption(); it != nil {
		return C.int(it.Status)
	}
	return 0
}

//export coraza_process_response_body
func coraza_process_response_body(txID C.uint64_t, body unsafe.Pointer, bodyLen C.int) C.int {
	val, ok := txInstances.Load(uint64(txID))
	if !ok {
		return -1
	}
	tx := val.(types.Transaction)

	if bodyLen > 0 && body != nil {
		buf := C.GoBytes(body, bodyLen)
		if it, _, err := tx.WriteResponseBody(buf); it != nil {
			return C.int(it.Status)
		} else if err != nil {
			return -1
		}
	}

	if it, err := tx.ProcessResponseBody(); it != nil {
		return C.int(it.Status)
	} else if err != nil {
		return -1
	}

	return 0
}

//export coraza_intervention_status
func coraza_intervention_status(txID C.uint64_t) C.int {
	val, ok := txInstances.Load(uint64(txID))
	if !ok {
		return 0
	}
	tx := val.(types.Transaction)

	if it := tx.Interruption(); it != nil {
		return C.int(it.Status)
	}
	return 0
}

//export coraza_intervention_url
func coraza_intervention_url(txID C.uint64_t) *C.char {
	val, ok := txInstances.Load(uint64(txID))
	if !ok {
		return nil
	}
	tx := val.(types.Transaction)

	it := tx.Interruption()
	if it == nil || it.Action != "redirect" {
		return nil
	}

	return C.CString(it.Data)
}

//export coraza_free_transaction
func coraza_free_transaction(txID C.uint64_t) {
	val, ok := txInstances.LoadAndDelete(uint64(txID))
	if !ok {
		return
	}
	tx := val.(types.Transaction)
	tx.Close()
}

//export coraza_free_waf
func coraza_free_waf(wafID C.uint64_t) {
	wafInstances.Delete(uint64(wafID))
}

func main() {}
