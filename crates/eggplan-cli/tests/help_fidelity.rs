//! The help page must agree with the option validation it documents.
//!
//! The command metadata table is one inventory; help, option validation, and
//! the shell completions are three renderings of it. These tests prove the
//! help rendering and the validation cannot disagree: for every command and
//! every subcommand, the options a help page lists are exactly the options
//! the validator accepts — asserted mechanically, by probing the validator
//! itself, rather than case by case.

use serde_json::Value;
use std::{collections::BTreeSet, path::Path, process::Command};
use tempfile::tempdir;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_eggplan")
}

fn invoke(args: &[&str]) -> std::process::Output {
    Command::new(bin()).args(args).output().unwrap()
}

/// The human output of a successful invocation.
fn human(args: &[&str]) -> String {
    let output = invoke(args);
    assert!(
        output.status.success(),
        "`{}` must succeed: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .unwrap()
        .replace("\r\n", "\n")
}

/// The machine code of a failed invocation.
fn failure_code(args: &[&str]) -> String {
    let mut with_json = args.to_vec();
    with_json.push("--json");
    let output = invoke(&with_json);
    assert_eq!(
        output.status.code(),
        Some(2),
        "`{}` must fail: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stdout)
    );
    let envelope: Value = serde_json::from_slice(&output.stdout).unwrap();
    envelope["error"]["code"]
        .as_str()
        .unwrap_or_default()
        .to_string()
}

/// The message of a failed invocation.
fn failure_message(args: &[&str]) -> String {
    let mut with_json = args.to_vec();
    with_json.push("--json");
    let output = invoke(&with_json);
    let envelope: Value = serde_json::from_slice(&output.stdout).unwrap();
    envelope["error"]["message"]
        .as_str()
        .unwrap_or_default()
        .to_string()
}

/// The help page for one command and subcommand, through `help COMMAND
/// SUBCOMMAND`.
fn help_page(command: &str, subcommand: Option<&str>) -> String {
    let mut args = vec!["help", command];
    if let Some(subcommand) = subcommand {
        args.push(subcommand);
    }
    human(&args)
}

/// The same page through the `COMMAND SUBCOMMAND --help` form.
fn help_page_via_flag(command: &str, subcommand: Option<&str>) -> String {
    let mut args = vec![command];
    if let Some(subcommand) = subcommand {
        args.push(subcommand);
    }
    args.push("--help");
    human(&args)
}

/// Every flag name the command table declares.
fn declared_flags() -> BTreeSet<&'static str> {
    eggplan_cli::command_metadata()
        .iter()
        .flat_map(|spec| spec.flags.iter().copied())
        .collect()
}

/// The option names a help page lists.
///
/// The renderer writes one line per option and one per flag. A flag line
/// carries no placeholder, so the metadata's flag list tells the two apart.
fn listed_options(page: &str, flags: &BTreeSet<&str>) -> BTreeSet<String> {
    page.lines()
        .filter_map(|line| line.strip_prefix("  "))
        .filter(|rest| rest.starts_with("--"))
        .filter_map(|rest| rest.split_whitespace().next())
        .filter(|name| !flags.contains(name))
        .map(str::to_string)
        .collect()
}

/// Every option name the command table declares, for any command.
fn declared_options() -> Vec<&'static str> {
    let mut names: Vec<&'static str> = Vec::new();
    for spec in eggplan_cli::command_metadata() {
        for option in spec.options {
            if !names.contains(&option) {
                names.push(option);
            }
        }
    }
    names
}

/// Whether the CLI's own validator accepts `option` for this command and
/// subcommand.
///
/// The validator refuses an option with one typed `usage` diagnostic; any
/// other outcome means the option passed validation. The probe reaches
/// validation first, so a later failure is not an option failure.
fn validator_accepts(command: &str, subcommand: Option<&str>, option: &str, value: &str) -> bool {
    let mut args = vec![command];
    if let Some(subcommand) = subcommand {
        args.push(subcommand);
    }
    args.extend([option, value, "--json"]);
    let output = invoke(&args);
    let envelope: Value = serde_json::from_slice(&output.stdout).unwrap();
    let message = envelope["error"]["message"].as_str().unwrap_or_default();
    !message.contains("is not valid for this command")
}

