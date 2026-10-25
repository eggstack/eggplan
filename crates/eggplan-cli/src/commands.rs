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
    /// Zero or one.
    ZeroOrOne,
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

// The serialized `PlanItemStatus` spellings. Help, validation, and completions
// all read this table, so it must name the values the parser actually accepts.
const ITEM_STATUS_VALUES: &[&str] = &[
    "pending",
    "actionable",
    "in_progress",
    "blocked",
    "completed",
    "cancelled",
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

/// The plan statuses `plan set-status` can set.
///
/// `Closed` is not here because only the guarded closure finalizer may write
/// it, and `Draft` is not here because no transition reaches it. The value set
/// is the same one the command accepts, so completion never offers a value
/// that cannot be applied.
const SETTABLE_STATUS_VALUES: &[&str] = &["active", "blocked", "cancelled"];

/// The settable plan statuses, shared with the command handler so that the
/// value set the command accepts and the value set completion offers cannot
/// drift apart.
pub(crate) const SETTABLE_PLAN_STATUS_VALUES: &[&str] = SETTABLE_STATUS_VALUES;
const PLAN_STATUS_OPTION: ValueOption =
    ValueOption::enum_of("--to", SETTABLE_STATUS_VALUES, "target plan status");
const PLAN_INPUT: ValueOption =
    ValueOption::flag("--input", "PLAN.json", "replacement plan definition");
const PLAN_OUTPUT: ValueOption = ValueOption::flag(
    "--output",
    "FILE",
    "write the canonical plan JSON to a file",
);

const MARKDOWN_SUBCOMMANDS: &[&str] = &["render", "inspect", "import"];
const CLOSURE_SUBCOMMANDS: &[&str] = &["show"];
const REGISTRY_SUBCOMMANDS: &[&str] = &["render"];
const COMPLETION_SUBCOMMANDS: &[&str] = &["bash", "zsh", "fish", "powershell"];
const EVIDENCE_SUBCOMMANDS: &[&str] = &["list", "show", "supersessions"];
const ITEM_SUBCOMMANDS: &[&str] = &["update"];
const PLAN_SUBCOMMANDS: &[&str] = &["set-status", "edit", "export"];
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
        Positional::None,
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
        Positional::ZeroOrOne,
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
        "plan",
        Positional::SubcommandThenOne,
        PLAN_SUBCOMMANDS,
        &[STATE_ROOT],
        &[],
        &[
            ("set-status", &[PLAN_STATUS_OPTION, EXPECTED_REVISION]),
            ("edit", &[PLAN_INPUT, EXPECTED_REVISION]),
            ("export", &[PLAN_OUTPUT]),
        ],
        "plan lifecycle and structure",
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
        Positional::ZeroOrOne,
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
                Positional::ZeroOrOne => " [ARG]".to_string(),
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
        Positional::ZeroOrOne => " [<ARG>]",
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

/// The depth every generated shell script completes to.
///
/// Completion is one capability offered on four shells, so all four consume
/// the command metadata to the same depth:
///
/// 1. the top-level command names;
/// 2. the subcommands of the command being completed;
/// 3. the options and flags declared for that command, including the options
///    scoped to one of its subcommands;
/// 4. the fixed values of the option being completed, when that option
///    declares a closed value set.
///
/// The depth belongs to the generator, not to one shell: a script that stops
/// at a shallower level offers a user less than the metadata declares. The
/// four generators below are therefore written from one per-command model,
/// and the CLI test asserts the depth per shell rather than asserting that a
/// script merely mentions a name.
const COMPLETION_DEPTH: &str = "\
commands, subcommands, options (including subcommand-scoped options), and option values";

/// One option or flag a command accepts.
#[derive(Debug, Clone)]
struct CompletionOption {
    name: &'static str,
    help: &'static str,
    /// The closed value set, when the option declares one.
    values: Vec<&'static str>,
}

/// One command's completion entries, resolved from the single metadata table.
#[derive(Debug, Clone)]
struct CommandCompletion {
    name: &'static str,
    help: &'static str,
    subcommands: &'static [&'static str],
    options: Vec<CompletionOption>,
}

