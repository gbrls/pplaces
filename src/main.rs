#![feature(type_alias_impl_trait, exit_status_error)]

use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use chrono::{DateTime, Duration, Local, NaiveDate, NaiveDateTime, TimeZone, Utc};
use clap::Parser;

use anyhow::{Context, Result};

type Cache = Vec<ProjectMetadata>;

#[derive(Serialize, Deserialize, Debug, Parser)]
enum CmdType {
    /// Recursively look for git repositories in given path
    Scan { path: String },
    /// Wrapper around git clone to check if the repo is already cloned
    Clone { args: Vec<String> },
    /// Show all git repos with some metadata
    Show,
    /// Upload repo to github
    Upload,
}

/// pplaces helps you manage local git repositories
#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct CliArgs {
    #[clap(subcommand)]
    cmd_type: CmdType,

    /// Only show repos with a commit in the last N days
    #[clap(short, long)]
    days_to_show: Option<u32>,

    /// Show full debug data
    #[clap(short, long)]
    full: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct ProjectMetadata {
    path: String,
    upstream: Vec<String>,
    latest_commit: Option<NaiveDateTime>,
    //latest_modification:
}

struct Config {
    paths: bool,
}

fn scan(path: &Path, cache: &mut Cache) {
    for e in fs::read_dir(path).unwrap() {
        let e = e.unwrap();
        if e.path().is_dir() {
            if e.path().ends_with(".git") {
                update_repo_data(&path, cache);
            } else {
                scan(&e.path(), cache);
            }
        }
    }
}

fn clone(args: &Vec<String>, data: &Cache) {
    let url = args
        .iter()
        .find(|s| s.starts_with("http") || s.starts_with("git@"))
        .expect("No url given");

    let user_and_repo_name = get_url_ending(url);

    let repo_matches = data.iter().find(|e| {
        e.upstream
            .iter()
            .any(|url| get_url_ending(url) == user_and_repo_name)
    });

    match repo_matches {
        Some(entry) => println!("{} already exists in\n{}", url, entry.path),
        None => {
            let output = Command::new("git")
                .arg("clone")
                .args(args)
                .output()
                .expect("Failed to run command");

            let stderr = String::from_utf8(output.stderr).unwrap();
            print!("{stderr}");
        }
    }
}

/// This is O(n)
fn update_repo_data(path: &Path, cache: &mut Cache) {
    // We assume that there won't be repetition, so a Vec is just fine.
    let data = fetch_metadata(path).unwrap();

    let idx = cache.iter().enumerate().find(|(_, e)| e.path == data.path);

    if let Some((i, _)) = idx {
        cache.swap_remove(i);
    }

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
    let mut data = match get_cache_from_disk() {
        Ok(cache) => cache,
        Err(_) => Vec::new(),
    };

    scan(path, &mut data);
    data.sort_by_key(|d| d.latest_commit);
    data.reverse();

    data
}

fn config_dir() -> Option<PathBuf> {
    if let Some(config_dir) = dirs::config_dir() {
        let config_dir = config_dir.join("pplaces");
        Some(config_dir)
    } else {
        None
    }
}

fn save_cache_to_disk(cache: &Cache) {
    if let Some(config_dir) = config_dir() {
        fs::create_dir_all(&config_dir).unwrap();

        // this is written as a JSON because it's easier to interface with web technologies
        let str = serde_json::to_string(cache).unwrap();
        fs::write(config_dir.join(".cache.json"), &str).unwrap();
    }

    // We don't have an else because it should work even without a disk cache.
}

fn get_cache_from_disk() -> Result<Cache> {
    let data_str = fs::read_to_string(config_dir().unwrap().join(".cache.json"))
        .context("Cache file not found")?;
    let data = serde_json::from_str::<Cache>(&data_str)?;

    Ok(data)
}

fn print_paths(data: &Cache) {
    for entry in data {
        println!("{}", entry.path);
    }
}

