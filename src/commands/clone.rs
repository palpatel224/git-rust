use std::fs;
use std::io::{Cursor, Read};
use flate2::bufread::ZlibDecoder;
use reqwest::Url;
use reqwest::blocking::Client;
use crate::error::GitError;
use crate::objects::{GitObject, Kind};
pub fn clone(args: Vec<String>) -> Result<(), GitError> {
    let [repo_url, rest @ ..] = args.as_slice() else {
        return Err(GitError::any("repo url missing"));
    };
    let [clone_dir, ..] = rest else {
        return Err(GitError::any("clone dir missing"));
    };
    let git_client = GitClient::new();
    let head_rev = git_client.get_head_rev(repo_url)?;
    let pack_data = git_client.fetch_pack(repo_url, &head_rev)?;
    fs::create_dir(clone_dir)?;
    std::env::set_current_dir(clone_dir)?;
    fs::create_dir(".git")?;
    fs::create_dir(".git/objects")?;
    fs::create_dir(".git/refs")?;
    fs::create_dir(".git/refs/heads")?;
    fs::write(".git/HEAD", "ref: refs/heads/master\n")?;
    fs::write(".git/refs/heads/master", &head_rev)?;
    unpack(pack_data)?;
    let commit = GitObject::from_hex_string(head_rev)?;
    commit.restore(".")?;
    Ok(())
}
struct GitClient {
    inner: Client,
}
impl GitClient {
    fn new() -> Self {
        Self {
            inner: Client::new(),
        }
    }
    fn get_head_rev(&self, repo_url: impl AsRef<str>) -> Result<String, GitError> {
        let repo_url = repo_url.as_ref();
        let refs_url = Url::parse(&format!("{repo_url}.git/info/refs?service=git-upload-pack"))?;
        let response = self.inner.get(refs_url).send()?;
        let body = response.bytes()?;
        let head_rev = String::from_utf8_lossy(
            body.strip_prefix(b"001e# service=git-upload-pack\n0000")
                .and_then(|s| s.split(|&b| b == b' ').next())
                .and_then(|s| s.split_at_checked(4))
                .ok_or(GitError::any("cannot parse head ref"))?
                .1,
        );
        Ok(head_rev.into())
    }
    fn fetch_pack(
        &self,
        repo_url: impl AsRef<str>,
        rev: impl AsRef<str>,
    ) -> Result<Vec<u8>, GitError> {
        let repo_url = repo_url.as_ref();
        let rev = rev.as_ref();
        let pack_url = Url::parse(&format!("{repo_url}.git/git-upload-pack"))?;
        let response = self
            .inner
            .post(pack_url)
            .header("Content-Type", "application/x-git-upload-pack-request")
            .body(format!("0032want {rev}\n00000009done\n"))
            .send()?;
        let pack_data = response
            .bytes()?
            .strip_prefix(b"0008NAK\n")
            .ok_or(GitError::any("cannot parse pack file"))?
            .to_vec();
        Ok(pack_data)
    }
}
fn unpack(data: impl AsRef<Vec<u8>>) -> Result<(), GitError> {
    let mut reader = Cursor::new(data.as_ref());
    let mut sig = [0u8; 4];
    reader.read_exact(&mut sig)?;
    assert_eq!(sig, [b'P', b'A', b'C', b'K']);
    let mut version = [0u8; 4];
    reader.read_exact(&mut version)?;
    assert_eq!(version, [0, 0, 0, 2]);
    let mut num_objects = [0u8; 4];
    reader.read_exact(&mut num_objects)?;
    let num_objects = u32::from_be_bytes(num_objects);
    let mut buf = [0u8; 1];
    for _ in 1..=num_objects {
        reader.read_exact(&mut buf)?;
        let object_type = buf[0] >> 4 & 0b111;
        let mut object_size: u32 = (buf[0] & 0b1111) as u32;
        let mut iter = 0;
        while buf[0] >> 7 != 0 {
            reader.read_exact(&mut buf)?;
            object_size |= ((buf[0] & 0b01111111) as u32) << (7 * iter + 4);
            iter += 1;
        }
        match object_type {
            object_type @ 1..=3 => {
                let mut object_data = vec![0u8; object_size as usize];
                if object_size == 0 {
                    reader.read_exact(&mut [0u8; 8])?;
                } else {
                    let mut zlib_reader = ZlibDecoder::new(&mut reader);
                    zlib_reader.read_exact(&mut object_data)?;
                }
                let kind = match object_type {
                    1 => Kind::Commit,
                    2 => Kind::Tree,
                    3 => Kind::Blob,
                    _ => unreachable!(),
                };
                let git_object = GitObject::build(kind, object_data)?;
                git_object.write()?;
            }
            7 => {
                let mut base_hash = vec![0u8; 20];
                reader.read_exact(&mut base_hash)?;
                let mut delta_data = vec![0u8; object_size as usize];
                let mut zlib_reader = ZlibDecoder::new(&mut reader);
                zlib_reader.read_exact(&mut delta_data)?;
                let base_hex_string = hex::encode(base_hash);
                let base_object = GitObject::from_hex_string(&base_hex_string)?;
                let mut base_object_data = Cursor::new(base_object.contents());
                let mut delta_data = Cursor::new(delta_data);
                let _base_object_size = read_varint(&mut delta_data)?;
                let target_object_size = read_varint(&mut delta_data)?;
                let mut target_object_data = Cursor::new(vec![0u8; target_object_size as usize]);
                loop {
                    let mut instruction_buf = vec![0u8; 1];
                    if delta_data.read_exact(&mut instruction_buf).is_err() {
                        break;
                    };
                    if instruction_buf[0] >> 7 == 0 {
                        let size = instruction_buf[0] & 0b01111111;
                        if size == 0 {
                            continue;
                        }
                        std::io::copy(
                            &mut (&mut delta_data).take(size as u64),
                            &mut target_object_data,
                        )?;
                    } else {
                        let mut offset = [0u8; 4];
                        let mut size = [0u8; 4];
                        let mut buf = [0u8; 1];
                        for (i, b) in offset.iter_mut().enumerate() {
                            if (instruction_buf[0] & 1 << i) > 0 {
                                delta_data.read_exact(&mut buf)?;
                                *b = buf[0];
                            }
                        }
                        for (i, b) in size.iter_mut().enumerate().take(3) {
                            if (instruction_buf[0] & 1 << (i + 4)) > 0 {
                                delta_data.read_exact(&mut buf)?;
                                *b = buf[0];
                            }
                        }
                        let offset = u32::from_ne_bytes(offset);
                        let mut size = u32::from_ne_bytes(size);
                        if size == 0 {
                            size = 0x10000;
                        }
                        base_object_data.set_position(offset as u64);
                        std::io::copy(
                            &mut (&mut base_object_data).take(size as u64),
                            &mut target_object_data,
                        )?;
                    };
                }
                let target_object =
                    GitObject::build(base_object.kind().clone(), target_object_data.into_inner())?;
                target_object.write()?;
            }
            object_type => {
                return Err(GitError::any(format!(
                    "unssuported pack object type: {}",
                    object_type
                )));
            }
        }
    }
    Ok(())
}
fn read_varint<R: Read>(mut r: R) -> Result<u32, GitError> {
    let mut buf = [0u8; 1];
    let mut iter = 1;
    r.read_exact(&mut buf)?;
    let mut varint: u32 = (buf[0] & 0b01111111) as u32;
    while buf[0] >> 7 != 0 {
        r.read_exact(&mut buf)?;
        varint |= ((buf[0] & 0b01111111) as u32) << (7 * iter);
        iter += 1;
    }
    Ok(varint)
}