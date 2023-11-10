use crate::utils::{make_pb, AppError, DirData, SubDirError};
use indicatif::{HumanBytes, ProgressBar, ProgressStyle};
use resolve_path::PathResolveExt;
use std::{
    ffi::OsString,
    fs::{remove_file, File},
    io::{self, BufRead},
    path::{Path, PathBuf},
    sync::mpsc,
    thread,
};
use walkdir::WalkDir;
fn scan_file(entry: &walkdir::DirEntry) -> Result<u64, AppError> {
    match entry.metadata() {
        Ok(e) => match e.is_file() && !e.is_symlink() {
            true => return Ok(e.len()),
            false => return Err(SubDirError()),
        },
        Err(e) => {
            return Err(e.into());
        }
    }
}
fn scan_dir(tx: mpsc::Sender<(OsString, Result<u64, AppError>)>, path: &Path) -> DirData {
    let mut size = 0;
    let mut completed = 0;
    let mut error = 0;
    for entry in WalkDir::new(&path) {
        if entry.is_err() {
            tx.send((
                path.as_os_str().to_os_string(),
                Err(entry.unwrap_err().into()),
            ))
            .unwrap();
            error += 1;
            continue;
        }
        let entry = entry.unwrap();
        let file_name = entry.file_name().to_os_string();
        match scan_file(&entry) {
            Ok(len) => {
                size += len;
                completed += 1;
                tx.send((file_name, Ok(len))).unwrap();
            }
            Err(e) => {
                if e != SubDirError() {
                    tx.send((file_name, Err(e.into()))).unwrap();
                    error += 1;
                }
            }
        }
        // thread::sleep(std::time::Duration::from_millis(500)); // for visualization only
    }
    drop(tx);
    DirData::from(size, completed, error)
}
fn nuke_dir(tx: mpsc::Sender<(OsString, Result<u64, AppError>)>, path: &Path) -> DirData {
    let mut size = 0;
    let mut completed = 0;
    let mut error = 0;
    for entry in WalkDir::new(&path) {
        let entry = entry.expect("Error in walking dir");
        let file_name = entry.file_name().to_os_string();
        match scan_file(&entry) {
            Ok(len) => match remove_file(entry.path()) {
                Ok(()) => {
                    size += len;
                    completed += 1;
                    tx.send((file_name, Ok(len))).unwrap();
                }
                Err(e) => {
                    tx.send((file_name, Err(e.into()))).unwrap();
                    error += 1;
                }
            },
            Err(e) => {
                if e != SubDirError() {
                    tx.send((file_name, Err(e.into()))).unwrap();
                    error += 1;
                }
            }
        }
        // thread::sleep(std::time::Duration::from_millis(500)); // for visualization only
    }
    drop(tx);
    DirData::from(size, completed, error)
}
fn threading_template(
    dir_list: &Vec<PathBuf>,
    pb: ProgressBar,
    f: &'static (dyn Fn(mpsc::Sender<(OsString, Result<u64, AppError>)>, &Path) -> DirData + Sync),
) -> DirData {
    let (tx, rx) = mpsc::channel();
    let handles: Vec<_> = dir_list
        .into_iter()
        .map(|line| {
            // it's ugly but it just works :(
            let line2 = line.clone();
            let txc = tx.clone();
            thread::spawn(move || f(txc, &line2))
        })
        .collect();
    drop(tx);
    for h in handles {
        let _ = h.join();
    }
    let mut total_size = 0;
    let mut error = 0;
    for received in rx {
        pb.inc(1);
        match received.1 {
            Ok(e) => {
                total_size += e;
                pb.set_message(format!(
                    "{} files, {}",
                    pb.position(),
                    HumanBytes(total_size).to_string()
                ));
            }
            Err(e) => {
                let tmp_pos = pb.position();
                pb.finish_and_clear();
                eprint!("Error processing '{}': {}", received.0.to_str().unwrap(), e);
                error += 1;
                pb.set_position(tmp_pos);
            }
        }
    }
    let final_pos = pb.position();
    pb.finish_with_message(format!(
        "Finished, {} total ({} files ok, {} files/dirs failed)",
        HumanBytes(total_size).to_string(),
        final_pos - error,
        error
    ));
    DirData::from(total_size, final_pos - error, error)
}
pub fn open_config(path: &Path) -> Result<Vec<PathBuf>, String> {
    match path.try_exists() {
        Ok(false) => Err(String::from(format!(
            "Config file '{}' does not exist",
            path.display()
        ))),
        Ok(true) => match File::open(&path) {
            Ok(file) => Ok(io::BufReader::new(&file)
                .lines()
                .into_iter()
                .map(|x| PathBuf::from(x.unwrap()))
                .collect()),
            Err(_) => Err(String::from(format!(
                "Could not open config '{}'",
                path.display()
            ))),
        },
        Err(e) => Err(String::from(format!(
            "Error parsing config '{}': {}",
            path.display(),
            e
        ))),
    }
}
pub fn validate(list: Vec<PathBuf>, quiet: bool) -> Result<Vec<PathBuf>, String> {
    let mut vec: Vec<PathBuf> = Vec::new();
    for line in list {
        let cur_path = &line.try_resolve().unwrap();
        let line = cur_path.to_path_buf();
        if let Ok(true) = cur_path.try_exists() {
        } else {
            if !quiet {
                println!("Path '{}' does not exist, skipping", cur_path.display());
            }
            continue;
        }
        let mut flag: bool = true;
        for i in &vec[..] {
            if i == &line {
                println!(
                    "Path '{}' is duplicated in config, only one kept",
                    &line.display()
                );
                flag = false;
                break;
            } else if i.starts_with(&line) || (&line).starts_with(i) {
                return Err(String::from(format!(
                    "Nested directories in config: '{}' and '{}'",
                    &line.display(),
                    i.display()
                )));
            }
        }
        if flag {
            vec.push(line);
        }
    }
    Ok(vec)
}
pub fn scan(dir_list: &Vec<PathBuf>, quiet: bool) -> DirData {
    let pb = make_pb(quiet);
    pb.set_style(ProgressStyle::with_template("{prefix:.green}: {wide_msg}").unwrap());
    pb.set_prefix("SCANNING");
    threading_template(dir_list, pb, &scan_dir)
}
pub fn nuke(dir_list: &Vec<PathBuf>, quiet: bool) -> DirData {
    let pb = make_pb(quiet);
    pb.set_style(ProgressStyle::with_template("{prefix:.magenta}: {wide_msg}").unwrap());
    pb.set_prefix("NUKING");
    threading_template(dir_list, pb, &nuke_dir)
}
