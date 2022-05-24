use async_trait::async_trait;
use kube::Client;

use crate::downscaler::Res;
use crate::Error;

#[derive(Debug, PartialEq)]
pub struct StatefulSet;
#[async_trait]
impl Res for StatefulSet {
    async fn downscale(&self, _c: Client, _is_uptime: bool) -> Result<(), Error> {
        Ok(())
    }
}
