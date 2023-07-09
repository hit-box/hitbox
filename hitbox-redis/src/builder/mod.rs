mod cluster;
mod sentinel;
mod standalone;

use cluster::Cluster;
use sentinel::Sentinel;
use standalone::Standalone;

pub struct Builder;

impl Builder {
    pub fn standalone() -> Standalone {
        Standalone::default()
    }

    pub fn sentinel() -> Sentinel {
        Sentinel::default()
    }

    pub fn cluster() -> Cluster {
        Cluster::default()
    }
}
