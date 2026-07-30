extern "C" {
    fn AllocLink();
    fn CountLinks();
    fn ForAllLinks();
    fn FreeLink();
    fn InitQueue();
    fn InsertQueue();
    fn PutQueue();
    fn ReinitQueue();
    fn RemoveQueue();
    fn UninitQueue();
}

fn main() {
    unsafe {
        AllocLink();
        CountLinks();
        ForAllLinks();
        FreeLink();
        InitQueue();
        InsertQueue();
        PutQueue();
        ReinitQueue();
        RemoveQueue();
        UninitQueue();
    }
}
