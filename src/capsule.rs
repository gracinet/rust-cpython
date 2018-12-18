use super::{PyErr, PyObject, PyResult, Python};
use ffi::PyCapsule_Import;
use std::ffi::CStr;
use std::mem::transmute;

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

pub unsafe fn retrieve_capsule<'a, T>(py: Python, name: &CStr) -> PyResult<&'a T> {
    let from_caps = PyCapsule_Import(name.as_ptr(), 0);
    if from_caps.is_null() {
        return Err(PyErr::fetch(py));
    }
    Ok(&*(from_caps as *const T))
}
