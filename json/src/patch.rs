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

pub enum Command {
    Test(String, Value),
    // NTest(&str, Value),
    Add(String, Value),
    // Path must exist, same as Remove then Add
    Replace(String, Value),
    // Path must exist
    Remove(String),
    // Path must exist, same as Remove then Add
    Move(String, String),
    // Path must exist, same as Read then Add
    Copy(String, String),
    // Undoes Move, same as Replace and then Move
    _Bump(String, String, Value),
}

/// Provides the `patch` method for manipulating objects using `Pointer` and various other operations.
pub trait Patcher {
    fn patch(&mut self, patch: &str) -> Result<Vec<Command>>;
    fn apply_patch(&mut self, cmds: Vec<Command>) -> Result<Vec<Command>>;
    fn apply_patch_command(&mut self, cmd: Command) -> Result<Command>;
    fn patch_add(&mut self, path: &str, value: Value) -> Result<Option<Value>>;
    fn patch_move(&mut self, from: &str, path: &str) -> Result<Option<Value>>;
    fn patch_copy(&mut self, from: &str, path: &str) -> Result<Option<Value>>;
    fn patch_bump(
        &mut self,
        from: &str,
        path: &str,
        value: Value
    ) -> Result<Option<Value>>;
    fn patch_replace(&mut self, path: &str, value: Value) -> Result<Value>;
    fn patch_remove(&mut self, path: &str) -> Result<Value>;
    fn patch_test(&self, path: &str, value: Value) -> Result<Value>;
}

impl Patcher for Value {
    fn patch(&mut self, patch: &str) -> Result<Vec<Command>> {
        unimplemented!()
    }

    fn apply_patch(&mut self, cmds: Vec<Command>) -> Result<Vec<Command>> {
        let mut rollbacks = Vec::with_capacity(cmds.len());

        for cmd in cmds {
            match self.apply_patch_command(cmd) {
                Ok(rev) => rollbacks.push(rev),
                Err(err) => {
                    while let Some(rb) = rollbacks.pop() {
                        self.apply_patch_command(rb);
                    }
                    return Err(err);
                }
            }
        }
        Ok(rollbacks)
    }

    fn apply_patch_command(&mut self, cmd: Command) -> Result<Command> {
        match cmd {
            Command::Test(path, value) => {
                match self.patch_test(&path, value) {
                    Ok(v) => Ok(Command::Test(path, v)),
                    Err(Error::TestNotEqual(_, src, _)) => {
                        Ok(Command::Test(path, src))
                    }
                    Err(err) => Err(err),
                }
            }
            Command::Remove(path) => {
                self.patch_remove(&path).map(|v| Command::Add(path, v))
            }
            Command::Replace(path, value) => {
                self.patch_replace(&path, value)
                    .map(|v| Command::Replace(path, v))
            }
            Command::Add(path, value) => {
                match self.patch_add(&path, value) {
                    Ok(None) => Ok(Command::Remove(path)),
                    Ok(Some(v)) => Ok(Command::Replace(path, v)),
                    Err(err) => Err(err),
                }
            }
            Command::Copy(from, path) => {
                match self.patch_copy(&from, &path) {
                    Ok(None) => Ok(Command::Remove(path)),
                    Ok(Some(v)) => Ok(Command::Replace(path, v)),
                    Err(err) => Err(err),
                }
            }
            Command::Move(from, path) => {
                match self.patch_move(&from, &path) {
                    Ok(None) => Ok(Command::Move(path, from)),
                    Ok(Some(v)) => Ok(Command::_Bump(path, from, v)),
                    Err(err) => Err(err),
                }
            }
            Command::_Bump(from, path, value) => {
                match self.patch_bump(&from, &path, value) {
                    Ok(None) => Ok(Command::Move(path, from)),
                    Ok(Some(v)) => Ok(Command::_Bump(path, from, v)),
                    Err(err) => Err(err),
                }
            }
        }
    }

