use super::Handler;
use crate::comm::races::alliance::AllianceApi;

pub(super) trait SlyhomeApi: AllianceApi<Handler> {}
impl<T: AllianceApi<Handler>> SlyhomeApi for T {}