/// The completion model the four shell generators read.
///
/// Built once per generation from `COMMANDS`. Nothing here touches the
/// filesystem, the repository, or the environment, so generation stays
/// deterministic and side-effect free.
#[derive(Debug, Clone)]
struct CompletionModel {
    commands: Vec<CommandCompletion>,
}

impl CompletionModel {
    fn build() -> Self {
        Self {
            commands: COMMANDS
                .iter()
                .map(|spec| CommandCompletion {
                    name: spec.name,
                    help: spec.help,
                    subcommands: spec.subcommands,
                    options: completion_options(spec),
                })
                .collect(),
        }
    }

    /// Every top-level command name, in table order.
    fn command_names(&self) -> Vec<&'static str> {
        self.commands.iter().map(|command| command.name).collect()
    }

    /// Every option and flag name, deduplicated, in table order.
    fn option_names(&self) -> Vec<&'static str> {
        let mut names: Vec<&'static str> = Vec::new();
        for command in &self.commands {
            for option in &command.options {
                if !names.contains(&option.name) {
                    names.push(option.name);
                }
            }
        }
        names
    }
}

/// The options and flags a command accepts, in table order.
///
/// The subcommand-scoped options are included because they are options the
/// command really accepts: `options_for` validates against the same set. A
/// name declared twice keeps one entry whose value set is the union of both,
/// so an option several subcommands share is offered once.
fn completion_options(spec: &CommandSpec) -> Vec<CompletionOption> {
    let mut declared: Vec<(&'static str, &'static str, &'static [&'static str])> = Vec::new();
    for option in spec.options {
        declared.push((option.name, option.help, option.values));
    }
    for (_, scoped) in spec.sub_options {
        for option in *scoped {
            declared.push((option.name, option.help, option.values));
        }
    }
    for flag in spec.flags {
        declared.push((flag.name, flag.help, &[]));
    }
    let mut options: Vec<CompletionOption> = Vec::new();
    for (name, help, values) in declared {
        match options.iter_mut().find(|option| option.name == name) {
            Some(existing) => {
                for value in values {
                    if !existing.values.contains(value) {
                        existing.values.push(value);
                    }
                }
            }
            None => options.push(CompletionOption {
                name,
                help,
                values: values.to_vec(),
            }),
        }
    }
    options
}

pub(crate) fn completion_script(shell: &str) -> Option<String> {
    let model = CompletionModel::build();
    match shell {
        "bash" => Some(model.bash()),
        "zsh" => Some(model.zsh()),
        "fish" => Some(model.fish()),
        "powershell" => Some(model.powershell()),
        _ => None,
    }
}

