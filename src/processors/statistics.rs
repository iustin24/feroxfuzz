use super::{Ordering, ProcessorHooks};

use crate::actions::Action;
use crate::observers::Observers;
use crate::requests::Request;
use crate::responses::Response;
use crate::state::SharedState;
use crate::statistics::Statistics;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use tracing::instrument;

use std::sync::{Arc, RwLock};

use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(docsrs)] {
        // just bringing in types for easier intra-doc linking during doc build
        use crate::fuzzers::Fuzzer;
        use crate::build_processors;
        use crate::deciders::Deciders;
        use crate::processors::Processors;
    }
}

/// a `StatisticsProcessor` provides access to the fuzzer's instance of [`Statistics`]
/// as well as the [`Action`] returned from calling the analogous hook on [`Deciders`].
/// Those two objects may be used to produce side-effects, such as printing, logging,
/// etc...
///
/// # Examples
///
/// While the example below works, the normal use-case for this struct is to pass
/// it, and any other [`Processors`] to the [`build_processors`] macro, and pass
/// the result of that call to your chosen [`Fuzzer`] implementation.
///
/// ```
/// # use http::response;
/// # use feroxfuzz::prelude::*;
/// # use feroxfuzz::processors::{Ordering, StatisticsProcessor, ProcessorHooks};
/// # use feroxfuzz::observers::ResponseObserver;
/// # use feroxfuzz::responses::BlockingResponse;
/// # use feroxfuzz::corpora::RangeCorpus;
/// # use feroxfuzz::requests::RequestId;
/// # use std::time::Duration;
/// # fn main() -> Result<(), FeroxFuzzError> {
/// // for testing, normal Response comes as a result of a sent request
/// let reqwest_response = http::response::Builder::new().status(200).body("").unwrap();
/// let id = RequestId::new(0);
/// let elapsed = Duration::from_secs(1);
/// let response = BlockingResponse::try_from_reqwest_response(id, reqwest_response.into(), elapsed)?;
///
/// // also not relevant to the current example, but it's needed to make the call to the hook
/// let mut state = SharedState::with_corpus(RangeCorpus::with_stop(3).name("range").build()?);
///
/// // a ResponseObserver should have already been created at some point
/// let response_observer = ResponseObserver::<_>::with_response(response);
/// let observers = build_observers!(response_observer);
///
/// // create a StatisticsProcessor that executes the provided closure after
/// // the client has sent the request (aka `PostSend`). Within the closure,
/// // access to the fuzzer's instance of `Statistics` is provided
/// let mut stats_printer = StatisticsProcessor::new(Ordering::PostSend, |statistics, _action, _state| {
///     if let Ok(guard) = statistics.read() {
///         println!(
///             "{} reqs/sec (requests: {}, elapsed: {:?})",
///             guard.requests_per_sec(),
///             guard.requests(),
///             guard.elapsed()
///         );
///     }
/// });
///
/// // finally, make the call to `post_send_hook`, allowing the side-effect to take place
/// stats_printer.post_send_hook(&mut state, &observers, None);
///
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[allow(clippy::derive_partial_eq_without_eq)] // known false-positive introduced in 1.63.0
pub struct StatisticsProcessor<F>
where
    F: Fn(Arc<RwLock<Statistics>>, Option<&Action>, &SharedState),
{
    processor: F,
    ordering: Ordering,
}

impl<F> StatisticsProcessor<F>
where
    F: Fn(Arc<RwLock<Statistics>>, Option<&Action>, &SharedState),
{
    /// create a new `StatisticsProcessor` that calls `processor` in
    /// either `pre_send_hook`, `post_send_hook`, or both, depending
    /// on the [`Ordering`] value passed to this constructor
    pub const fn new(ordering: Ordering, processor: F) -> Self {
        Self {
            processor,
            ordering,
        }
    }
}

impl<F> ProcessorHooks for StatisticsProcessor<F>
where
    F: Fn(Arc<RwLock<Statistics>>, Option<&Action>, &SharedState),
{
    #[instrument(skip_all, fields(?self.ordering, ?action), level = "trace")]
    fn pre_send_hook(
        &mut self,
        state: &SharedState,
        _request: &mut Request,
        action: Option<&Action>,
    ) {
        match self.ordering {
            Ordering::PreSend | Ordering::PreAndPostSend => {
                (self.processor)(state.stats(), action, state);
            }
            Ordering::PostSend => {}
        }
    }

    #[instrument(skip_all, fields(?self.ordering, ?action), level = "trace")]
    fn post_send_hook<O, R>(&mut self, state: &SharedState, _observers: &O, action: Option<&Action>)
    where
        O: Observers<R>,
        R: Response,
    {
        match self.ordering {
            Ordering::PostSend | Ordering::PreAndPostSend => {
                (self.processor)(state.stats(), action, state);
            }
            Ordering::PreSend => {}
        }
    }
}
