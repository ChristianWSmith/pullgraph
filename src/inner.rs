use std::any::TypeId;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::provider::{AnyProvider, wrap_provider};

pub(crate) struct ProviderEntry {
    pub(crate) provider: Option<Box<dyn AnyProvider>>,
    pub(crate) demand_count: usize,
    pub(crate) active: bool,
    pub(crate) dependencies: Vec<TypeId>,
}

pub(crate) struct State {
    pub(crate) providers: HashMap<TypeId, ProviderEntry>,
    pub(crate) dependents: HashMap<TypeId, Vec<TypeId>>,
}

pub(crate) struct PullGraphInner {
    pub(crate) state: Mutex<State>,
}

impl PullGraphInner {
    pub(crate) fn new() -> Self {
        Self {
            state: Mutex::new(State {
                providers: HashMap::new(),
                dependents: HashMap::new(),
            }),
        }
    }

    pub(crate) fn register_provider<T: 'static>(
        self: &Arc<Self>,
        provider: impl crate::provider::Provider<T> + 'static,
        dependencies: Vec<TypeId>,
    ) -> Result<(), crate::error::PullGraphError> {
        let type_id = TypeId::of::<T>();
        let mut state = self.state.lock().unwrap();

        if state.providers.contains_key(&type_id) {
            return Err(crate::error::PullGraphError::AlreadyRegistered(type_id));
        }

        let entry = ProviderEntry {
            provider: Some(wrap_provider(provider)),
            demand_count: 0,
            active: false,
            dependencies: dependencies.clone(),
        };
        state.providers.insert(type_id, entry);

        for dep in &dependencies {
            state.dependents.entry(*dep).or_default().push(type_id);
        }

        Ok(())
    }

    pub(crate) fn acquire_activation(
        self: &Arc<Self>,
        type_id: TypeId,
    ) -> crate::demand::ActivationToken {
        let (was_zero, deps) = {
            let mut state = self.state.lock().unwrap();
            let entry = state
                .providers
                .get_mut(&type_id)
                .expect("no provider registered");
            let was_zero = entry.demand_count == 0;
            entry.demand_count += 1;
            if was_zero {
                entry.active = true;
            }
            (was_zero, entry.dependencies.clone())
        };

        if was_zero {
            let mut provider_opt = None;
            {
                let mut state = self.state.lock().unwrap();
                if let Some(entry) = state.providers.get_mut(&type_id) {
                    provider_opt = entry.provider.take();
                }
            }

            if let Some(mut provider) = provider_opt {
                provider.activate();

                let mut state = self.state.lock().unwrap();
                if let Some(entry) = state.providers.get_mut(&type_id) {
                    entry.provider = Some(provider);
                }
            }
        }

        let mut dep_tokens = Vec::new();
        if was_zero {
            for dep in deps {
                dep_tokens.push(self.acquire_activation(dep));
            }
        }

        crate::demand::ActivationToken {
            type_id,
            inner: Arc::clone(self),
            dep_tokens,
        }
    }

    pub(crate) fn release_activation(&self, type_id: TypeId) {
        let should_deactivate = {
            let mut state = self.state.lock().unwrap();
            let entry = state
                .providers
                .get_mut(&type_id)
                .expect("no provider registered");
            entry.demand_count -= 1;
            entry.demand_count == 0
        };

        if should_deactivate {
            let mut provider_opt = None;
            {
                let mut state = self.state.lock().unwrap();
                if let Some(entry) = state.providers.get_mut(&type_id) {
                    entry.active = false;
                    provider_opt = entry.provider.take();
                }
            }

            if let Some(mut provider) = provider_opt {
                provider.deactivate();

                let mut state = self.state.lock().unwrap();
                if let Some(entry) = state.providers.get_mut(&type_id) {
                    entry.provider = Some(provider);
                }
            }
        }
    }

    pub(crate) fn is_demanded(&self, type_id: TypeId) -> bool {
        let state = self.state.lock().unwrap();
        state
            .providers
            .get(&type_id)
            .is_some_and(|e| e.demand_count > 0)
    }

    pub(crate) fn demand_count(&self, type_id: TypeId) -> usize {
        let state = self.state.lock().unwrap();
        state.providers.get(&type_id).map_or(0, |e| e.demand_count)
    }

    pub(crate) fn get_dependencies(&self, type_id: TypeId) -> Vec<TypeId> {
        let state = self.state.lock().unwrap();
        state
            .providers
            .get(&type_id)
            .map_or_else(Vec::new, |e| e.dependencies.clone())
    }

    pub(crate) fn take_provider(&self, type_id: TypeId) -> Option<Option<Box<dyn AnyProvider>>> {
        let mut state = self.state.lock().unwrap();
        state.providers.get_mut(&type_id).map(|e| e.provider.take())
    }

    pub(crate) fn restore_provider(&self, type_id: TypeId, provider: Box<dyn AnyProvider>) {
        let mut state = self.state.lock().unwrap();
        if let Some(entry) = state.providers.get_mut(&type_id) {
            entry.provider = Some(provider);
        }
    }

    #[cfg(test)]
    pub(crate) fn remove_provider(&self, type_id: TypeId) {
        let mut state = self.state.lock().unwrap();
        state.providers.remove(&type_id);
    }

    #[cfg(test)]
    pub(crate) fn null_provider(&self, type_id: TypeId) {
        let mut state = self.state.lock().unwrap();
        if let Some(entry) = state.providers.get_mut(&type_id) {
            entry.provider = None;
        }
    }
}
