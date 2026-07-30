#[export_name = "AllocLink"]
pub extern "C" fn alloc_link() {}
#[export_name = "CountLinks"]
pub extern "C" fn count_links() {}
#[export_name = "ForAllLinks"]
pub extern "C" fn for_all_links() {}
#[export_name = "FreeLink"]
pub extern "C" fn free_link() {}
#[export_name = "InitQueue"]
pub extern "C" fn init_queue() {}
#[export_name = "InsertQueue"]
pub extern "C" fn insert_queue() {}
#[export_name = "PutQueue"]
pub extern "C" fn put_queue() {}
#[export_name = "ReinitQueue"]
pub extern "C" fn reinit_queue() {}
#[export_name = "RemoveQueue"]
pub extern "C" fn remove_queue() {}
#[export_name = "UninitQueue"]
pub extern "C" fn uninit_queue() {}

#[export_name = "CharHashTable_add"]
pub extern "C" fn char_hash_table_add() {}
#[export_name = "CharHashTable_count"]
pub extern "C" fn char_hash_table_count() {}
#[export_name = "CharHashTable_deleteHashTable"]
pub extern "C" fn char_hash_table_delete_hash_table() {}
#[export_name = "CharHashTable_find"]
pub extern "C" fn char_hash_table_find() {}
#[export_name = "CharHashTable_freeIterator"]
pub extern "C" fn char_hash_table_free_iterator() {}
#[export_name = "CharHashTable_getIterator"]
pub extern "C" fn char_hash_table_get_iterator() {}
#[export_name = "CharHashTable_iteratorDone"]
pub extern "C" fn char_hash_table_iterator_done() {}
#[export_name = "CharHashTable_iteratorKey"]
pub extern "C" fn char_hash_table_iterator_key() {}
#[export_name = "CharHashTable_iteratorNext"]
pub extern "C" fn char_hash_table_iterator_next() {}
#[export_name = "CharHashTable_iteratorValue"]
pub extern "C" fn char_hash_table_iterator_value() {}
#[export_name = "CharHashTable_newHashTable"]
pub extern "C" fn char_hash_table_new_hash_table() {}
#[export_name = "CharHashTable_remove"]
pub extern "C" fn char_hash_table_remove() {}

#[export_name = "StringHashTable_add"]
pub extern "C" fn string_hash_table_add() {}
#[export_name = "StringHashTable_count"]
pub extern "C" fn string_hash_table_count() {}
#[export_name = "StringHashTable_deleteHashTable"]
pub extern "C" fn string_hash_table_delete_hash_table() {}
#[export_name = "StringHashTable_find"]
pub extern "C" fn string_hash_table_find() {}
#[export_name = "StringHashTable_freeIterator"]
pub extern "C" fn string_hash_table_free_iterator() {}
#[export_name = "StringHashTable_getIterator"]
pub extern "C" fn string_hash_table_get_iterator() {}
#[export_name = "StringHashTable_iteratorDone"]
pub extern "C" fn string_hash_table_iterator_done() {}
#[export_name = "StringHashTable_iteratorKey"]
pub extern "C" fn string_hash_table_iterator_key() {}
#[export_name = "StringHashTable_iteratorNext"]
pub extern "C" fn string_hash_table_iterator_next() {}
#[export_name = "StringHashTable_iteratorValue"]
pub extern "C" fn string_hash_table_iterator_value() {}
#[export_name = "StringHashTable_newHashTable"]
pub extern "C" fn string_hash_table_new_hash_table() {}
#[export_name = "StringHashTable_remove"]
pub extern "C" fn string_hash_table_remove() {}
