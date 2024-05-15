#![warn(missing_debug_implementations)]

use anyhow::Context as _;
use visualizer::Visualizer;

pub mod instruction;
pub mod interpreter;
pub mod visualizer;

fn main() -> anyhow::Result<()> {
    let mut visualizer = Visualizer::init().context("failed to initialize visualizer")?;

    loop {
        let result = visualizer.tick();
        match result {
            Ok(true) => {
                break;
            }
            Err(err) => {
                println!("failed to tick visualizer: {err}");
            }
            _ => {}
        }
    }

    visualizer.unload().context("failed to unload visualizer")?;
    Ok(())
}
