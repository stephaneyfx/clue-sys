// Copyright (C) 2017 Stephane Raux. Distributed under the MIT license.

//! FFI to/from Rust type conversions.

use ClueStringView;
use std::any::Any;
use std::borrow::Cow;
use std::os::raw::{c_char, c_void};
use std::panic::{catch_unwind, resume_unwind, UnwindSafe};
use std::slice;
use std::str::{self, Utf8Error};

/// Creates a `ClueStringView` from a string slice.
pub fn to_string_view(s: &str) -> ClueStringView {
    ClueStringView {
        s: s.as_ptr() as *const c_char,
        len: s.len(),
    }
}

/// Creates a string slice from a string view.
pub unsafe fn from_string_view<'a>(s: ClueStringView)
        -> Result<&'a str, Utf8Error> {
    str::from_utf8(slice::from_raw_parts(s.s as *const u8, s.len))
}

/// Creates a string slice from a string view, including invalid characters.
///
/// For cases where one does not trust that a `ClueStringView` value contains
/// UTF-8 as the documentation says it should.
pub unsafe fn from_string_view_lossy<'a>(s: ClueStringView) -> Cow<'a, str> {
    String::from_utf8_lossy(slice::from_raw_parts(s.s as *const u8, s.len))
}

/// Type of results sent across the FFI.
/// The outer error type is meant to store a potential panic.
pub type FfiResult<T, E> = Result<Result<T, E>, Box<Any + Send>>;

/// Trait for Rust types that can be converted from a C type by passing a
/// callback to the C code.
pub trait FromFfi: Sized {
    /// C type to convert from.
    type FfiType: UnwindSafe;

    /// Conversion error type.
    type Error;

    /// Method in charge of the conversion.
    unsafe fn from_ffi(data: Self::FfiType) -> Result<Self, Self::Error>;

    /// Wrapper around `from_ffi` suitable to be called from C code. It does not
    /// panick.
    ///
    /// `env` must be a pointer to `Option<FfiResult<Self, Self::Error>>`.
    unsafe extern fn get_ffi_value(env: *mut c_void, data: Self::FfiType) {
        let out = &mut *(env as *mut Option<FfiResult<Self, Self::Error>>);
        *out = Some(catch_unwind(|| Self::from_ffi(data)));
    }
}

impl FromFfi for String {
    type FfiType = ClueStringView;
    type Error = Utf8Error;

    unsafe fn from_ffi(data: Self::FfiType) -> Result<String, Utf8Error> {
        from_string_view(data).map(|s| s.to_string())
    }
}

/// Wraps a function that takes a callback to convert from a FFI type to a
/// Rust type.
///
/// The void pointers are expected to be pointers to
/// `Option<FfiResult<T, T::Error>>`.
pub unsafe fn get_ffi_value<T, F>(f: F) -> Result<T, T::Error>
    where
        T: FromFfi,
        F: FnOnce(*mut c_void, Option<unsafe extern fn(*mut c_void,
            T::FfiType)>) {
    let mut s = Option::None::<FfiResult<T, T::Error>>;
    f(&mut s as *mut _ as *mut c_void, Some(T::get_ffi_value));
    match s {
        Some(Ok(r)) => r,
        Some(Err(e)) => resume_unwind(e),
        None => panic!("The foreign code failed to set a value."),
    }
}

#[cfg(test)]
mod tests {
    use ClueStringView;
    use std::os::raw::c_void;
    use super::{get_ffi_value, to_string_view};

    #[test]
    fn round_trip() {
        const INPUT: &str = "blue";
        unsafe fn f(env: *mut c_void, callback: Option<unsafe extern fn(
                *mut c_void, ClueStringView)>) {
            callback.unwrap()(env, to_string_view(INPUT));
        }
        unsafe {
            let after: String = get_ffi_value(|e, c| f(e, c)).unwrap();
            assert_eq!(INPUT, after);
        }
    }
}
