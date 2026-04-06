// Attribute SOPs

pub mod create;
pub mod delete;
pub mod rename;
pub mod promote;
pub mod attrib_transfer;
pub mod attrib_copy;
pub mod attrib_randomize;
pub mod attrib_sort;
pub mod attrib_blur;
pub mod attrib_fill;
pub mod attrib_noise;

pub use create::{AttribCreateSop, AttribCreateParams};
pub use delete::{AttribDeleteSop, AttribDeleteParams};
pub use rename::{AttribRenameSop, AttribRenameParams};
pub use promote::{AttribPromoteSop, AttribPromoteParams, PromoteMethod};
pub use attrib_transfer::{AttribTransferSop, AttribTransferParams};
pub use attrib_copy::{AttribCopySop, AttribCopyParams};
pub use attrib_randomize::{AttribRandomizeSop, AttribRandomizeParams, RandomDistribution, RandomOperation};
pub use attrib_sort::{AttribSortSop, AttribSortParams, AttribSortOrder};
pub use attrib_blur::{AttribBlurSop, AttribBlurParams};
pub use attrib_fill::{AttribFillSop, AttribFillParams};
pub use attrib_noise::{AttribNoiseSop, AttribNoiseParams, NoiseType, NoiseOperation, FractalType, NoiseRange};
