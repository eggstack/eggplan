use crate::{
    AssessmentStatus, ClosureId, EvidenceObservation, EvidenceObservationId, EvidenceProviderId,
    EvidenceSupersessionId, Plan, PlanAssessment, PlanId, SubjectRevision, digest_json,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderPolicyEntry {
    pub provider_id: EvidenceProviderId,
    pub class: String,
    pub allowed_kinds: BTreeSet<crate::EvidenceKind>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceSupersessionRecord {
    pub schema_version: u32,
    pub id: EvidenceSupersessionId,
    pub plan_id: PlanId,
    pub superseded: EvidenceObservationId,
    pub replacement: EvidenceObservationId,
    pub reason: String,
    pub recorded_at_unix_ms: u64,
    pub content_digest: String,
}

impl EvidenceSupersessionRecord {
    pub fn new(
        id: EvidenceSupersessionId,
        plan_id: PlanId,
        superseded: EvidenceObservationId,
        replacement: EvidenceObservationId,
        reason: String,
        recorded_at_unix_ms: u64,
    ) -> Result<Self, String> {
        if superseded == replacement || reason.is_empty() || reason.chars().count() > 2000 {
            return Err("invalid supersession link or reason".into());
        }
        let mut result = Self {
            schema_version: 1,
            id,
            plan_id,
            superseded,
            replacement,
            reason,
            recorded_at_unix_ms,
            content_digest: String::new(),
        };
        result.content_digest = digest_json(&(
            &result.schema_version,
            &result.id,
            &result.plan_id,
            &result.superseded,
            &result.replacement,
            &result.reason,
            result.recorded_at_unix_ms,
        ))
        .map_err(|e| e.to_string())?;
        Ok(result)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1
            || self.superseded == self.replacement
            || self.reason.is_empty()
            || self.reason.chars().count() > 2000
        {
            return Err("invalid supersession record".into());
        }
        let expected = digest_json(&(
            &self.schema_version,
            &self.id,
            &self.plan_id,
            &self.superseded,
            &self.replacement,
            &self.reason,
            self.recorded_at_unix_ms,
        ))
        .map_err(|e| e.to_string())?;
        if expected != self.content_digest {
            return Err("supersession digest mismatch".into());
        }
        Ok(())
    }
}

pub fn effective_observations<'a>(
    observations: &'a [EvidenceObservation],
    links: &[EvidenceSupersessionRecord],
) -> Result<Vec<&'a EvidenceObservation>, String> {
    let by_id: std::collections::BTreeMap<_, _> =
        observations.iter().map(|o| (o.id(), o)).collect();
    if by_id.len() != observations.len() {
        return Err("duplicate observation ID".into());
    }
    let mut successors = std::collections::BTreeMap::new();
    let mut link_ids = BTreeSet::new();
    for link in links {
        link.validate()?;
        if !link_ids.insert(link.id.clone()) {
            return Err("duplicate supersession ID".into());
        }
        if !by_id.contains_key(&link.superseded) || !by_id.contains_key(&link.replacement) {
            return Err("dangling supersession link".into());
        }
        if successors
            .insert(link.superseded.clone(), link.replacement.clone())
            .is_some()
        {
            return Err("conflicting supersession successors".into());
        }
    }
    for start in successors.keys() {
        let mut seen = BTreeSet::new();
        let mut at = start;
        while let Some(next) = successors.get(at) {
            if !seen.insert(at.clone()) {
                return Err("supersession cycle".into());
            }
            at = next;
        }
    }
    let superseded: BTreeSet<_> = successors.keys().collect();
    let mut out: Vec<_> = observations
        .iter()
        .filter(|o| !superseded.contains(o.id()))
        .collect();
    out.sort_by(|a, b| a.id().cmp(b.id()));
    Ok(out)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClosureCandidate {
    pub plan_id: PlanId,
    pub source_revision: u64,
    pub source_plan_digest: String,
    pub subject: SubjectRevision,
    pub assessment: PlanAssessment,
    pub satisfying_observations: Vec<(EvidenceObservationId, String)>,
    pub supersession_digests: Vec<(EvidenceSupersessionId, String)>,
    pub provider_policy: Vec<ProviderPolicyEntry>,
    pub provider_policy_digest: String,
    pub created_at_unix_ms: u64,
}

impl ClosureCandidate {
    pub fn build(
        plan: &Plan,
        subject: SubjectRevision,
        assessment: PlanAssessment,
        observations: &[EvidenceObservation],
        supersessions: &[EvidenceSupersessionRecord],
        policy: Vec<ProviderPolicyEntry>,
        created_at_unix_ms: u64,
    ) -> Result<Self, String> {
        if assessment.status != AssessmentStatus::Complete
            || assessment.plan_id != plan.id
            || assessment.plan_revision != plan.revision
            || assessment.subject != subject
        {
            return Err(
                "closure requires complete assessment for exact plan revision and subject".into(),
            );
        }
        let ids: BTreeSet<_> = assessment
            .items
            .iter()
            .flat_map(|i| &i.criteria)
            .flat_map(|c| &c.requirements)
            .flat_map(|r| &r.satisfying_observation_ids)
            .cloned()
            .collect();
        let mut satisfying_observations = Vec::new();
        for id in ids {
            let o = observations
                .iter()
                .find(|o| o.id() == &id)
                .ok_or("missing satisfying observation")?;
            satisfying_observations.push((id, o.content_digest().to_string()));
        }
        let mut supersession_digests: Vec<_> = supersessions
            .iter()
            .map(|s| (s.id.clone(), s.content_digest.clone()))
            .collect();
        supersession_digests.sort();
        let mut provider_policy = policy;
        provider_policy.sort_by(|a, b| a.provider_id.cmp(&b.provider_id));
        let provider_policy_digest = digest_json(&provider_policy).map_err(|e| e.to_string())?;
        Ok(Self {
            plan_id: plan.id.clone(),
            source_revision: plan.revision,
            source_plan_digest: digest_json(plan).map_err(|e| e.to_string())?,
            subject,
            assessment,
            satisfying_observations,
            supersession_digests,
            provider_policy,
            provider_policy_digest,
            created_at_unix_ms,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClosureRecord {
    pub schema_version: u32,
    pub id: ClosureId,
    pub candidate: ClosureCandidate,
    pub final_plan_revision: u64,
    pub final_plan_digest: String,
    pub finalized_at_unix_ms: u64,
    pub content_digest: String,
}

impl ClosureRecord {
    pub fn finalize(
        id: ClosureId,
        candidate: ClosureCandidate,
        final_plan: &Plan,
        finalized_at_unix_ms: u64,
    ) -> Result<Self, String> {
        if final_plan.id != candidate.plan_id
            || final_plan.revision != candidate.source_revision + 1
            || final_plan.status != crate::PlanStatus::Closed
        {
            return Err("invalid closure target plan".into());
        }
        let mut record = Self {
            schema_version: 1,
            id,
            candidate,
            final_plan_revision: final_plan.revision,
            final_plan_digest: digest_json(final_plan).map_err(|e| e.to_string())?,
            finalized_at_unix_ms,
            content_digest: String::new(),
        };
        record.content_digest = digest_json(&(
            &record.schema_version,
            &record.id,
            &record.candidate,
            record.final_plan_revision,
            &record.final_plan_digest,
            record.finalized_at_unix_ms,
        ))
        .map_err(|e| e.to_string())?;
        Ok(record)
    }

    pub fn validate(&self, final_plan: &Plan) -> Result<(), String> {
        if self.schema_version != 1
            || self.candidate.plan_id != final_plan.id
            || self.final_plan_revision != final_plan.revision
            || self.final_plan_digest != digest_json(final_plan).map_err(|e| e.to_string())?
            || final_plan.status != crate::PlanStatus::Closed
            || self.final_plan_revision != self.candidate.source_revision + 1
        {
            return Err("closure record does not match final plan".into());
        }
        if digest_json(&self.candidate.provider_policy).map_err(|e| e.to_string())?
            != self.candidate.provider_policy_digest
        {
            return Err("closure provider policy digest mismatch".into());
        }
        let expected = digest_json(&(
            &self.schema_version,
            &self.id,
            &self.candidate,
            self.final_plan_revision,
            &self.final_plan_digest,
            self.finalized_at_unix_ms,
        ))
        .map_err(|e| e.to_string())?;
        if expected != self.content_digest {
            return Err("closure record digest mismatch".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EvidenceObservationInput, EvidenceProviderId, EvidenceStatus, SubjectState};
    use std::collections::BTreeMap;

    fn observation(id: &str) -> EvidenceObservation {
        EvidenceObservation::finalize(EvidenceObservationInput {
            id: EvidenceObservationId::new(format!("epe_{id}")).unwrap(),
            provider_id: EvidenceProviderId::new("epp_test").unwrap(),
            kind: crate::EvidenceKind::Research,
            status: EvidenceStatus::Failed,
            subject: SubjectRevision {
                subject_kind: "git".into(),
                repository_id: "epr_test".into(),
                revision: "abc".into(),
                state: SubjectState::Clean,
                dirty_digest: None,
            },
            observed_at_unix_ms: 1,
            invocation_ref: None,
            verification_digest: None,
            result_metadata: BTreeMap::new(),
            artifacts: vec![],
        })
        .unwrap()
    }

    #[test]
    fn supersession_is_append_only_effective_lineage_and_detects_cycles() {
        let old = observation("old");
        let replacement = observation("replacement");
        let first = EvidenceSupersessionRecord::new(
            EvidenceSupersessionId::new("eps_one").unwrap(),
            PlanId::new("ep_plan").unwrap(),
            old.id().clone(),
            replacement.id().clone(),
            "correction".into(),
            2,
        )
        .unwrap();
        let observations = [old.clone(), replacement.clone()];
        let active = effective_observations(&observations, std::slice::from_ref(&first)).unwrap();
        assert_eq!(active, vec![&replacement]);
        let second = EvidenceSupersessionRecord::new(
            EvidenceSupersessionId::new("eps_two").unwrap(),
            PlanId::new("ep_plan").unwrap(),
            replacement.id().clone(),
            old.id().clone(),
            "bad correction".into(),
            3,
        )
        .unwrap();
        assert!(effective_observations(&observations, &[first, second]).is_err());
        assert!(
            effective_observations(&observations[..1], &[])
                .unwrap()
                .contains(&&old)
        );
    }
}
