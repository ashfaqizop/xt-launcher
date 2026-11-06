//! The launcher engine: version metadata, downloads, Java, launching and supervision.

pub mod auth;
mod crash;
mod http;
mod install;
mod installer;
mod java;
mod launch;
mod loaders;
mod mods;
mod mojang;
mod paths;
pub mod session;
