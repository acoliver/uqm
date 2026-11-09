use super::Handler;
use crate::comm::races::alliance::AllianceApi;

pub(crate) trait SlyhomeApi: AllianceApi<Handler> {}
impl<T: AllianceApi<Handler>> SlyhomeApi for T {}
