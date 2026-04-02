package thetadatadx

/*
#include <stdlib.h>
#include <stdint.h>
#include <stddef.h>

// Forward declarations (already defined in thetadx.go, but CGo needs them per file).
typedef void TdxCredentials;
typedef void TdxConfig;
typedef void TdxFpssHandle;

extern TdxFpssHandle* tdx_fpss_connect(const TdxCredentials* creds, const TdxConfig* config);
extern int tdx_fpss_subscribe_quotes(const TdxFpssHandle* h, const char* symbol);
extern int tdx_fpss_subscribe_trades(const TdxFpssHandle* h, const char* symbol);
extern int tdx_fpss_subscribe_open_interest(const TdxFpssHandle* h, const char* symbol);
extern int tdx_fpss_subscribe_full_trades(const TdxFpssHandle* h, const char* sec_type);
extern int tdx_fpss_subscribe_full_open_interest(const TdxFpssHandle* h, const char* sec_type);
extern int tdx_fpss_unsubscribe_quotes(const TdxFpssHandle* h, const char* symbol);
extern int tdx_fpss_unsubscribe_trades(const TdxFpssHandle* h, const char* symbol);
extern int tdx_fpss_unsubscribe_open_interest(const TdxFpssHandle* h, const char* symbol);
extern int tdx_fpss_unsubscribe_full_trades(const TdxFpssHandle* h, const char* sec_type);
extern int tdx_fpss_unsubscribe_full_open_interest(const TdxFpssHandle* h, const char* sec_type);
extern int tdx_fpss_is_authenticated(const TdxFpssHandle* h);
extern char* tdx_fpss_contract_lookup(const TdxFpssHandle* h, int id);
extern char* tdx_fpss_active_subscriptions(const TdxFpssHandle* h);
extern char* tdx_fpss_next_event(const TdxFpssHandle* h, uint64_t timeout_ms);
extern void tdx_fpss_shutdown(const TdxFpssHandle* h);
extern void tdx_fpss_free(TdxFpssHandle* h);
extern const char* tdx_last_error();
extern void tdx_string_free(char* s);
*/
import "C"

import (
	"encoding/json"
	"fmt"
	"unsafe"
)

// FpssClient wraps the FPSS real-time streaming handle.
type FpssClient struct {
	handle *C.TdxFpssHandle
}

// NewFpssClient connects to the FPSS streaming servers and returns a client.
func NewFpssClient(creds *Credentials, config *Config) (*FpssClient, error) {
	if creds == nil || creds.handle == nil {
		return nil, fmt.Errorf("thetadatadx: credentials handle is nil")
	}
	if config == nil || config.handle == nil {
		return nil, fmt.Errorf("thetadatadx: config handle is nil")
	}
	h := C.tdx_fpss_connect(creds.handle, config.handle)
	if h == nil {
		return nil, fmt.Errorf("thetadatadx: %s", lastError())
	}
	return &FpssClient{handle: h}, nil
}

func (f *FpssClient) fpssCall(rc C.int) (int, error) {
	if rc < 0 {
		return int(rc), fmt.Errorf("thetadatadx: %s", lastError())
	}
	return int(rc), nil
}

// SubscribeQuotes subscribes to real-time quote data for a stock symbol.
func (f *FpssClient) SubscribeQuotes(symbol string) (int, error) {
	cs := C.CString(symbol)
	defer C.free(unsafe.Pointer(cs))
	return f.fpssCall(C.tdx_fpss_subscribe_quotes(f.handle, cs))
}

// SubscribeTrades subscribes to real-time trade data for a stock symbol.
func (f *FpssClient) SubscribeTrades(symbol string) (int, error) {
	cs := C.CString(symbol)
	defer C.free(unsafe.Pointer(cs))
	return f.fpssCall(C.tdx_fpss_subscribe_trades(f.handle, cs))
}

// SubscribeOpenInterest subscribes to open interest data for a stock symbol.
func (f *FpssClient) SubscribeOpenInterest(symbol string) (int, error) {
	cs := C.CString(symbol)
	defer C.free(unsafe.Pointer(cs))
	return f.fpssCall(C.tdx_fpss_subscribe_open_interest(f.handle, cs))
}

// SubscribeFullTrades subscribes to all trades for a security type ("STOCK", "OPTION", "INDEX").
func (f *FpssClient) SubscribeFullTrades(secType string) (int, error) {
	cs := C.CString(secType)
	defer C.free(unsafe.Pointer(cs))
	return f.fpssCall(C.tdx_fpss_subscribe_full_trades(f.handle, cs))
}

