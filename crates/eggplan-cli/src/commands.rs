//! One declarative command inventory.
//!
//! Help text, option validation, and shell completion generation all read this
//! table. Parser *execution* stays handwritten; the point is that the list of
//! commands and options cannot quietly become three independent inventories.
//!
//! Adding a command here changes help and every completion shell together,
//! which `metadata_drives_help_and_completions_together` asserts.

/// How many positional arguments a command accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Positional {
    /// No positionals.
    None,
    /// Exactly one.
    One,
    /// One or more, bounded by [`Bounds::MAX_EXPLICIT_PLAN_IDS`].
    OneOrMore,
    /// A subcommand word followed by exactly one more positional.
    SubcommandThenOne,
    /// A subcommand word followed by exactly two positionals.
    SubcommandThenTwo,
    /// A subcommand word only.
    Subcommand,
}

/// One `--option VALUE`.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ValueOption {
    pub(crate) name: &'static str,
    pub(crate) help: &'static str,
    /// Canonical values, when the option accepts a closed set.
    pub(crate) values: &'static [&'static str],
    /// Placeholder shown in help and emitted by completion scripts.
    pub(crate) placeholder: &'static str,
}

impl ValueOption {
    pub(crate) const fn flag(
        name: &'static str,
        placeholder: &'static str,
        help: &'static str,
    ) -> Self {
        Self {
            name,
            help,
            values: &[],
            placeholder,
        }
    }

    pub(crate) const fn enum_of(
        name: &'static str,
        values: &'static [&'static str],
        help: &'static str,
    ) -> Self {
        Self {
            name,
            help,
            values,
            placeholder: "VALUE",
        }
    }
}

/// One boolean flag.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Flag {
    pub(crate) name: &'static str,
    pub(crate) help: &'static str,
}

impl Flag {
    pub(crate) const fn new(name: &'static str, help: &'static str) -> Self {
        Self { name, help }
    }
}

/// A command, or a subcommand of one.
#[derive(Debug, Clone, Copy)]
pub(crate) struct CommandSpec {
    pub(crate) name: &'static str,
    pub(crate) positional: Positional,
    pub(crate) subcommands: &'static [&'static str],
    pub(crate) options: &'static [ValueOption],
    pub(crate) flags: &'static [Flag],
    /// Options valid only for a specific subcommand. Entries are
    /// `(subcommand, options)`.
    pub(crate) sub_options: &'static [(&'static str, &'static [ValueOption])],
    pub(crate) help: &'static str,
}

impl CommandSpec {
    pub(crate) const fn new(
        name: &'static str,
        positional: Positional,
        subcommands: &'static [&'static str],
        options: &'static [ValueOption],
        flags: &'static [Flag],
        sub_options: &'static [(&'static str, &'static [ValueOption])],
        help: &'static str,
    ) -> Self {
        Self {
            name,
            positional,
            subcommands,
            options,
            flags,
            sub_options,
            help,
        }
    }
}

/// Documented bounds for batch selection. Referenced by the plan bounds and
/// enforced in the command implementations.
pub(crate) mod bounds {
    /// Explicit Plan IDs accepted by `status` in one invocation.
    pub(crate) const MAX_EXPLICIT_PLAN_IDS: usize = 100;
    /// Maximum `--limit` for `list`, matching the existing projection cap.
    pub(crate) const MAX_LIMIT: usize = 100;
    /// Default `--limit` when the caller does not pass one.
    pub(crate) const DEFAULT_LIMIT: usize = 100;
}

const STATE_ROOT: ValueOption = ValueOption::flag(
    "--state-root",
    "PATH",
    "repository state root (default .eggplan)",
);
const PROVIDER_POLICY: ValueOption = ValueOption::flag(
    "--provider-policy",
    "FILE",
    "strict bounded provider policy; absence means no implicit trust",
);
const FORMAT: ValueOption = ValueOption::enum_of(
    "--format",
    &["eggplan", "codegg", "auto"],
    "interchange dialect",
);
const EXPECTED_REVISION: ValueOption = ValueOption::flag(
    "--expected-revision",
    "N",
    "compare-and-swap expected revision",
);
const RECOVER: Flag = Flag::new("--recover-pending", "explicitly recover pending closures");

const ITEM_STATUS_VALUES: &[&str] = &[
    "Pending",
    "Actionable",
    "InProgress",
    "Blocked",
    "Completed",
    "Cancelled",
];
const INPUT: ValueOption = ValueOption::flag("--input", "PLAN.json", "plan definition to import");
const OUTPUT: ValueOption =
    ValueOption::flag("--output", "FILE", "write rendered markdown to a file");
const BLOCKER: ValueOption = ValueOption::flag("--blocker", "TEXT", "set the item blocker");
const NEXT_ACTION: ValueOption =
    ValueOption::flag("--next-action", "TEXT", "set the item next action");
