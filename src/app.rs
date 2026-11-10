//! A private, single-user loopback server. Files own persistent state; the browser
//! receives only public state and a per-process CSRF token. Requests are serialized
//! so changing credentials cannot race an in-flight provider request.

use crate::core::{self, Completion, Settings};
use crate::retrieval;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const MAX_HEADER: usize = 8192;
const MAX_BODY: usize = 256 * 1024;
const MAX_SOURCES: usize = 64;
const MAX_SOURCE_TEXT: usize = 16 * 1024;
const MAX_CORPUS: usize = 180 * 1024;
const MAX_HISTORY: usize = 50;
const DEFAULT_PROFILE: &str = "groq-120b";
const ENV_TEMPLATE: &str =
    "# Study Coach credentials. Never share or commit this file.\nGROQ_API_KEY=\nNVIDIA_API_KEY=\n";

struct Keys {
    groq: String,
    nvidia: String,
}

impl Keys {
    fn contains_secret(&self, text: &str) -> bool {
        [&self.groq, &self.nvidia]
            .iter()
            .any(|key| !key.is_empty() && text.contains(key.as_str()))
    }

    fn install(&self) {
        for (name, value) in [
            ("GROQ_API_KEY", &self.groq),
            ("NVIDIA_API_KEY", &self.nvidia),
        ] {
            if value.is_empty() {
                env::remove_var(name);
            } else {
                env::set_var(name, value);
            }
        }
    }
}

struct App {
    // The OS releases this lock on normal exit or a crash. Keep the file itself
    // so another process can never lock a different inode for the same folder.
    _directory_lock: Option<File>,
    directory: PathBuf,
    sources: Value,
    history: Vec<Value>,
    profiles: Value,
    selected_profile: String,
    keys: Keys,
    token: String,
    authority: String,
    settings: Value,
    approach: String,
    context_mode: String,
    shutdown_requested: bool,
}

#[derive(Debug)]
struct Request {
    method: String,
    path: String,
    headers: BTreeMap<String, String>,
    body: Vec<u8>,
}

struct Response {
    status: u16,
    content_type: &'static str,
    body: Vec<u8>,
}

impl Response {
    fn json(status: u16, body: Value) -> Self {
        Self {
            status,
            content_type: "application/json; charset=utf-8",
            body: body.to_string().into_bytes(),
        }
    }

    fn error(status: u16, message: &str) -> Self {
        Self::json(status, json!({"error": message}))
    }
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |value| value.as_secs())
}

fn private_directory(path: &Path) -> Result<(), String> {
    let created = !path.exists();
    fs::create_dir_all(path).map_err(|_| "Cannot create the local app directory.")?;
    if !path.is_dir()
        || fs::symlink_metadata(path).is_ok_and(|value| value.file_type().is_symlink())
    {
        return Err("The app directory must be a real directory, not a link.".into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))
            .map_err(|_| "Cannot restrict permissions on the app directory.")?;
    }
    #[cfg(windows)]
    if created {
        // Fresh app directories inherit no broad parent grants. Existing directories
        // are left under their owner's ACL policy rather than silently rewriting it.
        let mut identity = Command::new("whoami.exe");
        hidden(&mut identity);
        let identity = identity
            .stderr(Stdio::null())
            .output()
            .map_err(|_| "Cannot identify the current Windows account.")?;
        if !identity.status.success() {
            return Err("Cannot identify the current Windows account.".into());
        }
        let account = String::from_utf8(identity.stdout)
            .map_err(|_| "Windows account name was not valid text.")?;
        let grant = format!("{}:(OI)(CI)F", account.trim());
        let mut acl = Command::new("icacls.exe");
        hidden(&mut acl);
        let success = acl
            .arg(path)
            .args(["/inheritance:r", "/grant:r", &grant])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success());
        if !success {
            return Err("Cannot restrict the new app directory's Windows permissions.".into());
        }
    }
    #[cfg(not(windows))]
    let _ = created;
    Ok(())
}

fn hidden(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    #[cfg(not(windows))]
    let _ = command;
}

