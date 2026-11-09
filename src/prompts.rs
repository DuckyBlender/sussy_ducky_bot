//! Shared library of named system prompts. Anyone can create prompts, only the
//! author can change or delete one, and every user picks which one (if any)
//! is used for their own messages. Persisted as JSON.

use std::{
    collections::{BTreeMap, HashMap},
    fs,
    path::PathBuf,
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

pub const MAX_NAME_CHARS: usize = 32;
pub const MAX_TEXT_CHARS: usize = 4000;
pub const MAX_PER_USER: usize = 20;
/// Reserved name for the built-in prompt used when a user hasn't picked one.
pub const DEFAULT_NAME: &str = "default";
pub const DEFAULT_TEXT: &str = "Keep replies short.";

#[derive(Serialize, Deserialize, Clone)]
pub struct Prompt {
    pub author_id: u64,
    pub author: String,
    pub text: String,
}

#[derive(Serialize, Deserialize, Default)]
struct Data {
    prompts: BTreeMap<String, Prompt>,
    /// user id -> name of the prompt they use
    active: HashMap<u64, String>,
}

pub struct Prompts {
    path: Option<PathBuf>,
    data: Data,
}

/// A user-facing reason an operation was refused.
pub type Refusal = String;

impl Prompts {
    /// Loads the store from `path`, starting empty if the file doesn't exist yet.
    pub fn load(path: PathBuf) -> Result<Self> {
        let data = match fs::read_to_string(&path) {
            Ok(json) => serde_json::from_str(&json)
                .with_context(|| format!("parsing {}", path.display()))?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Data::default(),
            Err(e) => return Err(e).with_context(|| format!("reading {}", path.display())),
        };
        Ok(Self {
            path: Some(path),
            data,
        })
    }

    pub fn save(&self) -> Result<()> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        let tmp = path.with_extension("json.tmp");
        fs::write(&tmp, serde_json::to_vec_pretty(&self.data)?)
            .with_context(|| format!("writing {}", tmp.display()))?;
        fs::rename(&tmp, path).with_context(|| format!("replacing {}", path.display()))?;
        Ok(())
    }

    /// Looks a prompt up by name, returning its normalized name too.
    pub fn get(&self, name: &str) -> Option<(&str, &Prompt)> {
        self.data
            .prompts
            .get_key_value(&normalize(name))
            .map(|(n, p)| (n.as_str(), p))
    }

    pub fn list(&self) -> impl Iterator<Item = (&String, &Prompt)> {
        self.data.prompts.iter()
    }

    /// The prompt `user` has chosen, with its name.
    pub fn active(&self, user: u64) -> Option<(&str, &Prompt)> {
        let name = self.data.active.get(&user)?;
        self.data.prompts.get(name).map(|p| (name.as_str(), p))
    }

    /// Creates a prompt, or updates it if `author_id` already owns one with that name.
    /// Returns the normalized name and whether it was an update.
    pub fn upsert(
        &mut self,
        name: &str,
        author_id: u64,
        author: String,
        text: &str,
    ) -> Result<(String, bool), Refusal> {
        let name = valid_name(name)?;
        let text = text.trim();
        if text.is_empty() {
            return Err("The prompt text is empty.".into());
        }
        if text.chars().count() > MAX_TEXT_CHARS {
            return Err(format!("Too long, max {MAX_TEXT_CHARS} characters."));
        }
        let updating = match self.data.prompts.get(&name) {
            Some(p) if p.author_id != author_id => {
                return Err(format!(
                    "\"{name}\" already exists and belongs to {}.",
                    p.author
                ));
            }
            Some(_) => true,
            None => {
                let owned = self
                    .data
                    .prompts
                    .values()
                    .filter(|p| p.author_id == author_id)
                    .count();
                if owned >= MAX_PER_USER {
                    return Err(format!(
                        "You already have {MAX_PER_USER} prompts, delete one first."
                    ));
                }
                false
            }
        };
        self.data.prompts.insert(
            name.clone(),
            Prompt {
                author_id,
                author,
                text: text.to_owned(),
            },
        );
        Ok((name, updating))
    }

    /// Deletes a prompt owned by `user`; anyone using it falls back to the default.
    /// Returns the normalized name.
    pub fn delete(&mut self, name: &str, user: u64) -> Result<String, Refusal> {
        let name = normalize(name);
        match self.data.prompts.get(&name) {
            None => return Err(not_found(&name)),
            Some(p) if p.author_id != user => {
                return Err(format!("Only {} can delete \"{name}\".", p.author));
            }
            Some(_) => {}
        }
        self.data.prompts.remove(&name);
        self.data.active.retain(|_, n| *n != name);
        Ok(name)
    }

    /// Makes `user` use the named prompt. Returns the normalized name.
    pub fn select(&mut self, user: u64, name: &str) -> Result<String, Refusal> {
        let name = normalize(name);
        if !self.data.prompts.contains_key(&name) {
            return Err(not_found(&name));
        }
        self.data.active.insert(user, name.clone());
        Ok(name)
    }

    /// Makes `user` use the default system prompt.
    pub fn clear(&mut self, user: u64) {
        self.data.active.remove(&user);
    }
}

