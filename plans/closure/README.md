# Eggplan Closure and Verification Records

Closure records determine whether an implementation milestone actually closed.

## Layout

    closure/<subsystem>/NNN-status.md

Use the same milestone number as the source implementation plan.

## Required sections

A closure record contains:

- status;
- source implementation plan and roadmap;
- repository baseline reviewed;
- implementation commits or PRs;
- executive finding;
- requirement-to-evidence matrix;
- production implementation evidence;
- exact verification commands actually run and their results;
- invariant review;
- failure/recovery/contention review;
- migration/compatibility review;
- security/trust/path review;
- documentation/operations;
- unresolved findings with severity;
- roadmap disposition;
- registry updates.

A milestone MUST NOT be marked closed when required verification was not run
and no justified substitute evidence exists, or when only compilation/formatting
was checked for a stronger correctness claim.

Historical closure remains evidence of what was accepted at the time. Later
findings use corrective plans and closure records rather than silently
rewriting history.
