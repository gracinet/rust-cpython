
pub type Py_uintptr_t = crate::libc::uintptr_t;
pub type Py_intptr_t = crate::libc::intptr_t;
pub type Py_ssize_t = crate::libc::ssize_t;

pub type Py_hash_t = Py_ssize_t;
pub type Py_uhash_t = crate::libc::size_t;

pub const PY_SSIZE_T_MIN : Py_ssize_t = crate::core::isize::MIN as Py_ssize_t;
pub const PY_SSIZE_T_MAX : Py_ssize_t = crate::core::isize::MAX as Py_ssize_t;

