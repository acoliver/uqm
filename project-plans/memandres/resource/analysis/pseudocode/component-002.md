# Component 002: Config API (Get/Put/Save)

Replaces C config functions in `resinit.c` (lines 466-651) and `SaveResourceIndex`.

## Pseudocode: process_resource_desc

```
 1: FUNCTION process_resource_desc(key: &str, value: &str)
 2:   LET map = get_current_index()
 3:
 4:   // Split TYPE:path on first ':'
 5:   LET (type_name, path) = MATCH value.find(':'):
 6:     Some(pos):
 7:       (value[0..pos], value[pos+1..])
 8:     None:
 9:       log_warning("Could not find type information for resource '{}'", key)
10:       ("UNKNOWNRES", value)
11:
12:   // Look up type handler
13:   LET type_key = format!("sys.{}", type_name)
14:   LET handler_desc = map.get(&type_key)
15:   IF handler_desc IS None:
16:     log_warning("Illegal type '{}' for resource '{}'; treating as UNKNOWNRES", type_key, key)
17:     handler_desc = map.get("sys.UNKNOWNRES")
18:
19:   LET vtable = handler_desc.resdata.ptr AS *const ResourceHandlers
20:
21:   // Validate loadFun exists
22:   IF vtable.load_fun IS None:
23:     log_warning("Unable to load '{}'; no handler for type {} defined", key, type_key)
24:     RETURN
25:
26:   // Create new ResourceDesc
27:   LET desc = ResourceDesc {
28:     fname: CString::new(path),
29:     vtable: vtable,
30:     resdata: zeroed ResourceData,
31:     refcount: 0,
32:   }
33:
34:   // Value types: parse immediately. Heap types: defer.
35:   IF vtable.free_fun IS None:
36:     CALL vtable.load_fun(desc.fname.as_ptr(), &mut desc.resdata)
37:   ELSE:
38:     desc.resdata.ptr = null_mut()
39:
40:   // Insert into map (replace if exists)
41:   IF map.contains_key(key):
42:     res_remove_internal(key)
43:   map.insert(key.to_string(), desc)
44: END FUNCTION
```

## Pseudocode: res_PutString

```
 1: FUNCTION res_PutString(key: &str, value: &str)
 2:   LET map = get_current_index()
 3:   LET desc = map.get_mut(key)
 4:
 5:   // Auto-create if missing or wrong type
 6:   IF desc IS None OR desc.vtable.res_type != "STRING":
 7:     process_resource_desc(key, "STRING:undefined")
 8:     desc = map.get_mut(key)
 9:
10:   // Update fname and resdata.str to new value
11:   LET new_cstring = CString::new(value)
12:   desc.resdata.str_ptr = new_cstring.as_ptr()
13:   desc.fname = new_cstring
14: END FUNCTION
```

## Pseudocode: res_PutInteger

```
 1: FUNCTION res_PutInteger(key: &str, value: i32)
 2:   LET map = get_current_index()
 3:   LET desc = map.get_mut(key)
 4:
 5:   IF desc IS None OR desc.vtable.res_type != "INT32":
 6:     process_resource_desc(key, "INT32:0")
 7:     desc = map.get_mut(key)
 8:
 9:   desc.resdata.num = value as u32
10: END FUNCTION
```

## Pseudocode: res_PutBoolean

```
 1: FUNCTION res_PutBoolean(key: &str, value: bool)
 2:   LET map = get_current_index()
 3:   LET desc = map.get_mut(key)
 4:
 5:   IF desc IS None OR desc.vtable.res_type != "BOOLEAN":
 6:     process_resource_desc(key, "BOOLEAN:false")
 7:     desc = map.get_mut(key)
 8:
 9:   desc.resdata.num = IF value THEN 1 ELSE 0
10: END FUNCTION
```

## Pseudocode: res_PutColor

```
 1: FUNCTION res_PutColor(key: &str, color: Color)
 2:   LET map = get_current_index()
 3:   LET desc = map.get_mut(key)
 4:
 5:   IF desc IS None OR desc.vtable.res_type != "COLOR":
 6:     process_resource_desc(key, "COLOR:rgb(0, 0, 0)")
 7:     desc = map.get_mut(key)
 8:
 9:   desc.resdata.num = (color.r << 24) | (color.g << 16) | (color.b << 8) | color.a
10: END FUNCTION
```

## Pseudocode: SaveResourceIndex

```
 1: FUNCTION save_resource_index(dir, filename, root: Option<&str>, strip_root: bool)
 2:   LET file = res_open_res_file(dir, filename, "wb")
 3:   IF file IS None: RETURN  // silent failure
 4:
 5:   LET prefix_len = root.map(|r| r.len()).unwrap_or(0)
 6:
 7:   FOR (key, desc) IN map.iter():
 8:     // Skip entries not matching root prefix
 9:     IF root IS Some(r) AND NOT key.starts_with(r): CONTINUE
10:
11:     // Skip entries without valid vtable or toString
12:     IF desc.vtable IS NULL:
13:       CONTINUE  // type registration entries have vtable=NULL
14:     IF desc.vtable.to_string IS None:
15:       CONTINUE  // no serializer
16:
17:     // Serialize value
18:     LET mut buf = [0u8; 256]
19:     CALL desc.vtable.to_string(&desc.resdata, buf.as_mut_ptr(), 256)
20:
21:     // Write line: key = TYPE:serialized_value\n
22:     LET output_key = IF strip_root AND root IS Some(_):
23:       &key[prefix_len..]
24:     ELSE:
25:       key
26:     write_res_file(output_key, f)
27:     write_res_file(" = ", f)
28:     write_res_file(desc.vtable.res_type, f)
29:     write_res_file(":", f)
30:     write_res_file(buf_as_str, f)
31:     put_res_file_newline(f)
32:
33:   res_close_res_file(file)
34: END FUNCTION
```

## Validation Points
- Line 6 (process_resource_desc): First `:` only — subsequent colons are part of path
- Lines 5-8 (Put functions): Auto-create on missing/wrong-type
- Line 9 (SaveResourceIndex): Root prefix filtering
- Line 12-15 (SaveResourceIndex): Skip type entries and entries without toString
- Line 22-25 (SaveResourceIndex): Optional root stripping
