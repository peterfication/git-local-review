use refinery::embed_migrations;

embed_migrations!("migrations");

pub use migrations::*;
