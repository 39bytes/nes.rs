use crate::cpu::Cpu6502;
use anyhow::Error;

mod bus;
mod cpu;
mod instructions;

pub type Result<T> = std::result::Result<T, anyhow::Error>;

fn main() {
    println!("Hello, world!");
}
