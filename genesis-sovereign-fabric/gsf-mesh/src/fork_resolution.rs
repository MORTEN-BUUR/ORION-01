use gsf_core::SignedLedger;

#[derive(Debug, Clone)]
pub struct ForkResolver;

impl ForkResolver {
    pub fn select_longest_valid(
        chains: Vec<&SignedLedger>,
        verifier: &ed25519_dalek::VerifyingKey,
    ) -> Option<usize> {
        let valid: Vec<_> = chains
            .iter()
            .enumerate()
            .filter(|(_, ledger)| ledger.verify_chain(verifier).is_ok())
            .collect();

        if valid.is_empty() {
            return None;
        }

        valid
            .into_iter()
            .max_by(|(_, a), (_, b)| {
                let len_a = a.entries().len();
                let len_b = b.entries().len();
                len_a.cmp(&len_b).then_with(|| {
                    a.last_hash().cmp(b.last_hash())
                })
            })
            .map(|(i, _)| i)
    }

    pub fn reject_conflict(
        local: &SignedLedger,
        remote: &SignedLedger,
        verifier: &ed25519_dalek::VerifyingKey,
    ) -> bool {
        if local.verify_chain(verifier).is_err() {
            return true;
        }
        if remote.verify_chain(verifier).is_err() {
            return true;
        }
        if local.entries().len() != remote.entries().len() {
            return false;
        }
        for (a, b) in local.entries().iter().zip(remote.entries().iter()) {
            if a.hash != b.hash {
                return true;
            }
        }
        false
    }
}
