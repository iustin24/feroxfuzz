//! `LibAFL`'s havoc mutations
use super::Mutator;
use crate::input::Data;
use crate::state::SharedState;
use crate::{atomic_load, error::FeroxFuzzError};

use std::sync::atomic::Ordering;

use libafl::bolts::rands::Rand;
use libafl::state::HasRand;

use super::afl::{
    BitFlipMutator, ByteAddMutator, ByteDecMutator, ByteFlipMutator, ByteIncMutator,
    ByteInterestingMutator, ByteNegMutator, ByteRandMutator, BytesCopyMutator, BytesDeleteMutator,
    BytesExpandMutator, BytesInsertCopyMutator, BytesInsertMutator, BytesRandInsertMutator,
    BytesRandSetMutator, BytesSetMutator, BytesSwapMutator, CrossoverInsertMutator,
    CrossoverReplaceMutator, DwordAddMutator, DwordInterestingMutator, LibAflMutator,
    QwordAddMutator, WordAddMutator, WordInterestingMutator,
};

use cfg_if::cfg_if;
use tracing::error;

cfg_if! {
    if #[cfg(docsrs)] {
        // just bringing in types for easier intra-doc linking during doc build
        use crate::build_mutators;
        use crate::fuzzers::Fuzzer;
        use crate::mutators::Mutators;
    }
}

/// Feroxfuzz's bridge to `LibAFL`'s havoc mutations
///
/// # Examples
///
/// While the example below works, the normal use-case for this struct is to pass
/// it, and any other [`Mutators`] to the [`build_mutators`] macro, and pass
/// the result of that call to your chosen [`Fuzzer`] implementation.
///
/// ```
/// # use feroxfuzz::corpora::RangeCorpus;
/// # use feroxfuzz::state::SharedState;
/// # use feroxfuzz::input::Data;
/// # use feroxfuzz::mutators::Mutator;
/// # use feroxfuzz::mutators::HavocMutator;
/// # use feroxfuzz::error::FeroxFuzzError;
/// # use crate::feroxfuzz::AsInner;
/// # fn main() -> Result<(), FeroxFuzzError> {
/// let corpus = RangeCorpus::with_stop(20).name("corpus").build()?;
///
/// let mut state = SharedState::with_corpus(corpus);
///
/// let mut mutator = HavocMutator::new("corpus");
///
/// let mut to_mutate = Data::Fuzzable(b"some seed string".to_vec());
///
/// mutator.mutate(&mut to_mutate, &mut state)?;
///
/// assert_ne!(to_mutate.inner(), &b"some seed string".to_vec());
/// # Ok(())
/// # }
/// ```
#[derive(Default, Debug)]
pub struct HavocMutator {
    corpus_name: String,
    max_power_of_two: u64,
    mutators: Vec<LibAflMutator>,
}

