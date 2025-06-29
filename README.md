# epithet

Advanced alias management.

## Getting started

Download the [epithet binary](https://github.com/dbradf/epithet/releases). And
place it in your `$PATH`.

Add an `epithet.toml` file to your `XDG_CONFIG_HOME`:

- **Linux** `$XDG_CONFIG_HOME` or `~/.config`
- **macOS** `~/Library/Application Support`

Add configuration for the aliases you want to define. See Features below for
examples.

Run `epithet install` to configure your defined aliases.

Be sure to include the epithet binary direction in your path.

```bash
export PATH="$PATH:$HOME/.local/epithet/bin"
```

## Features

### Aliases with subcommands

Define an alias with subcommands (a la git):

```toml
[c]
sub_aliases = [
    { name = "b", command = "cargo build" },
    { name = "f", command = "cargo fmt" },
    { name = "r", command = "cargo build --release" },
]
```

### Expansions

Define expansions that can be specified with `@`. These can be defined globally
or for a specific alias.

```toml
[global_expansions]
proj = "~/code/projects"
docs = "~/Documents"
...

[z]
command = "cd"
expansions = [
    { key = "s", value = "/tmp/scratch" }
]
```

### Pass parameter in specific locations

You can use `{}` in alias definitions to reference parameters when the alias is
called.

```toml
[t]
sub_aliases = [
    { name = "a", command = "test all" },
    { name = "1", command = "test {0}" }
]
```

### Create aliases of multiple commands

You can use `and` or `or` to create aliases that run multiple commands.

```toml
[b]
sub_aliases = [
    { 
        name = "p1", 
        and = [ 
            "build deps",
            "build p1"
       ] 
    },
    { 
        name = "either", 
        or = [ 
            "build project",
            "report build failure"
       ] 
    },
]
```
