use crate::models::{MinerInfo, MinerStats, PoolConfig};
use std::fmt::Debug;

pub trait AsicAdapter: Send + Sync + Debug {
    fn info(&self) -> Result<MinerInfo, String>;
    fn stats(&self) -> Result<MinerStats, String>;
    fn set_pools(&self, pools: &[PoolConfig]) -> Result<(), String>;
    fn reboot(&self) -> Result<(), String>;
}
