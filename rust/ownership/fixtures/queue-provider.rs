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