fn normalize(name: &str) -> String {
    name.trim().to_lowercase()
}

fn valid_name(name: &str) -> Result<String, Refusal> {
    let name = normalize(name);
    let ok = !name.is_empty()
        && name.chars().count() <= MAX_NAME_CHARS
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        && name != DEFAULT_NAME;
    if ok {
        Ok(name)
    } else {
        Err(format!(
            "Names are 1-{MAX_NAME_CHARS} letters, digits, _ or - (and not \"{DEFAULT_NAME}\")."
        ))
    }
}

pub fn not_found(name: &str) -> Refusal {
    format!("No prompt called \"{name}\". See /prompts.")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> Prompts {
        Prompts {
            path: None,
            data: Data::default(),
        }
    }

    #[test]
    fn create_use_delete() {
        let mut s = store();
        assert_eq!(
            s.upsert("Pirate", 1, "a".into(), "Talk like a pirate"),
            Ok(("pirate".into(), false))
        );
        assert_eq!(
            s.upsert("pirate", 1, "a".into(), "Arr"),
            Ok(("pirate".into(), true))
        );
        assert!(s.upsert("pirate", 2, "b".into(), "mine now").is_err());

        assert_eq!(s.select(2, "PIRATE"), Ok("pirate".into()));
        assert_eq!(s.active(2).unwrap().1.text, "Arr");
        assert!(s.select(2, "nope").is_err());

        assert!(s.delete("pirate", 2).is_err());
        s.delete("pirate", 1).unwrap();
        assert!(s.active(2).is_none());
        assert!(s.get("pirate").is_none());
    }

    #[test]
    fn rejects_bad_names_and_text() {
        let mut s = store();
        assert!(s.upsert("has space", 1, "a".into(), "x").is_err());
        assert!(s.upsert("Default", 1, "a".into(), "x").is_err());
        assert!(s.upsert("ok", 1, "a".into(), "   ").is_err());
        assert!(
            s.upsert("ok", 1, "a".into(), &"x".repeat(MAX_TEXT_CHARS + 1))
                .is_err()
        );
    }

    #[test]
    fn round_trips_through_disk() {
        let dir = std::env::temp_dir().join(format!("ducky-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("prompts.json");
        let mut s = Prompts::load(path.clone()).unwrap();
        s.upsert("x", 1, "a".into(), "hello").unwrap();
        s.select(1, "x").unwrap();
        s.save().unwrap();
        let s = Prompts::load(path).unwrap();
        assert_eq!(s.active(1).unwrap().1.text, "hello");
        fs::remove_dir_all(dir).unwrap();
    }
}
