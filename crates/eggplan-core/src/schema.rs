use crate::{Plan, VerificationDigest};
use serde::Serialize;
use sha2::{Digest, Sha256};

pub const SCHEMA_VERSION: u32 = 2;

/// Serialize a validated object as compact serde_json bytes. Versioned structs
/// have fixed declaration order; maps use serde_json's sorted key order.
pub fn canonical_json<T: Serialize>(value: &T) -> Result<Vec<u8>, serde_json::Error> {
    serde_json::to_vec(value)
}

pub fn digest_json<T: Serialize>(value: &T) -> Result<String, serde_json::Error> {
    let bytes = canonical_json(value)?;
    let digest = Sha256::digest(bytes);
    Ok(format!("sha256:{digest:x}"))
}

/// Build the shared, domain-separated verification identity used by evidence
/// providers. Payload values must be canonicalizable JSON and are bounded to
/// keep host supplied execution descriptions finite.
pub fn verification_digest(
    provider_namespace: &str,
    schema_version: u32,
    payload: &serde_json::Value,
) -> Result<VerificationDigest, String> {
    if provider_namespace.is_empty()
        || provider_namespace.len() > crate::bounds::ID_CHARS
        || provider_namespace.contains('\0')
        || schema_version == 0
    {
        return Err("invalid verification namespace or schema version".into());
    }
    fn validate(value: &serde_json::Value, depth: usize, nodes: &mut usize) -> Result<(), String> {
        *nodes += 1;
        if depth > 32 || *nodes > 4_096 {
            return Err("verification specification structure exceeds bound".into());
        }
        match value {
            serde_json::Value::String(text)
                if text.contains('\0') || text.chars().count() > 4_000 =>
            {
                Err("verification string contains NUL or exceeds bound".into())
            }
            serde_json::Value::Array(values) => {
                if values.len() > 512 {
                    return Err("verification array exceeds bound".into());
                }
                for value in values {
                    validate(value, depth + 1, nodes)?;
                }
                Ok(())
            }
            serde_json::Value::Object(values) => {
                if values.len() > 512 {
                    return Err("verification object exceeds bound".into());
                }
                for (key, value) in values {
                    if key.is_empty() || key.len() > crate::bounds::ID_CHARS || key.contains('\0') {
                        return Err("invalid verification key".into());
                    }
                    validate(value, depth + 1, nodes)?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }
    let mut nodes = 0;
    validate(payload, 0, &mut nodes)?;
    #[derive(Serialize)]
    struct Envelope<'a> {
        domain: &'static str,
        provider_namespace: &'a str,
        schema_version: u32,
        canonical_payload: &'a serde_json::Value,
    }
    let bytes = serde_json::to_vec(&Envelope {
        domain: "eggplan.provider-verification.v1",
        provider_namespace,
        schema_version,
        canonical_payload: payload,
    })
    .map_err(|error| error.to_string())?;
    if bytes.len() > 65_536 {
        return Err("verification specification exceeds byte bound".into());
    }
    let digest = Sha256::digest(bytes);
    VerificationDigest::new(format!("sha256:{digest:x}")).map_err(|error| error.to_string())
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
    fn bound_v2_plan() -> Plan {
        let mut plan = plan();
        plan.items[0].criteria.push(crate::AcceptanceCriterion {
            id: crate::CriterionId::new("epc_fixture").unwrap(),
            statement: "designated test passed".into(),
            human_judgment_allowed: false,
            requirements: vec![crate::EvidenceRequirement {
                description: "designated test invocation".into(),
                kind: crate::EvidenceKind::Test,
                provider: None,
                subject_policy: crate::SubjectPolicy::Exact,
                cardinality: crate::EvidenceCardinality::Any,
                min_count: 1,
                allow_human_judgment: false,
                expected_verification_digest: Some(
                    crate::VerificationDigest::new(format!("sha256:{}", "a".repeat(64))).unwrap(),
                ),
            }],
        });
        plan.validate().unwrap();
        plan
    }
    #[test]
    fn schema_v1_golden_plan_bytes_and_digest() {
        let mut p = plan();
        p.schema_version = 1;
        let bytes = canonical_json(&p).unwrap();
        assert_eq!(
            std::str::from_utf8(&bytes).unwrap(),
            include_str!("../tests/fixtures/schema-v1-plan.json").trim()
        );
        assert_eq!(
            digest_json(&p).unwrap(),
            include_str!("../tests/fixtures/schema-v1-plan.sha256").trim()
        );
        assert_eq!(parse_plan(&bytes).unwrap(), p);
    }

    #[test]
    fn schema_v2_golden_plan_bytes_and_digest() {
        let p = bound_v2_plan();
        let bytes = canonical_json(&p).unwrap();
        assert_eq!(
            std::str::from_utf8(&bytes).unwrap(),
            include_str!("../tests/fixtures/schema-v2-plan.json").trim()
        );
        assert_eq!(
            digest_json(&p).unwrap(),
            include_str!("../tests/fixtures/schema-v2-plan.sha256").trim()
        );
    }
    #[test]
    fn roundtrip_and_unknown_schema_rejected() {
        let p = plan();
        let parsed = parse_plan(&canonical_json(&p).unwrap()).unwrap();
        assert_eq!(p, parsed);
        let bad = serde_json::to_vec(&{
            let mut p = p;
            p.schema_version = 3;
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
