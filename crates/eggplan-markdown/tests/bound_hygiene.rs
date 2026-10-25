//! Regression coverage for the M003b C003 Markdown interchange panic-path and
//! bound hygiene pass.
//!
//! - The import-format dispatch must be total: every `ImportFormat` value must
//!   reach a typed result through the public `import` entry point, and the
//!   `Auto` request must resolve exactly like the detected format.
//! - The imported-source text bound must be enforced at every site that
//!   applies it, at exactly the bound and one character past it.
//! - Imported identifiers, and the bytes of imported plans and rendered
//!   documents, must match the recorded pre-pass golden fixtures.
//!
//! Golden fixture convention: a `.sha256` sidecar holds `sha256:<hex>` over
//! the golden file's exact bytes. The byte-level anchor files
//! (`ordinary.import.json`, `ordinary.render.md`) store the pinned bytes
//! verbatim, so their sidecars equal the digests recorded in
//! `import-render-digests.json`.

use eggplan_core::{Plan, PlanId, PlanItem, PlanItemId, PlanItemStatus, canonical_json};
use eggplan_markdown::{ImportFormat, detect_format, import, render};
use sha2::{Digest, Sha256};

const ORDINARY: &str = include_str!("fixtures/ordinary-implementation.md");
const CORRECTIVE: &str = include_str!("fixtures/corrective-plan.md");
const UNMAPPED: &str = include_str!("fixtures/unmapped-sections.md");

const IDENTIFIERS: &str = include_str!("fixtures/golden/imported-identifiers.json");
const IDENTIFIERS_SHA256: &str = include_str!("fixtures/golden/imported-identifiers.json.sha256");
const DIGESTS: &str = include_str!("fixtures/golden/import-render-digests.json");
const DIGESTS_SHA256: &str = include_str!("fixtures/golden/import-render-digests.json.sha256");
const GOLDEN_IMPORT: &str = include_str!("fixtures/golden/ordinary.import.json");
const GOLDEN_IMPORT_SHA256: &str = include_str!("fixtures/golden/ordinary.import.json.sha256");
const GOLDEN_RENDER: &str = include_str!("fixtures/golden/ordinary.render.md");
const GOLDEN_RENDER_SHA256: &str = include_str!("fixtures/golden/ordinary.render.md.sha256");

/// The imported-source text bound, pinned as a literal so this test fails if
/// the production constant ever changes value.
const SOURCE_TEXT_LIMIT: usize = 512;

const CODEGG_DOC: &str = "# Example\n## 1. Objective\n\nDo bounded work.\n";

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn sample_plan() -> Plan {
    Plan::new(
        PlanId::new("ep_md_bound_hygiene").unwrap(),
        "Bound hygiene objective",
        vec![PlanItem {
            id: PlanItemId::new("epi_md_bound_hygiene").unwrap(),
            position: 0,
            parent: None,
            dependencies: vec![],
            status: PlanItemStatus::Pending,
            description: "Bound hygiene item".into(),
            criteria: vec![],
            blocker: None,
            next_action: None,
        }],
    )
    .unwrap()
}

fn native_document() -> String {
    render(&sample_plan(), false).unwrap()
}

/// The recorded interchange cases: the corpus fixtures imported through the
/// CodeGG subset, the same plan rendered with the closure-display flag, and a
/// native round-trip of the rendered document.
fn corpus_cases() -> Vec<(
    &'static str,
    String,
    ImportFormat,
    Option<&'static str>,
    bool,
)> {
    let ordinary = import(
        ORDINARY.as_bytes(),
        ImportFormat::Codegg,
        Some("ordinary-implementation.md"),
    )
    .unwrap();
    let native = render(&ordinary.plan, false).unwrap();
    vec![
        (
            "ordinary",
            ORDINARY.to_string(),
            ImportFormat::Codegg,
            Some("ordinary-implementation.md"),
            false,
        ),
        (
            "ordinary-closure",
            ORDINARY.to_string(),
            ImportFormat::Codegg,
            Some("ordinary-implementation.md"),
            true,
        ),
        (
            "ordinary-native",
            native,
            ImportFormat::Eggplan,
            None,
            false,
        ),
        (
            "corrective",
            CORRECTIVE.to_string(),
            ImportFormat::Codegg,
            Some("corrective-plan.md"),
            false,
        ),
        (
            "unmapped",
            UNMAPPED.to_string(),
            ImportFormat::Codegg,
            Some("unmapped-sections.md"),
            false,
        ),
    ]
}

