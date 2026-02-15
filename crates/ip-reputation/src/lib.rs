mod trie;

use std::io::BufRead;
use std::net::IpAddr;
use std::path::Path;
use std::sync::Arc;

use arc_swap::ArcSwap;
use ipnet::IpNet;
use tracing::{debug, info, warn};

use crate::trie::IpTrie;

/// The result of checking an IP address against the reputation lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpAction {
    /// The IP is explicitly allowed (present in the allowlist).
    Allow,
    /// The IP is blocked (present in the blocklist).
    Block,
    /// The IP is not in any list; no opinion.
    None,
}

/// IP reputation engine backed by prefix tries for efficient CIDR matching.
///
/// Uses `ArcSwap` for lock-free reads, allowing blocklists and allowlists to
/// be hot-reloaded without blocking lookups in the request path.
pub struct IpReputation {
    blocklist: ArcSwap<IpTrie>,
    allowlist: ArcSwap<IpTrie>,
}

impl IpReputation {
    /// Create a new `IpReputation` instance with empty blocklist and allowlist.
    pub fn new() -> Self {
        Self {
            blocklist: ArcSwap::from_pointee(IpTrie::new()),
            allowlist: ArcSwap::from_pointee(IpTrie::new()),
        }
    }

    /// Load a blocklist from a file.
    ///
    /// The file should contain one IP address or CIDR range per line.
    /// Empty lines and lines starting with `#` are skipped. Single IP
    /// addresses without a prefix length are treated as /32 (IPv4) or
    /// /128 (IPv6).
    ///
    /// The new trie is atomically swapped in, so concurrent lookups are
    /// never blocked.
    ///
    /// Returns the number of entries successfully loaded.
    pub fn load_blocklist(&self, path: &Path) -> anyhow::Result<usize> {
        let trie = load_trie_from_file(path)?;
        let count = trie.len();
        self.blocklist.store(Arc::new(trie));
        info!(path = %path.display(), count, "loaded blocklist");
        Ok(count)
    }

    /// Load an allowlist from a file.
    ///
    /// Same format as the blocklist. See [`Self::load_blocklist`] for details.
    ///
    /// Returns the number of entries successfully loaded.
    pub fn load_allowlist(&self, path: &Path) -> anyhow::Result<usize> {
        let trie = load_trie_from_file(path)?;
        let count = trie.len();
        self.allowlist.store(Arc::new(trie));
        info!(path = %path.display(), count, "loaded allowlist");
        Ok(count)
    }

    /// Returns `true` if the address is in the blocklist.
    pub fn is_blocked(&self, addr: IpAddr) -> bool {
        self.blocklist.load().contains(addr)
    }

    /// Returns `true` if the address is in the allowlist.
    pub fn is_allowed(&self, addr: IpAddr) -> bool {
        self.allowlist.load().contains(addr)
    }

    /// Check an IP address against both lists.
    ///
    /// The allowlist takes precedence: if an address appears in both lists,
    /// `IpAction::Allow` is returned. If the address is only in the blocklist,
    /// `IpAction::Block` is returned. Otherwise, `IpAction::None` is returned.
    pub fn check(&self, addr: IpAddr) -> IpAction {
        if self.is_allowed(addr) {
            IpAction::Allow
        } else if self.is_blocked(addr) {
            IpAction::Block
        } else {
            IpAction::None
        }
    }

    /// Reload both lists from the given configuration paths.
    ///
    /// If a path is `None`, the corresponding list is reset to empty.
    /// If a path is `Some` but loading fails, an error is returned and the
    /// existing list is left unchanged.
    pub fn reload_from_config(
        &self,
        blocklist_path: Option<&Path>,
        allowlist_path: Option<&Path>,
    ) -> anyhow::Result<()> {
        match blocklist_path {
            Some(path) => {
                self.load_blocklist(path)?;
            }
            None => {
                self.blocklist.store(Arc::new(IpTrie::new()));
                debug!("cleared blocklist (no path configured)");
            }
        }

        match allowlist_path {
            Some(path) => {
                self.load_allowlist(path)?;
            }
            None => {
                self.allowlist.store(Arc::new(IpTrie::new()));
                debug!("cleared allowlist (no path configured)");
            }
        }

        Ok(())
    }
}