fn private_file(path: &Path, create_new: bool) -> Result<File, String> {
    let mut options = OpenOptions::new();
    options.write(true);
    if create_new {
        options.create_new(true);
    } else {
        options.create(true).truncate(true);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options
        .open(path)
        .map_err(|_| "Cannot write a private app file.".into())
}

fn directory_lock(directory: &Path) -> Result<File, String> {
    let path = directory.join(".instance.lock");
    if fs::symlink_metadata(&path).is_ok_and(|value| value.file_type().is_symlink()) {
        return Err("The app lock must not be a symbolic link.".into());
    }
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = options
        .open(path)
        .map_err(|_| "Cannot open the local app lock.")?;
    file.try_lock().map_err(|error| match error {
        std::fs::TryLockError::WouldBlock =>
            "Study Coach is already running for this data folder. Use its browser window or close its launcher before starting again.",
        std::fs::TryLockError::Error(_) => "Cannot lock the local app data folder.",
    })?;
    Ok(file)
}

fn atomic_write(directory: &Path, name: &str, bytes: &[u8]) -> Result<(), String> {
    let temporary = directory.join(format!(".{name}.{}.tmp", std::process::id()));
    let mut file = private_file(&temporary, true)?;
    let result = (|| {
        file.write_all(bytes)
            .map_err(|_| "Cannot save local app data.")?;
        file.sync_all()
            .map_err(|_| "Cannot finish saving local app data.")?;
        drop(file);
        fs::rename(&temporary, directory.join(name))
            .map_err(|_| "Cannot replace local app data.")?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}

fn load_text(path: &Path, limit: u64) -> Result<String, String> {
    if fs::symlink_metadata(path).is_ok_and(|value| value.file_type().is_symlink()) {
        return Err("Local app files must not be symbolic links.".into());
    }
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(|_| "Cannot open local app data.")?
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Cannot read local app data.")?;
    if bytes.len() as u64 > limit {
        return Err("Local app data exceeds the supported size.".into());
    }
    String::from_utf8(bytes).map_err(|_| "Local app data must use UTF-8 text.".into())
}

fn parse_keys(text: &str) -> Result<BTreeMap<String, String>, String> {
    let mut result = BTreeMap::new();
    for line in text.trim_start_matches('\u{feff}').lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (name, raw) = line
            .split_once('=')
            .ok_or("Invalid credentials file assignment.")?;
        let name = name.trim();
        if !["GROQ_API_KEY", "NVIDIA_API_KEY"].contains(&name) {
            return Err("The credentials file contains an unsupported variable name.".into());
        }
        let raw = raw.trim();
        let value = if raw.starts_with('"') || raw.starts_with('\'') {
            let quote = raw.chars().next().unwrap();
            let end = raw[1..]
                .find(quote)
                .map(|index| index + 1)
                .ok_or("Unclosed quote in credentials file.")?;
            let suffix = raw[end + 1..].trim();
            if !suffix.is_empty() && !suffix.starts_with('#') {
                return Err("Invalid quoted credentials file assignment.".into());
            }
            &raw[1..end]
        } else {
            let end = raw
                .char_indices()
                .find(|(index, character)| {
                    *character == '#'
                        && (*index == 0 || raw[..*index].ends_with(char::is_whitespace))
                })
                .map(|(index, _)| index)
                .unwrap_or(raw.len());
            raw[..end].trim()
        };
        validate_key(value)?;
        result.insert(name.to_owned(), value.to_owned());
    }
    Ok(result)
}

fn validate_key(value: &str) -> Result<(), String> {
    if value.len() > 4096
        || !value.is_ascii()
        || value.chars().any(char::is_whitespace)
        || value.chars().any(char::is_control)
        || value.contains(['"', '\'', '#', '='])
    {
        return Err("An API key contains unsupported characters or is too long.".into());
    }
    Ok(())
}

fn load_keys(directory: &Path) -> Result<Keys, String> {
    let path = directory.join(".env");
    if !path.exists() {
        let mut file = private_file(&path, true)?;
        file.write_all(ENV_TEMPLATE.as_bytes())
            .map_err(|_| "Cannot create the credentials file.")?;
        file.sync_all()
            .map_err(|_| "Cannot finish creating the credentials file.")?;
    }
    let values = parse_keys(&load_text(&path, 16 * 1024)?)?;
    let get = |name: &str| {
        let value = env::var(name)
            .ok()
            .filter(|value| !value.trim().is_empty())
            .or_else(|| values.get(name).cloned())
            .unwrap_or_default();
        validate_key(&value)?;
        Ok::<_, String>(value)
    };
    Ok(Keys {
        groq: get("GROQ_API_KEY")?,
        nvidia: get("NVIDIA_API_KEY")?,
    })
}

/// Load an explicitly chosen credential file for a command line experiment.
/// Existing nonempty process values win. This never creates or rewrites a file,
/// and reports parser errors without including credential values.
pub fn configure_process_credentials(path: &Path) -> Result<(), String> {
    let values = parse_keys(&load_text(path, 16 * 1024)?)?;
    for name in ["GROQ_API_KEY", "NVIDIA_API_KEY"] {
        if env::var(name).is_ok_and(|value| !value.trim().is_empty()) {
            continue;
        }
        if let Some(value) = values.get(name) {
            validate_key(value)?;
            if !value.is_empty() {
                env::set_var(name, value);
            }
        }
    }
    Ok(())
}

fn random_token() -> Result<String, String> {
    let mut bytes = [0u8; 32];
    #[cfg(windows)]
    {
        #[link(name = "bcrypt")]
        extern "system" {
            fn BCryptGenRandom(
                algorithm: *mut std::ffi::c_void,
                buffer: *mut u8,
                length: u32,
                flags: u32,
            ) -> i32;
        }
        // Windows supplies cryptographic randomness without creating a key handle.
        let status = unsafe {
            BCryptGenRandom(
                std::ptr::null_mut(),
                bytes.as_mut_ptr(),
                bytes.len() as u32,
                2,
            )
        };
        if status != 0 {
            return Err("Cannot create a secure local session token.".into());
        }
    }
    #[cfg(unix)]
    File::open("/dev/urandom")
        .and_then(|mut file| file.read_exact(&mut bytes))
        .map_err(|_| "Cannot create a secure local session token.")?;
    #[cfg(not(any(windows, unix)))]
    return Err("This operating system is not supported.".into());
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn default_directory() -> Result<PathBuf, String> {
    #[cfg(windows)]
    return env::var_os("APPDATA")
        .map(|path| PathBuf::from(path).join("StudyCoach"))
        .ok_or("APPDATA is missing; provide --data-dir.".into());
    #[cfg(target_os = "macos")]
    return env::var_os("HOME")
        .map(|path| PathBuf::from(path).join("Library/Application Support/StudyCoach"))
        .ok_or("HOME is missing; provide --data-dir.".into());
    #[cfg(all(unix, not(target_os = "macos")))]
    return env::var_os("XDG_DATA_HOME")
        .map(|path| PathBuf::from(path).join("study-coach"))
        .or_else(|| {
            env::var_os("HOME").map(|path| PathBuf::from(path).join(".local/share/study-coach"))
        })
        .ok_or("No user data directory was found; provide --data-dir.".into());
    #[cfg(not(any(unix, windows)))]
    Err("Provide --data-dir on this operating system.".into())
}

fn validate_sources(notes: &Value) -> Result<(), String> {
    let sources = notes["sources"]
        .as_array()
        .filter(|items| items.len() <= MAX_SOURCES)
        .ok_or("The notes library is invalid or has too many sources.")?;
    let mut seen = BTreeSet::new();
    let mut total = 0;
    for source in sources {
        let id = source["id"].as_str().ok_or("A source ID is invalid.")?;
        let title = source["title"]
            .as_str()
            .ok_or("A source title is invalid.")?;
        let text = source["text"].as_str().ok_or("A source text is invalid.")?;
        if id.is_empty()
            || id.len() > 64
            || !id.chars().all(|c| c.is_ascii_alphanumeric())
            || !seen.insert(id)
            || title.trim().is_empty()
            || title.len() > 160
            || text.trim().is_empty()
            || text.len() > MAX_SOURCE_TEXT
        {
            return Err("A source is empty, duplicated, or exceeds the supported limits.".into());
        }
        total += id.len() + title.len() + text.len();
    }
    if total > MAX_CORPUS {
        return Err("The notes library is too large. Remove a source before adding more.".into());
    }
    Ok(())
}

impl App {
    fn load(
        directory: PathBuf,
        authority: String,
        approach: String,
        context_mode: String,
    ) -> Result<Self, String> {
        private_directory(&directory)?;
        let directory =
            fs::canonicalize(directory).map_err(|_| "Cannot resolve the app directory.")?;
        let directory_lock = directory_lock(&directory)?;
        let keys = load_keys(&directory)?;
        let settings: Value = serde_json::from_str(include_str!("../models.json"))
            .map_err(|_| "Bundled model settings are invalid.")?;
        let profiles = Value::Array(
            settings["profiles"]
                .as_array()
                .ok_or("Bundled model profiles are invalid.")?
                .iter()
                .map(|profile| {
                    let mut profile = profile.clone();
                    profile["id"] = profile["name"].clone();
                    profile
                })
                .collect(),
        );
        let sources_path = directory.join("sources.json");
        let sources: Value = if sources_path.exists() {
            serde_json::from_str(&load_text(&sources_path, (MAX_CORPUS * 8) as u64)?)
                .map_err(|_| "Saved sources are invalid JSON.")?
        } else {
            let mut notes: Value = serde_json::from_str(include_str!("../data/notes.json"))
                .map_err(|_| "Bundled notes are invalid.")?;
            notes["next_source_id"] = json!(1);
            atomic_write(&directory, "sources.json", notes.to_string().as_bytes())?;
            notes
        };
        validate_sources(&sources)?;
        let history_path = directory.join("history.json");
        let history: Vec<Value> = if history_path.exists() {
            let history: Vec<Value> =
                serde_json::from_str(&load_text(&history_path, 8 * 1024 * 1024)?)
                    .map_err(|_| "Saved study history is invalid JSON.")?;
            if history.len() > MAX_HISTORY {
                return Err("Saved study history exceeds the supported size.".into());
            }
            history
        } else {
            Vec::new()
        };
        let saved_settings = directory.join("settings.json");
        let selected_profile = if saved_settings.exists() {
            let value: Value = serde_json::from_str(&load_text(&saved_settings, 4096)?)
                .map_err(|_| "Saved app settings are invalid JSON.")?;
            value["profile"]
                .as_str()
                .unwrap_or(DEFAULT_PROFILE)
                .to_owned()
        } else {
            DEFAULT_PROFILE.to_owned()
        };
        if !profiles
            .as_array()
            .unwrap()
            .iter()
            .any(|profile| profile["id"] == selected_profile)
        {
            return Err("Saved model profile is no longer supported.".into());
        }
        if keys.contains_secret(&sources.to_string())
            || keys.contains_secret(&json!(history).to_string())
        {
            return Err("Saved study data contains a configured credential. Remove it locally before continuing.".into());
        }
        keys.install();
        Ok(Self {
            _directory_lock: Some(directory_lock),
            directory,
            sources,
            history,
            profiles,
            selected_profile,
            keys,
            token: random_token()?,
            authority,
            settings,
            approach,
            context_mode,
            shutdown_requested: false,
        })
    }

    fn state(&self) -> Value {
        json!({"sources":self.sources["sources"],"profiles":self.profiles,
            "selected_profile":self.selected_profile,"keys":{"groq":!self.keys.groq.is_empty(),
            "nvidia":!self.keys.nvidia.is_empty()},"data_dir":self.directory.to_string_lossy(),
            "version":env!("CARGO_PKG_VERSION"),"csrf_token":self.token,"approach":self.approach,
            "context_mode":self.context_mode})
    }

    fn authorized(&self, request: &Request) -> bool {
        if request.headers.get("host") != Some(&self.authority) {
            return false;
        }
        let origin = format!("http://{}", self.authority);
        if request
            .headers
            .get("origin")
            .is_some_and(|value| value != &origin)
        {
            return false;
        }
        if request.method != "GET" {
            return request.headers.get("origin") == Some(&origin)
                && request
                    .headers
                    .get("x-coach-token")
                    .is_some_and(|value| secure_equal(value, &self.token));
        }
        true
    }

    fn dispatch(&mut self, request: &Request) -> Response {
        if !self.authorized(request) {
            return Response::error(
                403,
                "This request did not come from the local Study Coach session.",
            );
        }
        match (request.method.as_str(), request.path.as_str()) {
            ("GET", "/") => Response {
                status: 200,
                content_type: "text/html; charset=utf-8",
                body: include_str!("../web/index.html").as_bytes().to_vec(),
            },
            ("GET", "/app.css") => Response {
                status: 200,
                content_type: "text/css; charset=utf-8",
                body: include_str!("../web/app.css").as_bytes().to_vec(),
            },
            ("GET", "/app.js") => Response {
                status: 200,
                content_type: "text/javascript; charset=utf-8",
                body: include_str!("../web/app.js").as_bytes().to_vec(),
            },
            ("GET", "/api/state") => Response::json(200, self.state()),
            ("GET", "/api/history") => Response::json(200, json!({"entries":self.history})),
            ("POST", path)
                if [
                    "/api/settings",
                    "/api/sources",
                    "/api/answer",
                    "/api/shutdown",
                ]
                .contains(&path) =>
            {
                if request
                    .headers
                    .get("content-type")
                    .map(|value| value.split(';').next().unwrap_or("").trim())
                    != Some("application/json")
                {
                    return Response::error(415, "Send application/json content.");
                }
                let body: Value = match serde_json::from_slice(&request.body) {
                    Ok(body) if Value::is_object(&body) => body,
                    _ => return Response::error(400, "The request must contain a JSON object."),
                };
                let result = match path {
                    "/api/settings" => self.save_settings(&body),
                    "/api/sources" => self.add_source(&body),
                    "/api/shutdown" => {
                        self.shutdown_requested = true;
                        Ok(json!({"shutdown":true}))
                    }
                    _ => self.answer(&body),
                };
                match result {
                    Ok(body) => Response::json(200, body),
                    Err((status, message)) => Response::error(status, &message),
                }
            }
            ("DELETE", path) if path.starts_with("/api/sources/") => {
                match self.delete_source(&path["/api/sources/".len()..]) {
                    Ok(value) => Response::json(200, value),
                    Err((status, message)) => Response::error(status, &message),
                }
            }
            _ => Response::error(404, "That page or operation was not found."),
        }
    }

    fn save_settings(&mut self, body: &Value) -> Result<Value, (u16, String)> {
        let profile = body["profile"].as_str().unwrap_or(&self.selected_profile);
        if !self
            .profiles
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["id"] == profile)
        {
            return Err((400, "Choose one of the available model profiles.".into()));
        }
        let mut keys = Keys {
            groq: self.keys.groq.clone(),
            nvidia: self.keys.nvidia.clone(),
        };
        let mut saved = parse_keys(
            &load_text(&self.directory.join(".env"), 16 * 1024).map_err(|error| (500, error))?,
        )
        .map_err(|error| (500, error))?;
        let mut changed_credentials = false;
        for (field, variable, key) in [
            ("groq_api_key", "GROQ_API_KEY", &mut keys.groq),
            ("nvidia_api_key", "NVIDIA_API_KEY", &mut keys.nvidia),
        ] {
            if let Some(value) = body.get(field) {
                let value = value
                    .as_str()
                    .ok_or((400, "API keys must be text.".into()))?
                    .trim();
                validate_key(value).map_err(|error| (400, error))?;
                if !value.is_empty() {
                    *key = value.to_owned();
                    saved.insert(variable.to_owned(), value.to_owned());
                    changed_credentials = true;
                }
            }
        }
        if keys.contains_secret(&self.sources.to_string())
            || keys.contains_secret(&json!(self.history).to_string())
        {
            return Err((
                400,
                "This credential already appears in study data. Remove that study data first."
                    .into(),
            ));
        }
        if changed_credentials {
            let text = format!("# Study Coach credentials. Never share or commit this file.\nGROQ_API_KEY={}\nNVIDIA_API_KEY={}\n",
                saved.get("GROQ_API_KEY").map_or("", String::as_str),
                saved.get("NVIDIA_API_KEY").map_or("", String::as_str));
            atomic_write(&self.directory, ".env", text.as_bytes()).map_err(|error| (500, error))?;
            self.keys = keys;
            self.keys.install();
        }
        atomic_write(
            &self.directory,
            "settings.json",
            json!({"profile":profile}).to_string().as_bytes(),
        )
        .map_err(|error| (500, error))?;
        self.selected_profile = profile.to_owned();
        Ok(self.state())
    }

    fn add_source(&mut self, body: &Value) -> Result<Value, (u16, String)> {
        let title = body["title"].as_str().unwrap_or("").trim();
        let text = body["text"].as_str().unwrap_or("").trim();
        if title.is_empty() || title.len() > 160 || text.is_empty() || text.len() > MAX_SOURCE_TEXT
        {
            return Err((
                400,
                "Add a title of at most 160 bytes and notes of at most 16 KiB.".into(),
            ));
        }
        if self.keys.contains_secret(title) || self.keys.contains_secret(text) {
            return Err((
                400,
                "A configured credential cannot be added to study notes.".into(),
            ));
        }
        let number = self.sources["next_source_id"].as_u64().unwrap_or(1);
        let source = json!({"id":format!("U{number}"),"title":title,"text":text});
        let mut next = self.sources.clone();
        next["sources"].as_array_mut().unwrap().push(source.clone());
        next["next_source_id"] = json!(number
            .checked_add(1)
            .ok_or((400, "Source ID limit reached.".into()))?);
        validate_sources(&next).map_err(|error| (400, error))?;
        atomic_write(&self.directory, "sources.json", next.to_string().as_bytes())
            .map_err(|error| (500, error))?;
        self.sources = next;
        Ok(json!({"source":source,"sources":self.sources["sources"]}))
    }

    fn delete_source(&mut self, id: &str) -> Result<Value, (u16, String)> {
        let mut next = self.sources.clone();
        let sources = next["sources"].as_array_mut().unwrap();
        let original = sources.len();
        sources.retain(|source| source["id"] != id);
        if original == sources.len() {
            return Err((404, "This source was not found.".into()));
        }
        atomic_write(&self.directory, "sources.json", next.to_string().as_bytes())
            .map_err(|error| (500, error))?;
        self.sources = next;
        Ok(json!({"sources":self.sources["sources"]}))
    }

    fn answer(&mut self, body: &Value) -> Result<Value, (u16, String)> {
        let question = body["question"]
            .as_str()
            .ok_or((400, "Enter a study question.".into()))?
            .trim();
        if question.len() > 4000 {
            return Err((400, "Keep your question within 4,000 bytes.".into()));
        }
        if self.keys.contains_secret(question) {
            return Err((
                400,
                "A configured credential cannot be sent as a study question.".into(),
            ));
        }
        let profile_id = body["profile"].as_str().unwrap_or(&self.selected_profile);
        let profile = self
            .profiles
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["id"] == profile_id)
            .ok_or((400, "Choose one of the available model profiles.".into()))?;
        if question.is_empty() || !question.chars().any(char::is_alphanumeric) {
            return Ok(
                json!({"output":{"status":"clarify","answer":"Enter a question about your notes, or ask for a quiz on a named topic.","citations":[]},
                "latency_ms":0,"model":profile["model"],"provider":profile["provider"],"profile":profile_id,"approach":self.approach,"local_response":true}),
            );
        }
        if self.sources["sources"].as_array().unwrap().is_empty() {
            return Err((
                400,
                "Add some course notes before asking a question.".into(),
            ));
        }
        let provider = profile["provider"].as_str().unwrap();
        let key = if provider == "groq" {
            &self.keys.groq
        } else {
            &self.keys.nvidia
        };
        if key.is_empty() {
            return Err((
                400,
                "Add an API key for the selected provider in Settings first.".into(),
            ));
        }
        let settings = Settings {
            provider: provider.to_owned(),
            model: profile["model"].as_str().unwrap().to_owned(),
            temperature: self.settings["temperature"].as_f64().unwrap(),
            max_tokens: self.settings["max_tokens"].as_u64().unwrap(),
            timeout_seconds: self.settings["timeout_seconds"].as_u64().unwrap(),
            retries: self.settings["retries"].as_u64().unwrap() as u32,
            approach: self.approach.clone(),
        };
        let context = if self.context_mode == "retrieval" {
            retrieval::select_sources(question, &self.sources).map_err(|error| (400, error))?
        } else {
            self.sources.clone()
        };
        let completion =
            core::answer(question, &context, &settings).map_err(|error| (502, error))?;
        let output =
            checked_output(&completion, &context, &self.keys).map_err(|error| (502, error))?;
        let identifier = random_token().map_err(|error| (500, error))?;
        let entry = json!({"id":format!("{}-{}",now(),&identifier[..16]),"question":question,
            "output":output,"profile":profile_id,"model":settings.model,"provider":provider,
            "latency_ms":completion.latency_ms,"created_at_unix":now(),"approach":self.approach,
            "context_mode":self.context_mode});
        let mut history = self.history.clone();
        history.push(entry);
        if history.len() > MAX_HISTORY {
            history.remove(0);
        }
        atomic_write(
            &self.directory,
            "history.json",
            json!(history).to_string().as_bytes(),
        )
        .map_err(|error| (500, error))?;
        self.history = history;
        Ok(
            json!({"output":output,"latency_ms":completion.latency_ms,"model":settings.model,
            "provider":provider,"profile":profile_id,"approach":self.approach,"input_tokens":completion.input_tokens,
            "output_tokens":completion.output_tokens,"attempts":completion.attempts,
            "context_mode":self.context_mode}),
        )
    }
}

fn checked_output(completion: &Completion, notes: &Value, keys: &Keys) -> Result<Value, String> {
    if keys.contains_secret(&completion.text)
        || completion
            .parsed
            .as_ref()
            .is_some_and(|output| keys.contains_secret(&output.to_string()))
    {
        return Err(
            "The provider response contained a local credential and was suppressed.".into(),
        );
    }
    let output = completion
        .parsed
        .as_ref()
        .filter(|output| output.is_object())
        .ok_or("The model did not return a usable answer. Try again or choose another model.")?;
    let status = output["status"].as_str().unwrap_or("");
    let answer = output["answer"]
        .as_str()
        .filter(|text| !text.trim().is_empty() && text.len() <= 24 * 1024)
        .ok_or("The model returned an empty or oversized answer. Try a shorter question.")?;
    if !["answer", "clarify", "abstain"].contains(&status) {
        return Err("The model returned an invalid response status. Try again.".into());
    }
    let citations = output["citations"]
        .as_array()
        .filter(|items| items.len() <= MAX_SOURCES)
        .ok_or("The model returned invalid citations. Try again.")?;
    if (status == "answer" && citations.is_empty()) || (status != "answer" && !citations.is_empty())
    {
        return Err(
            "The model did not provide the required source evidence. Try rephrasing your question."
                .into(),
        );
    }
    let mut ids = BTreeSet::new();
    let mut checked = Vec::new();
    for citation in citations {
        let id = citation["source_id"]
            .as_str()
            .ok_or("The model returned an invalid source reference.")?;
        let quote = citation["quote"]
            .as_str()
            .ok_or("The model returned an invalid quotation.")?;
        let source = notes["sources"]
            .as_array()
            .unwrap()
            .iter()
            .find(|source| source["id"] == id);
        if !ids.insert(id)
            || quote.chars().count() < 12
            || !source.is_some_and(|source| {
                source["text"]
                    .as_str()
                    .is_some_and(|text| text.contains(quote))
            })
        {
            return Err("A model quotation could not be verified against your notes. The answer was withheld; try again.".into());
        }
        checked.push(json!({"source_id":id,"quote":quote}));
    }
    // Return only documented fields. Unrecognized model fields are not persisted
    // or rendered, even when the provider returns a larger JSON object.
    Ok(json!({"status":status,"answer":answer,"citations":checked}))
}

fn secure_equal(left: &str, right: &str) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.bytes()
        .zip(right.bytes())
        .fold(0u8, |difference, (a, b)| difference | (a ^ b))
        == 0
}

