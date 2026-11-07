//! The launcher engine: version metadata, downloads, Java, launching and supervision.

pub mod auth;
pub mod catalog;
mod crash;
mod http;
mod install;
mod installer;
mod java;
mod launch;
mod loaders;
pub mod modman;
mod mods;
mod mojang;
mod paths;
pub mod screenshots;
pub mod session;
pub mod storage;
pub mod system;
