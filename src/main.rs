#![feature(type_alias_impl_trait)]

use std::{collections::HashMap, fs, path::Path};

use clap::Parser;

type Cache = HashMap<String, ProjectMetadata>;

#[derive(Parser, Debug)]
enum CmdType {
    /// Recursively look for git repositories in given path
    Scan { path: String },
    /// Wrapper around git clone to check if the repo is already cloned
    Clone { url: String },
    /// Show all git repos with some metadata
    Show,
}

/// It'll have different types of commands such as scan, clone, show
#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct CliArgs {
    #[clap(subcommand)]
    cmd_type: CmdType,
}

#[derive(Debug)]
struct ProjectMetadata {
    path: String,
    author: String,
    //latest_commit: 
    //latest_modification: 
}

fn scan(path: &Path, cache: &mut Cache) {
    for e in fs::read_dir(path).unwrap() {
        let e = e.unwrap();
        if e.path().is_dir() {
            if e.path().ends_with(".git") {
                process_repo(&path, cache);
            } else {
                scan(&e.path(), cache);
            }
        }
    }
}

fn clone(_url: &str) {}

fn process_repo(path: &Path, _cache: &mut Cache) {
    println!("{path:?} is a git repo");
}

fn main() {
    let args = CliArgs::parse();

    let mut projects: Cache = HashMap::new();

    match args.cmd_type {
        CmdType::Scan { ref path } => {
            let path = Path::new(path);
            if !path.is_dir() {
                panic!("{path:?} is not a directory");
            }
            scan(path, &mut projects);
        }
        CmdType::Clone { ref url } => {
            clone(url);
        }
        CmdType::Show => {}
    }

    println!("{args:?}");
}
