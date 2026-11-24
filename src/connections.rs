//! User-added chat-completion connections live in one private local file.
//! Public profiles contain readiness flags; credential values stay in this module.

use crate::core;
use serde_json::{json, Value};
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;

const MAX_CONNECTIONS: usize = 32;
const MAX_FILE_BYTES: u64 = 256 * 1024;

#[derive(Clone)]
struct Connection {
    id: String,
    name: String,
    model: String,
    endpoint: String,
    provider_label: String,
    key: String,
}

pub struct Connections {
    entries: Vec<Connection>,
    next_id: u64,
}

impl Default for Connections {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            next_id: 1,
        }
    }
}

/// These templates use the OpenAI-compatible Chat Completions API only.
/// Model IDs and availability are supplied by each user's provider account.
pub fn catalog() -> Value {
    json!([
        {"id":"groq","label":"Groq","endpoint":"https://api.groq.com/openai/v1/chat/completions","requires_key":true,"docs_url":"https://console.groq.com/docs/openai"},
        {"id":"nvidia","label":"NVIDIA","endpoint":"https://integrate.api.nvidia.com/v1/chat/completions","requires_key":true,"docs_url":"https://docs.api.nvidia.com/nim/reference/llm-apis"},
        {"id":"openai","label":"OpenAI","endpoint":"https://api.openai.com/v1/chat/completions","requires_key":true,"docs_url":"https://platform.openai.com/docs/api-reference/chat/create"},
        {"id":"openrouter","label":"OpenRouter","endpoint":"https://openrouter.ai/api/v1/chat/completions","requires_key":true,"docs_url":"https://openrouter.ai/docs/api-reference/overview"},
        {"id":"ollama","label":"Ollama (local)","endpoint":"http://127.0.0.1:11434/v1/chat/completions","requires_key":false,"docs_url":"https://docs.ollama.com/api/openai-compatibility"},
        {"id":"custom","label":"Other OpenAI-compatible service","endpoint":"","requires_key":true}
    ])
}

fn valid_text(value: &str, maximum: usize) -> bool {
    !value.trim().is_empty() && value.len() <= maximum && !value.chars().any(char::is_control)
}

fn validate(connection: &Connection) -> Result<(), String> {
    if !valid_text(&connection.name, 80)
        || !valid_text(&connection.model, 256)
        || !valid_text(&connection.provider_label, 80)
        || connection.model.chars().any(char::is_whitespace)
    {
        return Err(
            "Enter a connection name within 80 bytes and a model ID within 256 bytes.".into(),
        );
    }
    let loopback = core::validate_endpoint(&connection.endpoint)?;
    if (!loopback && connection.key.is_empty())
        || connection.key.len() > 4096
        || !connection.key.is_ascii()
        || connection
            .key
            .chars()
            .any(|ch| ch.is_whitespace() || ch.is_control())
    {
        return Err(
            "Enter a valid API key. Only local loopback services can use an empty key.".into(),
        );
    }
    if !connection.key.is_empty()
        && [
            &connection.name,
            &connection.model,
            &connection.endpoint,
            &connection.provider_label,
        ]
        .iter()
        .any(|value| value.contains(&connection.key))
    {
        return Err("Keep credentials out of the connection name, model, and endpoint.".into());
    }
    Ok(())
}

fn stored(connection: &Connection) -> Value {
    json!({"id":connection.id,"name":connection.name,"model":connection.model,
        "endpoint":connection.endpoint,"provider_label":connection.provider_label,"api_key":connection.key})
}

fn validate_entries(entries: &[Connection]) -> Result<(), String> {
    for connection in entries {
        validate(connection)?;
        let metadata = [
            &connection.name,
            &connection.model,
            &connection.endpoint,
            &connection.provider_label,
        ];
        if entries.iter().any(|entry| {
            !entry.key.is_empty() && metadata.iter().any(|value| value.contains(&entry.key))
        }) {
            return Err(
                "Keep configured credentials out of connection names, models, and endpoints."
                    .into(),
            );
        }
    }
    Ok(())
}

