use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use anyhow::{Result, Context};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub groups: Vec<Group>,
    pub keys: Vec<SshKey>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub name: String,
    pub color: String,
    pub hosts: Vec<Host>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Host {
    pub name: String,
    pub host: String,
    pub user: String,
    pub port: u16,
    pub key_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshKey {
    pub name: String,
    pub path: String,
    pub is_default: bool,
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;
        
        if !config_path.exists() {
            let default_config = Self::default();
            default_config.save()?;
            return Ok(default_config);
        }

        let contents = fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read config file: {:?}", config_path))?;
        
        let mut config: Config = serde_json::from_str(&contents)
            .with_context(|| "Failed to parse config JSON")?;

        // Ensure "All" group exists
        config.ensure_all_group();
        
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;
        
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory: {:?}", parent))?;
        }

        let contents = serde_json::to_string_pretty(self)
            .with_context(|| "Failed to serialize config")?;
        
        fs::write(&config_path, contents)
            .with_context(|| format!("Failed to write config file: {:?}", config_path))?;
        
        Ok(())
    }

    fn config_path() -> Result<PathBuf> {
        let home = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
        Ok(home.join(".config").join("sshtui").join("config.json"))
    }

    fn ensure_all_group(&mut self) {
        // Check if "All" group exists as first group
        if self.groups.is_empty() || self.groups[0].name != "All" {
            let all_group = Group {
                name: "All".to_string(),
                color: "blue".to_string(),
                hosts: vec![],
            };
            self.groups.insert(0, all_group);
        }
    }

    pub fn get_hosts_for_group(&self, group_index: usize) -> Vec<Host> {
        if group_index >= self.groups.len() {
            return vec![];
        }

        // Special handling for "All" group
        if group_index == 0 && self.groups[0].name == "All" {
            // Collect all hosts from all real groups (skip the "All" group itself)
            let mut all_hosts = Vec::new();
            for group in self.groups.iter().skip(1) {
                all_hosts.extend(group.hosts.clone());
            }
            all_hosts
        } else {
            self.groups[group_index].hosts.clone()
        }
    }

    pub fn add_key(&mut self, key: SshKey) {
        // If this key is set as default, unset all other defaults
        if key.is_default {
            for existing_key in &mut self.keys {
                existing_key.is_default = false;
            }
        }
        self.keys.push(key);
    }

    pub fn add_group(&mut self, group: Group) {
        // Insert after "All" group if it exists
        if !self.groups.is_empty() && self.groups[0].name == "All" {
            self.groups.insert(1, group);
        } else {
            self.groups.push(group);
        }
    }

    pub fn add_host_to_group(&mut self, group_name: &str, host: Host) -> Result<()> {
        if group_name == "All" {
            return Err(anyhow::anyhow!("Cannot add hosts directly to 'All' group"));
        }

        let group = self.groups.iter_mut()
            .find(|g| g.name == group_name)
            .ok_or_else(|| anyhow::anyhow!("Group '{}' not found", group_name))?;
        
        group.hosts.push(host);
        Ok(())
    }

    pub fn get_default_key(&self) -> Option<&SshKey> {
        self.keys.iter().find(|key| key.is_default)
    }

    pub fn remove_key(&mut self, name: &str) {
        self.keys.retain(|key| key.name != name);
    }

    pub fn remove_group(&mut self, name: &str) {
        if name == "All" {
            return; // Cannot remove "All" group
        }
        self.groups.retain(|group| group.name != name);
    }

    pub fn remove_host(&mut self, group_name: &str, host_name: &str) -> Result<()> {
        if group_name == "All" {
            return Err(anyhow::anyhow!("Cannot remove hosts from 'All' group directly"));
        }

        let group = self.groups.iter_mut()
            .find(|g| g.name == group_name)
            .ok_or_else(|| anyhow::anyhow!("Group '{}' not found", group_name))?;
        
        group.hosts.retain(|host| host.name != host_name);
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        let all_group = Group {
            name: "All".to_string(),
            color: "blue".to_string(),
            hosts: vec![],
        };

        let default_group = Group {
            name: "Default".to_string(),
            color: "green".to_string(),
            hosts: vec![],
        };

        Config {
            groups: vec![all_group, default_group],
            keys: vec![],
        }
    }
}
