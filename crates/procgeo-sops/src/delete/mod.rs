// Delete SOPs

pub mod blast;
pub mod delete_sop;

pub use blast::{BlastEntity, BlastParams, BlastSop};
pub use delete_sop::{DeleteEntity, DeleteParams, DeleteSop};
