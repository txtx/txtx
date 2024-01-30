#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate rocket;

#[macro_use]
extern crate hiro_system_kit;

pub mod cli;
pub mod manifest;
pub mod term_ui;
pub mod web_ui;

fn main() {
    cli::main();
}