impl HavocMutator {
    /// create a new `HavocMutator` mutator
    ///
    /// # Examples
    ///
    /// ```
    /// # use feroxfuzz::mutators::Mutator;
    /// # use feroxfuzz::mutators::HavocMutator;
    /// let mutator = HavocMutator::new("associated-corpus-name");
    /// ```
    #[must_use]
    pub fn new(corpus_name: &str) -> Self {
        let mutators = vec![
            LibAflMutator::BitFlipMutator(BitFlipMutator::new()),
            LibAflMutator::ByteFlipMutator(ByteFlipMutator::new()),
            LibAflMutator::ByteIncMutator(ByteIncMutator::new()),
            LibAflMutator::ByteDecMutator(ByteDecMutator::new()),
            LibAflMutator::ByteNegMutator(ByteNegMutator::new()),
            LibAflMutator::ByteRandMutator(ByteRandMutator::new()),
            LibAflMutator::ByteAddMutator(ByteAddMutator::new()),
            LibAflMutator::WordAddMutator(WordAddMutator::new()),
            LibAflMutator::DwordAddMutator(DwordAddMutator::new()),
            LibAflMutator::QwordAddMutator(QwordAddMutator::new()),
            LibAflMutator::ByteInterestingMutator(ByteInterestingMutator::new()),
            LibAflMutator::WordInterestingMutator(WordInterestingMutator::new()),
            LibAflMutator::DwordInterestingMutator(DwordInterestingMutator::new()),
            LibAflMutator::BytesDeleteMutator(BytesDeleteMutator::new()),
            LibAflMutator::BytesDeleteMutator(BytesDeleteMutator::new()),
            LibAflMutator::BytesDeleteMutator(BytesDeleteMutator::new()),
            LibAflMutator::BytesDeleteMutator(BytesDeleteMutator::new()),
            LibAflMutator::BytesExpandMutator(BytesExpandMutator::new()),
            LibAflMutator::BytesInsertMutator(BytesInsertMutator::new()),
            LibAflMutator::BytesRandInsertMutator(BytesRandInsertMutator::new()),
            LibAflMutator::BytesSetMutator(BytesSetMutator::new()),
            LibAflMutator::BytesRandSetMutator(BytesRandSetMutator::new()),
            LibAflMutator::BytesCopyMutator(BytesCopyMutator::new()),
            LibAflMutator::BytesInsertCopyMutator(BytesInsertCopyMutator::new()),
            LibAflMutator::BytesSwapMutator(BytesSwapMutator::new()),
            LibAflMutator::CrossoverInsertMutator(CrossoverInsertMutator::new(corpus_name)),
            LibAflMutator::CrossoverReplaceMutator(CrossoverReplaceMutator::new(corpus_name)),
        ];

        Self {
            corpus_name: corpus_name.to_string(),
            max_power_of_two: 6,
            mutators,
        }
    }

    /// Compute the number of iterations used to apply stacked mutations
    fn iterations(&self, state: &mut SharedState) -> u64 {
        // determine # of iterations to perform, this mimics StdMutationalStage::iterations
        // iterations will a power of two:
        // i.e. when 7 is passed to .below(), the possible values would be
        // one of -> 2, 4, 8, 16, 32, 64, or 128
        //
        // per toka, can tweak the max_power_of_two value as needed. libafl uses 7, but
        // during testing, using 2^6 results in nearly 9K reqs/sec more than 2^7, so
        // I'm using 2^6 for now.
        1 << (1 + state.rand_mut().below(self.max_power_of_two))
    }
}

