use std::net::IpAddr;

use ipnet::IpNet;

/// A binary prefix trie for fast IP/CIDR lookups.
///
/// Maintains separate roots for IPv4 (32-bit) and IPv6 (128-bit) addresses,
/// allowing efficient longest-prefix matching and membership checks.
pub struct IpTrie {
    root_v4: TrieNode,
    root_v6: TrieNode,
}

struct TrieNode {
    children: [Option<Box<TrieNode>>; 2],
    is_terminal: bool,
}

impl TrieNode {
    fn new() -> Self {
        Self {
            children: [None, None],
            is_terminal: false,
        }
    }

    /// Recursively count the number of terminal nodes in this subtree
    /// (including this node).
    fn count_terminals(&self) -> usize {
        let mut count = if self.is_terminal { 1 } else { 0 };
        for child in &self.children {
            if let Some(ref node) = child {
                count += node.count_terminals();
            }
        }
        count
    }
}

/// Convert an IP address into a vector of individual bits (0 or 1).
///
/// IPv4 addresses produce 32 bits; IPv6 addresses produce 128 bits.
fn ip_to_bits(addr: IpAddr) -> Vec<u8> {
    match addr {
        IpAddr::V4(v4) => {
            let octets = v4.octets();
            let mut bits = Vec::with_capacity(32);
            for octet in &octets {
                for i in (0..8).rev() {
                    bits.push((octet >> i) & 1);
                }
            }
            bits
        }
        IpAddr::V6(v6) => {
            let octets = v6.octets();
            let mut bits = Vec::with_capacity(128);
            for octet in &octets {
                for i in (0..8).rev() {
                    bits.push((octet >> i) & 1);
                }
            }
            bits
        }
    }
}

impl IpTrie {
    /// Create a new empty trie.
    pub fn new() -> Self {
        Self {
            root_v4: TrieNode::new(),
            root_v6: TrieNode::new(),
        }
    }

    /// Insert a CIDR network into the trie.
    ///
    /// Converts the network address to bits, walks (or creates) nodes down to
    /// the prefix length, and marks the final node as terminal. Any IP that
    /// falls within this CIDR range will match during lookups.
    pub fn insert(&mut self, network: IpNet) {
        let addr = network.network();
        let prefix_len = network.prefix_len() as usize;
        let bits = ip_to_bits(addr);

        let root = match addr {
            IpAddr::V4(_) => &mut self.root_v4,
            IpAddr::V6(_) => &mut self.root_v6,
        };

        let mut current = root;
        for &bit in bits.iter().take(prefix_len) {
            let idx = bit as usize;
            if current.children[idx].is_none() {
                current.children[idx] = Some(Box::new(TrieNode::new()));
            }
            current = current.children[idx].as_mut().unwrap();
        }
        current.is_terminal = true;
    }

    /// Check if an IP address matches any inserted CIDR range.
    ///
    /// Walks the trie bit by bit. If any terminal node is encountered along
    /// the path, the address is contained within that CIDR and `true` is
    /// returned. This naturally handles prefix matching -- a /16 terminal
    /// will match all /32 addresses within it.
    pub fn contains(&self, addr: IpAddr) -> bool {
        let bits = ip_to_bits(addr);

        let root = match addr {
            IpAddr::V4(_) => &self.root_v4,
            IpAddr::V6(_) => &self.root_v6,
        };

        // Check if the root itself is terminal (a /0 network -- matches everything).
        if root.is_terminal {
            return true;
        }

        let mut current = root;
        for &bit in &bits {
            let idx = bit as usize;
            match &current.children[idx] {
                Some(node) => {
                    current = node;
                    if current.is_terminal {
                        return true;
                    }
                }
                None => return false,
            }
        }

        false
    }

    /// Count the total number of terminal nodes (inserted CIDR entries) in the
    /// trie, across both IPv4 and IPv6 roots.
    pub fn len(&self) -> usize {
        self.root_v4.count_terminals() + self.root_v6.count_terminals()
    }

