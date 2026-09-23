use crate::Plan;
use serde::Serialize;
use sha2::{Digest, Sha256};

pub const SCHEMA_VERSION: u32 = 1;

/// Serialize a validated object as compact serde_json bytes. Schema-v1 structs
/// have fixed declaration order; maps use serde_json's sorted key order.
pub fn canonical_json<T: Serialize>(value: &T) -> Result<Vec<u8>, serde_json::Error> {
    serde_json::to_vec(value)
}

pub fn digest_json<T: Serialize>(value: &T) -> Result<String, serde_json::Error> {
    let bytes = canonical_json(value)?;
    let digest = Sha256::digest(bytes);
    Ok(format!("sha256:{digest:x}"))
}

pub fn parse_plan(bytes: &[u8]) -> Result<Plan, Box<dyn std::error::Error + Send + Sync>> {
    let plan: Plan = serde_json::from_slice(bytes)?;
    plan.validate()?;
    Ok(plan)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PlanId, PlanItem};
    fn plan() -> Plan {
        Plan::new(
            PlanId::new("ep_fixture").unwrap(),
            "example",
            vec![PlanItem {
                id: crate::PlanItemId::new("epi_fixture").unwrap(),
                position: 0,
                parent: None,
                dependencies: vec![],
                status: crate::PlanItemStatus::Pending,
                description: "first step".into(),
                criteria: vec![],
                blocker: None,
                next_action: None,
            }],
        )
        .unwrap()
    }
    #[test]
    fn schema_v1_golden_plan_bytes_and_digest() {
        let p = plan();
        let bytes = canonical_json(&p).unwrap();
        assert_eq!(
            std::str::from_utf8(&bytes).unwrap(),
            include_str!("../tests/fixtures/schema-v1-plan.json").trim()
        );
        assert_eq!(
            digest_json(&p).unwrap(),
            include_str!("../tests/fixtures/schema-v1-plan.sha256").trim()
        );
    }
    #[test]
    fn roundtrip_and_unknown_schema_rejected() {
        let p = plan();
        let parsed = parse_plan(&canonical_json(&p).unwrap()).unwrap();
        assert_eq!(p, parsed);
        let bad = serde_json::to_vec(&{
            let mut p = p;
            p.schema_version = 2;
            p
        })
        .unwrap();
        assert!(parse_plan(&bad).is_err());
    }

    #[test]
    fn nested_unknown_plan_fields_fail_closed() {
        let base = plan();
        let mut value: serde_json::Value =
            serde_json::from_slice(&canonical_json(&base).unwrap()).unwrap();
        value["future"] = serde_json::json!(true);
        assert!(parse_plan(&serde_json::to_vec(&value).unwrap()).is_err());

        let mut value: serde_json::Value =
            serde_json::from_slice(&canonical_json(&base).unwrap()).unwrap();
        value["items"][0]["future"] = serde_json::json!(true);
        assert!(parse_plan(&serde_json::to_vec(&value).unwrap()).is_err());

        let mut value: serde_json::Value =
            serde_json::from_slice(&canonical_json(&base).unwrap()).unwrap();
        value["items"][0]["criteria"] = serde_json::json!([{
            "id": "epc_criterion", "statement": "check", "human_judgment_allowed": false,
            "requirements": [{
                "description": "test", "kind": "test", "subject_policy": "exact",
                "cardinality": "any", "min_count": 1, "allow_human_judgment": false,
                "provider": "epp_test", "future": true
            }], "future": true
        }]);
        assert!(parse_plan(&serde_json::to_vec(&value).unwrap()).is_err());

        let mut value: serde_json::Value =
            serde_json::from_slice(&canonical_json(&base).unwrap()).unwrap();
        value["subject"] = serde_json::json!({"subject_kind":"git","repository_id":"epr_test","revision":"abc","state":"clean","future":true});
        assert!(parse_plan(&serde_json::to_vec(&value).unwrap()).is_err());
    }
}
