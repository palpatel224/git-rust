use crate::error::GitError;
use crate::objects::{GitObject, Kind};
pub fn cat_file(args: Vec<String>) -> Result<(), GitError> {
    let hex_string = args.last().ok_or(GitError::any("missing object hash"))?;
    if hex_string.len() != 40 {
        return Err(GitError::any("invalid object hash"));
    };
    let git_object = GitObject::from_hex_string(hex_string)?;
    match git_object.kind() {
        Kind::Blob => {
            let content = std::str::from_utf8(git_object.contents())?;
            print!("{content}");
            Ok(())
        }
        kind => Err(GitError::any(format!(
            "support for {} git object not implemented",
            kind,
        ))),
    }
}