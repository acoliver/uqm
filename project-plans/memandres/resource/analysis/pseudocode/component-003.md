# Component 003: Resource Dispatch (Type Registration, Lazy Loading, Refcount)

Replaces C functions in `getres.c` and type registration in `resinit.c`.

## Pseudocode: InstallResTypeVectors

```
 1: FUNCTION install_res_type_vectors(res_type: &str, load_fun, free_fun, to_string_fun) -> bool
 2:   LET map = get_current_index()
 3:
 4:   // Build type key: "sys.<TYPE>"
 5:   LET key = format!("sys.{}", res_type)
 6:   IF key.len() >= 32: key.truncate(31)
 7:
 8:   // Allocate handlers struct
 9:   LET handlers = Box::new(ResourceHandlers {
10:     res_type: res_type as *const c_char,  // static lifetime expected
11:     load_fun: load_fun,
12:     free_fun: free_fun,
13:     to_string: to_string_fun,
14:   })
15:   LET handlers_ptr = Box::into_raw(handlers) as *mut c_void
16:
17:   // Create descriptor for the type entry
18:   LET desc = ResourceDesc {
19:     fname: CString::new(res_type),
20:     vtable: null(),       // NULL vtable distinguishes type entries
21:     resdata: ResourceData { ptr: handlers_ptr },
22:     refcount: 0,
23:   }
24:
25:   map.insert(key, desc)
26:   RETURN true
27: END FUNCTION
```

## Pseudocode: res_GetResource

```
 1: FUNCTION res_get_resource(key: &str) -> *mut c_void
 2:   IF key IS NULL:
 3:     log_warning("Trying to get null resource")
 4:     RETURN null
 5:
 6:   LET map = get_current_index()
 7:   LET desc = map.get_mut(key)
 8:
 9:   IF desc IS None:
10:     log_warning("Trying to get undefined resource '{}'", key)
11:     RETURN null
12:
13:   // Lazy load if not yet loaded
14:   IF desc.resdata.ptr IS null:
15:     load_resource_desc(desc)
16:
17:   // Check if load succeeded
18:   IF desc.resdata.ptr IS null:
19:     RETURN null  // load failed, don't increment refcount
20:
21:   desc.refcount += 1
22:   RETURN desc.resdata.ptr
23: END FUNCTION
```

## Pseudocode: load_resource_desc

```
 1: FUNCTION load_resource_desc(desc: &mut ResourceDesc)
 2:   // Call the type's loadFun via stored function pointer
 3:   IF desc.vtable IS NOT null AND desc.vtable.load_fun IS Some(load_fn):
 4:     unsafe { load_fn(desc.fname.as_ptr(), &mut desc.resdata) }
 5:   // After call, desc.resdata.ptr may be non-null (success) or null (failure)
 6: END FUNCTION
```

## Pseudocode: res_FreeResource

```
 1: FUNCTION res_free_resource(key: &str)
 2:   LET map = get_current_index()
 3:   LET desc = map.get_mut(key)
 4:
 5:   IF desc IS None:
 6:     log_warning("Trying to free undefined resource '{}'", key)
 7:     RETURN
 8:
 9:   IF desc.vtable IS null OR desc.vtable.free_fun IS None:
10:     log_warning("Trying to free a non-heap resource '{}'", key)
11:     RETURN
12:
13:   IF desc.resdata.ptr IS null:
14:     log_warning("Trying to free not loaded resource '{}'", key)
15:     RETURN
16:
17:   IF desc.refcount == 0:
18:     log_warning("Freeing an unreferenced resource '{}'", key)
19:
20:   IF desc.refcount > 0:
21:     desc.refcount -= 1
22:
23:   IF desc.refcount == 0:
24:     LET free_fn = desc.vtable.free_fun.unwrap()
25:     unsafe { free_fn(desc.resdata.ptr) }
26:     desc.resdata.ptr = null_mut()
27: END FUNCTION
```

## Pseudocode: res_DetachResource

```
 1: FUNCTION res_detach_resource(key: &str) -> *mut c_void
 2:   LET map = get_current_index()
 3:   LET desc = map.get_mut(key)
 4:
 5:   IF desc IS None:
 6:     log_warning("Trying to detach undefined resource '{}'", key)
 7:     RETURN null
 8:
 9:   IF desc.vtable IS null OR desc.vtable.free_fun IS None:
10:     log_warning("Trying to detach a non-heap resource")
11:     RETURN null
12:
13:   IF desc.resdata.ptr IS null:
14:     log_warning("Trying to detach not loaded resource '{}'", key)
15:     RETURN null
16:
17:   IF desc.refcount > 1:
18:     log_warning("Trying to detach a resource referenced {} times", desc.refcount)
19:     RETURN null
20:
21:   LET result = desc.resdata.ptr
22:   desc.resdata.ptr = null_mut()
23:   desc.refcount = 0
24:   RETURN result
25: END FUNCTION
```

## Pseudocode: res_Remove

```
 1: FUNCTION res_remove(key: &str) -> bool
 2:   LET map = get_current_index()
 3:   LET old_desc = map.get(key)
 4:
 5:   IF old_desc IS Some(desc):
 6:     IF desc.resdata.ptr IS NOT null:
 7:       IF desc.refcount > 0:
 8:         log_warning("Replacing '{}' while it is live", key)
 9:       IF desc.vtable IS NOT null AND desc.vtable.free_fun IS Some(free_fn):
10:         unsafe { free_fn(desc.resdata.ptr) }
11:     // Remove from map — Rust drops fname CString automatically
12:     map.remove(key)
13:     RETURN true
14:   ELSE:
15:     RETURN false  // key not found, C CharHashTable_remove returns false
16: END FUNCTION
```

## Pseudocode: LoadResourceFromPath

```
 1: FUNCTION load_resource_from_path(pathname: *const c_char, load_fn: fn) -> *mut c_void
 2:   LET stream = res_open_res_file(contentDir, pathname, "rb")
 3:   IF stream IS null:
 4:     log_warning("Cannot open resource file '{}'", pathname)
 5:     RETURN null
 6:
 7:   LET length = length_res_file(stream)
 8:   IF length == 0:
 9:     log_warning("Zero-length resource file '{}'", pathname)
10:     res_close_res_file(stream)
11:     RETURN null
12:
13:   // Set global for loaders that need current filename
14:   SET _cur_resfile_name = pathname
15:
16:   LET result = unsafe { load_fn(stream, length) }
17:
18:   SET _cur_resfile_name = null
19:   res_close_res_file(stream)
20:   RETURN result
21: END FUNCTION
```

## Ordering Constraints
- `InstallResTypeVectors` MUST be called before any `LoadResourceIndex`
- `InitResourceSystem` registers all 14 types before returning
- C subsystem init (`InstallGraphicResTypes`, etc.) runs after `InitResourceSystem`
  but before any `LoadResourceIndex` calls for content
- `_cur_resfile_name` must be set/cleared around `loadFun` calls

## Side Effects
- `load_resource_desc` calls C function pointers that may open files via UIO
- `res_Remove` calls C `freeFun` that may deallocate subsystem memory
- `res_PutString` replaces Rust-owned CString, invalidating prior `res_GetString` pointers
- `SaveResourceIndex` writes files via UIO
