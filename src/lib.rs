//! SigmaOS craft — crafting system.
//!
//! Item registry, recipes (ingredient -> output), inventory management,
//! and craft operations with atomic state persistence.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Errors produced by this component.
#[derive(Debug, thiserror::Error)]
pub enum CraftError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("recipe not found: {0}")]
    RecipeNotFound(String),
    #[error("item not found: {0}")]
    ItemNotFound(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("insufficient ingredients for '{0}'")]
    Insufficient(String),
    #[error("validation error: {0}")]
    Validation(String),
}

/// An item definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub stack_max: u32,
    #[serde(default)]
    pub description: String,
}

/// A recipe: ingredients consumed -> output produced.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recipe {
    pub id: String,
    pub name: String,
    /// ingredient item id -> count
    #[serde(default)]
    pub ingredients: BTreeMap<String, u32>,
    /// output item id
    pub output: String,
    #[serde(default = "default_one")]
    pub output_count: u32,
}

fn default_one() -> u32 {
    1
}

/// Player inventory: item id -> count.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Inventory {
    #[serde(default)]
    pub items: BTreeMap<String, u32>,
}

impl Inventory {
    /// Add items.
    pub fn add(&mut self, id: &str, count: u32) {
        *self.items.entry(id.to_string()).or_default() += count;
    }

    /// Remove items; fails if insufficient.
    pub fn remove(&mut self, id: &str, count: u32) -> Result<(), CraftError> {
        let have = self.items.get(id).copied().unwrap_or(0);
        if have < count {
            return Err(CraftError::Insufficient(id.to_string()));
        }
        let new = have - count;
        if new == 0 {
            self.items.remove(id);
        } else {
            self.items.insert(id.to_string(), new);
        }
        Ok(())
    }

    /// Check if inventory has all ingredients.
    pub fn has_ingredients(&self, ingredients: &BTreeMap<String, u32>) -> bool {
        ingredients.iter().all(|(id, n)| {
            self.items.get(id).copied().unwrap_or(0) >= *n
        })
    }
}

/// Full crafting state: item registry, recipes, inventory.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CraftState {
    #[serde(default)]
    pub items: BTreeMap<String, Item>,
    #[serde(default)]
    pub recipes: BTreeMap<String, Recipe>,
    #[serde(default)]
    pub inventory: Inventory,
}

impl CraftState {
    /// Register an item.
    pub fn add_item(&mut self, item: Item) -> Result<(), CraftError> {
        if item.id.is_empty() {
            return Err(CraftError::Validation("empty item id".into()));
        }
        self.items.insert(item.id.clone(), item);
        Ok(())
    }

    /// Register a recipe.
    pub fn add_recipe(&mut self, recipe: Recipe) -> Result<(), CraftError> {
        if recipe.id.is_empty() {
            return Err(CraftError::Validation("empty recipe id".into()));
        }
        self.recipes.insert(recipe.id.clone(), recipe);
        Ok(())
    }

    /// Can the inventory craft this recipe?
    pub fn can_craft(&self, recipe_id: &str) -> Result<bool, CraftError> {
        let r = self
            .recipes
            .get(recipe_id)
            .ok_or_else(|| CraftError::RecipeNotFound(recipe_id.into()))?;
        Ok(self.inventory.has_ingredients(&r.ingredients))
    }

    /// Craft a recipe: consume ingredients, produce output.
    pub fn craft(&mut self, recipe_id: &str) -> Result<u32, CraftError> {
        let recipe = self
            .recipes
            .get(recipe_id)
            .cloned()
            .ok_or_else(|| CraftError::RecipeNotFound(recipe_id.into()))?;
        if !self.inventory.has_ingredients(&recipe.ingredients) {
            return Err(CraftError::Insufficient(recipe_id.to_string()));
        }
        for (id, n) in &recipe.ingredients {
            self.inventory.remove(id, *n)?;
        }
        self.inventory.add(&recipe.output, recipe.output_count);
        Ok(recipe.output_count)
    }

    /// List craftable recipes given current inventory.
    pub fn craftable(&self) -> Vec<&Recipe> {
        self.recipes
            .values()
            .filter(|r| self.inventory.has_ingredients(&r.ingredients))
            .collect()
    }

    /// Save to JSON.
    pub fn save(&self, path: &Path) -> Result<(), CraftError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_string_pretty(self)?)?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }

    /// Load from JSON.
    pub fn load(path: &Path) -> Result<Self, CraftError> {
        let text = std::fs::read_to_string(path)
            .map_err(|_| CraftError::NotFound(path.display().to_string()))?;
        Ok(serde_json::from_str(&text)?)
    }
}

/// Default state path.
pub fn default_state_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("org", "sigmaos", "sigmaos")
        .map(|p| p.data_dir().join("craft").join("state.json"))
}

/// A starter state with a few example items/recipes.
pub fn starter_state() -> CraftState {
    let mut s = CraftState::default();
    for (id, name) in [
        ("wood", "Wood"),
        ("plank", "Wood Plank"),
        ("stick", "Stick"),
        ("stone", "Stone"),
        ("torch", "Torch"),
    ] {
        let _ = s.add_item(Item {
            id: id.into(),
            name: name.into(),
            stack_max: 64,
            description: String::new(),
        });
    }
    let mut ing = BTreeMap::new();
    ing.insert("wood".to_string(), 1);
    let _ = s.add_recipe(Recipe {
        id: "plank".into(),
        name: "Craft Planks".into(),
        ingredients: ing.clone(),
        output: "plank".into(),
        output_count: 4,
    });
    ing.clear();
    ing.insert("plank".to_string(), 2);
    let _ = s.add_recipe(Recipe {
        id: "stick".into(),
        name: "Craft Sticks".into(),
        ingredients: ing,
        output: "stick".into(),
        output_count: 4,
    });
    s
}
