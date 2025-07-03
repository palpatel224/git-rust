use crate::error::GitError;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use sha1::Digest;
use sha1::Sha1;
use std::fmt::Write as _;
use std::fs;
use std::io::Cursor;
use std::io::{BufRead, Read, Write as _};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
#[derive(Clone, Debug)]
pub enum Kind {
    Blob,
    Tree,
    Commit,
}
impl std::fmt::Display for Kind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Blob => write!(f, "blob"),
            Self::Tree => write!(f, "tree"),
            Self::Commit => write!(f, "commit"),
        }
    }
}
#[derive(Debug)]
pub struct TreeEntry {
    filename: String,
    kind: Kind,
    hash: Vec<u8>,
    mode: String,
}
impl TreeEntry {
    pub fn filename(&self) -> &String {
        &self.filename
    }
    pub fn kind(&self) -> &Kind {
        &self.kind
    }
    pub fn hex_string(&self) -> String {
        hex::encode(&self.hash)
    }
    pub fn mode(&self) -> &String {
        &self.mode
    }
}
pub struct GitObject {
    kind: Kind,
    header: Vec<u8>,
    contents: Vec<u8>,
    hash: Vec<u8>,
}
impl GitObject {
    fn objects_dir_path() -> PathBuf {
        [".git", "objects"].iter().collect::<PathBuf>()
    }
    fn committer() -> String {
        "Code Crafters <000000000+codecrafters@users.noreply.github.com> 1750973235 +0000".into()
    }
    pub fn build(kind: Kind, contents: Vec<u8>) -> Result<Self, GitError> {
        let header = {
            let mut buf = Vec::new();
            match kind {
                Kind::Blob => buf.write_all(b"blob ")?,
                Kind::Tree => buf.write_all(b"tree ")?,
                Kind::Commit => buf.write_all(b"commit ")?,
            };
            buf.write_all(contents.len().to_string().as_bytes())?;
            buf.write_all(b"\0")?;
            buf
        };
        let hash = {
            let mut buf = Vec::new();
            buf.write_all(&header)?;
            buf.write_all(&contents)?;
            Sha1::digest(&buf).to_vec()
        };
        Ok(Self {
            kind,
            header,
            contents,
            hash,
        })
    }
    pub fn build_commit(
        msg: impl AsRef<str>,
        tree_hash: impl AsRef<str>,
        parent_hash: Option<impl AsRef<str>>,
    ) -> Result<Self, GitError> {
        let mut contents = String::new();
        let committer = GitObject::committer();
        writeln!(contents, "tree {}", tree_hash.as_ref())?;
        if let Some(parent_hash) = parent_hash {
            writeln!(contents, "parent {}", parent_hash.as_ref())?;
        }
        writeln!(contents, "author {committer}")?;
        writeln!(contents, "committer {committer}")?;
        writeln!(contents)?;
        writeln!(contents, "{}", msg.as_ref())?;
        Self::build(Kind::Commit, contents.into())
    }
    pub fn from_hex_string(hex_string: impl AsRef<str>) -> Result<Self, GitError> {
        let hash = hex::decode(hex_string.as_ref())?;
        let (prefix, filename) = hex_string.as_ref().split_at(2);
        let path = Self::objects_dir_path().join(prefix).join(filename);
        let compressed_contents = std::fs::read(path)?;
        let mut reader = ZlibDecoder::new(compressed_contents.as_slice());
        let mut git_object = Vec::new();
        reader.read_to_end(&mut git_object)?;
        let mut git_object_parts = git_object.splitn(2, |&b| b == b'\0');
        let header = git_object_parts
            .next()
            .ok_or(GitError::invalid_object_format(
                "invalid git object format: cannot parse header",
            ))?;
        let mut header_parts = header.splitn(2, |&b| b == b' ');
        let kind = header_parts.next().ok_or(GitError::any(
            "invalid git object format: cannot parse kind",
        ))?;
        let kind = match kind {
            b"blob" => Kind::Blob,
            b"tree" => Kind::Tree,
            b"commit" => Kind::Commit,
            _ => {
                return Err(GitError::any(format!(
                    "unknown git object kind: {}",
                    String::from_utf8_lossy(kind)
                )));
            }
        };
        let size = header_parts
            .next()
            .and_then(|part| std::str::from_utf8(part).ok()?.parse::<usize>().ok())
            .ok_or(GitError::any(
                "invalid git object format: cannot parse content size",
            ))?;
        let contents: Vec<u8> = git_object_parts
            .next()
            .map(|part| &part[..size])
            .ok_or(GitError::any(String::from(
                "invalid git object format: cannot parse content",
            )))?
            .to_owned();
        let mut header = header.to_owned();
        header.push(b'\0');
        Ok(Self {
            kind,
            header,
            contents,
            hash,
        })
    }
    pub fn from_path(path: impl AsRef<Path>, write: bool) -> Result<Self, GitError> {
        let path = path.as_ref();
        if path.is_file() {
            let kind = Kind::Blob;
            let contents = fs::read(path)?;
            let git_object = Self::build(kind, contents)?;
            if write {
                git_object.write()?;
            }
            return Ok(git_object);
        }
        if path.is_dir() {
            let ignored = [".git"];
            let mut entries: Vec<_> = fs::read_dir(path)?
                .filter_map(|entry| {
                    let entry = entry.ok()?;
                    let filename: String = entry.file_name().into_string().ok()?;
                    if ignored.contains(&filename.as_str()) {
                        return None;
                    };
                    let meta = entry.metadata().ok()?;
                    let mode: String = if meta.is_dir() {
                        "40000".into()
                    } else if meta.is_symlink() {
                        "120000".into()
                    } else if meta.is_file() {
                        if (meta.permissions().mode() & 0o100) != 0 {
                            "100755".into()
                        } else {
                            "100644".into()
                        }
                    } else {
                        return None;
                    };
                    Some((filename, mode))
                })
                .collect();
            entries.sort_by(|e1, e2| e1.0.cmp(&e2.0));
            let mut contents = Vec::new();
            for (filename, mode) in entries {
                contents.write_all(mode.as_bytes())?;
                contents.write_all(b" ")?;
                contents.write_all(filename.as_bytes())?;
                contents.write_all(b"\0")?;
                let git_object = Self::from_path(path.join(filename), write)?;
                contents.write_all(&git_object.hash)?;
            }
            let git_object = Self::build(Kind::Tree, contents)?;
            if write {
                git_object.write()?;
            }
            return Ok(git_object);
        }
        Err(GitError::any(format!(
            "Unsupported filesystem object at path: {}",
            path.to_string_lossy()
        )))
    }
    pub fn write(&self) -> Result<(), GitError> {
        let mut zlib_encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        zlib_encoder.write_all(&self.header)?;
        zlib_encoder.write_all(&self.contents)?;
        let compressed_contents = zlib_encoder.finish()?;
        let hex_string = hex::encode(&self.hash);
        let (prefix, filename) = &hex_string.split_at(2);
        let dir = Self::objects_dir_path().join(prefix);
        if !dir.exists() {
            fs::create_dir(&dir)?;
        };
        let path = dir.join(filename);
        fs::write(&path, &compressed_contents)?;
        Ok(())
    }
    pub fn kind(&self) -> &Kind {
        &self.kind
    }
    pub fn contents(&self) -> &Vec<u8> {
        &self.contents
    }
    pub fn hex_string(&self) -> String {
        hex::encode(&self.hash)
    }
    pub fn tree_entries(&self) -> Result<impl IntoIterator<Item = TreeEntry>, GitError> {
        match self.kind {
            Kind::Tree => {
                let mut reader = Cursor::new(&self.contents);
                let mut entries = Vec::new();
                loop {
                    let mut mode = Vec::new();
                    let mut filename = Vec::new();
                    let mut hash = vec![0; 20];
                    reader.read_until(b' ', &mut mode)?;
                    reader.read_until(b'\0', &mut filename)?;
                    reader.read_exact(&mut hash)?;
                    filename.pop();
                    let filename = String::from_utf8(filename)?;
                    mode.pop();
                    let mode = String::from_utf8(mode)?;
                    let kind = match mode.as_str() {
                        "100644" | "100755" | "120000" => Kind::Blob,
                        "40000" => Kind::Tree,
                        _ => return Err(GitError::any("unrecognized tree entry")),
                    };
                    entries.push(TreeEntry {
                        filename,
                        kind,
                        hash,
                        mode,
                    });
                    if reader.position() as usize == self.contents.len() {
                        break;
                    };
                }
                Ok(entries)
            }
            _ => Err(GitError::any("not a tree object")),
        }
    }
    pub fn restore(&self, path: impl AsRef<Path>) -> Result<(), GitError> {
        match self.kind {
            Kind::Commit => {
                let Some(Ok(tree_line)) = self.contents.lines().next() else {
                    return Err(GitError::any("Cannot parse tree rev from commit object"));
                };
                let Some((_, tree_rev)) = tree_line.split_once(' ') else {
                    return Err(GitError::any("Cannot parse tree rev from commit object"));
                };
                let tree_object = GitObject::from_hex_string(tree_rev)?;
                tree_object.restore(path)?;
            }
            Kind::Tree => {
                for entry in self.tree_entries()? {
                    let tree_path = path.as_ref();
                    if !tree_path.exists() {
                        fs::create_dir(tree_path)?;
                    }
                    let entry_path = PathBuf::from(tree_path).join(entry.filename());
                    let entry_object = GitObject::from_hex_string(entry.hex_string())?;
                    entry_object.restore(&entry_path)?;
                }
            }
            Kind::Blob => {
                fs::write(path, &self.contents)?;
            }
        }
        Ok(())
    }
}
