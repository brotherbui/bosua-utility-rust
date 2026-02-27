use clap::Command;
use serde::Serialize;
use std::collections::HashMap;
use std::fmt;

/// Command categories matching the Go version's grouping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CommandCategory {
    Core,
    Media,
    Cloud,
    Network,
    Developer,
    System,
    Utility,
}

impl CommandCategory {
    /// Returns all category variants in display order.
    pub fn all() -> &'static [CommandCategory] {
        &[
            CommandCategory::Core,
            CommandCategory::Media,
            CommandCategory::Cloud,
            CommandCategory::Network,
            CommandCategory::Developer,
            CommandCategory::System,
            CommandCategory::Utility,
        ]
    }
}

impl fmt::Display for CommandCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommandCategory::Core => write!(f, "core"),
            CommandCategory::Media => write!(f, "media"),
            CommandCategory::Cloud => write!(f, "cloud"),
            CommandCategory::Network => write!(f, "network"),
            CommandCategory::Developer => write!(f, "developer"),
            CommandCategory::System => write!(f, "system"),
            CommandCategory::Utility => write!(f, "utility"),
        }
    }
}

/// Metadata for a registered command.
#[derive(Debug, Clone)]
pub struct CommandMeta {
    pub name: String,
    pub category: CommandCategory,
    pub description: String,
    pub aliases: Vec<String>,
    pub hidden: bool,
    pub deprecated: bool,
    pub command: Command,
}

/// Statistics about the command registry.
#[derive(Debug, Serialize)]
pub struct RegistryStats {
    pub total: usize,
    pub hidden: usize,
    pub deprecated: usize,
    pub per_category: HashMap<CommandCategory, usize>,
}

/// Centralized command registration and categorization system.
pub struct CommandRegistry {
    commands: HashMap<String, CommandMeta>,
    root: Command,
}

impl CommandRegistry {
    /// Creates a new registry with the given root command.
    pub fn new(root: Command) -> Self {
        Self {
            commands: HashMap::new(),
            root,
        }
    }

    /// Registers a command. Returns an error if a command with the same name already exists.
    pub fn register(&mut self, meta: CommandMeta) -> crate::errors::Result<()> {
        if self.commands.contains_key(&meta.name) {
            return Err(crate::errors::BosuaError::Command(format!(
                "duplicate command name: {}",
                meta.name
            )));
        }
        self.commands.insert(meta.name.clone(), meta);
        Ok(())
    }

    /// Returns all commands in the given category.
    pub fn get_by_category(&self, cat: CommandCategory) -> Vec<&CommandMeta> {
        self.commands
            .values()
            .filter(|m| m.category == cat)
            .collect()
    }

    /// Prints a human-readable listing of commands grouped by category.
    pub fn list_commands(&self) {
        for cat in CommandCategory::all() {
            let cmds = self.get_by_category(*cat);
            if cmds.is_empty() {
                continue;
            }
            println!("\n{}:", cat);
            let mut sorted: Vec<_> = cmds;
            sorted.sort_by(|a, b| a.name.cmp(&b.name));
            for cmd in sorted {
                let suffix = if cmd.deprecated {
                    " [deprecated]"
                } else if cmd.hidden {
                    " [hidden]"
                } else {
                    ""
                };
                println!("  {:<20} {}{}", cmd.name, cmd.description, suffix);
            }
        }
    }

    /// Prints a JSON listing of commands grouped by category.
    pub fn list_commands_json(&self) {
        let mut output: HashMap<String, Vec<CommandJsonEntry>> = HashMap::new();
        for cat in CommandCategory::all() {
            let cmds = self.get_by_category(*cat);
            if cmds.is_empty() {
                continue;
            }
            let mut entries: Vec<CommandJsonEntry> = cmds
                .iter()
                .map(|m| CommandJsonEntry {
                    name: m.name.clone(),
                    description: m.description.clone(),
                    aliases: m.aliases.clone(),
                    hidden: m.hidden,
                    deprecated: m.deprecated,
                })
                .collect();
            entries.sort_by(|a, b| a.name.cmp(&b.name));
            output.insert(cat.to_string(), entries);
        }
        if let Ok(json) = serde_json::to_string_pretty(&output) {
            println!("{}", json);
        }
    }

