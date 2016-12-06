//! JSON Patch
//!
//! This module implements JSON Patch behavior as described in [RFC6902](https://tools.ietf.org/html/rfc6902).

use std::error;
use std::fmt;
use std::io;
use std::mem;
use std::result;
use std::str;
use std::vec;

use value::Value;

#[derive(Debug)]
pub enum Match<T> {
    Found(T),
    Invalid(&'static str),
    MapMissing(String, T),
    ArrayMissing(u64, T),
}

#[derive(Debug)]
pub enum Error {
    ObjectPathInvalid,
    ObjectPathMissing,
    ArrayIndexInvalid,
    ArrayIndexBeyondLength,
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::ObjectPathInvalid => "JSON Pointer was not valid.",
            Error::ObjectPathMissing => {
                "JSON Pointer could not find some specified object members."
            }
            Error::ArrayIndexInvalid => {
                "JSON Pointer to array index was not valid."
            }
            Error::ArrayIndexBeyondLength => {
                "JSON Pointer to array index exceeded length of array."
            }
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            _ => None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        use std::error::Error;
        write!(fmt, "{}", self.description());
        Ok(())
    }
}

/// Helper alias for `Result` objects that return a JSON Patch `Error`.
pub type Result<T> = result::Result<T, Error>;


// [
// { "op": "test", "path": "/a/b/c", "value": "foo" },
// { "op": "remove", "path": "/a/b/c" },
// { "op": "add", "path": "/a/b/c", "value": [ "foo", "bar" ] },
// { "op": "replace", "path": "/a/b/c", "value": 42 },
// { "op": "move", "from": "/a/b/c", "path": "/a/b/d" },
// { "op": "copy", "from": "/a/b/d", "path": "/a/b/e" }
// ]
//

// pub enum PatchCommand {
//     Test(&str, Value),
//     Add(&str, Value),
//     Replace(&str, Value),
//     Remove(&str),
//     Move(&str, &str),
//     Copy(&str, &str),
// }


/// Provides the `patch` method for manipulating objects using `Pointer` and various other operations.
pub trait Patcher {
    // fn patch(self, cmd:PatchCommand) -> Result<Value,String>;
    // fn pointer_patch<'a>(&'a mut self, pointer: &str) -> Match<&'a mut Value>;
    fn patch_add(&mut self, path: &str, value: Value) -> Option<()>;
    fn patch_replace(&mut self, path: &str, value: Value) -> Option<()>;
    fn patch_remove(&mut self, path: &str) -> Option<()>;
    fn patch_move(&mut self, path: &str, from: &str) -> Option<()>;
    fn patch_copy(&mut self, path: &str, from: &str) -> Option<()>;
    fn patch_test(&mut self, path: &str, value: Value) -> Option<()>;
}

impl Patcher for Value {
    fn patch_add(&mut self, path: &str, value: Value) -> Option<()> {
        if path == "/" {
            mem::replace(self, value);
            return Some(());
        }

        let (target, parent_path) = break_path(path);
        match self.pointer_mut(parent_path) {
            Some(&mut Value::Object(ref mut map)) => {
                map.insert(target.to_owned(), value);
                Some(())
            }
            Some(&mut Value::Array(ref mut list)) => {
                if target == "-" {
                    list.push(value);
                    Some(())
                } else {
                    parse_index(&target).and_then(|x| if x < list.len() {
                        list.insert(x, value);
                        Some(())
                    } else {
                        None
                    })
                }
            }
            _ => None,
        };
        return Some(());
    }

    fn patch_replace(&mut self, path: &str, value: Value) -> Option<()> {
        match self.pointer_mut(path){
            Some(target) => { mem::replace(target, value); Some(()) },
            _ => None
        }
    }

    fn patch_move(&mut self, path: &str, from: &str) -> Option<()> {

    }

    fn patch_copy(&mut self, path: &str, from: &str) -> Option<()> {
        match (self.pointer(from), self.pointer_mut(path)) {
            (Some(source), Some(target)) => {
                mem::replace(target, source.clone());
            }
        }
    }
}

fn break_path(path: &str) -> (String, &str) {
    let parts: Vec<&str> = path.rsplitn(2, '/').collect();
    (parts[0].replace("~1", "/").replace("~0", "~"), parts[1])
}

fn parse_index(s: &str) -> Option<usize> {
    if s.starts_with('+') || (s.starts_with('0') && s.len() != 1) {
        return None;
    }
    s.parse().ok()
}

#[cfg(test)]
mod tests {
    #[test]
    fn basic_test() {
        assert!(true);
    }
}