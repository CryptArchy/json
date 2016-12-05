use value::Value;
use pointer::Pointer;

/*
  [
     { "op": "test", "path": "/a/b/c", "value": "foo" },
     { "op": "remove", "path": "/a/b/c" },
     { "op": "add", "path": "/a/b/c", "value": [ "foo", "bar" ] },
     { "op": "replace", "path": "/a/b/c", "value": 42 },
     { "op": "move", "from": "/a/b/c", "path": "/a/b/d" },
     { "op": "copy", "from": "/a/b/d", "path": "/a/b/e" }
   ]
*/

pub enum PatchCommand {
    Test(&str, Value),
    Add(&str, Value),
    Replace(&str, Value),
    Remove(&str),
    Move(&str, &str),
    Copy(&str, &str),
}

/// Provides the `patch` method for manipulating objects using `Pointer` and various other operations.
pub trait Patcher : Pointer {
    fn patch(self, cmd:PatchCommand) -> Result<Value,String>;
    fn patch_test(&self, path:&str, value:Value) -> Result<Value,String> {
        match self.pointer(path) {
            Some(lhs) => Ok(Value::Bool(lhs == value)),
            None => Err("Path was not valid.".to_owned()),
        }
    }
}