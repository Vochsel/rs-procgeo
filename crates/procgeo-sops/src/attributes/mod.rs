// Attribute SOPs

pub mod create;
pub mod delete;
pub mod rename;
pub mod promote;

pub use create::{AttribCreateSop, AttribCreateParams};
pub use delete::{AttribDeleteSop, AttribDeleteParams};
pub use rename::{AttribRenameSop, AttribRenameParams};
pub use promote::{AttribPromoteSop, AttribPromoteParams, PromoteMethod};