    /// Returns `true` if the trie contains no entries.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ip_to_bits_v4() {
        let addr: IpAddr = "192.168.1.1".parse().unwrap();
        let bits = ip_to_bits(addr);
        assert_eq!(bits.len(), 32);
        // 192 = 0b11000000
        assert_eq!(&bits[0..8], &[1, 1, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn test_ip_to_bits_v6() {
        let addr: IpAddr = "::1".parse().unwrap();
        let bits = ip_to_bits(addr);
        assert_eq!(bits.len(), 128);
        // Last bit should be 1
        assert_eq!(bits[127], 1);
        // All other bits should be 0
        assert!(bits[..127].iter().all(|&b| b == 0));
    }

    #[test]
    fn test_empty_trie() {
        let trie = IpTrie::new();
        assert!(trie.is_empty());
        assert_eq!(trie.len(), 0);
        assert!(!trie.contains("10.0.0.1".parse().unwrap()));
    }

    #[test]
    fn test_insert_and_contains_single_ip() {
        let mut trie = IpTrie::new();
        let net: IpNet = "10.0.0.1/32".parse().unwrap();
        trie.insert(net);

        assert_eq!(trie.len(), 1);
        assert!(trie.contains("10.0.0.1".parse().unwrap()));
        assert!(!trie.contains("10.0.0.2".parse().unwrap()));
    }

    #[test]
    fn test_insert_and_contains_cidr() {
        let mut trie = IpTrie::new();
        let net: IpNet = "192.168.1.0/24".parse().unwrap();
        trie.insert(net);

        assert!(trie.contains("192.168.1.0".parse().unwrap()));
        assert!(trie.contains("192.168.1.1".parse().unwrap()));
        assert!(trie.contains("192.168.1.255".parse().unwrap()));
        assert!(!trie.contains("192.168.2.0".parse().unwrap()));
        assert!(!trie.contains("192.168.0.255".parse().unwrap()));
    }

    #[test]
    fn test_insert_and_contains_cidr_16() {
        let mut trie = IpTrie::new();
        let net: IpNet = "10.0.0.0/16".parse().unwrap();
        trie.insert(net);

        assert!(trie.contains("10.0.0.0".parse().unwrap()));
        assert!(trie.contains("10.0.255.255".parse().unwrap()));
        assert!(trie.contains("10.0.128.42".parse().unwrap()));
        assert!(!trie.contains("10.1.0.0".parse().unwrap()));
        assert!(!trie.contains("11.0.0.0".parse().unwrap()));
    }

    #[test]
    fn test_multiple_entries() {
        let mut trie = IpTrie::new();
        trie.insert("10.0.0.0/8".parse().unwrap());
        trie.insert("172.16.0.0/12".parse().unwrap());
        trie.insert("192.168.0.0/16".parse().unwrap());

        assert_eq!(trie.len(), 3);
        assert!(trie.contains("10.255.255.255".parse().unwrap()));
        assert!(trie.contains("172.16.0.1".parse().unwrap()));
        assert!(trie.contains("172.31.255.255".parse().unwrap()));
        assert!(trie.contains("192.168.100.50".parse().unwrap()));
        assert!(!trie.contains("172.32.0.0".parse().unwrap()));
        assert!(!trie.contains("8.8.8.8".parse().unwrap()));
    }

    #[test]
    fn test_ipv6_single() {
        let mut trie = IpTrie::new();
        trie.insert("::1/128".parse().unwrap());

        assert!(trie.contains("::1".parse().unwrap()));
        assert!(!trie.contains("::2".parse().unwrap()));
    }

    #[test]
    fn test_ipv6_cidr() {
        let mut trie = IpTrie::new();
        trie.insert("fd00::/8".parse().unwrap());

        assert!(trie.contains("fd00::1".parse().unwrap()));
        assert!(trie.contains("fdff:ffff::1".parse().unwrap()));
        assert!(!trie.contains("fe00::1".parse().unwrap()));
    }

    #[test]
    fn test_mixed_v4_v6() {
        let mut trie = IpTrie::new();
        trie.insert("10.0.0.0/8".parse().unwrap());
        trie.insert("fd00::/8".parse().unwrap());

        assert_eq!(trie.len(), 2);
        assert!(trie.contains("10.0.0.1".parse().unwrap()));
        assert!(trie.contains("fd00::1".parse().unwrap()));
        // v4 address should not match v6 entry and vice versa
        assert!(!trie.contains("8.8.8.8".parse().unwrap()));
        assert!(!trie.contains("fe00::1".parse().unwrap()));
    }

    #[test]
    fn test_overlapping_cidrs() {
        let mut trie = IpTrie::new();
        trie.insert("10.0.0.0/8".parse().unwrap());
        trie.insert("10.0.0.0/24".parse().unwrap());

        // Both should match addresses in the /24
        assert!(trie.contains("10.0.0.1".parse().unwrap()));
        // The /8 should still match addresses outside the /24
        assert!(trie.contains("10.1.0.1".parse().unwrap()));
    }
}
