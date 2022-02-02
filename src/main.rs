#![feature(type_alias_impl_trait)]

use std::{collections::HashMap, fs, io::Write, path::Path, process::Command};

use chrono::{NaiveDate, NaiveDateTime};
use clap::Parser;

use std::io::{self};

type Cache = Vec<ProjectMetadata>;

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
    upstream: Vec<String>,
    latest_commit: Option<NaiveDateTime>,
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

fn process_repo(path: &Path, cache: &mut Cache) {
    // We assume that there won't be repetition, so a Vec is just fine.
    let data = fetch_metadata(path).unwrap();
    cache.push(data);
}

fn fetch_metadata(path: &Path) -> Option<ProjectMetadata> {
    let path_string = path.clone().join(".git").to_str().unwrap().to_owned();

    let cmd_stdout = Command::new("git")
        .args(["--git-dir", &path_string, "remote", "-v"])
        .output()
        .expect("Failed to run command")
        .stdout;

    let upstreams = String::from_utf8_lossy(&cmd_stdout)
        .to_string()
        .split('\n')
        .map(|s| {
            if !s.is_empty() {
                let s = s.split_once('\t').unwrap().1;
                s.to_owned()
            } else {
                s.to_owned()
            }
        })
        .filter(|s| !s.is_empty())
        .collect::<Vec<String>>();

    let cmd_stdout = Command::new("git")
        .args(["--git-dir", &path_string, "log", "-n", "1", "--format=%ci"])
        .output()
        .expect("Failed to run command")
        .stdout;

    let str = String::from_utf8_lossy(&cmd_stdout).to_string();

    let date = if !str.is_empty() {
        let s = str
            .split(" ")
            .map(|s| s.to_owned())
            .collect::<Vec<String>>();
        let yymmdd = s[0]
            .split("-")
            .map(|s| s.to_owned())
            .collect::<Vec<String>>();

        let y = yymmdd[0].parse::<i32>().unwrap();
        let month = yymmdd[1].parse::<u32>().unwrap();
        let d = yymmdd[2].parse::<u32>().unwrap();

        let hhmmss = s[1]
            .split(":")
            .map(|s| s.to_owned())
            .collect::<Vec<String>>();
        let h = hhmmss[0].parse::<u32>().unwrap();
        let m = hhmmss[1].parse::<u32>().unwrap();
        let s = hhmmss[2].parse::<u32>().unwrap();

        let date = NaiveDate::from_ymd(y, month, d).and_hms(h, m, s);

        Some(date)
    } else {
        None
    };

    Some(ProjectMetadata {
        path: path.to_str().unwrap().to_owned(),
        latest_commit: date,
        upstream: upstreams,
    })
}

fn build_cache(path: &Path) -> Cache {
    let mut data: Cache = Vec::new();
    scan(path, &mut data);
    data.sort_by_key(|d| d.latest_commit);

    data
}

fn main() {
    let args = CliArgs::parse();

    match args.cmd_type {
        CmdType::Scan { ref path } => {
            let path = Path::new(path);
            if !path.is_dir() {
                panic!("{path:?} is not a directory");
            }
            let data = build_cache(path);
            println!("{data:#?}");
        }
        CmdType::Clone { ref url } => {
            clone(url);
        }
        CmdType::Show => {}
    }

    println!("{args:?}");
}