const STATUS_FILTER: ValueOption =
    ValueOption::enum_of("--status", STATUS_VALUES, "filter by canonical plan status");
const LIMIT: ValueOption =
    ValueOption::flag("--limit", "N", "maximum rows (default 100, maximum 100)");
const AFTER: ValueOption = ValueOption::flag(
    "--after",
    "PLAN_ID",
    "keyset cursor: strictly greater plan IDs",
);
const ITEM_STATUS_OPTION: ValueOption =
    ValueOption::enum_of("--status", ITEM_STATUS_VALUES, "set the item status");
const HELP_FLAG: Flag = Flag::new("--help", "show this help");

const MARKDOWN_SUBCOMMANDS: &[&str] = &["render", "inspect", "import"];
const CLOSURE_SUBCOMMANDS: &[&str] = &["show"];
const REGISTRY_SUBCOMMANDS: &[&str] = &["render"];
const COMPLETION_SUBCOMMANDS: &[&str] = &["bash", "zsh", "fish", "powershell"];
const EVIDENCE_SUBCOMMANDS: &[&str] = &["list", "show", "supersessions"];
const ITEM_SUBCOMMANDS: &[&str] = &["update"];
const STATUS_VALUES: &[&str] = &["draft", "active", "blocked", "closed", "cancelled"];

pub(crate) const COMMANDS: &[CommandSpec] = &[
    CommandSpec::new(
        "init",
        Positional::None,
        &[],
        &[STATE_ROOT],
        &[],
        &[],
        "create a state root",
    ),
    CommandSpec::new(
        "new",
        Positional::One,
        &[],
        &[STATE_ROOT, INPUT],
        &[],
        &[],
        "create a plan from a definition",
    ),
    CommandSpec::new(
        "list",
        Positional::None,
        &[],
        &[STATE_ROOT, PROVIDER_POLICY, STATUS_FILTER, LIMIT, AFTER],
        &[],
        &[],
        "compact, bounded plan overview",
    ),
    CommandSpec::new(
        "show",
        Positional::One,
        &[],
        &[STATE_ROOT, PROVIDER_POLICY],
        &[],
        &[],
        "one plan with its items",
    ),
    CommandSpec::new(
        "status",
        Positional::OneOrMore,
        &[],
        &[STATE_ROOT, PROVIDER_POLICY],
        &[],
        &[],
        "plan status summary",
    ),
    CommandSpec::new(
        "ready",
        Positional::One,
        &[],
        &[STATE_ROOT],
        &[],
        &[],
        "ready-to-execute items",
    ),
    CommandSpec::new(
        "graph",
        Positional::One,
        &[],
        &[STATE_ROOT],
        &[],
        &[],
        "dependency graph",
    ),
    CommandSpec::new(
        "check",
        Positional::OneOrMore,
        &[],
        &[STATE_ROOT, PROVIDER_POLICY],
        &[RECOVER],
        &[],
        "read-only integrity and readiness check",
    ),
    CommandSpec::new(
        "activate",
        Positional::One,
        &[],
        &[STATE_ROOT, EXPECTED_REVISION],
        &[],
        &[],
        "activate a draft plan",
    ),
    CommandSpec::new(
        "item",
        Positional::SubcommandThenTwo,
        ITEM_SUBCOMMANDS,
        &[
            STATE_ROOT,
            EXPECTED_REVISION,
            ITEM_STATUS_OPTION,
            BLOCKER,
            NEXT_ACTION,
        ],
        &[],
        &[],
        "update a plan item",
    ),
    CommandSpec::new(
        "evidence",
        Positional::SubcommandThenOne,
        EVIDENCE_SUBCOMMANDS,
        &[STATE_ROOT],
        &[],
        &[],
        "inspect the evidence ledger",
    ),
    CommandSpec::new(
        "assess",
        Positional::One,
        &[],
        &[STATE_ROOT, PROVIDER_POLICY],
        &[],
        &[],
        "assess a plan against a policy",
    ),
    CommandSpec::new(
        "close",
        Positional::One,
        &[],
        &[STATE_ROOT, PROVIDER_POLICY, EXPECTED_REVISION],
        &[],
        &[],
        "guarded closure",
    ),
    CommandSpec::new(
        "closure",
        Positional::SubcommandThenOne,
        CLOSURE_SUBCOMMANDS,
        &[STATE_ROOT],
        &[],
        &[],
        "show a closure record",
    ),
    CommandSpec::new(
        "registry",
        Positional::Subcommand,
        REGISTRY_SUBCOMMANDS,
        &[STATE_ROOT, PROVIDER_POLICY],
        &[],
        &[],
        "render the plan registry",
    ),
    CommandSpec::new(
        "markdown",
        Positional::SubcommandThenOne,
        MARKDOWN_SUBCOMMANDS,
        &[STATE_ROOT],
        &[],
        &[
            ("render", &[OUTPUT]),
            ("inspect", &[FORMAT]),
            ("import", &[FORMAT]),
        ],
        "Markdown interchange",
    ),
    CommandSpec::new(
        "completions",
        Positional::Subcommand,
        COMPLETION_SUBCOMMANDS,
        &[STATE_ROOT],
        &[],
        &[],
        "print a shell completion script to stdout",
    ),
    CommandSpec::new(
        "help",
        Positional::One,
        &[],
        &[],
        &[HELP_FLAG],
        &[],
        "show usage",
    ),
];

