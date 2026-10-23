// Rust-owned engine ABI. Layout assertions preserve the clang/bindgen 0.70.1
// measurements from the C transition; this file is not generated during builds.

pub type CharHashTableKey = ::std::os::raw::c_char;
pub type CharHashTableValue = ::std::os::raw::c_void;
pub type UioBool = bool;
pub type UioUint32 = ::std::os::raw::c_uint;
pub type CharHashTableHashFunction =
    ::std::option::Option<unsafe extern "C" fn(arg1: *const CharHashTableKey) -> UioUint32>;
pub type CharHashTableEqualFunction = ::std::option::Option<
    unsafe extern "C" fn(arg1: *const CharHashTableKey, arg2: *const CharHashTableKey) -> UioBool,
>;
pub type CharHashTableCopyFunction = ::std::option::Option<
    unsafe extern "C" fn(arg1: *const CharHashTableKey) -> *mut CharHashTableValue,
>;
pub type CharHashTableFreeKeyFunction =
    ::std::option::Option<unsafe extern "C" fn(arg1: *mut CharHashTableKey)>;
pub type CharHashTableFreeValueFunction =
    ::std::option::Option<unsafe extern "C" fn(arg1: *mut CharHashTableValue)>;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct CharHashTableHashTable {
    pub hash_function: CharHashTableHashFunction,
    pub equal_function: CharHashTableEqualFunction,
    pub copy_function: CharHashTableCopyFunction,
    pub free_key_function: CharHashTableFreeKeyFunction,
    pub free_value_function: CharHashTableFreeValueFunction,
    pub min_fill_quotient: f64,
    pub max_fill_quotient: f64,
    pub min_size: UioUint32,
    pub max_size: UioUint32,
    pub size: UioUint32,
    pub hash_mask: UioUint32,
    pub entries: *mut *mut CharHashTableHashEntry,
    pub num_entries: UioUint32,
    pub num_collisions: UioUint32,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct CharHashTableHashEntry {
    pub hash: UioUint32,
    pub key: *mut CharHashTableKey,
    pub value: *mut CharHashTableValue,
    pub next: *mut CharHashTableHashEntry,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct CharHashTableIterator {
    pub hash_table: *const CharHashTableHashTable,
    pub bucket_nr: UioUint32,
    pub entry: *mut CharHashTableHashEntry,
}
extern "C" {
    #[link_name = "CharHashTable_newHashTable"]
    pub fn CharHashTablenewHashTable(
        hash_function: CharHashTableHashFunction,
        equal_function: CharHashTableEqualFunction,
        copy_function: CharHashTableCopyFunction,
        free_key_function: CharHashTableFreeKeyFunction,
        free_value_function: CharHashTableFreeValueFunction,
        initial_size: UioUint32,
        min_fill_quotient: f64,
        max_fill_quotient: f64,
    ) -> *mut CharHashTableHashTable;
}
extern "C" {
    #[link_name = "CharHashTable_add"]
    pub fn CharHashTableadd(
        hash_table: *mut CharHashTableHashTable,
        key: *const CharHashTableKey,
        value: *mut CharHashTableValue,
    ) -> UioBool;
}
extern "C" {
    #[link_name = "CharHashTable_remove"]
    pub fn CharHashTableremove(
        hash_table: *mut CharHashTableHashTable,
        key: *const CharHashTableKey,
    ) -> UioBool;
}
extern "C" {
    #[link_name = "CharHashTable_find"]
    pub fn CharHashTablefind(
        hash_table: *mut CharHashTableHashTable,
        key: *const CharHashTableKey,
    ) -> *mut CharHashTableValue;
}
extern "C" {
    #[link_name = "CharHashTable_count"]
    pub fn CharHashTablecount(hash_table: *const CharHashTableHashTable) -> UioUint32;
}
extern "C" {
    #[link_name = "CharHashTable_deleteHashTable"]
    pub fn CharHashTabledeleteHashTable(hash_table: *mut CharHashTableHashTable);
}
extern "C" {
    #[link_name = "CharHashTable_getIterator"]
    pub fn CharHashTablegetIterator(
        hash_table: *const CharHashTableHashTable,
    ) -> *mut CharHashTableIterator;
}
extern "C" {
    #[link_name = "CharHashTable_iteratorDone"]
    pub fn CharHashTableiteratorDone(
        iterator: *const CharHashTableIterator,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    #[link_name = "CharHashTable_iteratorKey"]
    pub fn CharHashTableiteratorKey(iterator: *mut CharHashTableIterator) -> *mut CharHashTableKey;
}
extern "C" {
    #[link_name = "CharHashTable_iteratorValue"]
    pub fn CharHashTableiteratorValue(
        iterator: *mut CharHashTableIterator,
    ) -> *mut CharHashTableValue;
}
extern "C" {
    #[link_name = "CharHashTable_iteratorNext"]
    pub fn CharHashTableiteratorNext(
        iterator: *mut CharHashTableIterator,
    ) -> *mut CharHashTableIterator;
}
extern "C" {
    #[link_name = "CharHashTable_freeIterator"]
    pub fn CharHashTablefreeIterator(iterator: *mut CharHashTableIterator);
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct StringTableEntry {
    _unused: [u8; 0],
}
pub type StringTableEntryDesc = StringTableEntry;
pub type StringHashTableKey = ::std::os::raw::c_char;
pub type StringHashTableValue = StringTableEntryDesc;
pub type StringHashTableHashFunction =
    ::std::option::Option<unsafe extern "C" fn(arg1: *const StringHashTableKey) -> UioUint32>;
pub type StringHashTableEqualFunction = ::std::option::Option<
    unsafe extern "C" fn(
        arg1: *const StringHashTableKey,
        arg2: *const StringHashTableKey,
    ) -> UioBool,
>;
pub type StringHashTableCopyFunction = ::std::option::Option<
    unsafe extern "C" fn(arg1: *const StringHashTableKey) -> *mut StringHashTableValue,
>;
pub type StringHashTableFreeKeyFunction =
    ::std::option::Option<unsafe extern "C" fn(arg1: *mut StringHashTableKey)>;
pub type StringHashTableFreeValueFunction =
    ::std::option::Option<unsafe extern "C" fn(arg1: *mut StringHashTableValue)>;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct StringHashTableHashTable {
    pub hash_function: StringHashTableHashFunction,
    pub equal_function: StringHashTableEqualFunction,
    pub copy_function: StringHashTableCopyFunction,
    pub free_key_function: StringHashTableFreeKeyFunction,
    pub free_value_function: StringHashTableFreeValueFunction,
    pub min_fill_quotient: f64,
    pub max_fill_quotient: f64,
    pub min_size: UioUint32,
    pub max_size: UioUint32,
    pub size: UioUint32,
    pub hash_mask: UioUint32,
    pub entries: *mut *mut StringHashTableHashEntry,
    pub num_entries: UioUint32,
    pub num_collisions: UioUint32,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct StringHashTableHashEntry {
    pub hash: UioUint32,
    pub key: *mut StringHashTableKey,
    pub value: *mut StringHashTableValue,
    pub next: *mut StringHashTableHashEntry,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct StringHashTableIterator {
    pub hash_table: *const StringHashTableHashTable,
    pub bucket_nr: UioUint32,
    pub entry: *mut StringHashTableHashEntry,
}
extern "C" {
    #[link_name = "StringHashTable_newHashTable"]
    pub fn StringHashTablenewHashTable(
        hash_function: StringHashTableHashFunction,
        equal_function: StringHashTableEqualFunction,
        copy_function: StringHashTableCopyFunction,
        free_key_function: StringHashTableFreeKeyFunction,
        free_value_function: StringHashTableFreeValueFunction,
        initial_size: UioUint32,
        min_fill_quotient: f64,
        max_fill_quotient: f64,
    ) -> *mut StringHashTableHashTable;
}
extern "C" {
    #[link_name = "StringHashTable_add"]
    pub fn StringHashTableadd(
        hash_table: *mut StringHashTableHashTable,
        key: *const StringHashTableKey,
        value: *mut StringHashTableValue,
    ) -> UioBool;
}
extern "C" {
    #[link_name = "StringHashTable_remove"]
    pub fn StringHashTableremove(
        hash_table: *mut StringHashTableHashTable,
        key: *const StringHashTableKey,
    ) -> UioBool;
}
extern "C" {
    #[link_name = "StringHashTable_find"]
    pub fn StringHashTablefind(
        hash_table: *mut StringHashTableHashTable,
        key: *const StringHashTableKey,
    ) -> *mut StringHashTableValue;
}
extern "C" {
    #[link_name = "StringHashTable_count"]
    pub fn StringHashTablecount(hash_table: *const StringHashTableHashTable) -> UioUint32;
}
extern "C" {
    #[link_name = "StringHashTable_deleteHashTable"]
    pub fn StringHashTabledeleteHashTable(hash_table: *mut StringHashTableHashTable);
}
extern "C" {
    #[link_name = "StringHashTable_getIterator"]
    pub fn StringHashTablegetIterator(
        hash_table: *const StringHashTableHashTable,
    ) -> *mut StringHashTableIterator;
}
extern "C" {
    #[link_name = "StringHashTable_iteratorDone"]
    pub fn StringHashTableiteratorDone(
        iterator: *const StringHashTableIterator,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    #[link_name = "StringHashTable_iteratorKey"]
    pub fn StringHashTableiteratorKey(
        iterator: *mut StringHashTableIterator,
    ) -> *mut StringHashTableKey;
}
extern "C" {
    #[link_name = "StringHashTable_iteratorValue"]
    pub fn StringHashTableiteratorValue(
        iterator: *mut StringHashTableIterator,
    ) -> *mut StringHashTableValue;
}
extern "C" {
    #[link_name = "StringHashTable_iteratorNext"]
    pub fn StringHashTableiteratorNext(
        iterator: *mut StringHashTableIterator,
    ) -> *mut StringHashTableIterator;
}
extern "C" {
    #[link_name = "StringHashTable_freeIterator"]
    pub fn StringHashTablefreeIterator(iterator: *mut StringHashTableIterator);
}
