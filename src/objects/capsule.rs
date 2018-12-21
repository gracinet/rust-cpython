//! Work wih Python capsules
//!
use super::object::PyObject;
use err::{self, PyErr, PyResult};
use ffi::{PyCapsule_GetPointer, PyCapsule_Import, PyCapsule_New};
use libc::c_void;
use python::{Python, ToPythonPointer};
use std::ffi::{CStr, CString, NulError};
use std::mem;

/// Represents a Python capsule object.
pub struct PyCapsule(PyObject);

pyobject_newtype!(PyCapsule, PyCapsule_CheckExact, PyCapsule_Type);

#[macro_export]
macro_rules! py_capsule_fn {
    ($($capsmod:ident).+, $capsname:ident, $rustmod:ident, $( $sig: tt)* ) => (
        mod $rustmod {
            use super::*;
            pub type CapsFn = unsafe extern "C" fn $( $sig )*;
            pub fn import(py: $crate::Python) -> $crate::PyResult<CapsFn> {
                unsafe {
                    let caps_name =
                        std::ffi::CStr::from_bytes_with_nul_unchecked(
                            concat!($( stringify!($capsmod), "."),*,
                                    stringify!($capsname),
                                    "\0").as_bytes());
                    Ok(::std::mem::transmute($crate::PyCapsule::import(py, caps_name)?))
                }
            }
        }
    )
}

/// Capsules are the preferred way to export/import C APIs between extension modules,
/// see [Providing a C API for an Extension Module](https://docs.python.org/3/extending/extending.html#using-capsules).
///
/// In particular, capsules can be very useful to start adding Rust extensions besides
/// existing traditional C ones, be it for gradual rewrites or to extend with new functionality.
/// They can also be used for interaction between independently compiled Rust extensions if needed.
///
/// # Examples
/// ## Using a capsule defined in another extension module
/// This retrieves and use one of the simplest capsules in the Python standard library, found in
/// the `unicodedata` module. The C API enclosed in this capsule is the same for all Python
/// versions supported by this crate. This is not the case of all capsules from the standard
/// library. For instance the `struct` referenced by `datetime.datetime_CAPI` gets a new member
/// in version 3.7.
///
/// ```
/// #[macro_use] extern crate cpython;
/// extern crate libc;
///
/// use cpython::{Python, PyCapsule};
/// use libc::{c_char, c_int};
/// use std::ffi::{c_void, CStr, CString};
/// use std::mem;
/// use std::ptr::null_mut;
///
/// #[allow(non_camel_case_types)]
/// type Py_UCS4 = u32;
/// const UNICODE_NAME_MAXLEN: usize = 256;
///
/// #[repr(C)]
/// pub struct unicode_name_CAPI {
///     // the `ucd` signature arguments are actually optional (can be `NULL`) FFI PyObject
///     // pointers used to pass alternate (former) versions of Unicode data.
///     // We won't need to use them with an actual value in these examples, so it's enough to
///     // specify them as `*mut c_void`, and it spares us a direct reference to the lower
///     // level Python FFI bindings.
///     size: c_int,
///     getname: unsafe extern "C" fn(
///         ucd: *mut c_void,
///         code: Py_UCS4,
///         buffer: *mut c_char,
///         buflen: c_int,
///         with_alias_and_seq: c_int,
///     ) -> c_int,
///     getcode: unsafe extern "C" fn(
///         ucd: *mut c_void,
///         name: *const c_char,
///         namelen: c_int,
///         code: *mut Py_UCS4,
///     ) -> c_int,
/// }

