//! Uses the production transport's shared error allowlist.
#![cfg(not(target_arch = "wasm32"))]
#![allow(dead_code)]
#[path = "../src/gateway.rs"]
mod gateway;
#[path = "../src/native_http.rs"]
mod native_http;