pub fn load(directory: &Path) -> Result<Connections, String> {
    let path = directory.join("connections.json");
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Connections::default());
        }
        Err(_) => return Err("Cannot inspect saved connections.".into()),
    };
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > MAX_FILE_BYTES {
        return Err("Saved connections must be a regular file within 256 KiB.".into());
    }
    let mut text = String::new();
    fs::File::open(path)
        .and_then(|file| file.take(MAX_FILE_BYTES + 1).read_to_string(&mut text))
        .map_err(|_| "Cannot read saved connections.")?;
    if text.len() as u64 > MAX_FILE_BYTES {
        return Err("Saved connections exceed 256 KiB.".into());
    }
    let saved: Value =
        serde_json::from_str(&text).map_err(|_| "Saved connections are invalid JSON.")?;
    if saved["version"] != 1 {
        return Err("Saved connections use an unsupported format.".into());
    }
    let entries = saved["connections"]
        .as_array()
        .filter(|entries| entries.len() <= MAX_CONNECTIONS)
        .ok_or("Saved connections exceed the supported limit.")?;
    let mut connections = Connections {
        entries: Vec::new(),
        next_id: saved["next_id"]
            .as_u64()
            .filter(|id| *id > 0)
            .ok_or("Saved connection IDs are invalid.")?,
    };
    for entry in entries {
        let field = |name: &str| {
            entry[name]
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| "Saved connection fields are invalid.".to_owned())
        };
        let connection = Connection {
            id: field("id")?,
            name: field("name")?,
            model: field("model")?,
            endpoint: field("endpoint")?,
            provider_label: field("provider_label")?,
            key: field("api_key")?,
        };
        let identifier = connection
            .id
            .strip_prefix("custom-")
            .and_then(|id| id.parse::<u64>().ok())
            .filter(|id| *id > 0 && *id < connections.next_id)
            .ok_or("Saved connection IDs are invalid.")?;
        if connection.id != format!("custom-{identifier}")
            || connections
                .entries
                .iter()
                .any(|entry| entry.id == connection.id)
        {
            return Err("Saved connection IDs are invalid.".into());
        }
        validate(&connection)?;
        connections.entries.push(connection);
    }
    validate_entries(&connections.entries)?;
    Ok(connections)
}

impl Connections {
    pub fn load(directory: &Path) -> Result<Self, String> {
        load(directory)
    }

    pub fn profiles(&self) -> Vec<Value> {
        self.entries.iter().map(|connection| {
            json!({"id":connection.id,"name":connection.name,"provider":"openai-compatible",
                "provider_label":connection.provider_label,"model":connection.model,"endpoint":connection.endpoint,
                "key_configured":!connection.key.is_empty(),
                "requires_key":!core::validate_endpoint(&connection.endpoint).unwrap_or(false)})
        }).collect()
    }

    pub fn credentials(&self, id: &str) -> Option<(&str, &str)> {
        self.entries
            .iter()
            .find(|entry| entry.id == id)
            .map(|entry| (entry.endpoint.as_str(), entry.key.as_str()))
    }

    pub fn contains_secret(&self, text: &str) -> bool {
        self.entries.iter().any(|entry| {
            if entry.key.is_empty() {
                return false;
            }
            let escaped = json!(entry.key).to_string();
            text.contains(&entry.key) || text.contains(&escaped[1..escaped.len() - 1])
        })
    }

