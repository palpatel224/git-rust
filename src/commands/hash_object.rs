use crate::error::GitError;
use crate::objects::GitObject;
pub fn hash_object(args: Vec<String>) -> Result<(), GitError> {
    let filename = args
        .last()
        .ok_or(GitError::any("missing filename to hash"))?;
    let write = args.contains(&String::from("-w"));
    let git_object = GitObject::from_path(filename, write)?;
    println!("{}", git_object.hex_string());
    Ok(())
}