fn print_recent(data: &Cache, since: Option<Duration>, location: &Path) {
    for entry in data.iter().filter(|e| {
        if e.latest_commit.is_some() {
            if let Some(date_time) = e.latest_commit {
                let date_time: DateTime<Local> = Local
                    .from_local_datetime(&e.latest_commit.unwrap())
                    .unwrap();
                let elapsed = Local::now() - date_time;

                let loc_str = location.to_str().unwrap();
                since.is_none() || (elapsed <= since.unwrap() && e.path.starts_with(loc_str))
            } else {
                false
            }
        } else {
            false
        }
    }) {
        println!("{}", entry.path);
    }
}

fn get_url_ending(url: &str) -> String {
    let url = url.split(" ").take(1).collect::<String>();
    let url = if url.ends_with(".git") {
        url.split_once(".git").unwrap().0
    } else {
        &url
    };

    if url.starts_with("git@") {
        // SSH repo
        // git@github.com:gbrls/gdb -FunEnd.git
        let url = url.split_once(":").unwrap().1;
        url.into()
    } else if url.starts_with("http") {
        // non-ssh repo
        let url = url.split("/").skip(3).collect::<Vec<_>>();
        let url = url.join("/");
        url
    } else {
        panic!("{} is not a URL", url);
    }
}

fn upload_repo(path: &str) -> Result<()> {
    //curl -H "Authorization: token $(cat .github-personal-token)" --data '{"name":"teste-api-00"}' https://api.github.com/user/repos

    //TODO: Use a Rust http client

    let auth_token = include_str!("../../.github-personal-token");
    let auth_str = format!("\"Authorization: token {}\"", auth_token);
    let req_json = format!("\'{{\"name\": \"{}\"}}\'", "teste-api-01");
    let args = &[
        "-H",
        //"\"Authorization: token $(cat .github-personal-token)\"",
        &auth_str,
        "--data",
        &req_json,
        "https://api.github.com/user/repos",
    ];

    dbg!(args);

    let output = Command::new("curl")
        .args(args)
        .status()
        .expect("Failed to exec")
        .exit_ok()
        .expect("Command failed");

    dbg!(output);

    //git remote add origin git@github.com:USER/REPO.git
    //git push origin main
    Ok(())
}

// https://stackoverflow.com/questions/2423777/is-it-possible-to-create-a-remote-repo-on-github-from-the-cli-without-opening-br

// Issues:
// - When cloning we don't update the cache.

fn main() -> Result<()> {
    let args = CliArgs::parse();

    let days = match args.days_to_show {
        Some(n) => n,
        None => 365 * 1_000,
    };
    //let days_to_show = Duration::days(days as i64);
    let days_to_show = None;
    let full_info = args.full;

    match args.cmd_type {
        CmdType::Scan { ref path } => {
            let path = Path::new(path);
            if !path.is_dir() {
                panic!("{path:?} is not a directory");
            }
            // This might be slow in some machines
            let data = build_cache(path);
            save_cache_to_disk(&data);
            print_recent(&data, days_to_show, path);
        }
        CmdType::Clone { ref args } => {
            let data = get_cache_from_disk()?;
            clone(args, &data);
        }
        CmdType::Show => {
            let data = get_cache_from_disk()?;
            if full_info {
                println!("{data:#?}")
            } else {
                print_recent(&data, days_to_show, Path::new("/"));
            }
        }

        CmdType::Upload => {
            //let path = working_directory();
            let path = ".";

            let is_repo = fs::read_dir(path)?.any(|e| e.unwrap().path().ends_with(".git"));

            upload_repo(path)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_parser() {
        let a = "https://github.com/linebender/runebender (fetch)";
        let b = "git@github.com:gbrls/Bootloader.git (fetch)";

        assert_eq!(get_url_ending(a), "linebender/runebender");
        assert_eq!(get_url_ending(b), "gbrls/Bootloader");
    }
}