impl CompletionModel {
    /// The Bash generator.
    ///
    /// Bash has no per-word completion helper for this shape, so the
    /// candidates are collected into one array and handed to `compgen`. Every
    /// level of the declared depth is reachable: the command names, the
    /// subcommands of the command in the line, that command's options, and
    /// the values of the option in the line.
    fn bash(&self) -> String {
        let mut out = String::from("# eggplan bash completion (generated; do not edit)\n");
        out.push_str(&format!("# completion depth: {COMPLETION_DEPTH}\n"));
        out.push_str("_eggplan() {\n");
        out.push_str("  local cur previous command\n");
        out.push_str("  local -a commands options candidates\n");
        out.push_str("  local index word\n");
        out.push_str(&format!(
            "  commands=({})\n",
            self.command_names().join(" ")
        ));
        out.push_str(&format!("  options=({})\n", self.option_names().join(" ")));
        out.push_str("  cur=\"${COMP_WORDS[COMP_CWORD]}\"\n");
        out.push_str("  previous=\"${COMP_WORDS[COMP_CWORD-1]}\"\n");
        out.push_str("  command=\"\"\n");
        out.push_str("  for ((index = 1; index < COMP_CWORD; index++)); do\n");
        out.push_str("    word=\"${COMP_WORDS[index]}\"\n");
        out.push_str("    case \" ${commands[*]} \" in\n");
        out.push_str("      *\" ${word} \"*) command=\"${word}\"; break ;;\n");
        out.push_str("    esac\n");
        out.push_str("  done\n");
        out.push_str("  candidates=()\n");
        out.push_str("  if [ \"${COMP_CWORD}\" -eq 1 ]; then\n");
        out.push_str("    candidates=(\"${commands[@]}\" \"${options[@]}\")\n");
        out.push_str("  else\n");
        out.push_str("    case \"${command}:${previous}\" in\n");
        for command in &self.commands {
            for option in &command.options {
                if option.values.is_empty() {
                    continue;
                }
                out.push_str(&format!(
                    "      {}:{}) candidates=({}) ;;\n",
                    command.name,
                    option.name,
                    option.values.join(" ")
                ));
            }
        }
        out.push_str("    esac\n");
        out.push_str("    if [ \"${#candidates[@]}\" -eq 0 ]; then\n");
        out.push_str("      case \"${command}\" in\n");
        for command in &self.commands {
            if command.subcommands.is_empty() {
                continue;
            }
            out.push_str(&format!(
                "        {}) candidates=({}) ;;\n",
                command.name,
                command.subcommands.join(" ")
            ));
        }
        out.push_str("      esac\n");
        out.push_str("      case \"${command}\" in\n");
        for command in &self.commands {
            if command.options.is_empty() {
                continue;
            }
            out.push_str(&format!(
                "        {}) candidates+=({}) ;;\n",
                command.name,
                command
                    .options
                    .iter()
                    .map(|option| option.name)
                    .collect::<Vec<_>>()
                    .join(" ")
            ));
        }
        out.push_str("      esac\n");
        out.push_str("    fi\n");
        out.push_str("  fi\n");
        out.push_str("  COMPREPLY=( $(compgen -W \"${candidates[*]}\" -- \"${cur}\") )\n");
        out.push_str("  return 0\n");
        out.push_str("}\n");
        out.push_str("complete -F _eggplan eggplan\n");
        out
    }

