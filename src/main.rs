use std::env;
mod commands;
mod error;
mod objects;

fn main() {
    let mut args = env::args().skip(1);
    if let Some(command) = args.next() {
        let args: Vec<String> = args.collect();
        let command = match command.as_str() {
            "init" => commands::init,
            "clone" => commands::clone,
            "cat-file" => commands::cat_file,
            "hash-object" => commands::hash_object,
            "ls-tree" => commands::ls_tree,
            "write-tree" => commands::write_tree,
            "commit-tree" => commands::commit_tree,
            _ => {
                eprintln!("Unknown `{}` command", command);
                std::process::exit(1);
            }
        };
        if let Err(e) = command(args) {
            eprintln!("Command error: {}", e);
            std::process::exit(1);
        };
    } else {
        eprintln!("No command provided");
        std::process::exit(1);
    };
}