    fn patch_add(&mut self, path: &str, value: Value) -> Result<Option<Value>> {
        if path == "/" {
            return Ok(Some(mem::replace(self, value)));
        }

        let (target, parent_path) = break_path(path);
        match self.pointer_mut(parent_path) {
            Some(&mut Value::Object(ref mut map)) => {
                Ok(map.insert(target.to_owned(), value))
            }
            Some(&mut Value::Array(ref mut list)) => {
                if target == "-" {
                    list.push(value);
                    Ok(None)
                } else {
                    match parse_index(&target) {
                        Some(idx) if idx < list.len() => {
                            list.insert(idx, value);
                            Ok(None)
                        }
                        _ => {
                            Err(Error::InvalidPath(path.to_owned(),
                                                   Some(value)))
                        }
                    }
                }
            }
            _ => Err(Error::InvalidPath(path.to_owned(), Some(value))),
        }
    }

    fn patch_replace(&mut self, path: &str, value: Value) -> Result<Value> {
        match self.pointer_mut(path) {
            Some(ref mut target) => Ok(mem::replace(target, value)),
            _ => Err(Error::InvalidPath(path.to_owned(), Some(value))),
        }
    }

    fn patch_remove(&mut self, path: &str) -> Result<Value> {
        if path == "/" {
            return Ok(mem::replace(self, Value::Null));
        }

        let (target, parent_path) = break_path(path);
        match self.pointer_mut(parent_path) {
            Some(&mut Value::Object(ref mut map)) => {
                map.remove(&target)
                    .ok_or(Error::InvalidPath(path.to_owned(), None))
            }
            Some(&mut Value::Array(ref mut list)) => {
                if target == "-" {
                    list.pop().ok_or(Error::InvalidPath(path.to_owned(), None))
                } else {
                    parse_index(&target)
                        .ok_or(Error::InvalidPath(path.to_owned(), None))
                        .and_then(|x| if x < list.len() {
                            Ok(list.remove(x))
                        } else {
                            Err(Error::InvalidPath(path.to_owned(), None))
                        })
                }
            }
            _ => Err(Error::InvalidPath(path.to_owned(), None)),
        }
    }

    fn patch_move(&mut self, from: &str, path: &str) -> Result<Option<Value>> {
        match self.patch_remove(from) {
            Err(err) => Err(err),
            Ok(source) => {
                match self.patch_add(path, source) {
                    Err(Error::InvalidPath(p, Some(src))) => {
                        self.patch_add(from, src);
                        Err(Error::InvalidPath(p, None))
                    }
                    res => res,
                }
            }
        }
    }

    fn patch_bump(
        &mut self,
        from: &str,
        path: &str,
        value: Value
    ) -> Result<Option<Value>> {
        match self.patch_replace(from, value) {
            Err(err) => Err(err),
            Ok(source) => {
                match self.patch_add(path, source) {
                    Err(Error::InvalidPath(p, Some(src))) => {
                        let val = self.patch_replace(from, src).unwrap();
                        Err(Error::InvalidPath(p, Some(val)))
                    }
                    res => res,
                }
            }
        }
    }

    fn patch_copy(&mut self, from: &str, path: &str) -> Result<Option<Value>> {
        self.pointer(from)
            .ok_or(Error::InvalidPath(path.to_owned(), None))
            .map(|src| src.clone())
            .and_then(|src| self.patch_add(path, src))
    }

    fn patch_test(&self, path: &str, value: Value) -> Result<Value> {
        match self.pointer(path) {
            None => Err(Error::InvalidPath(path.to_owned(), Some(value))),
            Some(source) => {
                if value == *source {
                    Ok(value)
                } else {
                    Err(Error::TestNotEqual(path.to_owned(),
                                            source.clone(),
                                            value))
                }
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

#[derive(Debug)]
pub enum Error {
    BadPatch,
    InvalidOp(String),
    InvalidPath(String, Option<Value>),
    TestNotEqual(String, Value, Value),
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::BadPatch => "Patch is not well structured",
            Error::InvalidOp(..) => "Unknown operation",
            Error::InvalidPath(..) => "Path does not point to a value",
            Error::TestNotEqual(..) => {
                "Value at Path was not equal to test value"
            }
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        None
    }
}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::BadPatch => write!(fmt, "Patch is not well structured"),
            Error::InvalidOp(ref op) => {
                write!(fmt, "Operation {} is unknown", op)
            }
            Error::InvalidPath(ref path, ..) => {
                write!(fmt, "Path {} does not point to a value", path)
            }
            Error::TestNotEqual(ref path, ref src, ref val) => {
                write!(fmt,
                       "Path {} value of {} was not equal to test value {}",
                       path,
                       src,
                       val)
            }
        }
    }
}

