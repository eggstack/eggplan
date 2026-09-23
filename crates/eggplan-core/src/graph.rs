use crate::{Plan, PlanItemId, PlanItemStatus, PlanStatus};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum GraphError {
    #[error("dependency graph contains a cycle")]
    DependencyCycle,
    #[error("parent graph contains a cycle")]
    ParentCycle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Readiness {
    Ready,
    WaitingOnDependencies(Vec<PlanItemId>),
    NotActionable,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemReadiness {
    pub item_id: PlanItemId,
    pub readiness: Readiness,
}

pub(crate) fn validate_graph(plan: &Plan) -> Result<(), GraphError> {
    let deps: BTreeMap<_, Vec<_>> = plan
        .items
        .iter()
        .map(|i| (i.id.clone(), i.dependencies.clone()))
        .collect();
    let parents: BTreeMap<_, Vec<_>> = plan
        .items
        .iter()
        .map(|i| (i.id.clone(), i.parent.clone().into_iter().collect()))
        .collect();
    if has_cycle(&deps) {
        return Err(GraphError::DependencyCycle);
    }
    if has_cycle(&parents) {
        return Err(GraphError::ParentCycle);
    }
    Ok(())
}
fn has_cycle(graph: &BTreeMap<PlanItemId, Vec<PlanItemId>>) -> bool {
    // Iterative DFS avoids recursion depth being attacker-controlled.
    let keys: Vec<PlanItemId> = graph.keys().cloned().collect();
    let mut done = BTreeSet::new();
    for start in keys {
        if done.contains(&start) {
            continue;
        }
        let mut stack = vec![(start.clone(), false)];
        let mut active = BTreeSet::new();
        while let Some((node, exiting)) = stack.pop() {
            if exiting {
                active.remove(&node);
                done.insert(node);
                continue;
            }
            if active.contains(&node) {
                return true;
            }
            if done.contains(&node) {
                continue;
            }
            active.insert(node.clone());
            stack.push((node.clone(), true));
            if let Some(value) = graph.get(&node) {
                for next in value {
                    if active.contains(next) {
                        return true;
                    }
                    if !done.contains(next) {
                        stack.push((next.clone(), false));
                    }
                }
            }
        }
    }
    false
}

pub fn readiness(plan: &Plan) -> Vec<ItemReadiness> {
    let by_id: BTreeMap<_, _> = plan.items.iter().map(|i| (&i.id, i)).collect();
    let mut items: Vec<_> = plan.items.iter().collect();
    items.sort_by_key(|i| (i.position, i.id.clone()));
    items
        .into_iter()
        .map(|item| {
            let readiness = if plan.status != PlanStatus::Active
                || !matches!(
                    item.status,
                    PlanItemStatus::Pending | PlanItemStatus::Actionable
                ) {
                Readiness::NotActionable
            } else {
                let waiting: Vec<_> = item
                    .dependencies
                    .iter()
                    .filter(|dep| {
                        !matches!(
                            by_id.get(dep).map(|i| &i.status),
                            Some(PlanItemStatus::Completed)
                        )
                    })
                    .cloned()
                    .collect();
                if waiting.is_empty() {
                    Readiness::Ready
                } else {
                    Readiness::WaitingOnDependencies(waiting)
                }
            };
            ItemReadiness {
                item_id: item.id.clone(),
                readiness,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PlanId, PlanItem, PlanStatus};
    fn item(id: &str, position: u32, dependencies: Vec<&str>) -> PlanItem {
        PlanItem {
            id: PlanItemId::new(format!("epi_{id}")).unwrap(),
            position,
            parent: None,
            dependencies: dependencies
                .into_iter()
                .map(|d| PlanItemId::new(format!("epi_{d}")).unwrap())
                .collect(),
            status: PlanItemStatus::Pending,
            description: id.into(),
            criteria: vec![],
            blocker: None,
            next_action: None,
        }
    }
    fn plan(items: Vec<PlanItem>) -> Plan {
        Plan {
            schema_version: 1,
            id: PlanId::new("ep_test").unwrap(),
            revision: 0,
            objective: "test".into(),
            status: PlanStatus::Active,
            provenance: Default::default(),
            items,
            subject: None,
        }
    }
    #[test]
    fn stable_order_and_waiting() {
        let p = plan(vec![item("b", 1, vec!["a"]), item("a", 0, vec![])]);
        let r = readiness(&p);
        assert_eq!(r[0].item_id.as_str(), "epi_a");
        assert_eq!(
            r[1].readiness,
            Readiness::WaitingOnDependencies(vec![PlanItemId::new("epi_a").unwrap()])
        );
    }
    #[test]
    fn dependency_cycle_rejected() {
        let p = plan(vec![item("a", 0, vec!["b"]), item("b", 1, vec!["a"])]);
        assert_eq!(validate_graph(&p), Err(GraphError::DependencyCycle));
    }
    #[test]
    fn parent_cycle_rejected() {
        let mut a = item("a", 0, vec![]);
        a.parent = Some(PlanItemId::new("epi_b").unwrap());
        let mut b = item("b", 1, vec![]);
        b.parent = Some(PlanItemId::new("epi_a").unwrap());
        assert_eq!(
            validate_graph(&plan(vec![a, b])),
            Err(GraphError::ParentCycle)
        );
    }
}
