use std::{ffi::CString, ptr::null};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("failed with errno {0}")]
    Errno(i32),
    #[error("invalid argument: {0}")]
    InvalidArgument(&'static str),
}

#[derive(Clone, Debug)]
pub struct Object {
    obj: *mut libbpf_sys::bpf_object,
}

impl Object {
    /// Create a BPF object from a buffer of a valid BPF ELF object file.
    #[must_use]
    pub fn create(obj_buf: &[u8]) -> Result<Object> {
        match unsafe {
            libbpf_sys::bpf_object__open_mem(obj_buf.as_ptr() as _, obj_buf.len() as u64, null())
        } {
            obj if obj.is_null() => Err(Error::Errno(errno())),
            obj => Ok(Object { obj }),
        }
    }

    #[must_use]
    pub fn load(&self) -> Result<()> {
        match unsafe { libbpf_sys::bpf_object__load(self.obj) } {
            ret if ret < 0 => Err(Error::Errno(-ret)),
            _ => Ok(()),
        }
    }

    #[must_use]
    pub fn find_program(&self, name: &str) -> Result<Program> {
        match unsafe {
            let name = CString::new(name)
                .map_err(|_| Error::InvalidArgument("could not convert to CString"))?;
            libbpf_sys::bpf_object__find_program_by_name(self.obj, name.as_ptr())
        } {
            ret if ret.is_null() => Err(Error::Errno(errno())),
            program => Ok(Program::new(program)),
        }
    }

    #[must_use]
    pub fn find_map(&self, name: &str) -> Result<Map> {
        match unsafe {
            let name = CString::new(name)
                .map_err(|_| Error::InvalidArgument("could not convert to CString"))?;
            libbpf_sys::bpf_object__find_map_by_name(self.obj, name.as_ptr())
        } {
            ret if ret.is_null() => Err(Error::Errno(errno())),
            map => Ok(Map::new(map)),
        }
    }

    pub fn close(self) {
        unsafe { libbpf_sys::bpf_object__close(self.obj) }
    }
}

#[derive(Clone, Debug)]
pub struct Program {
    prog: *mut libbpf_sys::bpf_program,
}

impl Program {
    #[must_use]
    pub fn new(prog: *mut libbpf_sys::bpf_program) -> Self {
        Program { prog }
    }

    #[must_use]
    pub fn attach_xdp(&self, ifindex: u32) -> Result<LinkedProgram> {
        match unsafe { libbpf_sys::bpf_program__attach_xdp(self.prog, ifindex as i32) } {
            ret if ret.is_null() => Err(Error::Errno(errno())),
            link => Ok(LinkedProgram::new(link)),
        }
    }
}

#[derive(Clone, Debug)]
pub struct LinkedProgram {
    link: *mut libbpf_sys::bpf_link,
}

impl LinkedProgram {
    #[must_use]
    pub fn new(link: *mut libbpf_sys::bpf_link) -> Self {
        LinkedProgram { link }
    }
}

#[derive(Clone, Debug)]
pub struct Map {
    map: *mut libbpf_sys::bpf_map,
}

impl Map {
    #[must_use]
    pub fn new(map: *mut libbpf_sys::bpf_map) -> Self {
        Map { map }
    }

    #[must_use]
    pub fn update(&self, key: &[u8], value: &[u8]) -> Result<()> {
        match unsafe {
            libbpf_sys::bpf_map__update_elem(
                self.map,
                key.as_ptr() as _,
                key.len() as u64,
                value.as_ptr() as _,
                value.len() as u64,
                0,
            )
        } {
            ret if ret < 0 => Err(Error::Errno(errno())),
            _ => Ok(()),
        }
    }
}

// LIBBPF_API int bpf_link__update_map (struct bpf_link *link, const struct bpf_map *map)

#[must_use]
pub(crate) fn errno() -> i32 {
    unsafe { *libc::__errno_location() }
}
