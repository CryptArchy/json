    // TODO Delete this file if I decide not to use these functions

    // fn pointer_verbose<'a>(&'a self, pointer: &str) -> Result<&'a Value> {
    //     if pointer == "" {
    //         return Ok(self);
    //     }
    //     if !pointer.starts_with('/') {
    //         return Err(Error::InvalidPath(path.to_owned()));
    //     }
    //     let tokens = pointer.split('/').skip(1).map(|x| x.replace("~1", "/").replace("~0", "~"));
    //     let mut target = self;

    //     for token in tokens {
    //         let target_opt = match *target {
    //             Value::Object(ref map) => map.get(&token),
    //             Value::Array(ref list) => parse_index(&token).and_then(|x| list.get(x)),
    //             _ => return Err(Error::InvalidPath(path.to_owned())),
    //         };
    //         if let Some(t) = target_opt {
    //             target = t;
    //         } else {
    //             return Err(Error::InvalidPath(path.to_owned()));
    //         }
    //     }
    //     Ok(target)
    // }

    // fn pointer_mut_verbose<'a>(&'a mut self, pointer: &str) -> Result<&'a mut Value> {
    //     if pointer == "" {
    //         return Ok(self);
    //     }
    //     if !pointer.starts_with('/') {
    //         return Err(Error::InvalidPath(path.to_owned()));
    //     }
    //     let tokens = pointer.split('/').skip(1).map(|x| x.replace("~1", "/").replace("~0", "~"));
    //     let mut target = self;

    //     for token in tokens {
    //         // borrow checker gets confused about `target` being mutably borrowed too many times because of the loop
    //         // this once-per-loop binding makes the scope clearer and circumvents the error
    //         let target_once = target;
    //         let target_opt = match *target_once {
    //             Value::Object(ref mut map) => map.get_mut(&token),
    //             Value::Array(ref mut list) => parse_index(&token).and_then(move |x| list.get_mut(x)),
    //             _ => return Err(Error::InvalidPath(path.to_owned())),
    //         };
    //         if let Some(t) = target_opt {
    //             target = t;
    //         } else {
    //             return Err(Error::InvalidPath(path.to_owned()));
    //         }
    //     }
    //     Ok(target)
    // }