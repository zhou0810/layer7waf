use layer7waf_common::UpstreamConfig;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Manages upstream server selection with weighted round-robin.
pub struct UpstreamSelector {
    pub name: String,
    servers: Vec<UpstreamEntry>,
    /// Weighted round-robin index (indexes into the expanded list).
    counter: AtomicUsize,
    /// Expanded list of server indices based on weights.
    weighted_indices: Vec<usize>,
}

struct UpstreamEntry {
    pub addr: String,
    pub weight: u32,
}

impl UpstreamSelector {
    pub fn from_config(config: &UpstreamConfig) -> Self {
        let servers: Vec<UpstreamEntry> = config
            .servers
            .iter()
            .map(|s| UpstreamEntry {
                addr: s.addr.clone(),
                weight: s.weight,
            })
            .collect();

        // Build weighted index list: server 0 with weight 3 → [0, 0, 0]
        let mut weighted_indices = Vec::new();
        for (i, server) in servers.iter().enumerate() {
            for _ in 0..server.weight {
                weighted_indices.push(i);
            }
        }
        if weighted_indices.is_empty() && !servers.is_empty() {
            // Fallback: equal weight
            for i in 0..servers.len() {
                weighted_indices.push(i);
            }
        }

        Self {
            name: config.name.clone(),
            servers,
            counter: AtomicUsize::new(0),
            weighted_indices,
        }
    }

    /// Select the next upstream server address using weighted round-robin.
    pub fn select(&self) -> Option<&str> {
        if self.weighted_indices.is_empty() {
            return None;
        }
        let idx = self.counter.fetch_add(1, Ordering::Relaxed) % self.weighted_indices.len();
        let server_idx = self.weighted_indices[idx];
        Some(&self.servers[server_idx].addr)
    }

    pub fn server_count(&self) -> usize {
        self.servers.len()
    }
}
