//! Bounded, content-omitting normalizer for Eggsearch EvidenceBundle JSON.
//!
//! This adapter consumes a host-acquired bundle and performs no search, fetch,
//! MCP, network, or credential work. Content fields are intentionally ignored.

use crate::{
    AdapterDescriptor, Capabilities, NormalizedProviderResult, ObservationContext, ProviderClass,
    SourceTrust, SpiError, finalize_observation,
};
use eggplan_core::{
    ArtifactRef, EvidenceKind, EvidenceObservation, EvidenceProviderId, EvidenceStatus,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub const REVIEWED_EGGSEARCH_SHA: &str = "dfa90e050c5434f3346902aeb4074901c58e90d1";
pub const MAX_BUNDLE_BYTES: usize = 2_097_152;
pub const MAX_SOURCES: usize = 200;
pub const MAX_FETCHED_ITEMS: usize = 100;
pub const MAX_GAPS: usize = 128;

const CAPS: Capabilities = Capabilities {
    supports_in_progress: false,
    artifacts: true,
    verification_binding: false,
    research_trust_metadata: true,
};

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum TrustLevel {
    ExternalUntrusted,
    LocalTrusted,
    Unknown,
}
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum FetchTrust {
    ExternalUntrusted,
}
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum GapKind {
    NoPrimarySourceFound,
    ProviderDegraded,
    NativeRepoFilterNotEnforced,
    SecurityApplicabilityUnknown,
    FetchFailed,
    SourceUnfetched,
    AllResultsExternalUntrusted,
    LocalCheckoutDirty,
    LocalRemoteMismatch,
    LocalGeneratedOrVendorOnly,
    LocalUntrackedFile,
    LocalSourceUnfetched,
    NativeAdvisoryUnavailable,
    SymbolHintNoNativeProvider,
    IssueSearchNoNativeProvider,
    ReleaseSearchNoNativeProvider,
    FreshnessNotEnforced,
    PackageResolutionFailed,
    NoFixedVersionFound,
    NoCounterpointFound,
    NoBenchmarksFound,
    MissingTests,
    MissingExamples,
    MissingManifest,
    MissingChangelog,
    MissingSecurityPolicy,
}
#[derive(Debug, Deserialize)]
struct Source {
    source_id: String,
    provider_id: Option<String>,
    trust: TrustLevel,
    stable: Option<bool>,
}
#[derive(Debug, Deserialize)]
struct Fetched {
    fetch_id: String,
    source_id: Option<String>,
    fetched: bool,
    truncated: bool,
    trust: FetchTrust,
    line_start: Option<u32>,
    line_end: Option<u32>,
}
#[derive(Debug, Deserialize)]
struct Gap {
    kind: GapKind,
    source_id: Option<String>,
    provider_id: Option<String>,
    #[serde(default)]
    affected_source_ids: Vec<String>,
}
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum LinkReason {
    UrlMatch,
    LocatorMatch,
    Explicit,
    SourceIdMatch,
}
#[derive(Debug, Deserialize)]
struct Link {
    source_id: String,
    fetch_id: String,
    link_reason: LinkReason,
}
#[derive(Debug, Deserialize)]
struct Limits {
    sources_truncated: bool,
    fetched_items_truncated: bool,
    total_chars_exceeded: bool,
}
#[derive(Debug, Deserialize)]
pub struct Bundle {
    bundle_id: String,
    created_at: String,
    sources: Vec<Source>,
    fetched_items: Vec<Fetched>,
    #[serde(default)]
    source_links: Vec<Link>,
    gaps: Vec<Gap>,
    limits: Limits,
    #[serde(default)]
    warnings: Vec<serde_json::Value>,
    #[serde(default)]
    structured_warnings: Vec<serde_json::Value>,
    #[serde(default)]
    research_claims: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    research_conflicts: Option<Vec<serde_json::Value>>,
}

pub fn descriptor() -> AdapterDescriptor {
    AdapterDescriptor::new(
        EvidenceProviderId::new("epp_eggsearch").expect("static id"),
        ProviderClass::Research,
        [EvidenceKind::Research],
        "1.0",
        CAPS,
    )
    .expect("static descriptor")
}

pub fn parse_bundle(bytes: &[u8]) -> Result<Bundle, SpiError> {
    if bytes.len() > MAX_BUNDLE_BYTES {
        return Err(SpiError::Invalid("Eggsearch bundle exceeds byte limit"));
    }
    let bundle: Bundle = serde_json::from_slice(bytes)
        .map_err(|_| SpiError::Invalid("invalid Eggsearch bundle JSON or contract"))?;
    validate(&bundle)?;
    Ok(bundle)
}

pub fn normalize(
    bundle: &Bundle,
    context: &ObservationContext,
) -> Result<EvidenceObservation, SpiError> {
    validate(bundle)?;
    if context.requested_kind != EvidenceKind::Research {
        return Err(SpiError::UnsupportedKind);
    }
    let mut source_ids = std::collections::BTreeSet::new();
    for s in &bundle.sources {
        source_ids.insert(s.source_id.as_str());
    }
    for f in &bundle.fetched_items {
        if f.source_id
            .as_ref()
            .is_some_and(|id| !source_ids.contains(id.as_str()))
        {
            return Err(SpiError::Invalid(
                "Eggsearch fetch references unknown source",
            ));
        }
    }
    let fetch_ids: std::collections::BTreeSet<_> = bundle
        .fetched_items
        .iter()
        .map(|f| f.fetch_id.as_str())
        .collect();
    if fetch_ids.len() != bundle.fetched_items.len() {
        return Err(SpiError::Invalid("duplicate Eggsearch fetch ID"));
    }
    for link in &bundle.source_links {
        if !source_ids.contains(link.source_id.as_str())
            || !fetch_ids.contains(link.fetch_id.as_str())
        {
            return Err(SpiError::Invalid(
                "Eggsearch link references unknown identity",
            ));
        }
    }
    let mut metadata = BTreeMap::new();
    metadata.insert("bundle_id".into(), bundle.bundle_id.clone());
    metadata.insert("created_at".into(), bundle.created_at.clone());
    metadata.insert("source_count".into(), bundle.sources.len().to_string());
    metadata.insert(
        "fetched_count".into(),
        bundle.fetched_items.len().to_string(),
    );
    metadata.insert("gap_count".into(), bundle.gaps.len().to_string());
    metadata.insert("warning_count".into(), bundle.warnings.len().to_string());
    metadata.insert(
        "structured_warning_count".into(),
        bundle.structured_warnings.len().to_string(),
    );
    metadata.insert(
        "research_claim_count".into(),
        bundle
            .research_claims
            .as_ref()
            .map_or(0, Vec::len)
            .to_string(),
    );
    metadata.insert(
        "research_conflict_count".into(),
        bundle
            .research_conflicts
            .as_ref()
            .map_or(0, Vec::len)
            .to_string(),
    );
    metadata.insert(
        "source_link_count".into(),
        bundle.source_links.len().to_string(),
    );
    let mut source_ids_sorted: Vec<_> = bundle
        .sources
        .iter()
        .map(|s| s.source_id.as_str())
        .collect();
    source_ids_sorted.sort_unstable();
    let mut fetch_ids_sorted: Vec<_> = bundle
        .fetched_items
        .iter()
        .map(|f| f.fetch_id.as_str())
        .collect();
    fetch_ids_sorted.sort_unstable();
    metadata.insert(
        "source_ids_digest".into(),
        hex_digest(source_ids_sorted.join("\0").as_bytes()),
    );
    metadata.insert(
        "fetch_ids_digest".into(),
        hex_digest(fetch_ids_sorted.join("\0").as_bytes()),
    );
    let mut providers: Vec<_> = bundle
        .sources
        .iter()
        .filter_map(|s| s.provider_id.as_deref())
        .collect();
    providers.sort_unstable();
    metadata.insert(
        "provider_ids_digest".into(),
        hex_digest(providers.join("\0").as_bytes()),
    );
    metadata.insert(
        "line_range_count".into(),
        bundle
            .fetched_items
            .iter()
            .filter(|f| f.line_start.is_some() || f.line_end.is_some())
            .count()
            .to_string(),
    );
    for (key, count) in [
        (
            "url_match",
            bundle
                .source_links
                .iter()
                .filter(|l| matches!(l.link_reason, LinkReason::UrlMatch))
                .count(),
        ),
        (
            "locator_match",
            bundle
                .source_links
                .iter()
                .filter(|l| matches!(l.link_reason, LinkReason::LocatorMatch))
                .count(),
        ),
        (
            "explicit",
            bundle
                .source_links
                .iter()
                .filter(|l| matches!(l.link_reason, LinkReason::Explicit))
                .count(),
        ),
        (
            "source_id_match",
            bundle
                .source_links
                .iter()
                .filter(|l| matches!(l.link_reason, LinkReason::SourceIdMatch))
                .count(),
        ),
    ] {
        metadata.insert(format!("link_{key}_count"), count.to_string());
    }
    let local = bundle
        .sources
        .iter()
        .filter(|s| matches!(s.trust, TrustLevel::LocalTrusted))
        .count();
    let external = bundle
        .sources
        .iter()
        .filter(|s| matches!(s.trust, TrustLevel::ExternalUntrusted))
        .count();
    let unknown = bundle
        .sources
        .iter()
        .filter(|s| matches!(s.trust, TrustLevel::Unknown))
        .count();
    metadata.insert("trust_external_count".into(), external.to_string());
    metadata.insert("trust_local_count".into(), local.to_string());
    metadata.insert("trust_unknown_count".into(), unknown.to_string());
    metadata.insert(
        "stable_source_count".into(),
        bundle
            .sources
            .iter()
            .filter(|s| s.stable == Some(true))
            .count()
            .to_string(),
    );
    metadata.insert(
        "fetched_success_count".into(),
        bundle
            .fetched_items
            .iter()
            .filter(|f| f.fetched)
            .count()
            .to_string(),
    );
    metadata.insert(
        "fetched_failure_count".into(),
        bundle
            .fetched_items
            .iter()
            .filter(|f| !f.fetched)
            .count()
            .to_string(),
    );
    metadata.insert(
        "truncated_fetch_count".into(),
        bundle
            .fetched_items
            .iter()
            .filter(|f| f.truncated)
            .count()
            .to_string(),
    );
    metadata.insert(
        "external_fetch_count".into(),
        bundle
            .fetched_items
            .iter()
            .filter(|f| matches!(f.trust, FetchTrust::ExternalUntrusted))
            .count()
            .to_string(),
    );
    let mut counts = BTreeMap::<&'static str, usize>::new();
    for gap in &bundle.gaps {
        *counts.entry(gap_code(gap.kind)).or_default() += 1;
    }
    for (code, count) in counts {
        metadata.insert(format!("gap_{code}"), count.to_string());
    }
    let truncated = bundle.limits.sources_truncated
        || bundle.limits.fetched_items_truncated
        || bundle.limits.total_chars_exceeded
        || bundle.fetched_items.iter().any(|f| f.truncated);
    metadata.insert("bundle_truncated".into(), truncated.to_string());
    // Eggsearch trust labels describe content provenance, never host provider
    // authority. The observation status describes bundle production only.
    let status = if bundle.sources.is_empty() {
        EvidenceStatus::Unavailable
    } else if truncated
        || !bundle.gaps.is_empty()
        || bundle.fetched_items.iter().any(|f| !f.fetched)
    {
        EvidenceStatus::Inconclusive
    } else {
        EvidenceStatus::Passed
    };
    let handle = ArtifactRef {
        reference: format!("eggsearch:bundle:{}", bundle.bundle_id),
        digest: None,
        media_type: Some("application/vnd.eggsearch.evidence-bundle+json".into()),
    };
    finalize_observation(
        &descriptor(),
        context,
        &NormalizedProviderResult {
            status,
            source_trust: Some(SourceTrust::ExternalUntrusted),
            result_metadata: metadata,
            artifacts: vec![handle],
        },
    )
}

fn validate(bundle: &Bundle) -> Result<(), SpiError> {
    if bundle.bundle_id.len() > 128
        || bundle.bundle_id.is_empty()
        || !bundle
            .bundle_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b))
        || !valid_timestamp(&bundle.created_at)
        || bundle.sources.len() > MAX_SOURCES
        || bundle.fetched_items.len() > MAX_FETCHED_ITEMS
        || bundle.gaps.len() > MAX_GAPS
        || bundle.source_links.len() > MAX_FETCHED_ITEMS
        || bundle.warnings.len() > 256
        || bundle.structured_warnings.len() > 256
        || bundle
            .research_claims
            .as_ref()
            .is_some_and(|v| v.len() > 1024)
        || bundle
            .research_conflicts
            .as_ref()
            .is_some_and(|v| v.len() > 1024)
    {
        return Err(SpiError::Invalid(
            "invalid or out-of-bounds Eggsearch bundle",
        ));
    }
    let mut ids = std::collections::BTreeSet::new();
    if bundle.sources.iter().any(|s| {
        s.source_id.is_empty()
            || s.source_id.len() > 128
            || !s
                .source_id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_.:".contains(&b))
            || !ids.insert(s.source_id.as_str())
            || s.provider_id.as_ref().is_some_and(|p| p.len() > 128)
    }) {
        return Err(SpiError::Invalid("invalid or duplicate Eggsearch source"));
    }
    if bundle.fetched_items.iter().any(|f| {
        f.fetch_id.is_empty()
            || f.fetch_id.len() > 128
            || !f
                .fetch_id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_.:".contains(&b))
            || f.line_start
                .zip(f.line_end)
                .is_some_and(|(a, b)| a == 0 || b < a)
    }) {
        return Err(SpiError::Invalid("invalid Eggsearch fetched item"));
    }
    if bundle.gaps.iter().any(|g| {
        g.source_id.as_ref().is_some_and(|s| s.len() > 128)
            || g.provider_id.as_ref().is_some_and(|s| s.len() > 128)
            || g.affected_source_ids.len() > MAX_SOURCES
            || g.affected_source_ids.iter().any(|s| s.len() > 128)
    }) {
        return Err(SpiError::Invalid("invalid Eggsearch gap"));
    }
    let source_ids: std::collections::BTreeSet<_> = bundle
        .sources
        .iter()
        .map(|s| s.source_id.as_str())
        .collect();
    if bundle.gaps.iter().any(|g| {
        g.source_id
            .as_ref()
            .is_some_and(|id| !source_ids.contains(id.as_str()))
            || g.affected_source_ids
                .iter()
                .any(|id| !source_ids.contains(id.as_str()))
    }) {
        return Err(SpiError::Invalid("Eggsearch gap references unknown source"));
    }
    Ok(())
}
fn valid_timestamp(value: &str) -> bool {
    let b = value.as_bytes();
    b.len() >= 20
        && b.len() <= 35
        && b[4] == b'-'
        && b[7] == b'-'
        && b[10] == b'T'
        && b[13] == b':'
        && b[16] == b':'
        && b.iter()
            .all(|v| v.is_ascii_digit() || b"-:TZ+.".contains(v))
        && b[..4].iter().all(u8::is_ascii_digit)
        && b[5..7].iter().all(u8::is_ascii_digit)
        && b[8..10].iter().all(u8::is_ascii_digit)
        && b[11..13].iter().all(u8::is_ascii_digit)
        && b[14..16].iter().all(u8::is_ascii_digit)
        && b[17..19].iter().all(u8::is_ascii_digit)
}
fn hex_digest(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let encoded: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
    format!("sha256:{encoded}")
}
fn gap_code(kind: GapKind) -> &'static str {
    match kind {
        GapKind::NoPrimarySourceFound => "no_primary_source",
        GapKind::ProviderDegraded => "provider_degraded",
        GapKind::NativeRepoFilterNotEnforced => "repo_filter_not_enforced",
        GapKind::SecurityApplicabilityUnknown => "security_applicability_unknown",
        GapKind::FetchFailed => "fetch_failed",
        GapKind::SourceUnfetched => "source_unfetched",
        GapKind::AllResultsExternalUntrusted => "all_external",
        GapKind::LocalCheckoutDirty => "local_dirty",
        GapKind::LocalRemoteMismatch => "remote_mismatch",
        GapKind::LocalGeneratedOrVendorOnly => "generated_vendor_only",
        GapKind::LocalUntrackedFile => "local_untracked",
        GapKind::LocalSourceUnfetched => "local_unfetched",
        GapKind::NativeAdvisoryUnavailable => "advisory_unavailable",
        GapKind::SymbolHintNoNativeProvider => "symbol_provider_missing",
        GapKind::IssueSearchNoNativeProvider => "issue_provider_missing",
        GapKind::ReleaseSearchNoNativeProvider => "release_provider_missing",
        GapKind::FreshnessNotEnforced => "freshness_not_enforced",
        GapKind::PackageResolutionFailed => "package_resolution_failed",
        GapKind::NoFixedVersionFound => "no_fixed_version",
        GapKind::NoCounterpointFound => "no_counterpoint",
        GapKind::NoBenchmarksFound => "no_benchmarks",
        GapKind::MissingTests => "missing_tests",
        GapKind::MissingExamples => "missing_examples",
        GapKind::MissingManifest => "missing_manifest",
        GapKind::MissingChangelog => "missing_changelog",
        GapKind::MissingSecurityPolicy => "missing_security_policy",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eggplan_core::{EvidenceObservationId, SubjectRevision, SubjectState};
    fn context() -> ObservationContext {
        ObservationContext {
            observation_id: EvidenceObservationId::new("epe_eggsearch_test").unwrap(),
            subject: SubjectRevision {
                subject_kind: "git".into(),
                repository_id: "epr_test".into(),
                revision: "abc".into(),
                state: SubjectState::Clean,
                dirty_digest: None,
            },
            requested_kind: EvidenceKind::Research,
            observed_at_unix_ms: 1_700_000_000_000,
            invocation_ref: None,
            verification_digest: None,
            metadata: BTreeMap::new(),
        }
    }
    fn bundle(
        trust: &str,
        truncated: bool,
        gaps: Vec<serde_json::Value>,
        sources: usize,
    ) -> Vec<u8> {
        let source = serde_json::json!({"source_id":"src_a","url":"https://secret.invalid/path","title":"secret title","provider_id":"web","trust":trust,"stable":true,"snippet":"do not persist this"});
        let fetched = serde_json::json!({"fetch_id":"fetch_a","source_id":"src_a","url":"https://secret.invalid/path","fetched":true,"truncated":truncated,"trust":"external_untrusted","text":"private fetched body","line_start":1,"line_end":2});
        serde_json::to_vec(&serde_json::json!({"bundle_id":"bundle_abcd","goal":"private query","created_at":"2026-09-24T00:00:00Z","sources":vec![source;sources],"fetched_items":if sources==0 {vec![]} else {vec![fetched]},"gaps":gaps,"limits":{"max_sources":50,"max_fetched_items":20,"max_total_chars":10000,"sources_truncated":false,"fetched_items_truncated":false,"total_chars_exceeded":truncated},"trust_summary":{"external_untrusted_count":1,"local_trusted_count":0}})).unwrap()
    }
    #[test]
    fn provenance_is_kept_without_copying_content_or_promoting_trust() {
        let parsed = parse_bundle(&bundle("local_trusted", false, vec![], 1)).unwrap();
        let first = normalize(&parsed, &context()).unwrap();
        let second = normalize(&parsed, &context()).unwrap();
        assert_eq!(first.content_digest(), second.content_digest());
        assert_eq!(first.status(), EvidenceStatus::Passed);
        assert_eq!(first.provider_id().as_str(), "epp_eggsearch");
        assert_eq!(
            first
                .result_metadata()
                .get("trust_local_count")
                .map(String::as_str),
            Some("1")
        );
        assert_eq!(
            first
                .result_metadata()
                .get("source_trust")
                .map(String::as_str),
            Some("external_untrusted")
        );
        let encoded = serde_json::to_string(&first).unwrap();
        for value in [
            "secret.invalid",
            "secret title",
            "private fetched body",
            "private query",
        ] {
            assert!(!encoded.contains(value));
        }
        assert_eq!(
            first.artifacts()[0].reference,
            "eggsearch:bundle:bundle_abcd"
        );
    }
    #[test]
    fn empty_gapped_truncated_and_unknown_contracts_are_truthful() {
        let empty = normalize(
            &parse_bundle(&bundle("external_untrusted", false, vec![], 0)).unwrap(),
            &context(),
        )
        .unwrap();
        assert_eq!(empty.status(), EvidenceStatus::Unavailable);
        let gap =
            serde_json::json!({"kind":"all_results_external_untrusted","message":"not persisted"});
        let gapped = normalize(
            &parse_bundle(&bundle("external_untrusted", false, vec![gap], 1)).unwrap(),
            &context(),
        )
        .unwrap();
        assert_eq!(gapped.status(), EvidenceStatus::Inconclusive);
        let truncated = normalize(
            &parse_bundle(&bundle("external_untrusted", true, vec![], 1)).unwrap(),
            &context(),
        )
        .unwrap();
        assert_eq!(truncated.status(), EvidenceStatus::Inconclusive);
        assert!(
            parse_bundle(&bundle(
                "external_untrusted",
                false,
                vec![serde_json::json!({"kind":"future_gap"})],
                1
            ))
            .is_err()
        );
        assert!(parse_bundle(&vec![b' '; MAX_BUNDLE_BYTES + 1]).is_err());
    }

    #[test]
    fn reviewed_sibling_bundle_fixtures_parse_and_normalize() {
        let cases: Vec<serde_json::Value> =
            serde_json::from_slice(include_bytes!("../tests/fixtures/eggsearch-bundles.json"))
                .unwrap();
        assert_eq!(cases.len(), 6);
        for mut case in cases {
            let name = case["case"].as_str().unwrap().to_owned();
            case.as_object_mut().unwrap().remove("case");
            let parsed = parse_bundle(&serde_json::to_vec(&case).unwrap()).unwrap();
            let observation = normalize(&parsed, &context()).unwrap();
            assert_eq!(observation.provider_id().as_str(), "epp_eggsearch");
            if name == "external_web" {
                assert_eq!(
                    observation
                        .result_metadata()
                        .get("source_link_count")
                        .map(String::as_str),
                    Some("1")
                );
            }
        }
    }
}
