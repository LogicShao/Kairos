/// Generate a stable, deterministic i32 notification ID from a string key.
///
/// Uses FNV-1a hash (64-bit) for cross-platform determinism — same input always yields
/// the same output, regardless of process restarts or platform.
///
/// The output is constrained to the positive i32 range (0 .. i32::MAX) so it is safe to
/// pass as a notification identifier across FFI/system APIs.
pub fn stable_id(key: &str) -> i32 {
    // FNV-1a 64-bit constants
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in key.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    // Keep in positive i32 range
    (hash & 0x7FFF_FFFF) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deterministic() {
        let a = stable_id("exam:abc-123:1440");
        let b = stable_id("exam:abc-123:1440");
        assert_eq!(a, b);
    }

    #[test]
    fn test_different_keys() {
        let a = stable_id("exam:abc-123:1440");
        let b = stable_id("exam:abc-123:60");
        assert_ne!(a, b);
    }

    #[test]
    fn test_positive_range() {
        for key in &["a", "b", "exam:test:1440", "longer-key-here:12345:60"] {
            let id = stable_id(key);
            assert!(id >= 0, "id {id} for key {key} should be non-negative");
        }
    }
}
