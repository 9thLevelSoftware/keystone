pub mod error;
pub mod sidecar;

pub use error::IoError;
pub use sidecar::{
    LoadedPack, METADATA_DIR, PackInputKind, ResolvedPackInput, SIDECAR_FILE,
    canonical_sidecar_path, read_pack_from_input, resolve_pack_input_path, write_pack_sidecar,
};
