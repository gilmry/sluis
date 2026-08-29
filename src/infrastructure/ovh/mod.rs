//! Adaptateur de l'API OVHcloud.

pub mod client;
pub mod signature;

pub use client::ClientOvh;
pub use signature::{signer, IdentiteOvh};
