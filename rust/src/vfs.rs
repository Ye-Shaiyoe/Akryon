use alloc::string::String;
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InodeType {
    File,
    Directory,
    CharDevice,
}

pub trait InodeOperations {
    fn read(&self, offset: usize, buf: &mut [u8]) -> Result<usize, i32>;
    fn write(&mut self, offset: usize, buf: &[u8]) -> Result<usize, i32>;
    fn get_size(&self) -> usize;
}

pub struct MemoryInode {
    pub name: String,
    pub inode_type: InodeType,
    pub data: Vec<u8>,
}

impl InodeOperations for MemoryInode {
    fn read(&self, offset: usize, buf: &mut [u8]) -> Result<usize, i32> {
        if offset >= self.data.len() {
            return Ok(0);
        }
        let available = self.data.len() - offset;
        let to_read = available.min(buf.len());
        buf[..to_read].copy_from_slice(&self.data[offset..offset + to_read]);
        Ok(to_read)
    }

    fn write(&mut self, offset: usize, buf: &[u8]) -> Result<usize, i32> {
        let needed_len = offset + buf.len();
        if needed_len > self.data.len() {
            self.data.resize(needed_len, 0);
        }
        self.data[offset..needed_len].copy_from_slice(buf);
        Ok(buf.len())
    }

    fn get_size(&self) -> usize {
        self.data.len()
    }
}

use core::cell::UnsafeCell;

pub struct RamFs {
    files: Vec<MemoryInode>,
}

struct SafeRamFs(UnsafeCell<Option<RamFs>>);
unsafe impl Sync for SafeRamFs {}

static RAMFS: SafeRamFs = SafeRamFs(UnsafeCell::new(None));

pub fn init() {
    let mut fs = RamFs { files: Vec::new() };

    let mut motd = MemoryInode {
        name: String::from("motd"),
        inode_type: InodeType::File,
        data: Vec::new(),
    };
    motd.data.extend_from_slice(b"Welcome to Akryon Unix-like Operating System!\n");
    fs.files.push(motd);

    let mut readme = MemoryInode {
        name: String::from("readme.txt"),
        inode_type: InodeType::File,
        data: Vec::new(),
    };
    readme.data.extend_from_slice(b"Akryon kernel v2 with POSIX syscalls and VFS.\n");
    fs.files.push(readme);

    unsafe {
        *RAMFS.0.get() = Some(fs);
    }
}

pub fn list_files() -> Vec<(String, usize)> {
    unsafe {
        match &*RAMFS.0.get() {
            Some(fs) => fs.files.iter().map(|f| (f.name.clone(), f.get_size())).collect(),
            None => Vec::new(),
        }
    }
}

pub fn read_file(name: &str) -> Option<Vec<u8>> {
    unsafe {
        (*RAMFS.0.get()).as_ref()?.files.iter().find(|f| f.name == name).map(|f| f.data.clone())
    }
}

pub fn write_file(name: &str, data: &[u8]) -> Result<(), i32> {
    unsafe {
        if let Some(fs) = (&mut *RAMFS.0.get()).as_mut() {
            if let Some(f) = fs.files.iter_mut().find(|f| f.name == name) {
                f.data.clear();
                f.data.extend_from_slice(data);
                return Ok(());
            }

            let mut new_file = MemoryInode {
                name: String::from(name),
                inode_type: InodeType::File,
                data: Vec::new(),
            };
            new_file.data.extend_from_slice(data);
            fs.files.push(new_file);
            Ok(())
        } else {
            Err(-5) // -EIO
        }
    }
}
