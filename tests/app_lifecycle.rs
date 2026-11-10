//! Lifecycle checks use an isolated data folder, fake credentials, and no model calls.
use serde_json::{json, Value};
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "study-coach-lifecycle-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        if self.0.parent() == Some(std::env::temp_dir().as_path())
            && self
                .0
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("study-coach-lifecycle-")
        {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
}

struct RunningApp {
    child: Child,
    authority: String,
}

impl RunningApp {
    fn start(directory: &Path) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_study-coach-app"))
            .args(["--no-open", "--data-dir"])
            .arg(directory)
            .env_remove("GROQ_API_KEY")
            .env_remove("NVIDIA_API_KEY")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let stdout = child.stdout.take().unwrap();
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if let Some(url) = line.strip_prefix("Study Coach is running at http://") {
                    let _ = sender.send(url.trim_end_matches('/').to_owned());
                    break;
                }
            }
        });
        let mut app = Self {
            child,
            authority: String::new(),
        };
        app.authority = receiver
            .recv_timeout(Duration::from_secs(10))
            .expect("the local app did not announce its loopback address");
        app
    }

    fn request(
        &self,
        method: &str,
        path: &str,
        token: Option<&str>,
        origin: Option<&str>,
        body: Option<&str>,
    ) -> (u16, Value) {
        let body = body.unwrap_or("");
        let mut stream = TcpStream::connect(&self.authority).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut headers = format!("{method} {path} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n", self.authority, body.len());
        if let Some(origin) = origin {
            headers.push_str(&format!("Origin: {origin}\r\n"));
        }
        if let Some(token) = token {
            headers.push_str(&format!("X-Coach-Token: {token}\r\n"));
        }
        stream
            .write_all(format!("{headers}\r\n{body}").as_bytes())
            .unwrap();
        stream.flush().unwrap();
        let mut response = Vec::new();
        stream.read_to_end(&mut response).unwrap();
        let separator = response
            .windows(4)
            .position(|bytes| bytes == b"\r\n\r\n")
            .unwrap();
        let status = std::str::from_utf8(&response[..separator])
            .unwrap()
            .split_whitespace()
            .nth(1)
            .unwrap()
            .parse()
            .unwrap();
        (
            status,
            serde_json::from_slice(&response[separator + 4..]).unwrap(),
        )
    }

    fn state(&self) -> Value {
        let (status, state) = self.request("GET", "/api/state", None, None, None);
        assert_eq!(status, 200);
        state
    }

    fn post(&self, path: &str, token: &str, body: Value) -> (u16, Value) {
        self.request(
            "POST",
            path,
            Some(token),
            Some(&format!("http://{}", self.authority)),
            Some(&body.to_string()),
        )
    }

    fn wait_for_shutdown(&mut self) {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                assert!(status.success(), "shutdown must exit normally");
                break;
            }
            assert!(
                Instant::now() < deadline,
                "authorized shutdown did not stop the app"
            );
            thread::sleep(Duration::from_millis(20));
        }
    }
}

impl Drop for RunningApp {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

#[test]
fn authenticated_shutdown_exits_and_reopening_retains_local_data() {
    let directory = TestDirectory::new();
    let history = json!([{"id":"fixture-history","question":"What is a unit test?","output":{"status":"answer","answer":"A unit test checks one function in isolation.","citations":[{"source_id":"S3","quote":"A unit test checks one function in isolation."}]},"model":"fixture-only","created_at_unix":1}]);
    fs::write(directory.0.join("history.json"), history.to_string()).unwrap();
    let mut first = RunningApp::start(&directory.0);
    let state = first.state();
    let token = state["csrf_token"].as_str().unwrap();
    let origin = format!("http://{}", first.authority);
    assert_eq!(
        first.request("GET", "/api/shutdown", None, None, None).0,
        404
    );
    assert_eq!(
        first
            .request("POST", "/api/shutdown", None, Some(&origin), Some("{}"))
            .0,
        403
    );
    assert_eq!(
        first
            .request(
                "POST",
                "/api/shutdown",
                Some(token),
                Some("https://untrusted.invalid"),
                Some("{}")
            )
            .0,
        403
    );
    assert_eq!(
        first
            .request(
                "POST",
                "/api/shutdown",
                Some("wrong-token"),
                Some(&origin),
                Some("{}")
            )
            .0,
        403
    );
    assert_eq!(
        first
            .request(
                "POST",
                "/api/shutdown",
                Some(token),
                Some(&origin),
                Some("not-json")
            )
            .0,
        400
    );
    assert_eq!(first.state()["csrf_token"], token);
    assert!(first.child.try_wait().unwrap().is_none());
    let (status, _) = first.post(
        "/api/settings",
        token,
        json!({"profile":"groq-20b","groq_api_key":"lifecycle-only-fake-credential"}),
    );
    assert_eq!(status, 200);
    let (status, source) = first.post("/api/sources", token, json!({"title":"Keep my notes","text":"Closing the app must preserve this locally saved source."}));
    assert_eq!(status, 200);
    let credential_file = fs::read(directory.0.join(".env")).unwrap();
    let (status, response) = first.post("/api/shutdown", token, json!({}));
    assert_eq!(status, 200);
    assert_eq!(response, json!({"shutdown":true}));
    first.wait_for_shutdown();
    assert_eq!(fs::read(directory.0.join(".env")).unwrap(), credential_file);
    let mut reopened = RunningApp::start(&directory.0);
    let restored = reopened.state();
    assert_eq!(restored["selected_profile"], "groq-20b");
    assert_eq!(restored["keys"]["groq"], true);
    assert!(restored["sources"]
        .as_array()
        .unwrap()
        .contains(&source["source"]));
    assert_ne!(restored["csrf_token"], state["csrf_token"]);
    let (status, saved_history) = reopened.request("GET", "/api/history", None, None, None);
    assert_eq!(status, 200);
    assert_eq!(saved_history["entries"], history);
    assert_eq!(fs::read(directory.0.join(".env")).unwrap(), credential_file);
    assert_eq!(
        reopened
            .post(
                "/api/shutdown",
                restored["csrf_token"].as_str().unwrap(),
                json!({})
            )
            .0,
        200
    );
    reopened.wait_for_shutdown();
}