pub(crate) fn spec(name: &str) -> Option<&'static CommandSpec> {
    COMMANDS.iter().find(|spec| spec.name == name)
}

pub(crate) fn is_value_option(name: &str) -> bool {
    COMMANDS.iter().any(|spec| {
        spec.options.iter().any(|option| option.name == name)
            || spec
                .sub_options
                .iter()
                .any(|(_, options)| options.iter().any(|option| option.name == name))
    })
}

pub(crate) fn is_flag(name: &str) -> bool {
    COMMANDS
        .iter()
        .any(|spec| spec.flags.iter().any(|flag| flag.name == name))
}

pub(crate) fn is_subcommand_of(command: &str, subcommand: &str) -> bool {
    spec(command).is_some_and(|spec| spec.subcommands.contains(&subcommand))
}

/// Value options valid for `command`, including the subcommand-scoped ones when
/// a subcommand word is present. Used by validation and by completions.
pub(crate) fn options_for(
    command: &str,
    subcommand: Option<&str>,
) -> (&'static [ValueOption], &'static [Flag]) {
    let Some(spec) = spec(command) else {
        return (&[], &[]);
    };
    let mut options = spec.options.to_vec();
    if let Some(subcommand) = subcommand
        && let Some((_, extra)) = spec
            .sub_options
            .iter()
            .find(|(name, _)| *name == subcommand)
    {
        options.extend(extra.iter().copied());
    }
    // Leaked once per process per distinct command/subcommand pair; the table is
    // static and the set is bounded by the table itself.
    let leaked: &'static [ValueOption] = Box::leak(options.into_boxed_slice());
    (leaked, spec.flags)
}

/// One-line usage summary, rendered from the same table the parser validates
/// against so help and validation cannot drift.
pub(crate) fn usage() -> String {
    let rendered: Vec<String> = COMMANDS
        .iter()
        .map(|spec| {
            let shape = match spec.positional {
                Positional::None => String::new(),
                Positional::One => " ARG".to_string(),
                Positional::OneOrMore => " [ARG ...]".to_string(),
                Positional::Subcommand => " SUBCOMMAND".to_string(),
                Positional::SubcommandThenOne => " SUBCOMMAND ARG".to_string(),
                Positional::SubcommandThenTwo => " SUBCOMMAND ARG ARG".to_string(),
            };
            format!("{}{shape} - {}", spec.name, spec.help)
        })
        .collect();
    format!(
        "eggplan [--state-root PATH] [--json] COMMAND\n{}",
        rendered.join("\n")
    )
}

/// Per-command help, listing the exact options, flags, and subcommands the
/// metadata declares. Rendered rather than hand-maintained.
pub(crate) fn command_help(name: &str) -> Option<String> {
    let spec = spec(name)?;
    let mut out = format!("eggplan {} - {}", spec.name, spec.help);
    let shape = match spec.positional {
        Positional::None => "",
        Positional::One => " <ARG>",
        Positional::OneOrMore => " <ARG>...",
        Positional::Subcommand => " <SUBCOMMAND>",
        Positional::SubcommandThenOne => " <SUBCOMMAND> <ARG>",
        Positional::SubcommandThenTwo => " <SUBCOMMAND> <ARG> <ARG>",
    };
    out.push_str(shape);
    if !spec.subcommands.is_empty() {
        out.push_str(&format!("\n  subcommands: {}", spec.subcommands.join(", ")));
    }
    for option in spec.options {
        out.push_str(&format!(
            "\n  {} <{}>  {}",
            option.name, option.placeholder, option.help
        ));
        if !option.values.is_empty() {
            out.push_str(&format!(" [{}]", option.values.join("|")));
        }
    }
    for flag in spec.flags {
        out.push_str(&format!("\n  {}  {}", flag.name, flag.help));
    }
    Some(out)
}
// ---------------------------------------------------------------------------
// Shell completion generation
// ---------------------------------------------------------------------------

/// Shells for which a completion script can be generated.
///
/// Static only: command names, options, subcommands, and closed value sets.
/// Dynamic Plan-ID completion is deliberately out of scope because it would
/// couple the shell to repository discovery and its latency.
pub(crate) const SHELLS: &[&str] = &["bash", "zsh", "fish", "powershell"];

