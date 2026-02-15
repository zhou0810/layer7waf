use layer7waf_ip_reputation::{IpAction, IpReputation};
use std::io::Write;
use std::net::IpAddr;

fn write_temp_file(content: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "layer7waf_test_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let mut f = std::fs::File::create(&path).unwrap();
    f.write_all(content.as_bytes()).unwrap();
    path
}

#[test]
fn test_blocklist_cidr() {
    let path = write_temp_file("10.0.0.0/8\n192.168.0.0/16\n");
    let rep = IpReputation::new();
    rep.load_blocklist(&path).unwrap();

    assert!(rep.is_blocked("10.0.0.1".parse().unwrap()));
    assert!(rep.is_blocked("10.255.255.255".parse().unwrap()));
    assert!(rep.is_blocked("192.168.1.1".parse().unwrap()));
    assert!(!rep.is_blocked("8.8.8.8".parse().unwrap()));

    std::fs::remove_file(path).ok();
}

#[test]
fn test_allowlist_takes_precedence() {
    let blocklist_path = write_temp_file("10.0.0.0/8\n");
    let allowlist_path = write_temp_file("10.0.0.5\n");

    let rep = IpReputation::new();
    rep.load_blocklist(&blocklist_path).unwrap();
    rep.load_allowlist(&allowlist_path).unwrap();

    // Allowlisted IP should be allowed even though it's in the /8 block
    assert_eq!(rep.check("10.0.0.5".parse().unwrap()), IpAction::Allow);
    // Other IPs in the /8 should still be blocked
    assert_eq!(rep.check("10.0.0.6".parse().unwrap()), IpAction::Block);
    // IPs outside both lists
    assert_eq!(rep.check("8.8.8.8".parse().unwrap()), IpAction::None);

    std::fs::remove_file(blocklist_path).ok();
    std::fs::remove_file(allowlist_path).ok();
}

#[test]
fn test_ipv6_support() {
    let path = write_temp_file("fd00::/8\n::1\n");
    let rep = IpReputation::new();
    rep.load_blocklist(&path).unwrap();

    assert!(rep.is_blocked("fd00::1".parse().unwrap()));
    assert!(rep.is_blocked("::1".parse().unwrap()));
    assert!(!rep.is_blocked("2001:db8::1".parse().unwrap()));

    std::fs::remove_file(path).ok();
}
