#[cfg(any(test, feature = "testutils"))]
pub trait LedgerMutExt {
    fn with_mut<F>(&self, f: F)
    where
        F: FnOnce(&mut soroban_sdk::testutils::LedgerInfo);
}

#[cfg(any(test, feature = "testutils"))]
impl LedgerMutExt for soroban_sdk::ledger::Ledger {
    fn with_mut<F>(&self, f: F)
    where
        F: FnOnce(&mut soroban_sdk::testutils::LedgerInfo),
    {
        use soroban_sdk::testutils::Ledger as _;
        let mut info = self.get();
        f(&mut info);
        self.set(info);
    }
}
