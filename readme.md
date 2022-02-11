# pplaces
`pplaces` is a tool to help manage local git repositories.
# Usage
```
pplaces 0.1.0
pplaces helps you manage local git repositories

USAGE:
    pplaces [OPTIONS] <SUBCOMMAND>

OPTIONS:
    -d, --days-to-show <DAYS_TO_SHOW>    Only show repos with a commit in the last N days
    -f, --full                           Show full debug data
    -h, --help                           Print help information
    -V, --version                        Print version information

SUBCOMMANDS:
    clone     Wrapper around git clone to check if the repo is already cloned
    help      Print this message or the help of the given subcommand(s)
    scan      Recursively look for git repositories in given path
    show      Show all git repos with some metadata
    upload    Upload repo to github
```