    pub fn add(&mut self, directory: &Path, body: &Value) -> Result<(), String> {
        if self.entries.len() >= MAX_CONNECTIONS {
            return Err(
                "You can save up to 32 custom connections. Remove one before adding another."
                    .into(),
            );
        }
        let field = |name: &str| {
            body[name]
                .as_str()
                .map(str::trim)
                .ok_or_else(|| format!("Connection {name} must be text."))
        };
        let provider = body
            .get("provider")
            .and_then(Value::as_str)
            .unwrap_or("custom");
        let templates = catalog();
        let template = templates
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["id"] == provider)
            .ok_or("Choose an available connection provider.")?;
        let endpoint = body
            .get("endpoint")
            .and_then(Value::as_str)
            .filter(|endpoint| !endpoint.trim().is_empty())
            .map(str::trim)
            .unwrap_or_else(|| template["endpoint"].as_str().unwrap());
        let connection = Connection {
            id: format!("custom-{}", self.next_id),
            name: field("name")?.to_owned(),
            model: field("model")?.to_owned(),
            endpoint: endpoint.to_owned(),
            provider_label: template["label"].as_str().unwrap().to_owned(),
            key: body
                .get("api_key")
                .map_or(Ok(""), |value| {
                    value.as_str().ok_or("Connection API key must be text.")
                })?
                .trim()
                .to_owned(),
        };
        validate(&connection)?;
        let next_id = self
            .next_id
            .checked_add(1)
            .ok_or("Connection ID limit reached.")?;
        let mut entries = self.entries.clone();
        entries.push(connection);
        Self::persist(directory, &entries, next_id)?;
        self.entries = entries;
        self.next_id = next_id;
        Ok(())
    }

    pub fn remove(&mut self, directory: &Path, id: &str) -> Result<(), String> {
        let mut entries = self.entries.clone();
        let position = entries
            .iter()
            .position(|entry| entry.id == id)
            .ok_or("That custom connection was not found.")?;
        entries.remove(position);
        Self::persist(directory, &entries, self.next_id)?;
        self.entries = entries;
        Ok(())
    }

    fn persist(directory: &Path, entries: &[Connection], next_id: u64) -> Result<(), String> {
        validate_entries(entries)?;
        let path = directory.join("connections.json");
        if fs::symlink_metadata(&path)
            .is_ok_and(|metadata| !metadata.is_file() || metadata.file_type().is_symlink())
        {
            return Err("Saved connections must be a regular file.".into());
        }
        let text = json!({"version":1,"next_id":next_id,
            "connections":entries.iter().map(stored).collect::<Vec<_>>()})
        .to_string();
        if text.len() as u64 > MAX_FILE_BYTES {
            return Err("Saved connections exceed 256 KiB.".into());
        }
        let temporary = directory.join(format!(".connections.{}.tmp", std::process::id()));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options
            .open(&temporary)
            .map_err(|_| "Cannot write a private connections file.")?;
        let result = (|| {
            file.write_all(text.as_bytes())
                .map_err(|_| "Cannot save connections.")?;
            file.sync_all()
                .map_err(|_| "Cannot finish saving connections.")?;
            drop(file);
            fs::rename(&temporary, path).map_err(|_| "Cannot replace saved connections.")?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(temporary);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn directory() -> std::path::PathBuf {
        static SERIAL: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "study-coach-connections-{}-{}",
            std::process::id(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn remote() -> Value {
        json!({"name":"My hosted model","model":"model-2026","provider":"custom",
            "endpoint":"https://models.example.org/v1/chat/completions","api_key":"test-connection-secret-001"})
    }

    #[test]
    fn saves_credentials_privately_reloads_and_never_reuses_deleted_ids() {
        let directory = directory();
        let mut connections = load(&directory).unwrap();
        connections.add(&directory, &remote()).unwrap();
        let profiles = connections.profiles();
        assert_eq!(profiles[0]["id"], "custom-1");
        assert_eq!(profiles[0]["key_configured"], true);
        assert!(!json!(profiles)
            .to_string()
            .contains("test-connection-secret-001"));
        assert!(connections.contains_secret("prefix test-connection-secret-001 suffix"));
        let mut reloaded = load(&directory).unwrap();
        assert_eq!(
            reloaded.credentials("custom-1").unwrap().1,
            "test-connection-secret-001"
        );
        reloaded.remove(&directory, "custom-1").unwrap();
        reloaded.add(&directory, &remote()).unwrap();
        assert_eq!(load(&directory).unwrap().profiles()[0]["id"], "custom-2");
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn rejects_invalid_or_leaking_credentials_before_persistence() {
        let directory = directory();
        let mut connections = load(&directory).unwrap();
        for patch in [
            json!({"api_key":""}),
            json!({"api_key":"dummy\r\nInjected: value"}),
            json!({"endpoint":"http://remote.example.org/v1/chat/completions"}),
            json!({"name":"test-connection-secret-001"}),
            json!({"endpoint":"https://models.example.org/v1?api_key=test-connection-secret-001"}),
            json!({"model":"bad model"}),
        ] {
            let mut value = remote();
            for (key, value_patch) in patch.as_object().unwrap() {
                value[key] = value_patch.clone();
            }
            assert!(connections.add(&directory, &value).is_err());
            assert!(connections.profiles().is_empty());
            assert!(!directory.join("connections.json").exists());
        }
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn local_service_is_ready_without_an_api_key() {
        let directory = directory();
        let mut connections = load(&directory).unwrap();
        connections
            .add(
                &directory,
                &json!({"name":"My local model","model":"local-model:8b","provider":"ollama"}),
            )
            .unwrap();
        let profile = &connections.profiles()[0];
        assert_eq!(profile["requires_key"], false);
        assert_eq!(profile["key_configured"], false);
        assert_eq!(connections.credentials("custom-1").unwrap().1, "");
        assert_eq!(load(&directory).unwrap().profiles(), connections.profiles());
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn failed_save_does_not_change_memory_and_malformed_storage_is_rejected() {
        let directory = directory();
        let mut connections = load(&directory).unwrap();
        let missing = directory.join("missing");
        assert!(connections.add(&missing, &remote()).is_err());
        assert!(connections.profiles().is_empty());
        fs::write(directory.join("connections.json"), "{broken").unwrap();
        assert!(load(&directory).is_err());
        fs::write(directory.join("connections.json"), json!({"version":1,"next_id":1,"connections":[{
            "id":"custom-1","name":"My model","model":"test","endpoint":"https://models.example.org/v1/chat/completions","provider_label":"Custom","api_key":"test-key-001"
        }]}).to_string()).unwrap();
        assert!(load(&directory).is_err());
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn refuses_cross_connection_secret_leaks_and_detects_escaped_output() {
        let directory = directory();
        let mut connections = Connections::default();
        let mut first = remote();
        first["api_key"] = json!("dummy-quoted-\"-key");
        connections.add(&directory, &first).unwrap();
        let mut second = remote();
        second["name"] = json!("dummy-quoted-\"-key");
        assert!(connections.add(&directory, &second).is_err());
        assert!(connections.contains_secret(&json!({"answer":"dummy-quoted-\"-key"}).to_string()));
        assert_eq!(load(&directory).unwrap().profiles().len(), 1);
        fs::remove_dir_all(directory).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn refuses_symlinked_storage_and_creates_owner_only_files() {
        use std::os::unix::fs::{symlink, PermissionsExt};
        let directory = directory();
        symlink(
            directory.join("missing"),
            directory.join("connections.json"),
        )
        .unwrap();
        assert!(load(&directory).is_err());
        fs::remove_file(directory.join("connections.json")).unwrap();
        let mut connections = load(&directory).unwrap();
        connections.add(&directory, &remote()).unwrap();
        assert_eq!(
            fs::metadata(directory.join("connections.json"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        fs::remove_dir_all(directory).unwrap();
    }
}
