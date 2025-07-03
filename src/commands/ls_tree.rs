use crate::error::GitError;
use crate::objects::GitObject;
pub fn ls_tree(args: Vec<String>) -> Result<(), GitError> {
    let object_hash = args.last().ok_or(GitError::any("missing tree hash"))?;
    if object_hash.len() != 40 {
        return Err(GitError::any("invalid object hash"));
    };
    let name_only = args.contains(&String::from("--name-only"));
    let git_object = GitObject::from_hex_string(object_hash)?;
    let tree_entries = git_object.tree_entries()?;
    for e in tree_entries {
        if name_only {
            println!("{}", e.filename());
        } else {
            println!(
                "{:0>6} {} {}\t{}",
                e.mode(),
                e.kind(),
                e.hex_string(),
                e.filename()
            );
        };
    }
    Ok(())
}