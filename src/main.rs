use std::fs::OpenOptions;
use std::fs::{copy, create_dir_all, read_dir, read_to_string, remove_dir_all, remove_file};
use std::io::{Result, Write};
use std::path::Path;
use std::path::PathBuf;

const K_STORE_PATH: &str = ".clip_store";

fn get_store_path() -> PathBuf {
    let home = home::home_dir().unwrap();

    home.join(K_STORE_PATH)
}

fn expect_empty_arguments(arguments: &[String]) {
    expect_n_arguments(arguments, 0)
}

fn expect_n_arguments(arguments: &[String], n: usize) {
    if arguments.len() != n {
        panic!(
            "Subcommand {} does not take further arguments",
            K_ERASE_COMMAND
        );
    }
}

const K_CLIP_COMMAND: &str = "c";
fn clip_command(arguments: &[String]) {
    let store_path = get_store_path();
    let store_display = store_path.display();

    let mut file = match OpenOptions::new()
        .append(true)
        .write(true)
        .create(true)
        .open(&store_path)
    {
        Err(why) => panic!("Couldn't open {}: {}", store_display, why),
        Ok(file) => file,
    };

    for arg in arguments {
        let argument_path = Path::new(arg);
        if !(argument_path).exists() {
            println!("Skipped argument {}; path does not exist", arg);
            continue;
        }
        let absolute_path = argument_path.canonicalize().unwrap();
        let absolute_path_display = absolute_path.display();

        if let Err(e) = writeln!(file, "{}", absolute_path_display) {
            panic!("Could add path {} to {}. Reason: {}", arg, store_display, e);
        }
    }
}

const K_LIST_COMMAND : &str = "l";
fn list_command(arguments: &[String]) {
    expect_empty_arguments(arguments);
    let store_path = get_store_path();

    for line in read_to_string(store_path).unwrap().lines() {
        println!("{}", line);
    }
}

const K_ERASE_COMMAND: &str = "e";
fn erase_command(arguments: &[String]) {
    expect_empty_arguments(arguments);

    let store_path = get_store_path();
    if store_path.exists() {
        match std::fs::remove_file(store_path) {
            Ok(_) => (),
            Err(err) => panic!("Could not clear store path: {}", err),
        }
    }
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<u64> {
    create_dir_all(&dst)?;
    if !src.is_dir() {
        let target_path_copy = dst.clone();
        let full_target_path = target_path_copy.join(src.file_name().unwrap());
        return copy(src, full_target_path);
    }

    let mut total_bytes = 0;
    for entry in read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            match copy_dir_all(
                entry.path().as_path(),
                dst.join(entry.file_name()).as_path(),
            ) {
                Ok(count) => total_bytes += count,
                Err(err) => {
                    return Err(err);
                }
            }
        } else {
            match copy(entry.path(), dst.join(entry.file_name())) {
                Ok(count) => total_bytes += count,
                Err(err) => {
                    return Err(err);
                }
            }
        }
    }
    Ok(total_bytes)
}

fn remove_file_or_dir(path: &Path) {
    let path_display = path.display();
    if path.is_file() {
        match remove_file(path) {
            Ok(_) => (),
            Err(err) => println!("Could not remove file {}: {}", path_display, err),
        }
    } else if path.is_dir() {
        match remove_dir_all(path) {
            Ok(_) => (),
            Err(err) => println!("Could not remove directory {}: {}", path_display, err),
        }
    }
}

const K_PASTE_COMMAND: &str = "v";
fn paste_maybe_remove(arguments: &[String], remove_afterwards: bool) {
    expect_n_arguments(arguments, 1);

    let target_path = Path::new(arguments.first().unwrap());
    let target_path_display = target_path.to_str().unwrap();
    if !target_path.is_dir() {
        panic!("Could not paste clipped files: {target_path_display} is not a directory.");
    }
    let absolute_target_path_buf = target_path.canonicalize().unwrap();
    let absolute_target_path = absolute_target_path_buf.as_path();

    let store_path = get_store_path();

    for line in read_to_string(store_path).unwrap().lines() {
        let source_path = Path::new(line);
        let source_path_display = source_path.display();
        if !source_path.exists() {
            println!(
                "Could not paste {}: path does not exist",
                source_path_display
            );
            continue;
        }

        match copy_dir_all(source_path, absolute_target_path) {
            Ok(_) => (),
            Err(err) => {
                println!("{}", err);
                continue;
            }
        }

        if remove_afterwards {
            remove_file_or_dir(source_path);
        }
    }
}

fn paste_command(arguments: &[String]) {
    paste_maybe_remove(arguments, false);
}

const K_MOVE_COMMAND: &str = "m";
fn move_command(arguments: &[String]) {
    paste_maybe_remove(arguments, true)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let command_index = 1;
    let arguments_index = command_index + 1;
    let command_token_opt = args.get(command_index);
    let command_token = match command_token_opt {
        Some(s) => s,
        None => panic!("Empty command string."),
    };

    let command_function = match command_token.as_str() {
        K_CLIP_COMMAND => clip_command,
        K_ERASE_COMMAND => erase_command,
        K_PASTE_COMMAND => paste_command,
        K_MOVE_COMMAND => move_command,
        K_LIST_COMMAND => list_command,
        other => panic!("Subcommand '{}' not found", other),
    };

    let arguments_slice = &args[arguments_index..];
    command_function(arguments_slice);
}
