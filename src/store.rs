use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default)]
    pub inheritance: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerData {
    #[serde(default)]
    pub username: String,
    #[serde(default = "default_group")]
    pub group: String,
    #[serde(default)]
    pub extra_permissions: Vec<String>,
    #[serde(default)]
    pub denied_permissions: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
}

fn default_group() -> String {
    "default".to_string()
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct PlayersFile {
    #[serde(default)]
    players: HashMap<String, PlayerData>,
}

pub struct PermissionStore {
    pub data_folder: PathBuf,
    pub groups: HashMap<String, Group>,
    pub players: HashMap<Uuid, PlayerData>,
}

impl PermissionStore {
    pub fn load(data_folder: PathBuf) -> Result<Self, String> {
        fs::create_dir_all(&data_folder)
            .map_err(|e| format!("Failed to create data folder: {e}"))?;

        let groups_path = data_folder.join("groups.toml");
        let players_path = data_folder.join("players.toml");

        let groups = if groups_path.exists() {
            let content = fs::read_to_string(&groups_path)
                .map_err(|e| format!("Failed to read groups.toml: {e}"))?;
            toml::from_str(&content).map_err(|e| format!("Failed to parse groups.toml: {e}"))?
        } else {
            HashMap::from([(
                "default".to_string(),
                Group {
                    permissions: vec![
                        "minecraft:command.help".to_string(),
                        "minecraft:command.list".to_string(),
                    ],
                    inheritance: Vec::new(),
                    prefix: None,
                },
            )])
        };

        let players = if players_path.exists() {
            let content = fs::read_to_string(&players_path)
                .map_err(|e| format!("Failed to read players.toml: {e}"))?;
            let file: PlayersFile = toml::from_str(&content)
                .map_err(|e| format!("Failed to parse players.toml: {e}"))?;
            file.players
                .into_iter()
                .map(|(uuid_str, data)| {
                    Uuid::parse_str(&uuid_str)
                        .map(|uuid| (uuid, data))
                        .map_err(|e| format!("Invalid UUID '{uuid_str}': {e}"))
                })
                .collect::<Result<HashMap<_, _>, _>>()?
        } else {
            HashMap::new()
        };

        let store = Self {
            data_folder,
            groups,
            players,
        };
        store.save()?;
        Ok(store)
    }

    pub fn save(&self) -> Result<(), String> {
        let groups_content = toml::to_string_pretty(&self.groups)
            .map_err(|e| format!("Failed to serialize groups: {e}"))?;
        fs::write(self.data_folder.join("groups.toml"), groups_content)
            .map_err(|e| format!("Failed to write groups.toml: {e}"))?;

        let file = PlayersFile {
            players: self
                .players
                .iter()
                .map(|(uuid, data)| (uuid.to_string(), data.clone()))
                .collect(),
        };
        let players_content = toml::to_string_pretty(&file)
            .map_err(|e| format!("Failed to serialize players: {e}"))?;
        fs::write(self.data_folder.join("players.toml"), players_content)
            .map_err(|e| format!("Failed to write players.toml: {e}"))?;

        Ok(())
    }

    /// Resolve a permission for a player. Returns Some(true/false) if Seed has
    /// an opinion, None to fall through to Pumpkin's default.
    pub fn check_permission(&self, uuid: &Uuid, node: &str) -> Option<bool> {
        let player_data = self.players.get(uuid);
        let group_name = player_data.map(|p| p.group.as_str()).unwrap_or("default");

        if let Some(pd) = player_data {
            // Denied overrides everything
            if pd.denied_permissions.iter().any(|p| p == node) {
                return Some(false);
            }
            // Per-player extras override group
            if pd.extra_permissions.iter().any(|p| p == node || p == "*") {
                return Some(true);
            }
        }

        let group_perms = self.resolve_group_permissions(group_name);
        if group_perms.contains(node) || group_perms.contains("*") {
            return Some(true);
        }

        None
    }

    pub fn resolve_group_permissions(&self, group_name: &str) -> HashSet<String> {
        let mut result = HashSet::new();
        let mut visited = HashSet::new();
        self.collect_permissions(group_name, &mut result, &mut visited);
        result
    }

    fn collect_permissions(
        &self,
        group_name: &str,
        result: &mut HashSet<String>,
        visited: &mut HashSet<String>,
    ) {
        if !visited.insert(group_name.to_string()) {
            return;
        }
        if let Some(group) = self.groups.get(group_name) {
            for perm in &group.permissions {
                result.insert(perm.clone());
            }
            for parent in &group.inheritance {
                self.collect_permissions(parent, result, visited);
            }
        }
    }

    /// Resolve the effective prefix for a player.
    /// Player-level prefix overrides group prefix.
    pub fn resolve_prefix(&self, uuid: &Uuid) -> Option<String> {
        if let Some(pd) = self.players.get(uuid) {
            if pd.prefix.is_some() {
                return pd.prefix.clone();
            }
            return self.resolve_group_prefix(&pd.group);
        }
        self.resolve_group_prefix("default")
    }

    /// Resolve the prefix for a group by walking the inheritance chain.
    /// First match wins.
    pub fn resolve_group_prefix(&self, group_name: &str) -> Option<String> {
        let mut visited = HashSet::new();
        self.find_group_prefix(group_name, &mut visited)
    }

    fn find_group_prefix(&self, group_name: &str, visited: &mut HashSet<String>) -> Option<String> {
        if !visited.insert(group_name.to_string()) {
            return None;
        }
        if let Some(group) = self.groups.get(group_name) {
            if group.prefix.is_some() {
                return group.prefix.clone();
            }
            for parent in &group.inheritance {
                if let Some(prefix) = self.find_group_prefix(parent, visited) {
                    return Some(prefix);
                }
            }
        }
        None
    }

    pub fn get_or_create_player(&mut self, uuid: Uuid, username: &str) -> &mut PlayerData {
        let pd = self.players.entry(uuid).or_insert_with(|| PlayerData {
            username: username.to_string(),
            group: "default".to_string(),
            extra_permissions: Vec::new(),
            denied_permissions: Vec::new(),
            prefix: None,
        });
        pd.username = username.to_string();
        pd
    }
}