    /// Returns statistics about the registry.
    pub fn stats(&self) -> RegistryStats {
        let mut per_category: HashMap<CommandCategory, usize> = HashMap::new();
        let mut hidden = 0;
        let mut deprecated = 0;

        for meta in self.commands.values() {
            *per_category.entry(meta.category).or_insert(0) += 1;
            if meta.hidden {
                hidden += 1;
            }
            if meta.deprecated {
                deprecated += 1;
            }
        }

        RegistryStats {
            total: self.commands.len(),
            hidden,
            deprecated,
            per_category,
        }
    }

    /// Validates the registry and returns a list of issues found.
    ///
    /// Checks for:
    /// - Duplicate names (prevented at registration, but checked for completeness)
    /// - Commands without a category (all commands have one by construction)
    /// - Empty categories (categories with zero commands)
    pub fn validate(&self) -> Vec<String> {
        let mut issues = Vec::new();

        // Check for duplicate names (track via a separate set)
        let mut seen = std::collections::HashSet::new();
        for name in self.commands.keys() {
            if !seen.insert(name.clone()) {
                issues.push(format!("duplicate command name: {}", name));
            }
        }

        // Check for empty categories
        for cat in CommandCategory::all() {
            if self.get_by_category(*cat).is_empty() {
                issues.push(format!("empty category: {}", cat));
            }
        }

        issues
    }

    /// Consumes the registry and builds the final clap Command with all subcommands attached.
    pub fn build_root(self) -> Command {
        let mut root = self.root;
        for (_, meta) in self.commands {
            root = root.subcommand(meta.command);
        }
        root
    }

    /// Returns the number of registered commands.
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Returns a sorted list of all registered command names.
    pub fn command_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.commands.keys().cloned().collect();
        names.sort();
        names
    }

    /// Returns true if no commands are registered.
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

/// JSON-serializable entry for command listing.
#[derive(Debug, Serialize)]
struct CommandJsonEntry {
    name: String,
    description: String,
    aliases: Vec<String>,
    hidden: bool,
    deprecated: bool,
}

/// Builder for constructing `CommandMeta` instances.
pub struct CommandBuilder {
    name: String,
    category: Option<CommandCategory>,
    description: String,
    aliases: Vec<String>,
    hidden: bool,
    deprecated: bool,
    command: Option<Command>,
}