fn read_request(stream: &mut TcpStream) -> Result<Request, (u16, &'static str)> {
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|_| (400, "Cannot read this request."))?;
    let mut bytes = Vec::new();
    let end = loop {
        if bytes.len() >= MAX_HEADER {
            return Err((431, "Request headers are too large."));
        }
        let mut byte = [0u8; 1];
        if stream
            .read(&mut byte)
            .map_err(|_| (408, "Request timed out."))?
            == 0
        {
            return Err((400, "Request headers were incomplete."));
        }
        bytes.push(byte[0]);
        if bytes.ends_with(b"\r\n\r\n") {
            break bytes.len();
        }
    };
    let text = std::str::from_utf8(&bytes[..end - 4])
        .map_err(|_| (400, "Request headers must be valid text."))?;
    let mut lines = text.split("\r\n");
    let parts: Vec<_> = lines.next().unwrap_or("").split(' ').collect();
    if parts.len() != 3
        || !["GET", "POST", "DELETE"].contains(&parts[0])
        || parts[2] != "HTTP/1.1"
        || !parts[1].starts_with('/')
        || parts[1].chars().any(char::is_control)
    {
        return Err((400, "Unsupported HTTP request."));
    }
    let method = parts[0].to_owned();
    let path = parts[1].to_owned();
    let mut headers = BTreeMap::new();
    for line in lines {
        let (name, value) = line
            .split_once(':')
            .ok_or((400, "Malformed HTTP header."))?;
        if name.is_empty()
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
            || value
                .chars()
                .any(|character| character.is_control() && character != '\t')
        {
            return Err((400, "Malformed HTTP header."));
        }
        if headers
            .insert(name.to_ascii_lowercase(), value.trim().to_owned())
            .is_some()
        {
            return Err((400, "Duplicate HTTP headers are not supported."));
        }
    }
    if headers.contains_key("transfer-encoding") || headers.contains_key("expect") {
        return Err((400, "Transfer encoding and expectations are not supported."));
    }
    let length = match headers.get("content-length") {
        Some(value) if !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()) => {
            value
                .parse::<usize>()
                .map_err(|_| (413, "Request body is too large."))?
        }
        Some(_) => return Err((400, "Invalid Content-Length.")),
        None if method == "POST" => return Err((411, "Content-Length is required.")),
        None => 0,
    };
    if length > MAX_BODY {
        return Err((413, "Request body is too large."));
    }
    let mut body = vec![0u8; length];
    stream
        .read_exact(&mut body)
        .map_err(|_| (400, "Request body was incomplete or timed out."))?;
    Ok(Request {
        method,
        path,
        headers,
        body,
    })
}

