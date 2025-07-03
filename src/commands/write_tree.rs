use crate::error::GitError;
use crate::objects::GitObject;
pub fn write_tree(_args: Vec<String>) -> Result<(), GitError> {
    let git_object = GitObject::from_path(".", true)?;
    println!("{}", git_object.hex_string());
    Ok(())
}