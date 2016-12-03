//! JSON Pointer
//!
//! This module presents JSON Pointers through the `Pointer` trait and implements it for the `Value` type.
//!
//! For more information read [RFC6901](https://tools.ietf.org/html/rfc6901).

use value::Value;

/// Provides the `pointer` method for locating values within an object using a string path.
pub trait Pointer {
    /// Looks up a value by a JSON Pointer.
    ///
    /// JSON Pointer defines a string syntax for identifying a specific value
    /// within a JavaScript Object Notation (JSON) document.
    ///
    /// A Pointer is a Unicode string with the reference tokens separated by `/`.
    /// Inside tokens `/` is replaced by `~1` and `~` is replaced by `~0`. The
    /// addressed value is returned and if there is no such value `None` is
    /// returned.
    fn pointer<'a>(&'a self, pointer: &str) -> Option<&'a Value>;

    /// Looks up a value by a JSON Pointer and returns a mutable reference to
    /// that value.
    ///
    /// JSON Pointer defines a string syntax for identifying a specific value
    /// within a JavaScript Object Notation (JSON) document.
    ///
    /// A Pointer is a Unicode string with the reference tokens separated by `/`.
    /// Inside tokens `/` is replaced by `~1` and `~` is replaced by `~0`. The
    /// addressed value is returned and if there is no such value `None` is
    /// returned.
    fn pointer_mut<'a>(&'a mut self, pointer: &str) -> Option<&'a mut Value>;

    /// Looks up a value by a JSON Pointer while consuming the object to return
    /// the value as owned, immutable data.
    ///
    /// JSON Pointer defines a string syntax for identifying a specific value
    /// within a JavaScript Object Notation (JSON) document.
    ///
    /// A Pointer is a Unicode string with the reference tokens separated by `/`.
    /// Inside tokens `/` is replaced by `~1` and `~` is replaced by `~0`. The
    /// addressed value is returned and if there is no such value `None` is
    /// returned.
    fn pointer_owned(self, pointer: &str) -> Option<Value>;
}

fn parse_index(s: &str) -> Option<usize> {
    if s.starts_with('+') || (s.starts_with('0') && s.len() != 1) {
        return None;
    }
    s.parse().ok()
}

impl Pointer for Value {
    fn pointer<'a>(&'a self, pointer: &str) -> Option<&'a Value> {
        if pointer == "" {
            return Some(self);
        }
        if !pointer.starts_with('/') {
            return None;
        }
        let tokens = pointer.split('/').skip(1).map(|x| x.replace("~1", "/").replace("~0", "~"));
        let mut target = self;

        for token in tokens {
            let target_opt = match *target {
                Value::Object(ref map) => map.get(&token[..]),
                Value::Array(ref list) => {
                    parse_index(&token[..]).and_then(|x| list.get(x))
                }
                _ => return None,
            };
            if let Some(t) = target_opt {
                target = t;
            } else {
                return None;
            }
        }
        Some(target)
    }

    fn pointer_mut<'a>(&'a mut self, pointer: &str) -> Option<&'a mut Value> {
        if pointer == "" {
            return Some(self);
        }
        if !pointer.starts_with('/') {
            return None;
        }
        let tokens = pointer.split('/').skip(1).map(|x| x.replace("~1", "/").replace("~0", "~"));
        let mut target = self;

        for token in tokens {
            let tgt = target;
            let target_opt = match *tgt {
                Value::Object(ref mut map) => map.get_mut(&token[..]),
                Value::Array(ref mut list) => {
                    if let Some(idx) = parse_index(&token[..]) {
                        list.get_mut(idx)
                    }
                    else {
                        None
                    }
                }
                _ => return None,
            };
            if let Some(t) = target_opt {
                target = t;
            } else {
                return None;
            }
        }
        Some(target)
    }

    fn pointer_owned(self, pointer: &str) -> Option<Value> {
        if pointer == "" {
            return Some(self);
        }
        if !pointer.starts_with('/') {
            return None;
        }
        let tokens = pointer.split('/').skip(1).map(|x| x.replace("~1", "/").replace("~0", "~"));
        let mut target = self;

        for token in tokens {
            let target_opt = match target {
                Value::Object(mut map) => map.remove(&token[..]),
                Value::Array(mut list) => {
                    parse_index(&token[..]).and_then(|x| Some(list.remove(x)))
                }
                _ => return None,
            };
            if let Some(t) = target_opt {
                target = t;
            } else {
                return None;
            }
        }
        Some(target)
    }
}