fn write_response(stream: &mut TcpStream, response: &Response) -> std::io::Result<()> {
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    let reason = match response.status {
        200 => "OK",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        408 => "Request Timeout",
        411 => "Length Required",
        413 => "Content Too Large",
        415 => "Unsupported Media Type",
        431 => "Request Header Fields Too Large",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        _ => "Error",
    };
    write!(stream,"HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nReferrer-Policy: no-referrer\r\nContent-Security-Policy: default-src 'self'; connect-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data:; base-uri 'none'; frame-ancestors 'none'; form-action 'self'\r\n\r\n",
        response.status,reason,response.content_type,response.body.len())?;
    stream.write_all(&response.body)?;
    stream.flush()
}

fn open_browser(url: &str) {
    #[cfg(windows)]
    let mut command = {
        let mut command = Command::new("rundll32.exe");
        command.args(["url.dll,FileProtocolHandler", url]);
        command
    };
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = Command::new("open");
        command.arg(url);
        command
    };
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = {
        let mut command = Command::new("xdg-open");
        command.arg(url);
        command
    };
    #[cfg(any(unix, windows))]
    {
        hidden(&mut command);
        let _ = command.stdout(Stdio::null()).stderr(Stdio::null()).spawn();
    }
    #[cfg(not(any(unix, windows)))]
    let _ = url;
}