/// A value for each declared option, chosen so the probe needs no real file,
/// plan, or repository.
fn probe_value(option: &str, temp: &Path) -> String {
    match option {
        "--state-root" => temp.join("state").to_string_lossy().into_owned(),
        "--output" => temp.join("probe output.md").to_string_lossy().into_owned(),
        "--input" => temp.join("absent-plan.json").to_string_lossy().into_owned(),
        "--provider-policy" => temp
            .join("absent-policy.json")
            .to_string_lossy()
            .into_owned(),
        "--expected-revision" => "1".to_string(),
        "--format" => "eggplan".to_string(),
        "--status" => "active".to_string(),
        "--to" => "active".to_string(),
        "--limit" => "1".to_string(),
        "--after" => "ep_probe_000".to_string(),
        "--blocker" | "--next-action" => "probe".to_string(),
        other => panic!("no probe value declared for {other}"),
    }
}

/// Every command and subcommand pair the table declares.
fn command_and_subcommands() -> Vec<(String, Option<String>)> {
    let mut pairs = Vec::new();
    for spec in eggplan_cli::command_metadata() {
        pairs.push((spec.name.to_string(), None));
        for subcommand in spec.subcommands {
            pairs.push((spec.name.to_string(), Some(subcommand.to_string())));
        }
    }
    pairs
}

#[test]
fn a_subcommand_page_lists_the_options_that_subcommand_accepts() {
    let flags = declared_flags();

    // `markdown render` accepts `--output`; `markdown inspect` does not.
    let render = help_page("markdown", Some("render"));
    assert!(
        listed_options(&render, &flags).contains("--output"),
        "the render page must list --output: {render}"
    );
    let inspect = help_page("markdown", Some("inspect"));
    assert!(
        !listed_options(&inspect, &flags).contains("--output"),
        "the inspect page must not list --output: {inspect}"
    );

    // `plan set-status` accepts `--to` and `--expected-revision`; `plan
    // export` accepts neither.
    let set_status = help_page("plan", Some("set-status"));
    let set_status_options = listed_options(&set_status, &flags);
    assert!(
        set_status_options.contains("--to"),
        "the set-status page must list --to: {set_status}"
    );
    assert!(
        set_status_options.contains("--expected-revision"),
        "the set-status page must list --expected-revision: {set_status}"
    );
    let export = help_page("plan", Some("export"));
    assert!(
        !listed_options(&export, &flags).contains("--to"),
        "the export page must not list --to: {export}"
    );
}

#[test]
fn both_help_forms_render_the_same_page() {
    for (command, subcommand) in command_and_subcommands() {
        // `help --help` is the bare usage, not the help command's own page,
        // so the `help` command has no `--help` form to compare against.
        if command == "help" {
            continue;
        }
        let via_help = help_page(&command, subcommand.as_deref());
        let via_flag = help_page_via_flag(&command, subcommand.as_deref());
        assert_eq!(
            via_help, via_flag,
            "`help {command} {subcommand:?}` and `{command} {subcommand:?} --help` must render the same page"
        );
    }
}

#[test]
fn a_subcommand_help_flag_does_not_fall_back_to_the_top_level_usage() {
    let page = human(&["markdown", "render", "--help"]);
    assert!(
        !page.contains("init - create a state root"),
        "the subcommand page must not be the top-level usage: {page}"
    );
    assert!(
        page.starts_with("eggplan markdown render - "),
        "the subcommand page must be the subcommand's own: {page}"
    );
    assert!(
        page.contains("<ARG>"),
        "the subcommand page must advertise the shape it consumes: {page}"
    );
    // `item update` consumes two positionals after its subcommand word.
    let item = human(&["item", "update", "--help"]);
    assert!(
        item.starts_with("eggplan item update - "),
        "the item update page must be the subcommand's own: {item}"
    );
    assert!(
        item.contains("<ARG> <ARG>"),
        "the item update page must advertise two arguments: {item}"
    );
}

