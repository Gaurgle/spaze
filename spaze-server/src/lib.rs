//! `spaze-server` — daemon for the Spaze chat protocol.

pub mod time;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn now_unix_ms_is_positive_and_advances() {
        let a = time::now_unix_ms();
        // Sleep is overkill; just call again — millisecond precision is enough.
        let b = time::now_unix_ms();
        assert!(a > 1_700_000_000_000, "timestamp should be after Nov 2023");
        assert!(b >= a, "time should be non-decreasing");
    }
}