/// Start the installed app. The service always binds to IPv4 loopback and never
/// exposes an option to listen on a public interface.
pub fn run(arguments: impl Iterator<Item = String>) -> Result<(), String> {
    let mut directory = None;
    let mut port = 0u16;
    let mut browser = true;
    let mut approach = "grounded".to_owned();
    let mut context_mode = "full".to_owned();
    let mut credentials_file = None;
    let mut arguments = arguments.peekable();
    let mut seen = BTreeSet::new();
    while let Some(argument) = arguments.next() {
        if !seen.insert(argument.clone()) {
            return Err("Do not repeat command options.".into());
        }
        match argument.as_str() {
            "--data-dir" => {
                directory = Some(PathBuf::from(
                    arguments.next().ok_or("--data-dir needs a path.")?,
                ));
            }
            "--port" => {
                port = arguments
                    .next()
                    .ok_or("--port needs a number.")?
                    .parse()
                    .map_err(|_| "--port must be between 0 and 65535.")?;
            }
            "--no-open" => browser = false,
            "--credentials-file" => {
                credentials_file = Some(PathBuf::from(
                    arguments.next().ok_or("--credentials-file needs a path.")?,
                ));
            }
            "--approach" => {
                approach = arguments
                    .next()
                    .ok_or("--approach needs baseline or grounded.")?;
                if !["baseline", "grounded"].contains(&approach.as_str()) {
                    return Err("--approach must be baseline or grounded.".into());
                }
            }
            "--context" => {
                context_mode = arguments
                    .next()
                    .ok_or("--context needs full or retrieval.")?;
                if !["full", "retrieval"].contains(&context_mode.as_str()) {
                    return Err("--context must be full or retrieval.".into());
                }
            }
            "--help" | "-h" => {
                println!("Study Coach\nUsage: study-coach-app [--data-dir PATH] [--port 0..65535] [--no-open] [--approach baseline|grounded] [--context full|retrieval] [--credentials-file PATH]\nThe app stores notes, history, and .env locally. Only loopback connections are accepted. An explicit credential file is read without copying it into app data.");
                return Ok(());
            }
            _ => return Err("Unknown option. Run study-coach-app --help.".into()),
        }
    }
    let listener = TcpListener::bind(("127.0.0.1", port))
        .map_err(|_| "Cannot bind the local Study Coach port.")?;
    let authority = listener
        .local_addr()
        .map_err(|_| "Cannot read the local address.")?
        .to_string();
    if let Some(path) = credentials_file {
        configure_process_credentials(&path)?;
    }
    let mut app = App::load(
        directory.map_or_else(default_directory, Ok)?,
        authority.clone(),
        approach,
        context_mode,
    )?;
    let url = format!("http://{authority}/");
    println!("Study Coach is running at {url}\nNotes and credentials stay in the local app directory. Cloud answers send the question and current notes to the selected provider.\nPress Ctrl+C to stop.");
    if browser {
        open_browser(&url);
    }
    for connection in listener.incoming() {
        let Ok(mut stream) = connection else {
            continue;
        };
        if !stream
            .peer_addr()
            .is_ok_and(|address| address.ip().is_loopback())
        {
            continue;
        }
        let response = match read_request(&mut stream) {
            Ok(request) => app.dispatch(&request),
            Err((status, message)) => Response::error(status, message),
        };
        let _ = write_response(&mut stream, &response);
        if app.shutdown_requested {
            break;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys() -> Keys {
        Keys {
            groq: "dummy-groq-key-not-a-secret".into(),
            nvidia: String::new(),
        }
    }

    fn completion(output: Value) -> Completion {
        Completion {
            text: output.to_string(),
            parsed: Some(output),
            latency_ms: 1.0,
            input_tokens: None,
            output_tokens: None,
            attempts: 1,
        }
    }

    fn notes() -> Value {
        json!({"sources":[{"id":"S1","title":"Tests","text":"A unit test checks one function in isolation."}]})
    }

    fn request(method: &str, path: &str, origin: Option<&str>, token: Option<&str>) -> Request {
        let mut headers = BTreeMap::from([("host".into(), "127.0.0.1:4000".into())]);
        if let Some(origin) = origin {
            headers.insert("origin".into(), origin.into());
        }
        if let Some(token) = token {
            headers.insert("x-coach-token".into(), token.into());
        }
        Request {
            method: method.into(),
            path: path.into(),
            headers,
            body: Vec::new(),
        }
    }

    fn app() -> App {
        App {
            _directory_lock: None,
            directory: PathBuf::new(),
            sources: notes(),
            history: Vec::new(),
            profiles: json!([]),
            selected_profile: DEFAULT_PROFILE.into(),
            keys: keys(),
            token: "0123456789abcdef".into(),
            authority: "127.0.0.1:4000".into(),
            settings: json!({}),
            approach: "grounded".into(),
            context_mode: "full".into(),
            shutdown_requested: false,
        }
    }

    #[test]
    fn csrf_origin_and_host_are_all_required_for_mutations() {
        let app = app();
        assert!(app.authorized(&request("GET", "/api/state", None, None)));
        assert!(!app.authorized(&request(
            "GET",
            "/api/state",
            Some("https://evil.example"),
            None
        )));
        assert!(!app.authorized(&request("POST", "/api/settings", None, Some(&app.token))));
        assert!(!app.authorized(&request(
            "POST",
            "/api/settings",
            Some("http://127.0.0.1:4000"),
            None
        )));
        assert!(!app.authorized(&request(
            "POST",
            "/api/settings",
            Some("https://evil.example"),
            Some(&app.token)
        )));
        assert!(app.authorized(&request(
            "POST",
            "/api/settings",
            Some("http://127.0.0.1:4000"),
            Some(&app.token)
        )));
        let mut rebound = request("GET", "/api/state", None, None);
        rebound
            .headers
            .insert("host".into(), "evil.example:4000".into());
        assert!(!app.authorized(&rebound));
    }

    #[test]
    fn output_checks_reject_stitched_invented_missing_and_duplicate_quotes() {
        let notes = notes();
        let valid = json!({"status":"answer","answer":"One function.","citations":[{"source_id":"S1","quote":"A unit test checks one function in isolation."}]});
        assert!(checked_output(&completion(valid.clone()), &notes, &keys()).is_ok());
        for citations in [
            json!([]),
            json!([{"source_id":"S1","quote":"A unit test ... in isolation."}]),
            json!([{"source_id":"S99","quote":"A unit test checks one function in isolation."}]),
            json!([{"source_id":"S1","quote":"A unit test checks one function in isolation."},{"source_id":"S1","quote":"A unit test checks one function in isolation."}]),
        ] {
            let mut output = valid.clone();
            output["citations"] = citations;
            assert!(checked_output(&completion(output), &notes, &keys()).is_err());
        }
    }

    #[test]
    fn secrets_never_appear_in_public_state_or_accepted_model_output() {
        let app = app();
        assert!(!app.state().to_string().contains(&app.keys.groq));
        assert_eq!(app.state()["keys"]["groq"], true);
        let output = json!({"status":"abstain","answer":app.keys.groq,"citations":[]});
        let error = checked_output(&completion(output), &notes(), &app.keys).unwrap_err();
        assert!(!error.contains(&app.keys.groq));
        let mut escaped =
            completion(json!({"status":"abstain","answer":app.keys.groq,"citations":[]}));
        escaped.text = escaped.text.replace('-', "\\u002d");
        assert!(!escaped.text.contains(&app.keys.groq));
        assert!(checked_output(&escaped, &notes(), &app.keys).is_err());
    }

    #[test]
    fn clarify_and_abstain_require_empty_citations() {
        for status in ["clarify", "abstain"] {
            assert!(checked_output(
                &completion(
                    json!({"status":status,"answer":"Please clarify the question.","citations":[]})
                ),
                &notes(),
                &keys()
            )
            .is_ok());
            assert!(checked_output(&completion(json!({"status":status,"answer":"Please clarify the question.","citations":[{"source_id":"S1","quote":"A unit test checks one function in isolation."}]})),&notes(),&keys()).is_err());
        }
    }

    #[test]
    fn source_limits_enforce_unique_ids_nonempty_text_and_total_size() {
        assert!(validate_sources(&notes()).is_ok());
        assert!(validate_sources(&json!({"sources":[]})).is_ok());
        assert!(validate_sources(
            &json!({"sources":[{"id":"../secret","title":"Bad","text":"bad"}]})
        )
        .is_err());
        let mut duplicate = notes();
        duplicate["sources"]
            .as_array_mut()
            .unwrap()
            .push(notes()["sources"][0].clone());
        assert!(validate_sources(&duplicate).is_err());
        let mut large = notes();
        large["sources"][0]["text"] = json!("x".repeat(MAX_SOURCE_TEXT + 1));
        assert!(validate_sources(&large).is_err());
    }

    #[test]
    fn credentials_parser_accepts_quotes_and_comments_but_not_injection() {
        let result =
            parse_keys("\u{feff}# note\nGROQ_API_KEY='dummy-a' # comment\nNVIDIA_API_KEY=\n")
                .unwrap();
        assert_eq!(result["GROQ_API_KEY"], "dummy-a");
        for text in [
            "GROQ_API_KEY=bad value",
            "UNSUPPORTED_KEY=dummy",
            "GROQ_API_KEY='unterminated",
        ] {
            let error = parse_keys(text).unwrap_err();
            assert!(!error.contains("dummy"));
            assert!(!error.contains("bad value"));
        }
    }

    fn wire_request(text: &str) -> Result<Request, (u16, &'static str)> {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let text = text.to_owned();
        let sender = std::thread::spawn(move || {
            let mut stream = TcpStream::connect(address).unwrap();
            stream.write_all(text.as_bytes()).unwrap();
        });
        let (mut stream, _) = listener.accept().unwrap();
        let result = read_request(&mut stream);
        sender.join().unwrap();
        result
    }

    #[test]
    fn real_http_parser_rejects_ambiguous_lengths_and_oversize_bodies() {
        assert_eq!(wire_request("POST /api/settings HTTP/1.1\r\nHost: 127.0.0.1:4000\r\nContent-Length: 0\r\nContent-Length: 0\r\n\r\n").unwrap_err().0,400);
        assert_eq!(wire_request("POST /api/settings HTTP/1.1\r\nHost: 127.0.0.1:4000\r\nTransfer-Encoding: chunked\r\n\r\n").unwrap_err().0,400);
        assert_eq!(wire_request("POST /api/settings HTTP/1.1\r\nHost: 127.0.0.1:4000\r\nContent-Length: 99999999\r\n\r\n").unwrap_err().0,413);
        let request = wire_request(
            "POST /api/settings HTTP/1.1\r\nHost: 127.0.0.1:4000\r\nContent-Length: 2\r\n\r\n{}",
        )
        .unwrap();
        assert_eq!(request.body, b"{}");
    }

    #[test]
    fn static_routes_do_not_allow_path_traversal() {
        let mut app = app();
        for path in [
            "/../.env",
            "/%2e%2e/.env",
            "/sources.json",
            "/.env",
            "/app.js?../../.env",
        ] {
            assert_eq!(app.dispatch(&request("GET", path, None, None)).status, 404);
        }
        assert_eq!(app.dispatch(&request("GET", "/", None, None)).status, 200);
    }

    #[test]
    fn csrf_tokens_use_32_random_bytes_and_differ_between_sessions() {
        let first = random_token().unwrap();
        assert_eq!(first.len(), 64);
        assert!(first.bytes().all(|byte| byte.is_ascii_hexdigit()));
        assert_ne!(first, random_token().unwrap());
    }

    fn storage_app() -> App {
        let directory =
            env::temp_dir().join(format!("study-coach-app-test-{}", random_token().unwrap()));
        private_directory(&directory).unwrap();
        private_file(&directory.join(".env"), true)
            .unwrap()
            .write_all(ENV_TEMPLATE.as_bytes())
            .unwrap();
        let mut app = app();
        app.directory = directory;
        app.profiles = json!([{"id":DEFAULT_PROFILE}]);
        app.sources["next_source_id"] = json!(1);
        app
    }

    #[test]
    fn profile_changes_do_not_persist_credentials_from_the_process() {
        let mut app = storage_app();
        let before = load_text(&app.directory.join(".env"), 16384).unwrap();
        app.save_settings(
            &json!({"profile":DEFAULT_PROFILE,"groq_api_key":"","nvidia_api_key":""}),
        )
        .unwrap();
        let after = load_text(&app.directory.join(".env"), 16384).unwrap();
        assert_eq!(before, after);
        assert!(!after.contains(&app.keys.groq));
        assert!(!load_text(&app.directory.join("settings.json"), 4096)
            .unwrap()
            .contains(&app.keys.groq));
        fs::remove_dir_all(&app.directory).unwrap();
    }

    #[test]
    fn source_storage_survives_reload_and_does_not_reuse_deleted_ids() {
        let mut app = storage_app();
        let first=app.add_source(&json!({"title":"Private notes","text":"Retrieval finds relevant course passages."})).unwrap();
        assert_eq!(first["source"]["id"], "U1");
        let saved: Value = serde_json::from_str(
            &load_text(&app.directory.join("sources.json"), MAX_BODY as u64).unwrap(),
        )
        .unwrap();
        assert_eq!(saved, app.sources);
        app.delete_source("U1").unwrap();
        let second = app
            .add_source(
                &json!({"title":"More notes","text":"A quiz includes a question and its answer."}),
            )
            .unwrap();
        assert_eq!(second["source"]["id"], "U2");
        assert!(app
            .add_source(&json!({"title":"Key","text":app.keys.groq}))
            .is_err());
        assert_eq!(app.delete_source("doesnotexist").unwrap_err().0, 404);
        fs::remove_dir_all(&app.directory).unwrap();
    }

    #[test]
    fn directory_lock_refuses_second_owner_and_releases_on_drop() {
        let mut app = storage_app();
        let directory = app.directory.clone();
        app._directory_lock = Some(directory_lock(&directory).unwrap());
        app.add_source(&json!({"title":"Keep this source","text":"The first writer must not lose its saved notes."})).unwrap();
        assert!(directory_lock(&directory)
            .unwrap_err()
            .contains("already running"));
        let expected = app.sources.clone();
        drop(app);
        let replacement = directory_lock(&directory).unwrap();
        let saved: Value = serde_json::from_str(
            &load_text(&directory.join("sources.json"), MAX_BODY as u64).unwrap(),
        )
        .unwrap();
        assert_eq!(saved, expected);
        drop(replacement);
        fs::remove_dir_all(directory).unwrap();
    }
}
