use std::any::TypeId;
use std::fmt;

/// Errors that can occur when interacting with a [`PullGraph`](crate::PullGraph).
#[derive(Debug)]
pub enum PullGraphError {
    /// A provider is already registered for the given type.
    AlreadyRegistered(TypeId),
    /// No provider has been registered for the requested type.
    NoProvider(TypeId),
    /// Registering the provider would create a dependency cycle.
    DependencyCycle(Vec<TypeId>),
}

impl fmt::Display for PullGraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PullGraphError::AlreadyRegistered(id) => {
                write!(f, "provider already registered for type {:?}", id)
            }
            PullGraphError::NoProvider(id) => {
                write!(f, "no provider registered for type {:?}", id)
            }
            PullGraphError::DependencyCycle(ids) => {
                write!(f, "dependency cycle detected involving {} types", ids.len())
            }
        }
    }
}

impl std::error::Error for PullGraphError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn display_already_registered() {
        let err = PullGraphError::AlreadyRegistered(TypeId::of::<i32>());
        let msg = format!("{}", err);
        assert!(msg.contains("already registered"));
    }

    #[test]
    fn display_no_provider() {
        let err = PullGraphError::NoProvider(TypeId::of::<i32>());
        let msg = format!("{}", err);
        assert!(msg.contains("no provider"));
    }

    #[test]
    fn display_dependency_cycle() {
        let err = PullGraphError::DependencyCycle(vec![
            TypeId::of::<i32>(),
            TypeId::of::<i64>(),
            TypeId::of::<f64>(),
        ]);
        let msg = format!("{}", err);
        assert!(msg.contains("3 types"));
    }

    #[test]
    fn error_trait_implemented() {
        let err: &dyn Error = &PullGraphError::AlreadyRegistered(TypeId::of::<i32>());
        let _ = err.source();
    }

    #[test]
    fn debug_is_derived() {
        let err = PullGraphError::NoProvider(TypeId::of::<i32>());
        let debug = format!("{:?}", err);
        assert!(debug.contains("NoProvider"));
    }
}