#[test]
fn every_import_format_dispatches_without_a_panic() {
    let native = native_document();
    for (label, document) in [("native", native.as_str()), ("codegg", CODEGG_DOC)] {
        for format in [
            ImportFormat::Auto,
            ImportFormat::Eggplan,
            ImportFormat::Codegg,
        ] {
            let imported = import(document.as_bytes(), format, Some("x.md"));
            // A panic would fail the test outright; every format must instead
            // produce a typed, deterministic result.
            let again = import(document.as_bytes(), format, Some("x.md"));
            assert_eq!(
                imported.as_ref().map(|i| &i.plan),
                again.as_ref().map(|i| &i.plan),
                "{label} {format:?} is not deterministic"
            );
            assert_eq!(
                imported.as_ref().map(|i| &i.report),
                again.as_ref().map(|i| &i.report),
                "{label} {format:?} is not deterministic"
            );
            if format == ImportFormat::Auto {
                // The `Auto` request has no dispatch arm of its own: it must
                // resolve exactly like the detected concrete format.
                let detected = detect_format(document.as_bytes()).unwrap();
                let explicit = import(document.as_bytes(), detected, Some("x.md"));
                assert_eq!(
                    imported.as_ref().map(|i| &i.plan),
                    explicit.as_ref().map(|i| &i.plan),
                    "{label}: Auto did not resolve like {detected:?}"
                );
                assert_eq!(
                    imported.as_ref().map(|i| &i.report),
                    explicit.as_ref().map(|i| &i.report),
                    "{label}: Auto did not resolve like {detected:?}"
                );
            }
        }
    }
    // Explicit formats are honoured, not auto-detected.
    assert!(
        import(CODEGG_DOC.as_bytes(), ImportFormat::Codegg, Some("x.md")).is_ok(),
        "the CodeGG subset must import as a CodeGG document"
    );
    assert!(
        import(CODEGG_DOC.as_bytes(), ImportFormat::Eggplan, Some("x.md")).is_err(),
        "a document without a native payload must not import as native"
    );
    assert!(
        import(native.as_bytes(), ImportFormat::Eggplan, Some("x.md")).is_ok(),
        "a native document must import as native"
    );
}

#[test]
fn detect_format_never_reports_auto() {
    let native = native_document();
    for document in [native.as_str(), CODEGG_DOC, "", "# Title\n"] {
        let detected = detect_format(document.as_bytes()).unwrap();
        assert_ne!(
            detected,
            ImportFormat::Auto,
            "detect_format reported Auto for {document:?}"
        );
    }
    // Rejected inputs keep their typed error.
    assert_eq!(
        detect_format(b"# x\0").map_err(|e| e.to_string()),
        Err("Markdown input contains NUL".to_string())
    );
}

#[test]
fn source_text_bound_is_enforced_at_every_site_that_applies_it() {
    let at_limit = "n".repeat(SOURCE_TEXT_LIMIT);
    let over_limit = "n".repeat(SOURCE_TEXT_LIMIT + 1);

    // Site 1: the caller-provided source name is rejected past the bound.
    let imported = import(CODEGG_DOC.as_bytes(), ImportFormat::Codegg, Some(&at_limit)).unwrap();
    assert_eq!(
        imported.report.source_name.as_deref(),
        Some(at_limit.as_str())
    );
    assert!(!imported.report.truncated);
    let error = import(
        CODEGG_DOC.as_bytes(),
        ImportFormat::Codegg,
        Some(&over_limit),
    )
    .expect_err("an over-limit source name must be rejected");
    assert_eq!(error.to_string(), "source name exceeds 512 characters");

    // Site 2: the source `Status:` value copied into the loss report.
    let status = "s".repeat(SOURCE_TEXT_LIMIT + 1);
    let document = format!("# Example\nStatus: {status}\n\n## 1. Objective\n\nDo it.\n");
    let imported = import(document.as_bytes(), ImportFormat::Codegg, Some("x.md")).unwrap();
    assert_eq!(
        imported.report.original_source_status.as_deref(),
        Some(&status[..SOURCE_TEXT_LIMIT])
    );
    assert!(imported.report.truncated);
    assert!(
        imported
            .report
            .warning_codes
            .contains(&"text_truncated".to_string())
    );
    // The canonical Plan keeps the full value: the loss report bounds the same
    // metadata more tightly than `plan.provenance` does.
    assert_eq!(
        imported.plan.provenance["markdown_source_status"],
        status.as_str()
    );

    // Site 3: the source `Repository baseline:` value copied into the loss
    // report.
    let baseline = "r".repeat(SOURCE_TEXT_LIMIT + 1);
    let document =
        format!("# Example\nRepository baseline: {baseline}\n\n## 1. Objective\n\nDo it.\n");
    let imported = import(document.as_bytes(), ImportFormat::Codegg, Some("x.md")).unwrap();
    assert_eq!(
        imported.report.original_source_revision.as_deref(),
        Some(&baseline[..SOURCE_TEXT_LIMIT])
    );
    assert!(imported.report.truncated);

    // Site 4: the title of an unmapped section named in the loss report.
    let title = "t".repeat(SOURCE_TEXT_LIMIT + 1);
    let document = format!("# Example\n## 1. Objective\n\nDo it.\n## {title}\n\nBody.\n");
    let imported = import(document.as_bytes(), ImportFormat::Codegg, Some("x.md")).unwrap();
    assert_eq!(
        imported.report.dropped_fields,
        vec![
            "source_status_and_lifecycle_authority".to_string(),
            title[..SOURCE_TEXT_LIMIT].to_string(),
        ]
    );
    assert!(imported.report.truncated);

    // Exactly at the bound nothing is truncated.
    let status = "s".repeat(SOURCE_TEXT_LIMIT);
    let document = format!("# Example\nStatus: {status}\n\n## 1. Objective\n\nDo it.\n");
    let imported = import(document.as_bytes(), ImportFormat::Codegg, Some("x.md")).unwrap();
    assert_eq!(
        imported.report.original_source_status.as_deref(),
        Some(status.as_str())
    );
    assert!(!imported.report.truncated);
}

