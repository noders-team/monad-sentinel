use std::sync::{Mutex, MutexGuard};

/// `.lock().unwrap()` panics forever once a holder panicked (poisoning), which
/// would wedge the whole unattended service until a manual restart. All shared
/// state here is simple value data that stays consistent even if a holder
/// panicked mid-update, so recovering the guard is always the right call.
pub trait LockExt<T> {
    fn lock_ok(&self) -> MutexGuard<'_, T>;
}

impl<T> LockExt<T> for Mutex<T> {
    fn lock_ok(&self) -> MutexGuard<'_, T> {
        self.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn lock_ok_recovers_from_poison() {
        let m = Arc::new(Mutex::new(41));
        let m2 = m.clone();
        // Poison the mutex: panic while holding the guard.
        let _ = std::thread::spawn(move || {
            let _g = m2.lock_ok();
            panic!("poison it");
        })
        .join();
        assert!(m.is_poisoned(), "precondition: mutex must be poisoned");

        let mut g = m.lock_ok(); // must not panic
        *g += 1;
        assert_eq!(*g, 42);
    }
}
