//! CLI for sigmaos-craft.

use clap::{Parser, Subcommand};
use sigmaos_craft::{default_state_path, starter_state, CraftState};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "sigmaos-craft", about = "SigmaOS crafting system")]
struct Cli {
    #[arg(long, global = true)]
    state: Option<PathBuf>,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List inventory.
    Inventory,
    /// List recipes.
    Recipes,
    /// List craftable recipes.
    Craftable,
    /// Craft a recipe.
    Craft { recipe: String },
    /// Give an item.
    Give { item: String, #[arg(long, default_value_t = 1)] count: u32 },
    /// Initialize state with starter items/recipes.
    Init,
    /// Print state path.
    Path,
}

fn state_path(s: &Option<PathBuf>) -> Result<PathBuf, Box<dyn std::error::Error>> {
    s.clone()
        .or_else(default_state_path)
        .ok_or_else(|| "cannot determine state path".into())
}

fn load(s: &Option<PathBuf>) -> Result<CraftState, Box<dyn std::error::Error>> {
    let p = state_path(s)?;
    if p.exists() {
        Ok(CraftState::load(&p)?)
    } else {
        Ok(starter_state())
    }
}

fn save(st: &CraftState, path: &Option<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    st.save(&state_path(path)?)?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let cli = Cli::parse();
    let mut st = load(&cli.state)?;
    match cli.command {
        Commands::Inventory => {
            for (id, n) in &st.inventory.items {
                let name = st.items.get(id).map(|i| i.name.as_str()).unwrap_or(id);
                println!("{id} ({name}) x{n}");
            }
        }
        Commands::Recipes => {
            for r in st.recipes.values() {
                let ings: Vec<String> = r.ingredients.iter()
                    .map(|(i, n)| format!("{i}x{n}")).collect();
                println!("{}: {} -> {} x{}", r.id, ings.join("+"), r.output, r.output_count);
            }
        }
        Commands::Craftable => {
            for r in st.craftable() {
                println!("{} ({})", r.id, r.name);
            }
        }
        Commands::Craft { recipe } => {
            let n = st.craft(&recipe)?;
            save(&st, &cli.state)?;
            println!("crafted {} x{}", recipe, n);
        }
        Commands::Give { item, count } => {
            st.inventory.add(&item, count);
            save(&st, &cli.state)?;
            println!("gave {item} x{count}");
        }
        Commands::Init => {
            save(&starter_state(), &cli.state)?;
            println!("initialized: {}", state_path(&cli.state)?.display());
        }
        Commands::Path => println!("{}", state_path(&cli.state)?.display()),
    }
    Ok(())
}