// SubscribeFullOpenInterest subscribes to all open interest for a security type ("STOCK", "OPTION", "INDEX").
func (f *FpssClient) SubscribeFullOpenInterest(secType string) (int, error) {
	cs := C.CString(secType)
	defer C.free(unsafe.Pointer(cs))
	return f.fpssCall(C.tdx_fpss_subscribe_full_open_interest(f.handle, cs))
}

// UnsubscribeQuotes unsubscribes from quote data for a stock symbol.
func (f *FpssClient) UnsubscribeQuotes(symbol string) (int, error) {
	cs := C.CString(symbol)
	defer C.free(unsafe.Pointer(cs))
	return f.fpssCall(C.tdx_fpss_unsubscribe_quotes(f.handle, cs))
}

// UnsubscribeTrades unsubscribes from trade data for a stock symbol.
func (f *FpssClient) UnsubscribeTrades(symbol string) (int, error) {
	cs := C.CString(symbol)
	defer C.free(unsafe.Pointer(cs))
	return f.fpssCall(C.tdx_fpss_unsubscribe_trades(f.handle, cs))
}

// UnsubscribeOpenInterest unsubscribes from open interest data for a stock symbol.
func (f *FpssClient) UnsubscribeOpenInterest(symbol string) (int, error) {
	cs := C.CString(symbol)
	defer C.free(unsafe.Pointer(cs))
	return f.fpssCall(C.tdx_fpss_unsubscribe_open_interest(f.handle, cs))
}

// UnsubscribeFullTrades unsubscribes from all trades for a security type ("STOCK", "OPTION", "INDEX").
func (f *FpssClient) UnsubscribeFullTrades(secType string) (int, error) {
	cs := C.CString(secType)
	defer C.free(unsafe.Pointer(cs))
	return f.fpssCall(C.tdx_fpss_unsubscribe_full_trades(f.handle, cs))
}

// UnsubscribeFullOpenInterest unsubscribes from all open interest for a security type ("STOCK", "OPTION", "INDEX").
func (f *FpssClient) UnsubscribeFullOpenInterest(secType string) (int, error) {
	cs := C.CString(secType)
	defer C.free(unsafe.Pointer(cs))
	return f.fpssCall(C.tdx_fpss_unsubscribe_full_open_interest(f.handle, cs))
}

// IsAuthenticated returns true if the FPSS client is currently authenticated.
func (f *FpssClient) IsAuthenticated() bool {
	return C.tdx_fpss_is_authenticated(f.handle) != 0
}

// ContractLookup looks up a contract by its server-assigned ID.
func (f *FpssClient) ContractLookup(id int) (string, error) {
	cstr := C.tdx_fpss_contract_lookup(f.handle, C.int(id))
	if cstr == nil {
		return "", fmt.Errorf("thetadatadx: %s", lastError())
	}
	goStr := C.GoString(cstr)
	C.tdx_string_free(cstr)
	return goStr, nil
}

// ActiveSubscriptions returns the currently active subscriptions as JSON.
func (f *FpssClient) ActiveSubscriptions() (json.RawMessage, error) {
	cstr := C.tdx_fpss_active_subscriptions(f.handle)
	if cstr == nil {
		return nil, fmt.Errorf("thetadatadx: %s", lastError())
	}
	goStr := C.GoString(cstr)
	C.tdx_string_free(cstr)
	return json.RawMessage(goStr), nil
}

// NextEvent polls for the next streaming event with the given timeout in milliseconds.
// Returns nil if the timeout expires with no event.
func (f *FpssClient) NextEvent(timeoutMs uint64) (json.RawMessage, error) {
	cstr := C.tdx_fpss_next_event(f.handle, C.uint64_t(timeoutMs))
	if cstr == nil {
		// nil means timeout (no event), not an error
		return nil, nil
	}
	goStr := C.GoString(cstr)
	C.tdx_string_free(cstr)
	return json.RawMessage(goStr), nil
}

// Shutdown gracefully shuts down the FPSS streaming connection.
func (f *FpssClient) Shutdown() {
	if f.handle != nil {
		C.tdx_fpss_shutdown(f.handle)
	}
}

// Close frees the FPSS handle. Call after Shutdown.
func (f *FpssClient) Close() {
	if f.handle != nil {
		C.tdx_fpss_free(f.handle)
		f.handle = nil
	}
}
