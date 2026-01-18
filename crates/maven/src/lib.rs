#![deny(warnings)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::redundant_clone)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::too_many_lines)]
pub mod compile;
pub mod config;
pub mod m2;
pub mod metadata;
pub mod pom;
pub mod project;
pub mod repository;
pub mod settings;
pub mod tree;
pub mod update;