impl Default for IpReputation {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse a file into an `IpTrie`.
///
/// Each line is parsed as either an `IpNet` (CIDR notation) or a bare `IpAddr`
/// (which is wrapped in /32 or /128). Empty lines and comment lines (starting
/// with `#`) are skipped. Lines that fail to parse are logged as warnings and
/// skipped.
fn load_trie_from_file(path: &Path) -> anyhow::Result<IpTrie> {
    let file = std::fs::File::open(path)
        .map_err(|e| anyhow::anyhow!("failed to open {}: {}", path.display(), e))?;
    let reader = std::io::BufReader::new(file);

    let mut trie = IpTrie::new();

    for (line_num, line_result) in reader.lines().enumerate() {
        let line = line_result?;
        let trimmed = line.trim();

        // Skip empty lines and comments.
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Try parsing as CIDR first, then as a bare IP address.
        if let Ok(network) = trimmed.parse::<IpNet>() {
            trie.insert(network);
        } else if let Ok(addr) = trimmed.parse::<IpAddr>() {
            let network = match addr {
                IpAddr::V4(_) => IpNet::new(addr, 32),
                IpAddr::V6(_) => IpNet::new(addr, 128),
            }
            .expect("valid prefix length for host address");
            trie.insert(network);
        } else {
            warn!(
                path = %path.display(),
                line = line_num + 1,
                content = trimmed,
                "skipping unparseable line"
            );
        }
    }

    Ok(trie)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    /// Helper: write contents to a temporary file and return its path.
    /// The caller is responsible for cleaning up the file.
    struct TempFile {
        path: std::path::PathBuf,
    }

    impl TempFile {
        fn new(contents: &str) -> Self {
            let dir = std::env::temp_dir();
            let id = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path =
                dir.join(format!("layer7waf_ip_rep_test_{}_{}", id, std::process::id()));
            let mut f = std::fs::File::create(&path).unwrap();
            f.write_all(contents.as_bytes()).unwrap();
            f.flush().unwrap();
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempFile {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.path);
        }
    }

    #[test]
    fn test_new_is_empty() {
        let rep = IpReputation::new();
        let addr: IpAddr = "10.0.0.1".parse().unwrap();
        assert_eq!(rep.check(addr), IpAction::None);
        assert!(!rep.is_blocked(addr));
        assert!(!rep.is_allowed(addr));
    }

    #[test]
    fn test_load_blocklist() {
        let file = TempFile::new(
            "# Blocklist\n\
             10.0.0.0/8\n\
             192.168.1.1\n\
             \n\
             # Another comment\n\
             172.16.0.0/12\n",
        );

        let rep = IpReputation::new();
        let count = rep.load_blocklist(file.path()).unwrap();
        assert_eq!(count, 3);

        assert!(rep.is_blocked("10.0.0.1".parse().unwrap()));
        assert!(rep.is_blocked("10.255.255.255".parse().unwrap()));
        assert!(rep.is_blocked("192.168.1.1".parse().unwrap()));
        assert!(!rep.is_blocked("192.168.1.2".parse().unwrap()));
        assert!(rep.is_blocked("172.20.0.1".parse().unwrap()));
        assert!(!rep.is_blocked("8.8.8.8".parse().unwrap()));
    }

    #[test]
    fn test_load_allowlist() {
        let file = TempFile::new("127.0.0.1\n::1\n");

        let rep = IpReputation::new();
        let count = rep.load_allowlist(file.path()).unwrap();
        assert_eq!(count, 2);

        assert!(rep.is_allowed("127.0.0.1".parse().unwrap()));
        assert!(rep.is_allowed("::1".parse().unwrap()));
        assert!(!rep.is_allowed("10.0.0.1".parse().unwrap()));
    }

    #[test]
    fn test_check_allow_takes_precedence() {
        let blocklist_file = TempFile::new("10.0.0.0/8\n");
        let allowlist_file = TempFile::new("10.0.0.1\n");

        let rep = IpReputation::new();
        rep.load_blocklist(blocklist_file.path()).unwrap();
        rep.load_allowlist(allowlist_file.path()).unwrap();

        // The specific IP is in both lists; allowlist wins.
        assert_eq!(rep.check("10.0.0.1".parse().unwrap()), IpAction::Allow);
        // Other IPs in the /8 are only blocked.
        assert_eq!(rep.check("10.0.0.2".parse().unwrap()), IpAction::Block);
        // IPs outside both lists return None.
        assert_eq!(rep.check("8.8.8.8".parse().unwrap()), IpAction::None);
    }

    #[test]
    fn test_reload_from_config() {
        let blocklist_file = TempFile::new("10.0.0.0/8\n");
        let allowlist_file = TempFile::new("192.168.1.1\n");

        let rep = IpReputation::new();
        rep.reload_from_config(
            Some(blocklist_file.path()),
            Some(allowlist_file.path()),
        )
        .unwrap();

        assert!(rep.is_blocked("10.0.0.1".parse().unwrap()));
        assert!(rep.is_allowed("192.168.1.1".parse().unwrap()));

        // Reload with no blocklist -- should clear it.
        rep.reload_from_config(None, Some(allowlist_file.path()))
            .unwrap();
        assert!(!rep.is_blocked("10.0.0.1".parse().unwrap()));
        assert!(rep.is_allowed("192.168.1.1".parse().unwrap()));
    }

    #[test]
    fn test_skip_bad_lines() {
        let file = TempFile::new(
            "10.0.0.1\n\
             not-an-ip\n\
             192.168.1.0/24\n",
        );

        let rep = IpReputation::new();
        let count = rep.load_blocklist(file.path()).unwrap();
        // Only the two valid entries should be loaded.
        assert_eq!(count, 2);
    }

    #[test]
    fn test_file_not_found() {
        let rep = IpReputation::new();
        let result = rep.load_blocklist(Path::new("/nonexistent/blocklist.txt"));
        assert!(result.is_err());
    }

    #[test]
    fn test_ipv6_blocklist() {
        let file = TempFile::new("fd00::/8\n2001:db8::1\n");

        let rep = IpReputation::new();
        let count = rep.load_blocklist(file.path()).unwrap();
        assert_eq!(count, 2);

        assert!(rep.is_blocked("fd00::1".parse().unwrap()));
        assert!(rep.is_blocked("fdff::1".parse().unwrap()));
        assert!(rep.is_blocked("2001:db8::1".parse().unwrap()));
        assert!(!rep.is_blocked("2001:db8::2".parse().unwrap()));
    }
}
