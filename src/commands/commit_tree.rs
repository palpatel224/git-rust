use std::collections::HashMap;
use crate::error::GitError;
use crate::objects::GitObject;
pub fn commit_tree(args: Vec<String>) -> Result<(), GitError> {
    let mut args_iter = args.iter();
    let mut args = Vec::new();
    let mut options_map = HashMap::new();
    while let Some(arg) = args_iter.next() {
        if arg.starts_with('-') {
            if let Some(value) = args_iter.next() {
                options_map.insert(arg, value);
            }
        } else {
            args.push(arg);
        }
    }
    let msg_option = String::from("-m");
    let msg = options_map
        .get(&msg_option)
        .ok_or(GitError::any("missing msg"))?;
    let tree_hash = args.pop().ok_or(GitError::any("missing tree hash"))?;
    if tree_hash.len() != 40 {
        return Err(GitError::any("invalid tree hash"));
    };
    let parent_hash_option = String::from("-p");
    let parent_hash = options_map.get(&parent_hash_option);
    let git_object = GitObject::build_commit(msg, tree_hash, parent_hash)?;
    git_object.write()?;
    println!("{}", git_object.hex_string());
    Ok(())
}