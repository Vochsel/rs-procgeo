// Delete SOPs

pub mod blast;
pub mod delete_sop;

pub use blast::{BlastSop, BlastParams, BlastEntity};
pub use delete_sop::{DeleteSop, DeleteParams, DeleteEntity};
