use super::Handler;
use crate::comm::races::alliance::AllianceApi;

pub trait SlyhomeApi: AllianceApi<Handler> {}
impl<T: AllianceApi<Handler>> SlyhomeApi for T {}