pub(crate) fn completion_script(shell: &str) -> Option<String> {
    let commands = COMMANDS
        .iter()
        .map(|spec| spec.name)
        .collect::<Vec<_>>()
        .join(" ");
    let subcommands = COMMANDS
        .iter()
        .filter(|spec| !spec.subcommands.is_empty())
        .map(|spec| format!("{}:{}", spec.name, spec.subcommands.join("|")))
        .collect::<Vec<_>>()
        .join(" ");
    let options = COMMANDS
        .iter()
        .flat_map(|spec| {
            spec.options
                .iter()
                .map(|option| option.name)
                .chain(spec.flags.iter().map(|flag| flag.name))
        })
        .collect::<Vec<_>>()
        .join(" ");
    let values = COMMANDS
        .iter()
        .flat_map(|spec| spec.options.iter())
        .filter(|option| !option.values.is_empty())
        .map(|option| format!("{}={}", option.name, option.values.join(",")))
        .collect::<Vec<_>>()
        .join(" ");

    match shell {
        "bash" => Some(format!(
            "# eggplan bash completion (generated; do not edit)\n\
             _eggplan() {{\n\
             \x20 local cur commands=\"{commands}\" subs=\"{subcommands}\" opts=\"{options}\"\n\
             \x20 COMPREPLY=()\n\
             \x20 cur=\"${{COMP_WORDS[COMP_CWORD]}}\"\n\
             \x20 COMPREPLY=( $(compgen -W \"$commands $opts\" -- \"$cur\") )\n\
             \x20 return 0\n\
             }}\n\
             complete -F _eggplan eggplan\n"
        )),
        "zsh" => Some(format!(
            "#compdef eggplan\n\
             # eggplan zsh completion (generated; do not edit)\n\
             _eggplan() {{\n\
             \x20 local -a commands subcommands options values\n\
             \x20 commands=({commands})\n\
             \x20 subcommands=({subcommands})\n\
             \x20 options=({options})\n\
             \x20 values=({values})\n\
             \x20 _describe 'command' commands\n\
             }}\n\
             compdef _eggplan eggplan\n"
        )),
        "fish" => {
            let mut out = String::from("# eggplan fish completion (generated; do not edit)\n");
            for command in COMMANDS {
                out.push_str(&format!(
                    "complete -c eggplan -n '__fish_use_subcommand' -a '{}' -d '{}'\n",
                    command.name, command.help
                ));
                for subcommand in command.subcommands {
                    out.push_str(&format!(
                        "complete -c eggplan -n '__fish_seen_subcommand_from {}' -a '{subcommand}' -d '{} {subcommand}'\n",
                        command.name, command.name
                    ));
                }
                for option in command.options {
                    out.push_str(&format!(
                        "complete -c eggplan -n '__fish_seen_subcommand_from {}' -l '{}' -d '{}'\n",
                        command.name,
                        option.name.trim_start_matches("--"),
                        option.help
                    ));
                    if !option.values.is_empty() {
                        out.push_str(&format!(
                            "complete -c eggplan -n '__fish_seen_subcommand_from {}' -l '{}' -x -a '{}'\n",
                            command.name,
                            option.name.trim_start_matches("--"),
                            option.values.join(" ")
                        ));
                    }
                }
                for flag in command.flags {
                    out.push_str(&format!(
                        "complete -c eggplan -n '__fish_seen_subcommand_from {}' -l '{}' -d '{}'\n",
                        command.name,
                        flag.name.trim_start_matches("--"),
                        flag.help
                    ));
                }
            }
            Some(out)
        }
        "powershell" => {
            let mut out = String::from(
                "# eggplan powershell completion (generated; do not edit)\n\
                 Register-ArgumentCompleter -Native -CommandName eggplan -ScriptBlock {\n\
                 \x20 param($wordToComplete, $commandAst, $cursorPosition)\n\
                 \x20 $commands = @(",
            );
            out.push_str(
                &commands
                    .split_whitespace()
                    .map(|name| format!("'{name}'"))
                    .collect::<Vec<_>>()
                    .join(", "),
            );
            out.push_str(
                ")\n\
                 \x20 $options = @(",
            );
            out.push_str(
                &options
                    .split_whitespace()
                    .map(|name| format!("'{name}'"))
                    .collect::<Vec<_>>()
                    .join(", "),
            );
            out.push_str(
                ")\n\
                 \x20 $commands + $options | Where-Object { $_ -like \"$wordToComplete*\" } | ForEach-Object {\n\
                 \x20 \x20 [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', $_)\n\
                 \x20 }\n\
                 }\n",
            );
            Some(out)
        }
        _ => None,
    }
}
