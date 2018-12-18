#[macro_use]
extern crate cpython;
extern crate libc;

use cpython::capsule::retrieve_capsule;
use cpython::Python;
use libc::{c_char, c_int};
use std::ffi::{c_void, CStr, CString};
use std::mem;
use std::ptr::null_mut;

#[allow(non_camel_case_types)]
type Py_UCS4 = u32;
const UNICODE_NAME_MAXLEN: usize = 256;

#[repr(C)]
pub struct unicode_name_CAPI {
    // the `ucd` signature arguments are actually optional FFI PyObject pointer
    // used to pass alternate (prior) versions of Unicode data. We won't
    // need them in these examples, so *mut c_void is enough and it spares us a direct
    // reference to the relevant `python*-sys` crate.
    size: c_int,
    getname: unsafe extern "C" fn(
        ucd: *mut c_void,
        code: Py_UCS4,
        buffer: *mut c_char,
        buflen: c_int,
        with_alias_and_seq: c_int,
    ) -> c_int,
    getcode: unsafe extern "C" fn(
        ucd: *mut c_void,
        name: *const c_char,
        namelen: c_int,
        code: *mut Py_UCS4,
    ) -> c_int,
}

#[derive(Debug, PartialEq)]
pub enum UnicodeDataError {
    InvalidCode,
    UnknownName,
}

impl unicode_name_CAPI {
    pub fn get_name(&self, code: Py_UCS4) -> Result<CString, UnicodeDataError> {
        let mut buf: Vec<c_char> = Vec::with_capacity(UNICODE_NAME_MAXLEN);
        let buf_ptr = buf.as_mut_ptr();
        if unsafe { ((*self).getname)(null_mut(), code, buf_ptr, UNICODE_NAME_MAXLEN as c_int, 0) }
            != 1
        {
            return Err(UnicodeDataError::InvalidCode);
        }
        mem::forget(buf);
        Ok(unsafe { CString::from_raw(buf_ptr) })
    }

    pub fn get_code(&self, name: &CStr) -> Result<Py_UCS4, UnicodeDataError> {
        let namelen = name.to_bytes().len() as c_int;
        let mut code: [Py_UCS4; 1] = [0; 1];
        if unsafe { ((*self).getcode)(null_mut(), name.as_ptr(), namelen, code.as_mut_ptr()) } != 1
        {
            return Err(UnicodeDataError::UnknownName);
        }
        Ok(code[0])
    }
}

py_capsule!(
    unicodedata,
    ucnhash_CAPI,
    retrieve_ucn_caps,
    *const unicode_name_CAPI
);

#[test]
fn use_capsule() {
    let gil = Python::acquire_gil();
    let py = gil.python();

    let capi: &unicode_name_CAPI = unsafe {
        retrieve_capsule(
            py,
            CStr::from_bytes_with_nul_unchecked(b"unicodedata.ucnhash_CAPI\0"),
        )
    }
    .unwrap();

    assert_eq!(capi.get_name(32).unwrap().to_str(), Ok("SPACE"));
    assert_eq!(capi.get_name(0), Err(UnicodeDataError::InvalidCode));

    assert_eq!(
        capi.get_code(CStr::from_bytes_with_nul(b"COMMA\0").unwrap()),
        Ok(44)
    );
    assert_eq!(
        capi.get_code(CStr::from_bytes_with_nul(b"\0").unwrap()),
        Err(UnicodeDataError::UnknownName)
    );
}
