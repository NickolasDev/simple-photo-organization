use clap::{App, Arg};
use console::style;
use exif::{DateTime, In, Reader, Tag, Value};
use indicatif::{ProgressBar};
use rayon::prelude::*;
use std::fs::create_dir_all;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf, MAIN_SEPARATOR};
use walkdir::{WalkDir};

fn main() {
    let matches = App::new("Simple Photo Organization")
        .version("0.8")
        .about("Organization of photos for storage")
        .arg(
            Arg::with_name("SOURCE")
                .short("s")
                .help("Source directory")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name("TARGET")
                .short("t")
                .help("Target directory")
                .required(true)
                .index(2),
        )
        .get_matches();

    let source_dir = matches.value_of("SOURCE").unwrap();
    let target_dir = matches.value_of("TARGET").unwrap();

    let source = Path::new(source_dir);
    let target = Path::new(target_dir);

    if !source.exists() && !source.is_dir() {
        panic!("Invalid source directory");
    }

    let source_is_blank = source
        .read_dir()
        .map(|mut i| i.next().is_none())
        .unwrap_or(false);
    if source_is_blank {
        panic!("Source directory is blank");
    }

    if target.exists() {
        let target_is_blank = target
            .read_dir()
            .map(|mut i| i.next().is_none())
            .unwrap_or(false);

        if !target_is_blank {
            panic!("Target directory is not blank");
        }
    } else {
        create_dir_all(target)
            .expect("It is not possible to create the target directory, check the permissions");
    }

    println!("{}", style("Please wait, indexing files...").cyan());

    let walkdir = WalkDir::new(source).follow_links(true);

    let files: Vec<PathBuf> = walkdir
        .into_iter()
        .par_bridge()
        .map(|file| file.unwrap().into_path())
        .filter(|item| item.is_file())
        .collect();

    let total_files: u64 = files.iter().count() as u64;

    let message_processing_files = format!("{}", style("Please wait, processing files...").cyan());

    let bar = ProgressBar::new(total_files).with_message(message_processing_files.clone());

    println!("{}", message_processing_files);

    files.par_iter().for_each(|file| {
        let date_time = get_date_time_from_file(file);
        let file_name = file.file_name().unwrap().to_str().unwrap();

        if let Some(date_time_file) = date_time {
            let mut file_target_path = PathBuf::from(target);
            file_target_path.push(format!(
                "{}{}{}{}{}",
                date_time_file.year,
                MAIN_SEPARATOR,
                date_time_file.month,
                MAIN_SEPARATOR,
                date_time_file.day
            ));

            std::fs::create_dir_all(file_target_path.clone()).expect("Error in create dir");

            let mut extendion = String::new();

            if let Some(extension) = file.extension() {
                extendion.push('.');
                extendion.push_str(extension.to_str().unwrap());
            }

            let mut new_file_name = format!(
                "{}_{}_{}_{}_{}_{}",
                date_time_file.year,
                date_time_file.month,
                date_time_file.day,
                date_time_file.hour,
                date_time_file.minute,
                date_time_file.second
            );

            new_file_name = change_name_if_exist(&new_file_name, &file_target_path, &extendion, 0);

            file_target_path.push(format!("{}{}", new_file_name, extendion));

            std::fs::copy(file, &file_target_path).expect("Copy error");
        } else {
            let mut path_to_new_file = file
                .parent()
                .unwrap()
                .to_str()
                .unwrap()
                .replace(source.to_str().unwrap(), "");
            if path_to_new_file.starts_with(MAIN_SEPARATOR) {
                path_to_new_file = path_to_new_file[1..path_to_new_file.len()].to_string();
            }

            if path_to_new_file.ends_with(MAIN_SEPARATOR) {
                path_to_new_file = path_to_new_file[0..path_to_new_file.len() - 1].to_string();
            }
            let mut path = PathBuf::from(target);
            path.push("other");
            path.push(path_to_new_file);
            path.push(file_name);
            std::fs::create_dir_all(path.parent().expect("Error in parent")).expect("Error in create dir");

            std::fs::copy(file, path).expect("Copy error");
        }
        bar.inc(1);
    });

    bar.finish();
}

fn change_name_if_exist(
    name: &String,
    path: &PathBuf,
    extension: &String,
    last_item: u16,
) -> String {
    let mut new_file_name = String::from(name);

    if last_item != 0 {
        new_file_name = format!("{}{}", new_file_name, last_item);
    }
    let mut new_path = PathBuf::from(path);
    new_path.push(format!("{}{}", &new_file_name, &extension).as_str());

    if new_path.exists() {
        let last_item = last_item + 1;
        new_file_name = change_name_if_exist(name, path, extension, last_item)
    }
    String::from(new_file_name)
}

fn get_date_time_from_file(path: &PathBuf) -> Option<DateTime> {
    let mut date_time_file: Option<DateTime> = None;
    let file = File::open(&path).expect("It is not possible open file, check the permissions");
    let data = Reader::new().read_from_container(&mut BufReader::new(&file));

    if let Ok(exif) = data {
        if let Some(field) = exif.get_field(Tag::DateTimeOriginal, In::PRIMARY) {
            if let Value::Ascii(ref vec) = field.value {
                if !vec.is_empty() {
                    if let Ok(datetime) = DateTime::from_ascii(&vec[0]) {
                        date_time_file = Some(datetime);
                    }
                }
            }
        }
    }
    date_time_file
}