    /// The Zsh generator.
    ///
    /// The script reads the command word and the word before the cursor out
    /// of the completion system's `words` array, then offers the matching
    /// level of the declared depth.
    fn zsh(&self) -> String {
        let mut out = String::from("#compdef eggplan\n");
        out.push_str("# eggplan zsh completion (generated; do not edit)\n");
        out.push_str(&format!("# completion depth: {COMPLETION_DEPTH}\n"));
        out.push_str("_eggplan() {\n");
        out.push_str("  local -a commands options subs opts vals\n");
        out.push_str("  local command_name previous index\n");
        out.push_str(&format!(
            "  commands=({})\n",
            self.command_names().join(" ")
        ));
        out.push_str(&format!("  options=({})\n", self.option_names().join(" ")));
        out.push_str("  subs=()\n");
        out.push_str("  opts=()\n");
        out.push_str("  vals=()\n");
        out.push_str("  command_name=\"\"\n");
        out.push_str("  previous=\"\"\n");
        out.push_str("  if [ \"${CURRENT}\" -gt 1 ]; then\n");
        out.push_str("    previous=\"${words[CURRENT-1]}\"\n");
        out.push_str("  fi\n");
        out.push_str("  if [ \"${CURRENT}\" -gt 2 ]; then\n");
        out.push_str("    index=2\n");
        out.push_str("    while [ \"${index}\" -lt \"${CURRENT}\" ]; do\n");
        out.push_str("      case \"${words[index]}\" in\n");
        out.push_str(&format!(
            "        {}) command_name=\"${{words[index]}}\"; break ;;\n",
            self.command_names().join("|")
        ));
        out.push_str("      esac\n");
        out.push_str("      index=$((index + 1))\n");
        out.push_str("    done\n");
        out.push_str("  fi\n");
        out.push_str("  case \"${command_name}:${previous}\" in\n");
        for command in &self.commands {
            for option in &command.options {
                if option.values.is_empty() {
                    continue;
                }
                out.push_str(&format!(
                    "    {}:{}) vals=({}) ;;\n",
                    command.name,
                    option.name,
                    option.values.join(" ")
                ));
            }
        }
        out.push_str("  esac\n");
        out.push_str("  case \"${command_name}\" in\n");
        for command in &self.commands {
            if command.subcommands.is_empty() {
                continue;
            }
            out.push_str(&format!(
                "    {}) subs=({}) ;;\n",
                command.name,
                command.subcommands.join(" ")
            ));
        }
        out.push_str("    *) subs=() ;;\n");
        out.push_str("  esac\n");
        out.push_str("  case \"${command_name}\" in\n");
        for command in &self.commands {
            if command.options.is_empty() {
                continue;
            }
            out.push_str(&format!(
                "    {}) opts=({}) ;;\n",
                command.name,
                command
                    .options
                    .iter()
                    .map(|option| option.name)
                    .collect::<Vec<_>>()
                    .join(" ")
            ));
        }
        out.push_str("    *) opts=(\"${options[@]}\") ;;\n");
        out.push_str("  esac\n");
        out.push_str("  if [ \"${#vals[@]}\" -gt 0 ]; then\n");
        out.push_str("    _describe 'value' vals\n");
        out.push_str("    return\n");
        out.push_str("  fi\n");
        out.push_str("  if [ \"${CURRENT}\" -eq 2 ]; then\n");
        out.push_str("    _describe 'command' commands\n");
        out.push_str("    _describe 'option' options\n");
        out.push_str("    return\n");
        out.push_str("  fi\n");
        out.push_str("  if [ \"${#subs[@]}\" -gt 0 ]; then\n");
        out.push_str("    _describe 'subcommand' subs\n");
        out.push_str("  fi\n");
        out.push_str("  if [ \"${#opts[@]}\" -gt 0 ]; then\n");
        out.push_str("    _describe 'option' opts\n");
        out.push_str("  fi\n");
        out.push_str("}\n");
        out.push_str("compdef _eggplan eggplan\n");
        out
    }