impl Mutator for HavocMutator {
    fn mutate(&mut self, input: &mut Data, state: &mut SharedState) -> Result<(), FeroxFuzzError> {
        // havoc mutations differ from the wordlist token mutator in a very key way:
        //
        // the wordlist token mutator pulls a fuzzable field from the request, a static
        // field from the corpus, and then performs a mutation on the request's field.
        // the mutation is applied to the request field directly, and not to the corpus item.
        //
        // the havoc mutations, on the other hand, expect to mutate the entries pulled from
        // the corpus.
        //
        // as a result, the code below flips the paradigm and allows the libafl mutators
        // to work in the way they were designed. basically, we just pull the corpus item
        // from the corpus, and then swap it with the request field and make it fuzzable,
        // by calling `toggle_type`.

        // get the current index from the schduler via the state
        let corpus = state.corpus_by_name(&self.corpus_name)?;
        let scheduled_idx = state.corpus_index_by_name(&self.corpus_name)?;
        let index = atomic_load!(scheduled_idx);

        if let Ok(guard) = corpus.read() {
            // get the corpus item at the current index
            let result = guard.get(index);

            // if the index is out of bounds, return an error
            if result.is_none() {
                // using is_none with a return instead of a map_err because the
                // guard can't be borrowed twice in order to get the .name() of the corpus
                error!(name=guard.name(), %index, "corpus entry not found");

                return Err(FeroxFuzzError::CorpusEntryNotFound {
                    name: guard.name().to_string(),
                    index,
                });
            }

            // overwrite the request's fuzzable field with the static field from the corpus
            //
            // i.e. Data::Static(b"foo-from-corpus") => Data::Fuzzable(b"Request-field")
            *input = result.unwrap().clone();

            // make the new field fuzzable
            //
            // i.e. Data::Static(b"foo-from-corpus") => Data::Fuzzable(b"foo-from-corpus")
            if !input.is_fuzzable() {
                input.toggle_type();
            }
        }

        // with the corpus item in hand, we can now perform the mutations

        for _ in 0..self.iterations(state) {
            // select a random offset into the list of mutators
            let upper_bound = self.mutators.len() as u64 + 1;

            #[allow(clippy::cast_possible_truncation)]
            let mutator_idx = state.rand_mut().below(upper_bound) as usize;

            // walk the list of mutators from the offset to the beginning, calling mutate on each
            //
            // the pattern is derived from libafl, and the intuition is that the more expensive
            // mutations are at the end of the list, so we want to limit their use.
            for mutator in self.mutators[..mutator_idx].iter_mut().rev() {
                mutator.mutate(input, state)?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::corpora::{RangeCorpus, Wordlist};
    use crate::std_ext::convert::AsInner;
    use std::collections::HashMap;

    macro_rules! test_mutator {
        ($name:ident, $mutator:expr) => {
            #[test]
            fn $name() {
                let mut mutator = $mutator;
                let corpus = Wordlist::with_words(["foooooooooo", "barrrrrrrrrrr"])
                    .name("corpus")
                    .build();
                let mut state = SharedState::with_corpus(corpus);
                state.set_seed(0x12);
                let mut data = Data::Fuzzable(b"stuff and things".to_vec());
                mutator.mutate(&mut data, &mut state).unwrap();
                assert_ne!(data.inner(), &b"stuff and things".to_vec());
            }
        };
    }

    test_mutator!(bit_flip, BitFlipMutator::new());
    test_mutator!(byte_flip, ByteFlipMutator::new());
    test_mutator!(byte_inc, ByteIncMutator::new());
    test_mutator!(byte_dec, ByteDecMutator::new());
    test_mutator!(byte_neg, ByteNegMutator::new());
    test_mutator!(byte_rand, ByteRandMutator::new());
    test_mutator!(byte_add, ByteAddMutator::new());
    test_mutator!(word_add, WordAddMutator::new());
    test_mutator!(dword_add, DwordAddMutator::new());
    test_mutator!(qword_add, QwordAddMutator::new());
    test_mutator!(byte_interesting, ByteInterestingMutator::new());
    test_mutator!(word_interesting, WordInterestingMutator::new());
    test_mutator!(dword_interesting, DwordInterestingMutator::new());
    test_mutator!(bytes_delete, BytesDeleteMutator::new());
    test_mutator!(bytes_expand, BytesExpandMutator::new());
    test_mutator!(bytes_insert, BytesInsertMutator::new());
    test_mutator!(bytes_rand_insert, BytesRandInsertMutator::new());
    test_mutator!(bytes_set, BytesSetMutator::new());
    test_mutator!(bytes_rand_set, BytesRandSetMutator::new());
    test_mutator!(bytes_copy, BytesCopyMutator::new());
    test_mutator!(bytes_insert_copy, BytesInsertCopyMutator::new());
    test_mutator!(bytes_swap, BytesSwapMutator::new());
    test_mutator!(crossover_replace, CrossoverReplaceMutator::new("corpus"));
    test_mutator!(crossover_insert, CrossoverInsertMutator::new("corpus"));

    #[test]
    fn test_havoc_mutator_new() {
        let mutator = HavocMutator::new("corpus");
        assert_eq!(mutator.mutators.len(), 27);
    }

    #[test]
    fn test_havoc_mutator_max_power_of_two() {
        let mutator = HavocMutator::new("corpus");
        assert_eq!(mutator.max_power_of_two, 6);
    }

    #[test]
    fn test_havoc_mutator_iterations() {
        let mutator = HavocMutator::new("corpus");
        let corpus = RangeCorpus::with_stop(10).name("corpus").build().unwrap();
        let mut state = SharedState::with_corpus(corpus);

        let mut counter = HashMap::new();

        (0..200).for_each(|_| {
            let iterations = mutator.iterations(&mut state);
            *counter.entry(iterations).or_insert(0) += 1;
        });

        let expected = [2, 4, 8, 16, 32, 64];

        (0..129).for_each(|i| {
            if expected.contains(&i) {
                assert!(counter.get(&i).is_some());
            } else {
                assert!(counter.get(&i).is_none());
            }
        });
    }
}
