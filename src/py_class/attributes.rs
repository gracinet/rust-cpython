use ffi;

#[macro_export]
#[doc(hidden)]
macro_rules! py_class_init_attributes {
    ($class:ident, $py:ident, $type_object: ident, { }) => {{}};
    ($class:ident, $py:ident, $type_object: ident, { $( $prop:expr; )+ }) =>
    { unsafe {
        let mut defs = Vec::new();
        $(defs.push($prop);)+
        defs.push(
            $crate::_detail::ffi::PyGetSetDef {
                name: 0 as *mut $crate::_detail::libc::c_char,
                get: None,
                set: None,
                doc: 0 as *mut $crate::_detail::libc::c_char,
                closure: 0 as *mut $crate::_detail::libc::c_void,
            });
        let props = defs.into_boxed_slice();

        $type_object.tp_getset =
            props.as_ptr() as *mut $crate::_detail::ffi::PyGetSetDef;

        use std::mem;
        mem::forget(props);
    }};
}

#[macro_export]
#[doc(hidden)]
macro_rules! py_class_attribute_impl {

    ({} $class:ident $py:ident $name:ident { $( $descr_name:ident = $descr_expr:expr; )* } ) =>
    {{
        let mut getset_def: $crate::_detail::ffi::PyGetSetDef =
            $crate::_detail::ffi::PyGetSetDef {
                name: 0 as *mut $crate::_detail::libc::c_char,
                get: None,
                set: None,
                doc: 0 as *mut $crate::_detail::libc::c_char,
                closure: 0 as *mut $crate::_detail::libc::c_void,
            };
        getset_def.name = concat!(stringify!($name), "\0").as_ptr() as *mut _;

        $( getset_def.$descr_name = Some($descr_expr); )*

        getset_def
    }};

    ( { getter = $meth:ident; $($tail:tt)* }
       $class:ident $py:ident $name:ident { $( $descr_name:ident = $descr_expr:expr; )* } ) =>
    {
        py_class_attribute_impl!{
            { $($tail)* } $class $py $name
            /* methods: */ {
                $( $descr_name = $descr_expr; )*
                get = {
                    unsafe extern "C" fn wrap_getter_method(
                        slf: *mut $crate::_detail::ffi::PyObject,
                        _: *mut $crate::_detail::libc::c_void)
                        -> *mut $crate::_detail::ffi::PyObject
                    {
                        const LOCATION: &'static str = concat!(
                            stringify!($class), ".getter_", stringify!($name), "()");

                        $crate::_detail::handle_callback(
                            LOCATION, $crate::_detail::PyObjectCallbackConverter,
                            |py| {
                                let slf = $crate::PyObject::from_borrowed_ptr(
                                    py, slf).unchecked_cast_into::<$class>();
                                let ret = slf.$meth(py);
                                $crate::PyDrop::release_ref(slf, py);
                                ret
                            })
                    }
                    wrap_getter_method
                };
            }
        }
    };

   ( { setter($value_type:ty) = $meth:ident; $($tail:tt)* }
       $class:ident $py:ident $name:ident { $( $descr_name:ident = $descr_expr:expr; )* } ) =>
    {
        py_class_attribute_impl! {
            { $($tail)* } $class $py $name
            /* methods: */ {
                $( $descr_name = $descr_expr; )*
                set = {
                    unsafe extern "C" fn wrap_setter_method(
                        slf: *mut $crate::_detail::ffi::PyObject,
                        value: *mut $crate::_detail::ffi::PyObject,
                        _: *mut $crate::_detail::libc::c_void)
                        -> $crate::_detail::libc::c_int
                    {
                        const LOCATION: &'static str = concat!(
                            stringify!($class), ".setter_", stringify!($name), "()");

                        $crate::_detail::handle_callback(
                            LOCATION, $crate::py_class::slots::UnitCallbackConverter, move |py| {
                                let slf = $crate::PyObject::from_borrowed_ptr(py, slf)
                                    .unchecked_cast_into::<$class>();
                                let value = $crate::PyObject::from_borrowed_ptr(py, value);

                                let ret =<$value_type as $crate::FromPyObject>::extract(py, &value)
                                    .and_then(|v| slf.$meth(py, v));
                                $crate::PyDrop::release_ref(slf, py);
                                $crate::PyDrop::release_ref(value, py);
                                ret
                            })
                    }
                    wrap_setter_method
                };
            }
        }
    };
}