    /// The Fish generator.
    ///
    /// Fish conditions are per command, so the script offers the subcommands
    /// of the command in the line, that command's options, and the values of
    /// the option in the line.
    fn fish(&self) -> String {
        let mut out = String::from("# eggplan fish completion (generated; do not edit)\n");
        out.push_str(&format!("# completion depth: {COMPLETION_DEPTH}\n"));
        for command in &self.commands {
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
            for option in &command.options {
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
        }
        out
    }

    /// The PowerShell generator.
    ///
    /// The native argument completer receives the command AST, so the command
    /// word and the word before the cursor are read from the elements the
    /// user has typed. The same four levels of the declared depth are
    /// offered.
    fn powershell(&self) -> String {
        let mut out = String::from("# eggplan powershell completion (generated; do not edit)\n");
        out.push_str(&format!("# completion depth: {COMPLETION_DEPTH}\n"));
        out.push_str("Register-ArgumentCompleter -Native -CommandName eggplan -ScriptBlock {\n");
        out.push_str("  param($wordToComplete, $commandAst)\n");
        out.push_str(&format!(
            "  $commands = @({})\n",
            powershell_list(&self.command_names())
        ));
        out.push_str(&format!(
            "  $options = @({})\n",
            powershell_list(&self.option_names())
        ));
        out.push_str(&format!(
            "  $subcommands = @{{{}}}\n",
            powershell_map(&self.subcommand_map())
        ));
        out.push_str(&format!(
            "  $values = @{{{}}}\n",
            powershell_map(&self.value_map())
        ));
        out.push_str(&format!(
            "  $commandOptions = @{{{}}}\n",
            powershell_map(&self.command_option_map())
        ));
        out.push_str("  $candidates = @()\n");
        out.push_str("  $command = ''\n");
        out.push_str("  $previous = ''\n");
        out.push_str("  $elements = @()\n");
        out.push_str(
            "  if ($null -ne $commandAst) { $elements = @($commandAst.CommandElements) }\n",
        );
        out.push_str("  for ($index = 1; $index -lt $elements.Count; $index++) {\n");
        out.push_str("    if ($commands -contains $elements[$index].Extent.Text) {\n");
        out.push_str("      $command = $elements[$index].Extent.Text\n");
        out.push_str("      break\n");
        out.push_str("    }\n");
        out.push_str("  }\n");
        out.push_str("  if ($elements.Count -gt 1) {\n");
        out.push_str("    $last = $elements[$elements.Count - 1].Extent.Text\n");
        out.push_str("    if ($last -eq $wordToComplete) {\n");
        out.push_str(
            "      if ($elements.Count -gt 2) { $previous = $elements[$elements.Count - 2].Extent.Text }\n",
        );
        out.push_str("    } else {\n");
        out.push_str("      $previous = $last\n");
        out.push_str("    }\n");
        out.push_str("  }\n");
        out.push_str("  $key = $command + ':' + $previous\n");
        out.push_str("  if ($values.ContainsKey($key)) {\n");
        out.push_str("    $candidates = @($values[$key])\n");
        out.push_str("  } else {\n");
        out.push_str("    if ($command -eq '') {\n");
        out.push_str("      $candidates = @($commands)\n");
        out.push_str("    } elseif ($subcommands.ContainsKey($command)) {\n");
        out.push_str("      $candidates = @($subcommands[$command])\n");
        out.push_str("    }\n");
        out.push_str("    if ($commandOptions.ContainsKey($command)) {\n");
        out.push_str("      $candidates += @($commandOptions[$command])\n");
        out.push_str("    } else {\n");
        out.push_str("      $candidates += @($options)\n");
        out.push_str("    }\n");
        out.push_str("  }\n");
        out.push_str(
            "  $candidates | Where-Object { $_ -like \"$wordToComplete*\" } | ForEach-Object {\n",
        );
        out.push_str(
            "    [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', $_)\n",
        );
        out.push_str("  }\n");
        out.push_str("}\n");
        out
    }

    /// The subcommand list of every command that has one, keyed by command.
    fn subcommand_map(&self) -> Vec<(String, Vec<&'static str>)> {
        self.commands
            .iter()
            .filter(|command| !command.subcommands.is_empty())
            .map(|command| (command.name.to_string(), command.subcommands.to_vec()))
            .collect()
    }

    /// The value set of every option that declares one, keyed by
    /// `command:option`.
    fn value_map(&self) -> Vec<(String, Vec<&'static str>)> {
        let mut entries = Vec::new();
        for command in &self.commands {
            for option in &command.options {
                if option.values.is_empty() {
                    continue;
                }
                entries.push((
                    format!("{}:{}", command.name, option.name),
                    option.values.clone(),
                ));
            }
        }
        entries
    }

    /// The options of every command, keyed by command.
    fn command_option_map(&self) -> Vec<(String, Vec<&'static str>)> {
        self.commands
            .iter()
            .filter(|command| !command.options.is_empty())
            .map(|command| {
                (
                    command.name.to_string(),
                    command.options.iter().map(|option| option.name).collect(),
                )
            })
            .collect()
    }
}

/// A PowerShell array literal body, e.g. `'a', 'b'`.
fn powershell_list<S: AsRef<str>>(names: &[S]) -> String {
    names
        .iter()
        .map(|name| format!("'{}'", name.as_ref()))
        .collect::<Vec<_>>()
        .join(", ")
}

/// A PowerShell hashtable literal body, e.g. `'a' = @('b')`.
fn powershell_map(entries: &[(String, Vec<&'static str>)]) -> String {
    entries
        .iter()
        .map(|(key, values)| format!("'{key}' = @({})", powershell_list(values)))
        .collect::<Vec<_>>()
        .join("; ")
}