/// #[derive(Debug, PartialEq)]
/// pub enum UnicodeDataError {
///     InvalidCode,
///     UnknownName,
/// }
/// impl unicode_name_CAPI {
///     pub fn get_name(&self, code: Py_UCS4) -> Result<CString, UnicodeDataError> {
///         let mut buf: Vec<c_char> = Vec::with_capacity(UNICODE_NAME_MAXLEN);
///         let buf_ptr = buf.as_mut_ptr();
///         if unsafe {
///           ((*self).getname)(null_mut(), code, buf_ptr, UNICODE_NAME_MAXLEN as c_int, 0)
///         } != 1 {
///             return Err(UnicodeDataError::InvalidCode);
///         }
///         mem::forget(buf);
///         Ok(unsafe { CString::from_raw(buf_ptr) })
///     }
///
///     pub fn get_code(&self, name: &CStr) -> Result<Py_UCS4, UnicodeDataError> {
///         let namelen = name.to_bytes().len() as c_int;
///         let mut code: [Py_UCS4; 1] = [0; 1];
///         if unsafe {
///             ((*self).getcode)(null_mut(), name.as_ptr(), namelen, code.as_mut_ptr())
///         } != 1 {
///             return Err(UnicodeDataError::UnknownName);
///         }
///         Ok(code[0])
///     }
/// }
///
/// let gil = Python::acquire_gil();
/// let py = gil.python();
///
/// let capi: &unicode_name_CAPI = unsafe {
///     PyCapsule::import_data(
///         py,
///         CStr::from_bytes_with_nul_unchecked(b"unicodedata.ucnhash_CAPI\0"),
///     )
/// }
/// .unwrap();
///
/// assert_eq!(capi.get_name(32).unwrap().to_str(), Ok("SPACE"));
/// assert_eq!(capi.get_name(0), Err(UnicodeDataError::InvalidCode));
///
/// assert_eq!(
///     capi.get_code(CStr::from_bytes_with_nul(b"COMMA\0").unwrap()),
///     Ok(44)
/// );
/// assert_eq!(
///     capi.get_code(CStr::from_bytes_with_nul(b"\0").unwrap()),
///     Err(UnicodeDataError::UnknownName)
/// );
/// ```
///
/// ## Creating a capsule from Rust
/// In this example, we enclose some data and a function in a capsule, using an intermediate
/// `struct` as enclosing type, then retrieve them back and use them.
///
/// ```
/// extern crate cpython;
/// extern crate libc;
///
/// use libc::c_int;
/// use cpython::{PyCapsule, Python};
/// use std::ffi::{c_void, CStr, CString};
///
/// #[repr(C)]
/// struct CapsData {
///     value: c_int,
///     fun: fn(c_int, c_int) -> c_int,
/// }
///
/// fn add(a: c_int, b: c_int) -> c_int {
///     a + b
/// }
///
/// const DATA: CapsData = CapsData{value: 1, fun: add};
///
/// let gil = Python::acquire_gil();
/// let py = gil.python();
/// let caps = PyCapsule::new_data(py, &mut DATA, "somemod.capsdata").unwrap();
///
/// let retrieved: &CapsData = unsafe {caps.data_ref("somemod.capsdata")}.unwrap();
/// assert_eq!(retrieved.value, 1);
/// assert_eq!((retrieved.fun)(2 as c_int, 3 as c_int), 5);
/// ```
///
/// Of course, a more realistic example would be to store the capsule in a Python module,
/// allowing another extension (possibly foreign) to retrieve and use it.
/// Note that in that case, the capsule `name` must be full dotted name of the capsule object,
/// as we're doing here.
/// ```
/// # #[macro_use] extern crate cpython;
/// # extern crate libc;
/// # use libc::c_int;
/// # use cpython::PyCapsule;
/// # #[repr(C)]
/// # struct CapsData {
/// #     value: c_int,
/// #     fun: fn(c_int, c_int) -> c_int,
/// # }
/// # fn add(a: c_int, b: c_int) -> c_int {
/// #     a + b
/// # }
/// # const DATA: CapsData = CapsData{value: 1, fun: add};
/// py_module_initializer!(somemod, initsomemod, PyInit_somemod, |py, m| {
///   m.add(py, "__doc__", "A module holding a capsule")?;
///   m.add(py, "capsdata", PyCapsule::new_data(py, &mut DATA, "somemod.capsdata").unwrap())?;
///   Ok(())
/// });
/// ```
/// Another Rust extension could then declare `CapsData` and use `PyCapsule::import_data` to
/// fetch it back.
///
/// ## Retrieving a function pointer capsule
///
/// There is in the Python library no capsule enclosing a function pointer directly,
/// although the documentation presents it as a valid use-case. For this example, we'll
/// therefore have to create one, and to set it in an existing module (not to imply that
/// a true extension should follow that example and set capsules in modules they don't
/// define!)
///
/// ```
/// #[macro_use] extern crate cpython;
/// extern crate libc;
/// use cpython::{PyCapsule, Python, FromPyObject};
/// use libc::{c_int, c_void};
///
/// extern "C" fn inc(a: c_int) -> c_int {
///     a + 1
/// }
///
/// /// for testing purposes, stores a capsule named `sys.capsfn`` pointing to `inc()`.
/// fn create_capsule() {
///     let gil = Python::acquire_gil();
///     let py = gil.python();
///     let pymod = py.import("sys").unwrap();
///     let caps = PyCapsule::new(py, inc as *mut c_void, "sys.capsfn").unwrap();
///     pymod.add(py, "capsfn", caps).unwrap();
///  }
///
/// py_capsule_fn!(sys, capsfn, capsmod, (a: c_int) -> c_int);
/// // we now have a `capsmod` Rust module, that defines
/// // - `CapsFn`: type for the target function
/// // - `import(py: Python) -> PyResult<CapsFn>`: to fetch the encapsulated function
///
/// // One could, e.g., reexport if needed:
/// pub use capsmod::CapsFn;
///
/// fn retrieve_use_capsule() {
///     let gil = Python::acquire_gil();
///     let py = gil.python();
///     let fun = capsmod::import(py).unwrap();
///     assert_eq!( unsafe { fun(1) }, 2);
///
///     // let's demonstrate the (reexported) function type
///     let mut g: Option<CapsFn> = None;
///     g = Some(fun);
/// }
///
/// fn main() {
///     create_capsule();
///     retrieve_use_capsule();
/// }
/// ```
impl PyCapsule {
    /// Retrieve the contents of a capsule pointing to some data as a reference.
    ///
    /// The retrieved data would typically be an array of static data and/or function pointers.
    /// This method doesn't work for standalone function pointers.
    ///
    /// # Safety
    /// This method is unsafe, because
    /// - nothing guarantees that the `T` type is appropriate for the data referenced by the capsule
    ///   pointer
    /// - the returned lifetime doesn't guarantee either to cover the actual lifetime of the data
    ///   (although capsule data is usually static)
    pub unsafe fn import_data<'a, T>(py: Python, name: &CStr) -> PyResult<&'a T> {
        Ok(&*(Self::import(py, name)? as *const T))
    }

