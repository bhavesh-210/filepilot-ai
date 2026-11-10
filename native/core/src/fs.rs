use serde::{Deserialize, Serialize};
use std::{
    ffi::CString,
    fs::File,
    io,
    os::fd::{AsRawFd, FromRawFd},
    path::{Component, Path},
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Identity {
    pub dev: u64,
    pub ino: u64,
    pub size: u64,
    pub mtime: i64,
    pub nanos: i64,
    pub ctime: i64,
    pub cnanos: i64,
}
fn err(s: &str) -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, s)
}
pub fn excluded(name: &str) -> bool {
    [
        ".ssh",
        ".aws",
        ".gnupg",
        ".git",
        ".Trash",
        "node_modules",
        "venv",
        "Library",
        "System",
        "__pycache__",
    ]
    .contains(&name)
        || name.starts_with(".env")
        || name.starts_with(".venv")
        || [
            ".pem",
            ".key",
            ".p12",
            ".app",
            ".bundle",
            ".photoslibrary",
            ".keychain-db",
        ]
        .iter()
        .any(|x| name.to_lowercase().ends_with(x))
}
pub fn parts(path: &str) -> io::Result<Vec<&str>> {
    if path.is_empty()
        || path.len() > 4096
        || path.contains('\\')
        || path.chars().any(|x| x.is_control())
    {
        return Err(err("Invalid path"));
    }
    let result: Vec<_> = path.split('/').collect();
    if result
        .iter()
        .any(|x| x.is_empty() || *x == "." || *x == ".." || excluded(x))
    {
        return Err(err("Excluded or noncanonical path"));
    }
    Ok(result)
}
fn c(s: &str) -> io::Result<CString> {
    CString::new(s).map_err(|_| err("Invalid path"))
}
fn open_at(parent: i32, name: &str) -> io::Result<File> {
    let fd = unsafe {
        libc::openat(
            parent,
            c(name)?.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(unsafe { File::from_raw_fd(fd) })
    }
}
pub fn root(path: &Path) -> io::Result<File> {
    if !path.is_absolute() || path == Path::new("/") {
        return Err(err("Choose a specific absolute folder"));
    }
    let mut fd = open_at(libc::AT_FDCWD, "/")?;
    for component in path.components() {
        match component {
            Component::RootDir => {}
            Component::Normal(name) => {
                let name = name
                    .to_str()
                    .ok_or_else(|| err("Unsupported path encoding"))?;
                if excluded(name) {
                    return Err(err("Excluded root"));
                }
                fd = open_at(fd.as_raw_fd(), name)?;
                if dataless(fd.as_raw_fd())? {
                    return Err(err("Cloud placeholder folder"));
                }
            }
            _ => return Err(err("Noncanonical root")),
        }
    }
    Ok(fd)
}
pub fn dataless(fd: i32) -> io::Result<bool> {
    let mut st = unsafe { std::mem::zeroed::<libc::stat>() };
    if unsafe { libc::fstat(fd, &mut st) } != 0 {
        return Err(io::Error::last_os_error());
    }
    #[cfg(target_os = "macos")]
    {
        return Ok(st.st_flags & 0x40000000 != 0);
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(false)
    }
}
pub fn project(fd: i32) -> bool {
    [
        ".git",
        "package.json",
        "pyproject.toml",
        "Cargo.toml",
        "go.mod",
        "pom.xml",
        "build.gradle",
        "Package.swift",
        "CMakeLists.txt",
    ]
    .iter()
    .any(|name| {
        let mut st = unsafe { std::mem::zeroed::<libc::stat>() };
        (unsafe {
            libc::fstatat(
                fd,
                c(name).unwrap().as_ptr(),
                &mut st,
                libc::AT_SYMLINK_NOFOLLOW,
            )
        }) == 0
            && (st.st_mode & libc::S_IFMT == libc::S_IFREG
                || (*name == ".git" && st.st_mode & libc::S_IFMT == libc::S_IFDIR))
    })
}
pub fn parent(root: &Path, relative: &str) -> io::Result<(File, String)> {
    let components = parts(relative)?;
    let mut fd = self::root(root)?;
    if project(fd.as_raw_fd()) {
        return Err(err("Project root is protected"));
    }
    for name in &components[..components.len() - 1] {
        fd = open_at(fd.as_raw_fd(), name)?;
        if project(fd.as_raw_fd()) || dataless(fd.as_raw_fd())? {
            return Err(err("Project or placeholder is protected"));
        }
    }
    Ok((fd, components.last().unwrap().to_string()))
}
pub fn identity(parent: &File, name: &str) -> io::Result<Identity> {
    let mut st = unsafe { std::mem::zeroed::<libc::stat>() };
    if unsafe {
        libc::fstatat(
            parent.as_raw_fd(),
            c(name)?.as_ptr(),
            &mut st,
            libc::AT_SYMLINK_NOFOLLOW,
        )
    } != 0
    {
        return Err(io::Error::last_os_error());
    }
    if st.st_mode & libc::S_IFMT != libc::S_IFREG || st.st_nlink != 1 {
        return Err(err("Only single link regular files are permitted"));
    }
    #[cfg(target_os = "macos")]
    if st.st_flags & 0x40000000 != 0 {
        return Err(err("Cloud placeholder file"));
    }
    Ok(Identity {
        dev: st.st_dev as u64,
        ino: st.st_ino as u64,
        size: st.st_size as u64,
        mtime: st.st_mtime,
        nanos: st.st_mtime_nsec,
        ctime: st.st_ctime,
        cnanos: st.st_ctime_nsec,
    })
}
pub fn same_file(a: &Identity, b: &Identity) -> bool {
    a.dev == b.dev && a.ino == b.ino && a.size == b.size && a.mtime == b.mtime && a.nanos == b.nanos
}
pub fn exists(parent: &File, name: &str) -> io::Result<bool> {
    let mut st = unsafe { std::mem::zeroed::<libc::stat>() };
    if unsafe {
        libc::fstatat(
            parent.as_raw_fd(),
            c(name)?.as_ptr(),
            &mut st,
            libc::AT_SYMLINK_NOFOLLOW,
        )
    } == 0
    {
        return Ok(true);
    }
    let e = io::Error::last_os_error();
    if e.kind() == io::ErrorKind::NotFound {
        Ok(false)
    } else {
        Err(e)
    }
}
pub fn move_exclusive(from: &File, source: &str, to: &File, destination: &str) -> io::Result<()> {
    #[cfg(target_os = "linux")]
    let result = unsafe {
        libc::syscall(
            libc::SYS_renameat2,
            from.as_raw_fd(),
            c(source)?.as_ptr(),
            to.as_raw_fd(),
            c(destination)?.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    };
    #[cfg(target_os = "macos")]
    let result = unsafe {
        libc::renameatx_np(
            from.as_raw_fd(),
            c(source)?.as_ptr(),
            to.as_raw_fd(),
            c(destination)?.as_ptr(),
            libc::RENAME_EXCL,
        )
    };
    if result != 0 {
        return Err(io::Error::last_os_error());
    }
    from.sync_all()?;
    to.sync_all()?;
    Ok(())
}
pub fn names(directory: &File) -> io::Result<Vec<String>> {
    let duplicate = unsafe { libc::dup(directory.as_raw_fd()) };
    if duplicate < 0 {
        return Err(io::Error::last_os_error());
    }
    let dir = unsafe { libc::fdopendir(duplicate) };
    if dir.is_null() {
        unsafe { libc::close(duplicate) };
        return Err(io::Error::last_os_error());
    }
    let result = (|| {
        let mut names = Vec::new();
        loop {
            let entry = unsafe { libc::readdir(dir) };
            if entry.is_null() {
                break;
            }
            let name = unsafe { std::ffi::CStr::from_ptr((*entry).d_name.as_ptr()) }
                .to_str()
                .map_err(|_| err("Unsupported filename encoding"))?;
            if name != "." && name != ".." {
                names.push(name.to_owned());
            }
            if names.len() > 10000 {
                return Err(err("Directory limit reached"));
            }
        }
        names.sort();
        Ok(names)
    })();
    unsafe { libc::closedir(dir) };
    result
}
pub fn child_directory(directory: &File, name: &str) -> io::Result<File> {
    open_at(directory.as_raw_fd(), name)
}
pub fn mkdir(directory: &File, name: &str) -> io::Result<()> {
    parts(name)?;
    if name.contains('/') {
        return Err(err("Expected directory basename"));
    }
    if unsafe { libc::mkdirat(directory.as_raw_fd(), c(name)?.as_ptr(), 0o700) } != 0 {
        let e = io::Error::last_os_error();
        if e.kind() != io::ErrorKind::AlreadyExists {
            return Err(e);
        }
    }
    let child = open_at(directory.as_raw_fd(), name)?;
    if project(child.as_raw_fd()) || dataless(child.as_raw_fd())? {
        return Err(err("Protected destination"));
    }
    directory.sync_all()
}
pub fn digest(parent: &File, name: &str, expected: &Identity) -> io::Result<String> {
    use sha2::{Digest, Sha256};
    use std::io::Read;
    if expected.size > 64 * 1024 * 1024 {
        return Err(err(
            "Files above 64 MiB require a later large file workflow",
        ));
    }
    if &identity(parent, name)? != expected {
        return Err(err("File changed before hashing"));
    }
    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            c(name)?.as_ptr(),
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    let mut file = unsafe { File::from_raw_fd(fd) };
    let metadata = file.metadata()?;
    use std::os::unix::fs::MetadataExt;
    if metadata.dev() != expected.dev
        || metadata.ino() != expected.ino
        || !metadata.is_file()
        || metadata.nlink() != 1
        || dataless(fd)?
    {
        return Err(err("File changed while opening"));
    }
    let mut hash = Sha256::new();
    let mut chunk = [0u8; 65536];
    let mut count = 0;
    loop {
        let read = file.read(&mut chunk)?;
        if read == 0 {
            break;
        }
        count += read as u64;
        if count > 64 * 1024 * 1024 {
            return Err(err("Byte limit"));
        }
        hash.update(&chunk[..read]);
    }
    if count != expected.size || &identity(parent, name)? != expected {
        return Err(err("File changed while hashing"));
    }
    Ok(format!("{:x}", hash.finalize()))
}