impl CommandBuilder {
    /// Creates a new builder with the given command name.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            category: None,
            description: String::new(),
            aliases: Vec::new(),
            hidden: false,
            deprecated: false,
            command: None,
        }
    }

    /// Creates a builder from an existing clap Command, extracting name and description.
    pub fn from_clap(cmd: Command) -> Self {
        let name = cmd.get_name().to_string();
        let description = cmd
            .get_about()
            .map(|s| s.to_string())
            .unwrap_or_default();
        Self {
            name,
            category: None,
            description,
            aliases: Vec::new(),
            hidden: false,
            deprecated: false,
            command: Some(cmd),
        }
    }

    /// Sets the command category.
    pub fn category(mut self, cat: CommandCategory) -> Self {
        self.category = Some(cat);
        self
    }

    /// Sets the command description.
    pub fn description(mut self, desc: &str) -> Self {
        self.description = desc.to_string();
        self
    }

    /// Sets the command aliases.
    pub fn aliases(mut self, aliases: &[&str]) -> Self {
        self.aliases = aliases.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Marks the command as hidden.
    pub fn hidden(mut self) -> Self {
        self.hidden = true;
        self
    }

    /// Marks the command as deprecated.
    pub fn deprecated(mut self) -> Self {
        self.deprecated = true;
        self
    }

    /// Builds the `CommandMeta`. Defaults category to `Utility` if not set.
    /// Creates a basic clap Command if none was provided via `from_clap`.
    pub fn build(self) -> CommandMeta {
        let category = self.category.unwrap_or(CommandCategory::Utility);
        let command = self.command.unwrap_or_else(|| {
            let mut cmd = Command::new(self.name.clone());
            if !self.description.is_empty() {
                cmd = cmd.about(self.description.clone());
            }
            cmd
        });

        CommandMeta {
            name: self.name,
            category,
            description: self.description,
            aliases: self.aliases,
            hidden: self.hidden,
            deprecated: self.deprecated,
            command,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_root() -> Command {
        Command::new("bosua")
    }

    fn make_meta(name: &str, cat: CommandCategory) -> CommandMeta {
        CommandBuilder::new(name)
            .category(cat)
            .description(&format!("{} command", name))
            .build()
    }

    #[test]
    fn test_register_and_lookup() {
        let mut reg = CommandRegistry::new(make_root());
        reg.register(make_meta("version", CommandCategory::Core))
            .unwrap();
        reg.register(make_meta("info", CommandCategory::Core))
            .unwrap();
        reg.register(make_meta("play", CommandCategory::Media))
            .unwrap();

        assert_eq!(reg.len(), 3);

        let core = reg.get_by_category(CommandCategory::Core);
        assert_eq!(core.len(), 2);

        let media = reg.get_by_category(CommandCategory::Media);
        assert_eq!(media.len(), 1);
        assert_eq!(media[0].name, "play");
    }

    #[test]
    fn test_duplicate_detection() {
        let mut reg = CommandRegistry::new(make_root());
        reg.register(make_meta("version", CommandCategory::Core))
            .unwrap();
        let result = reg.register(make_meta("version", CommandCategory::Core));
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("duplicate"));
    }

    #[test]
    fn test_stats() {
        let mut reg = CommandRegistry::new(make_root());
        reg.register(make_meta("version", CommandCategory::Core))
            .unwrap();
        reg.register(make_meta("info", CommandCategory::Core))
            .unwrap();

        let hidden_meta = CommandBuilder::new("secret")
            .category(CommandCategory::System)
            .hidden()
            .build();
        reg.register(hidden_meta).unwrap();

        let dep_meta = CommandBuilder::new("old-cmd")
            .category(CommandCategory::Utility)
            .deprecated()
            .build();
        reg.register(dep_meta).unwrap();

        let stats = reg.stats();
        assert_eq!(stats.total, 4);
        assert_eq!(stats.hidden, 1);
        assert_eq!(stats.deprecated, 1);
        assert_eq!(*stats.per_category.get(&CommandCategory::Core).unwrap(), 2);
        assert_eq!(
            *stats.per_category.get(&CommandCategory::System).unwrap(),
            1
        );
    }

    #[test]
    fn test_validate_empty_categories() {
        let mut reg = CommandRegistry::new(make_root());
        // Only register in Core — all other categories are empty
        reg.register(make_meta("version", CommandCategory::Core))
            .unwrap();

        let issues = reg.validate();
        // Should report empty categories for all except Core
        assert!(issues.iter().any(|i| i.contains("empty category: media")));
        assert!(issues.iter().any(|i| i.contains("empty category: cloud")));
        assert!(!issues.iter().any(|i| i.contains("empty category: core")));
    }

    #[test]
    fn test_category_grouping_union() {
        let mut reg = CommandRegistry::new(make_root());
        for cat in CommandCategory::all() {
            reg.register(make_meta(&format!("cmd-{}", cat), *cat))
                .unwrap();
        }

        // Union of all categories should equal total
        let total: usize = CommandCategory::all()
            .iter()
            .map(|c| reg.get_by_category(*c).len())
            .sum();
        assert_eq!(total, reg.len());
    }

    #[test]
    fn test_builder_from_clap() {
        let cmd = Command::new("download").about("Download files");
        let meta = CommandBuilder::from_clap(cmd)
            .category(CommandCategory::Core)
            .aliases(&["dl", "d"])
            .build();

        assert_eq!(meta.name, "download");
        assert_eq!(meta.description, "Download files");
        assert_eq!(meta.aliases, vec!["dl", "d"]);
        assert_eq!(meta.category, CommandCategory::Core);
        assert!(!meta.hidden);
        assert!(!meta.deprecated);
    }

    #[test]
    fn test_builder_defaults() {
        let meta = CommandBuilder::new("test-cmd").build();
        assert_eq!(meta.name, "test-cmd");
        assert_eq!(meta.category, CommandCategory::Utility); // default
        assert!(meta.description.is_empty());
        assert!(meta.aliases.is_empty());
        assert!(!meta.hidden);
        assert!(!meta.deprecated);
    }

    #[test]
    fn test_build_root_attaches_subcommands() {
        let mut reg = CommandRegistry::new(make_root());
        reg.register(make_meta("version", CommandCategory::Core))
            .unwrap();
        reg.register(make_meta("info", CommandCategory::Core))
            .unwrap();

        let root = reg.build_root();
        let sub_names: Vec<_> = root.get_subcommands().map(|c| c.get_name().to_string()).collect();
        assert!(sub_names.contains(&"version".to_string()));
        assert!(sub_names.contains(&"info".to_string()));
    }
}