#[test]
fn imported_identifiers_match_the_recorded_fixture() {
    let recorded: serde_json::Value = serde_json::from_str(IDENTIFIERS).unwrap();
    for (case, document, format, source_name, _) in corpus_cases() {
        let imported = import(document.as_bytes(), format, source_name).unwrap();
        let entry = &recorded[case];
        assert_eq!(
            entry["plan_id"],
            imported.plan.id.as_str(),
            "{case}: the imported plan ID changed"
        );
        assert_eq!(
            entry["generated_ids"],
            serde_json::json!(imported.report.generated_ids),
            "{case}: the generated identifiers changed"
        );
    }
}

#[test]
fn imported_plans_and_rendered_documents_match_the_recorded_bytes() {
    let digests: serde_json::Value = serde_json::from_str(DIGESTS).unwrap();
    for (case, document, format, source_name, closure) in corpus_cases() {
        let imported = import(document.as_bytes(), format, source_name).unwrap();
        let import_bytes =
            canonical_json(&(imported.plan.clone(), imported.report.clone())).unwrap();
        let rendered = render(&imported.plan, closure).unwrap();
        assert_eq!(
            format!("sha256:{}", sha256_hex(&import_bytes)),
            digests[case]["import_sha256"].as_str().unwrap(),
            "{case}: the imported plan bytes changed"
        );
        assert_eq!(
            format!("sha256:{}", sha256_hex(rendered.as_bytes())),
            digests[case]["render_sha256"].as_str().unwrap(),
            "{case}: the rendered document bytes changed"
        );
    }
}

#[test]
fn recorded_golden_files_match_their_digest_sidecars() {
    for (label, body, sidecar) in [
        ("imported-identifiers.json", IDENTIFIERS, IDENTIFIERS_SHA256),
        ("import-render-digests.json", DIGESTS, DIGESTS_SHA256),
        ("ordinary.import.json", GOLDEN_IMPORT, GOLDEN_IMPORT_SHA256),
        ("ordinary.render.md", GOLDEN_RENDER, GOLDEN_RENDER_SHA256),
    ] {
        assert_eq!(
            format!("sha256:{}", sha256_hex(body.as_bytes())),
            sidecar.trim(),
            "{label} no longer matches its digest sidecar"
        );
    }
}

#[test]
fn the_byte_level_anchor_still_matches_the_imported_plan_and_rendered_document() {
    let imported = import(
        ORDINARY.as_bytes(),
        ImportFormat::Codegg,
        Some("ordinary-implementation.md"),
    )
    .unwrap();
    let import_bytes = canonical_json(&(imported.plan.clone(), imported.report.clone())).unwrap();
    assert_eq!(
        import_bytes,
        GOLDEN_IMPORT.as_bytes(),
        "the imported plan bytes changed"
    );
    assert_eq!(
        render(&imported.plan, false).unwrap(),
        GOLDEN_RENDER,
        "the rendered document bytes changed"
    );
}