/// Helper alias for `Result` objects that return a JSON `Error`.
pub type Result<T> = result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use de::from_str;
    use super::{Patcher, Error, Result};
    use value::{Value, Map};

    // [
    // { "op": "test", "path": "/a/b/c", "value": "foo" },
    // { "op": "remove", "path": "/a/b/c" },
    // { "op": "add", "path": "/a/b/c", "value": [ "foo", "bar" ] },
    // { "op": "replace", "path": "/a/b/c", "value": 42 },
    // { "op": "move", "from": "/a/b/c", "path": "/a/b/d" },
    // { "op": "copy", "from": "/a/b/d", "path": "/a/b/e" }
    // ]
    //

    #[test]
    fn test_patch_test() {
        let json_obj = r#"{ "a": { "b": { "c": 1, "carr": [9,8,7], "cint": 10, "cobj": { "x":0, "y":1.0 }}}}"#;
        let obj: Value = from_str(json_obj).unwrap();

        // Test successful comparison
        assert_eq!(obj.patch_test("/a/b/c", Value::U64(1)).unwrap(),
                   Value::U64(1));

        // Test failed comparison (values not equal)
        assert!(match obj.patch_test("/a/b/c", Value::U64(0)) {
            Err(Error::TestNotEqual(ref path,
                                    Value::U64(1),
                                    Value::U64(0))) if path == "/a/b/c" => true,
            _ => false,
        });

        // Test failed comparison (path does not exist)
        assert!(match obj.patch_test("/a/b/d", Value::U64(1)) {
            Err(Error::InvalidPath(ref path, Some(Value::U64(1)))) if path ==
                                                                      "/a/b/d" => {
                true
            }
            _ => false,
        });

        // Test successful comparison on arrays (order matters)
        assert!(match obj.patch_test("/a/b/carr",
                                     Value::Array(vec![Value::U64(9),
                                                       Value::U64(8),
                                                       Value::U64(7)])) {
            Ok(Value::Array(ref vtest)) => true,
            _ => false,
        });

        // Test failed comparison on arrays (order matters)
        assert!(match obj.patch_test("/a/b/carr",
                                     Value::Array(vec![Value::U64(7),
                                                       Value::U64(8),
                                                       Value::U64(9)])) {
            Err(Error::TestNotEqual(ref path,
                                    Value::Array(ref vsource),
                                    Value::Array(ref vtest))) if path == "/a/b/carr" => {
                true
            }
            _ => false,
        });

        // Test succsesful object comparison
        let mut map = Map::new();
        map.insert(String::from("x"), Value::U64(0));
        map.insert(String::from("y"), Value::F64(1.0));
        assert_eq!(obj.patch_test("/a/b/cobj", Value::Object(map.clone()))
                       .unwrap(),
                   Value::Object(map));
    }

    #[test]
    fn test_patch_add() {
        let json_obj = r#"{ "a": { "b": { "c": "foo", "carr": [9,8,7], "cint": 10, "cobj": { "x":0, "y":1.0 }}}}"#;
        let mut obj: Value = from_str(json_obj).unwrap();

        assert!(obj.patch_add("/a/b/d", Value::String("bar".to_owned()))
            .unwrap()
            .is_none());
        assert_eq!(obj.pointer("/a/b/d").unwrap(),
                   &Value::String("bar".to_owned()));

        assert_eq!(obj.patch_add("/a/b/cint", Value::U64(20))
                       .unwrap()
                       .unwrap(),
                   Value::U64(10));
        assert_eq!(obj.pointer("/a/b/cint").unwrap(), &Value::U64(20));

        assert!(obj.patch_add("/a/b/carr/-", Value::U64(6)).unwrap().is_none());
        assert_eq!(obj.pointer("/a/b/carr/3").unwrap(), &Value::U64(6));

        assert!(obj.patch_add("/a/b/carr/1", Value::U64(0)).unwrap().is_none());
        assert_eq!(obj.pointer("/a/b/carr/1").unwrap(), &Value::U64(0));
        assert_eq!(obj.pointer("/a/b/carr/4").unwrap(), &Value::U64(6));

        // Check values that should fail
        assert!(obj.patch_add("/a/b/c/d", Value::Null).is_err());
        assert!(obj.patch_add("/z/y/x", Value::Null).is_err());
        assert!(obj.patch_add("/a/d/c", Value::Null).is_err());
    }

    #[test]
    fn test_patch_replace() {
        let json_obj = r#"{ "a": { "b": { "c": "foo", "carr": [9,8,7], "cint": 10, "cobj": { "x":0, "y":1.0 }}}}"#;
        let mut obj: Value = from_str(json_obj).unwrap();

        assert_eq!(obj.patch_replace("/a/b/c",
                                      Value::String("bar".to_owned()))
                       .unwrap(),
                   Value::String("foo".to_owned()));
        assert_eq!(obj.pointer("/a/b/c").unwrap(),
                   &Value::String("bar".to_owned()));

        assert_eq!(obj.patch_replace("/a/b/cint", Value::U64(20)).unwrap(),
                   Value::U64(10));
        assert_eq!(obj.pointer("/a/b/cint").unwrap(), &Value::U64(20));

        assert_eq!(obj.patch_replace("/a/b/carr/1", Value::U64(0)).unwrap(),
                   Value::U64(8));
        assert_eq!(obj.pointer("/a/b/carr/1").unwrap(), &Value::U64(0));
        assert_eq!(obj.pointer("/a/b/carr/2").unwrap(), &Value::U64(7));
        assert_eq!(obj.pointer("/a/b/carr/4"), None);

        // Check values that should fail
        assert!(obj.patch_replace("/a/b/c/d", Value::Null).is_err());
        assert!(obj.patch_replace("/a/b/carr/-", Value::Null).is_err());
        assert!(obj.patch_replace("/a/b/carr/4", Value::Null).is_err());
    }

    #[test]
    fn test_patch_remove() {
        let json_obj = r#"{ "a": { "b": { "c": "foo", "carr": [9,8,7], "cint": 10, "cobj": { "x":0, "y":1.0 }}}}"#;
        let mut obj: Value = from_str(json_obj).unwrap();

        assert_eq!(obj.patch_remove("/a/b/c").unwrap(),
                   Value::String("foo".to_owned()));
        assert_eq!(obj.pointer("/a/b/c"), None);
    }

    #[test]
    fn test_patch_move() {
        let json_obj = r#"{ "a": { "b": { "c": "foo", "carr": [9,8,7], "cint": 10, "cobj": { "x":0, "y":1.0 }}}}"#;
        let mut obj: Value = from_str(json_obj).unwrap();

        assert!(obj.patch_move("/a/b/c", "/a/b/cstr").unwrap().is_none());
        assert_eq!(obj.pointer("/a/b/c"), None);
        assert_eq!(obj.pointer("/a/b/cstr").unwrap(),
                   &Value::String("foo".to_owned()));
    }

    #[test]
    fn test_patch_copy() {
        let json_obj = r#"{ "a": { "b": { "c": "foo", "carr": [9,8,7], "cint": 10, "cobj": { "x":0, "y":1.0 }}}}"#;
        let mut obj: Value = from_str(json_obj).unwrap();

        assert!(obj.patch_copy("/a/b/c", "/a/b/cstr").unwrap().is_none());
        assert_eq!(obj.pointer("/a/b/c").unwrap(),
                   &Value::String("foo".to_owned()));
        assert_eq!(obj.pointer("/a/b/cstr").unwrap(),
                   &Value::String("foo".to_owned()));
    }

    #[test]
    fn test_patch() {
        let json_obj =
            r#"{ "a": { "b": { "c": "foo", "carr": [9,8,7], "cint": 10 }}}"#;
        let obj: Value = from_str(json_obj).unwrap();
        let json_patch =
            r#"[{ "op": "test", "path": "/a/b/c", "value": "foo" }]"#;
        let patch: Value = from_str(json_patch).unwrap();
        assert!(true);
    }
}