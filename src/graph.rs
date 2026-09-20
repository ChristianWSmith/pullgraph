use std::any::TypeId;
use std::collections::{HashMap, HashSet};

pub(crate) fn has_cycle(
    dependents: &HashMap<TypeId, Vec<TypeId>>,
    from: TypeId,
    to: TypeId,
) -> bool {
    let mut visited = HashSet::new();
    dfs(dependents, from, to, &mut visited)
}

fn dfs(
    dependents: &HashMap<TypeId, Vec<TypeId>>,
    current: TypeId,
    target: TypeId,
    visited: &mut HashSet<TypeId>,
) -> bool {
    if current == target {
        return true;
    }
    if !visited.insert(current) {
        return false;
    }
    if let Some(deps) = dependents.get(&current) {
        for &dep in deps {
            if dfs(dependents, dep, target, visited) {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycle_found_direct() {
        let mut deps = HashMap::new();
        let a = TypeId::of::<u8>();
        let b = TypeId::of::<u16>();
        deps.insert(b, vec![a]);
        deps.insert(a, vec![b]);
        assert!(has_cycle(&deps, a, b));
    }

    #[test]
    fn cycle_found_transitive() {
        let mut deps = HashMap::new();
        let a = TypeId::of::<u8>();
        let b = TypeId::of::<u16>();
        let c = TypeId::of::<u32>();
        deps.insert(a, vec![c]);
        deps.insert(c, vec![b]);
        deps.insert(b, vec![a]);
        assert!(has_cycle(&deps, a, b));
    }

    #[test]
    fn no_cycle_empty_graph() {
        let deps = HashMap::new();
        let a = TypeId::of::<u8>();
        let b = TypeId::of::<u16>();
        assert!(!has_cycle(&deps, a, b));
    }

    #[test]
    fn no_cycle_one_way() {
        let mut deps = HashMap::new();
        let a = TypeId::of::<u8>();
        let b = TypeId::of::<u16>();
        deps.insert(a, vec![b]);
        assert!(!has_cycle(&deps, b, a));
    }

    #[test]
    fn visited_node_skipped() {
        let mut deps = HashMap::new();
        let a = TypeId::of::<u8>();
        let b = TypeId::of::<u16>();
        let c = TypeId::of::<u32>();
        let d = TypeId::of::<u64>();
        deps.insert(a, vec![b, c]);
        deps.insert(b, vec![d]);
        deps.insert(c, vec![d]);
        assert!(!has_cycle(&deps, d, a));
    }

    #[test]
    fn visited_node_prevents_infinite_loop() {
        let mut deps = HashMap::new();
        let a = TypeId::of::<u8>();
        let b = TypeId::of::<u16>();
        deps.insert(a, vec![b]);
        deps.insert(b, vec![a]);
        assert!(has_cycle(&deps, a, a));
    }

    #[test]
    fn visited_node_skipped_on_revisit() {
        let mut deps = HashMap::new();
        let a = TypeId::of::<u8>();
        let b = TypeId::of::<u16>();
        let c = TypeId::of::<u32>();
        let d = TypeId::of::<u64>();
        let e = TypeId::of::<u128>();
        deps.insert(a, vec![b, c]);
        deps.insert(b, vec![d]);
        deps.insert(c, vec![d]);
        assert!(!has_cycle(&deps, a, e));
    }

    #[test]
    fn diamond_no_false_positive() {
        let mut deps = HashMap::new();
        let a = TypeId::of::<u8>();
        let b = TypeId::of::<u16>();
        let c = TypeId::of::<u32>();
        let d = TypeId::of::<u64>();
        deps.insert(a, vec![b, c]);
        deps.insert(b, vec![d]);
        deps.insert(c, vec![d]);
        assert!(!has_cycle(&deps, d, a));
    }

    #[test]
    fn no_cycle_long_chain() {
        let mut deps = HashMap::new();
        let a = TypeId::of::<u8>();
        let b = TypeId::of::<u16>();
        let c = TypeId::of::<u32>();
        let d = TypeId::of::<u64>();
        deps.insert(a, vec![b]);
        deps.insert(b, vec![c]);
        deps.insert(c, vec![d]);
        assert!(!has_cycle(&deps, d, a));
    }

    #[test]
    fn no_path_to_target() {
        let mut deps = HashMap::new();
        let a = TypeId::of::<u8>();
        let b = TypeId::of::<u16>();
        let c = TypeId::of::<u32>();
        deps.insert(a, vec![b]);
        deps.insert(c, vec![b]);
        assert!(!has_cycle(&deps, b, a));
    }
}