#[test]
fn an_unknown_subcommand_is_a_usage_error_naming_the_accepted_subcommands() {
    assert_eq!(failure_code(&["help", "markdown", "nonsense"]), "usage");
    let message = failure_message(&["help", "markdown", "nonsense"]);
    assert!(
        message.contains("unknown subcommand"),
        "the error must name the mistake: {message}"
    );
    for subcommand in ["render", "inspect", "import"] {
        assert!(
            message.contains(subcommand),
            "the error must name the accepted subcommand {subcommand}: {message}"
        );
    }
    // The help path and the validator word the same mistake the same way.
    assert_eq!(failure_message(&["markdown", "nonsense"]), message);
    // The `--help` form refuses the same word.
    assert_eq!(failure_code(&["markdown", "nonsense", "--help"]), "usage");
}

#[test]
fn a_third_positional_to_help_is_a_usage_error() {
    assert_eq!(
        failure_code(&["help", "markdown", "render", "extra"]),
        "usage"
    );
    let message = failure_message(&["help", "markdown", "render", "extra"]);
    assert!(
        message.contains("help accepts a command and a subcommand"),
        "the error must say what help accepts: {message}"
    );
}

#[test]
fn a_bare_help_still_prints_the_top_level_usage() {
    let top = human(&["--help"]);
    assert!(
        top.contains("init - create a state root"),
        "the top-level usage must list the commands: {top}"
    );
    assert_eq!(human(&["help"]), top);
    let list = human(&["help", "list"]);
    assert!(
        list.starts_with("eggplan list - "),
        "`help list` must print the list command's own page: {list}"
    );
    assert!(
        !list.contains("init - create a state root"),
        "`help list` must not print the top-level usage: {list}"
    );
}

#[test]
fn an_unknown_command_still_falls_back_to_the_top_level_usage() {
    assert_eq!(human(&["help", "nonexistent"]), human(&["--help"]));
}

#[test]
fn the_json_envelope_carries_the_same_page_as_the_human_output() {
    let output = invoke(&["help", "markdown", "render", "--json"]);
    assert!(output.status.success());
    let envelope: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(envelope["schema_version"], 1);
    assert_eq!(envelope["ok"], true);
    assert_eq!(
        envelope["data"]["command"].as_str().unwrap(),
        human(&["help", "markdown", "render"]).trim_end()
    );
}

/// The agreement property: the options a help page lists are exactly the
/// options the validator accepts, for every command and every subcommand.
///
/// The expectation is not written out case by case. It is read off the
/// validator itself: every option the table declares is offered to the
/// validator for that command and subcommand, and the ones the validator
/// refuses are exactly the ones the help page omits.
#[test]
fn every_help_page_lists_exactly_the_options_the_validator_accepts() {
    let flags = declared_flags();
    let pool = declared_options();
    assert!(
        pool.contains(&"--output") && pool.contains(&"--to"),
        "the probe pool must carry the subcommand-scoped options"
    );
    let temp = tempdir().unwrap();
    for (command, subcommand) in command_and_subcommands() {
        let page = help_page(&command, subcommand.as_deref());
        let listed = listed_options(&page, &flags);
        let mut accepted = BTreeSet::new();
        for option in &pool {
            if validator_accepts(
                &command,
                subcommand.as_deref(),
                option,
                &probe_value(option, temp.path()),
            ) {
                accepted.insert((*option).to_string());
            }
        }
        assert_eq!(
            listed, accepted,
            "the help page for {command} {subcommand:?} must list exactly the options the validator accepts"
        );
    }
}
