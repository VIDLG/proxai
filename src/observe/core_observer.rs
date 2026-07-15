use proxai_core::observe::{Observation, Observer};

use super::ObserveContext;

impl Observer for ObserveContext {
    fn observe(&self, observation: &Observation) {
        self.span.in_scope(|| self.sinks.observe_core(observation));
    }
}
