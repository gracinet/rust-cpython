//! Work wih Python capsules
//!
//! Capsules are the preferred way to export a C API to be consumed by other extension modules,
//! see [Providing a C API for an Extension Module](https://docs.python.org/3/extending/extending.html#using-capsules).
//!
//! In particular, capsules can be very useful to start adding Rust extensions besides
//! existing traditional C ones, be it for gradual rewrites or to extend with new functionality.
//!
//! # Example
//! This retrieves and use one of the simplest capsules in the Python standard library, found in
//! the `unicodedata` module. The C API enclosed in this capsule is the same for all Python
//! versions supported by this crate. This is not the case of all capsules from the standard
//! library. For instance the `struct` referenced by `datetime.datetime_CAPI` gets a new member
//! in version 3.7.
//!
//! ```
//! #[macro_use] extern crate cpython;
//! extern crate libc;
//!
//! use cpython::{Python, PyCapsule};
//! use libc::{c_char, c_int};
//! use std::ffi::{c_void, CStr, CString};
//! use std::mem;
//! use std::ptr::null_mut;
//!
//! #[allow(non_camel_case_types)]
//! type Py_UCS4 = u32;
//! const UNICODE_NAME_MAXLEN: usize = 256;
//!
//! #[repr(C)]
//! pub struct unicode_name_CAPI {
//!     // the `ucd` signature arguments are actually optional (can be `NULL`) FFI PyObject
//!     // pointers used to pass alternate (former) versions of Unicode data.
//!     // We won't need to use them with an actual value in these examples, so it's enough to
//!     // specify them as `*mut c_void`, and it spares us a direct reference to the lower
//!     // level Python FFI bindings.
//!     size: c_int,
//!     getname: unsafe extern "C" fn(
//!         ucd: *mut c_void,
//!         code: Py_UCS4,
//!         buffer: *mut c_char,
//!         buflen: c_int,
//!         with_alias_and_seq: c_int,
//!     ) -> c_int,
//!     getcode: unsafe extern "C" fn(
//!         ucd: *mut c_void,
//!         name: *const c_char,
//!         namelen: c_int,
//!         code: *mut Py_UCS4,
//!     ) -> c_int,
//! }

//! #[derive(Debug, PartialEq)]
//! pub enum UnicodeDataError {
//!     InvalidCode,
//!     UnknownName,
//! }
//! impl unicode_name_CAPI {
//!     pub fn get_name(&self, code: Py_UCS4) -> Result<CString, UnicodeDataError> {
//!         let mut buf: Vec<c_char> = Vec::with_capacity(UNICODE_NAME_MAXLEN);
//!         let buf_ptr = buf.as_mut_ptr();
//!         if unsafe {
//!           ((*self).getname)(null_mut(), code, buf_ptr, UNICODE_NAME_MAXLEN as c_int, 0)
//!         } != 1 {
//!             return Err(UnicodeDataError::InvalidCode);
//!         }
//!         mem::forget(buf);
//!         Ok(unsafe { CString::from_raw(buf_ptr) })
//!     }
//!
//!     pub fn get_code(&self, name: &CStr) -> Result<Py_UCS4, UnicodeDataError> {
//!         let namelen = name.to_bytes().len() as c_int;
//!         let mut code: [Py_UCS4; 1] = [0; 1];
//!         if unsafe {
//!             ((*self).getcode)(null_mut(), name.as_ptr(), namelen, code.as_mut_ptr())
//!         } != 1 {
//!             return Err(UnicodeDataError::UnknownName);
//!         }
//!         Ok(code[0])
//!     }
//! }
//!
//! let gil = Python::acquire_gil();
//! let py = gil.python();
//!
//! let capi: &unicode_name_CAPI = unsafe {
//!     PyCapsule::import_data(
//!         py,
//!         CStr::from_bytes_with_nul_unchecked(b"unicodedata.ucnhash_CAPI\0"),
//!     )
//! }
//! .unwrap();
//!
//! assert_eq!(capi.get_name(32).unwrap().to_str(), Ok("SPACE"));
//! assert_eq!(capi.get_name(0), Err(UnicodeDataError::InvalidCode));
//!
//! assert_eq!(
//!     capi.get_code(CStr::from_bytes_with_nul(b"COMMA\0").unwrap()),
//!     Ok(44)
//! );
//! assert_eq!(
//!     capi.get_code(CStr::from_bytes_with_nul(b"\0").unwrap()),
//!     Err(UnicodeDataError::UnknownName)
//! );
//! ```
use super::object::PyObject;
use err::{PyErr, PyResult};
use ffi::PyCapsule_Import;
use python::Python;
use std::ffi::CStr;
use std::mem::transmute;

/// Represents a Python capsule object.
pub struct PyCapsule(PyObject);

#[macro_export]
macro_rules! py_capsule {
    ($($capsmod:ident).+, $capsname:ident, $retrieve:ident, $sig:ty) => (
        unsafe fn $retrieve(py: $crate::Python) -> $crate::PyResult<$sig> {
            let caps_name =
                std::ffi::CStr::from_bytes_with_nul_unchecked(
                    concat!($( stringify!($capsmod), "."),*,
                            stringify!($capsname),
                            "\0").as_bytes());
            let from_caps = $crate::_detail::ffi::PyCapsule_Import(caps_name.as_ptr(), 0);
            if from_caps.is_null() {
                return Err($crate::PyErr::fetch(py));
            }
            Ok(::std::mem::transmute(from_caps))
        }
    )
}

impl PyCapsule {
    /// Retrieve the contents of a capsule pointing to some data as a reference.
    ///
    /// The retrieved data would typically be an array of static data and/or function pointers.
    /// This method doesn't work for standalone function pointers.
    ///
    /// This is very unsafe, because
    /// - nothing guarantees that the `T` type is appropriate for the data referenced by the capsule
    ///   pointer
    /// - the returned lifetime doesn't guarantee either to cover the actual lifetime of the data
    ///   (although capsule data is usually static)
    pub unsafe fn import_data<'a, T>(py: Python, name: &CStr) -> PyResult<&'a T> {
        let from_caps = PyCapsule_Import(name.as_ptr(), 0);
        if from_caps.is_null() {
            return Err(PyErr::fetch(py));
        }
        Ok(&*(from_caps as *const T))
    }
}