    /// Retrieves the contents of a capsule as a void pointer by its name.
    ///
    /// This is suitable in particular for later conversion as a function pointer
    /// with `mem::transmute`, for architectures where data and function pointers have
    /// the same size (see details about this the documentation of the Rust standard library).
    pub fn import(py: Python, name: &CStr) -> PyResult<*mut c_void> {
        let caps_ptr = unsafe { PyCapsule_Import(name.as_ptr(), 0) };
        if caps_ptr.is_null() {
            return Err(PyErr::fetch(py));
        }
        Ok(caps_ptr)
    }

    /// Convenience method to create a capsule for some data
    ///
    /// The encapsuled data may be an array of functions, but it can't be itself a
    /// function directly.
    ///
    /// May panic when running out of memory.
    ///
    pub fn new_data<T>(
        py: Python,
        data: &mut T,
        name: impl Into<Vec<u8>>,
    ) -> Result<Self, NulError> {
        Self::new(py, data as *mut T as *mut c_void, name)
    }

    /// Creates a new capsule from a raw void pointer
    ///
    /// This is suitable in particular to store a function pointer in a capsule. These
    /// can be obtained simply by a simple cast:
    ///
    /// ```
    /// extern crate libc;
    /// use libc::c_void;
    ///
    /// extern "C" fn inc(a: i32) -> i32 {
    ///     a + 1
    /// }
    /// let ptr = inc as *mut c_void;
    /// ```
    ///
    /// # Errors
    /// This method returns `NulError` if `name` contains a 0 byte (see also `CString::new`)
    pub fn new(
        py: Python,
        pointer: *mut c_void,
        name: impl Into<Vec<u8>>,
    ) -> Result<Self, NulError> {
        let name = CString::new(name)?;
        let caps = unsafe {
            Ok(err::cast_from_owned_ptr_or_panic(
                py,
                PyCapsule_New(pointer, name.as_ptr(), None),
            ))
        };
        // it is required that the capsule name outlives the call as a char*
        // TODO implement a proper PyCapsule_Destructor to release it properly
        mem::forget(name);
        caps
    }

    /// Returns a reference to the capsule data.
    ///
    /// The name must match exactly the one given at capsule creation time (see `new_data`) and
    /// is converted to a C string under the hood. If that's too much overhead, consider using
    /// `data_ref_cstr()` or caching strategies.
    ///
    /// This is unsafe, because
    /// - nothing guarantees that the `T` type is appropriate for the data referenced by the capsule
    ///   pointer
    /// - the returned lifetime doesn't guarantee either to cover the actual lifetime of the data
    ///   (although capsule data is usually static)
    ///
    /// # Errors
    /// This method returns `NulError` if `name` contains a 0 byte (see also `CString::new`)
    pub unsafe fn data_ref<'a, T>(&self, name: impl Into<Vec<u8>>) -> Result<&'a T, NulError> {
        Ok(self.data_ref_cstr(&CString::new(name)?))
    }

    /// Returns a reference to the capsule data.
    ///
    /// This is identical to `data_ref`, except for the name passing. This allows to use
    /// lower level constructs without overhead, such as `CStr::from_bytes_with_nul_unchecked`
    pub unsafe fn data_ref_cstr<'a, T>(&self, name: &CStr) -> &'a T {
        &*(PyCapsule_GetPointer(self.as_ptr(), name.as_ptr()) as *const T)
    }
}
