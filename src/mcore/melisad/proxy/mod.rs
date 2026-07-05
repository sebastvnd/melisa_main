// mcore/melisad/proxy/mod.rs
// Copyright (c) 2026 Erick Adriano
// Licensed under the MIT License.

pub mod forwarder;
pub mod handler;
pub mod loadbalancer;
/// Proxy Module - HTTP reverse proxy dengan load balancing
///
/// Structure:
/// - metrics.rs      : ProxyMetrics untuk tracking requests, errors, bytes
/// - loadbalancer.rs : LoadBalancingStrategy (RoundRobin, LeastConnections, Random)
/// - forwarder.rs    : Request forwarding dengan retry logic
/// - handler.rs      : HTTP request handling dan routing
/// - server.rs       : Main proxy server loop
pub mod metrics;
pub mod server;

// Backward compatibility
pub use server::run_proxy_server;
