// Topology SOPs

pub mod sort;
pub mod fuse;
pub mod connectivity;

pub use sort::{SortSop, SortParams, SortEntity, SortMode, SortAxis};
pub use fuse::{FuseSop, FuseParams};
pub use connectivity::{ConnectivitySop, ConnectivityParams};
