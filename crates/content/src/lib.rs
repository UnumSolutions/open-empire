//! Shared, renderer-independent original rules and safe content manifests.
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const FORMAT_VERSION: u32 = 1;
pub const RULESET_VERSION: &str = "original-0.1";
pub const MAX_PACK_BYTES: u64 = 4 * 1024 * 1024 * 1024;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Civilization {
    Aster,
    Veyran,
}
impl Civilization {
    pub fn name(self) -> &'static str {
        match self {
            Self::Aster => "Aster Marches",
            Self::Veyran => "Veyran League",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Resource {
    Food,
    Wood,
    Gold,
    Stone,
}
impl Resource {
    pub fn index(self) -> usize {
        self as usize
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Kind {
    Villager,
    Spearman,
    Archer,
    Cavalry,
    Ram,
    Monk,
    Warship,
    TownCenter,
    House,
    Barracks,
    ArcheryRange,
    Stable,
    Workshop,
    Monastery,
    Dock,
    Market,
    Tower,
    Wonder,
}
pub const ALL_KINDS: [Kind; 18] = [
    Kind::Villager,
    Kind::Spearman,
    Kind::Archer,
    Kind::Cavalry,
    Kind::Ram,
    Kind::Monk,
    Kind::Warship,
    Kind::TownCenter,
    Kind::House,
    Kind::Barracks,
    Kind::ArcheryRange,
    Kind::Stable,
    Kind::Workshop,
    Kind::Monastery,
    Kind::Dock,
    Kind::Market,
    Kind::Tower,
    Kind::Wonder,
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stats {
    pub cost: [u32; 4],
    pub hp: i32,
    pub damage: i32,
    pub range: u16,
    pub age: u8,
    pub work: u16,
    pub building: bool,
}
impl Kind {
    pub fn stats(self) -> Stats {
        use Kind::*;
        let (cost, hp, damage, range, age, work, building) = match self {
            Villager => ([50, 0, 0, 0], 35, 3, 1, 1, 30, false),
            Spearman => ([35, 25, 0, 0], 60, 7, 1, 2, 35, false),
            Archer => ([0, 25, 45, 0], 40, 6, 5, 2, 40, false),
            Cavalry => ([60, 0, 75, 0], 110, 12, 1, 3, 50, false),
            Ram => ([0, 160, 75, 0], 220, 8, 1, 3, 60, false),
            Monk => ([0, 0, 100, 0], 35, 0, 5, 3, 50, false),
            Warship => ([0, 90, 40, 0], 140, 12, 6, 2, 60, false),
            TownCenter => ([0, 275, 0, 100], 1800, 8, 6, 1, 160, true),
            House => ([0, 25, 0, 0], 350, 0, 0, 1, 30, true),
            Barracks => ([0, 175, 0, 0], 900, 0, 0, 1, 90, true),
            ArcheryRange | Stable => ([0, 175, 0, 0], 900, 0, 0, 2, 90, true),
            Workshop | Monastery => ([0, 200, 0, 0], 900, 0, 0, 3, 110, true),
            Dock => ([0, 150, 0, 0], 900, 0, 0, 1, 90, true),
            Market => ([0, 175, 0, 0], 900, 0, 0, 2, 90, true),
            Tower => ([0, 50, 0, 125], 1000, 9, 6, 2, 100, true),
            Wonder => ([1000, 1000, 1000, 1000], 4000, 0, 0, 4, 600, true),
        };
        Stats {
            cost,
            hp,
            damage,
            range,
            age,
            work,
            building,
        }
    }
    pub fn producer(self) -> Option<Kind> {
        use Kind::*;
        match self {
            Villager => Some(TownCenter),
            Spearman => Some(Barracks),
            Archer => Some(ArcheryRange),
            Cavalry => Some(Stable),
            Ram => Some(Workshop),
            Monk => Some(Monastery),
            Warship => Some(Dock),
            _ => None,
        }
    }
}

/// Hash actual serialized rule definitions, rather than a display label.
pub fn original_hash() -> String {
    let rules: Vec<_> = ALL_KINDS.iter().map(|k| (*k, k.stats())).collect();
    let bytes = serde_json::to_vec(&(
        RULESET_VERSION,
        rules,
        "Aster building hp +20%; Veyran cavalry hp +15%",
    ))
    .expect("serializable rules");
    blake3::hash(&bytes).to_hex().to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackFile {
    pub path: String,
    pub bytes: u64,
    pub blake3: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub format: u32,
    pub ruleset: String,
    pub source_build: Option<String>,
    pub proprietary: bool,
    pub files: Vec<PackFile>,
}
impl Manifest {
    /// Validate before accessing files; archives must use these same checks before extraction.
    pub fn validate(&self) -> Result<(), String> {
        if self.format != FORMAT_VERSION {
            return Err("unsupported pack format".into());
        }
        if self.ruleset.is_empty() || self.ruleset.len() > 128 {
            return Err("invalid ruleset".into());
        }
        if self.proprietary && self.source_build.as_ref().is_none_or(|s| s.is_empty()) {
            return Err("DE content needs a pinned source build".into());
        }
        if self.files.len() > 100_000 {
            return Err("too many files".into());
        }
        let mut total = 0u64;
        let mut names = BTreeSet::new();
        for file in &self.files {
            if file.path.is_empty()
                || file.path.len() > 1024
                || file.path.contains(['\\', ':', '\0'])
                || file
                    .path
                    .split('/')
                    .any(|s| s.is_empty() || s == "." || s == "..")
            {
                return Err(format!("unsafe content path: {}", file.path));
            }
            if !names.insert(file.path.to_lowercase()) {
                return Err("duplicate content path".into());
            }
            total = total.checked_add(file.bytes).ok_or("pack size overflow")?;
            if total > MAX_PACK_BYTES {
                return Err("pack exceeds 4 GiB limit".into());
            }
            if file.blake3.len() != 64 || !file.blake3.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err("invalid BLAKE3 hash".into());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_paths_and_duplicates() {
        let mut m = Manifest {
            format: 1,
            ruleset: "original".into(),
            source_build: None,
            proprietary: false,
            files: vec![],
        };
        for path in ["../secret", "/root", "a/../b", "C:/x", "a\\b", "a//b"] {
            m.files = vec![PackFile {
                path: path.into(),
                bytes: 1,
                blake3: "a".repeat(64),
            }];
            assert!(m.validate().is_err());
        }
        m.files = ["Units/a.png", "units/A.png"]
            .iter()
            .map(|p| PackFile {
                path: p.to_string(),
                bytes: 1,
                blake3: "a".repeat(64),
            })
            .collect();
        assert!(m.validate().is_err());
    }
}
