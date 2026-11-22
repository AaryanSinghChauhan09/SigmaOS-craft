//! SigmaOS component binary: craft

use sigmaos_craft::Component;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    Component::new().operate()?;
    println!("sigmaos-craft: ok");
    Ok(())
}
