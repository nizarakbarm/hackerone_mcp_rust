//! Layered configuration loader (ADR-0004).
//!
//! Highest precedence first: `--config` file, CLI flags, environment,
//! `./hackerone-mcp.toml`, `$XDG_CONFIG_HOME/hackerone-mcp/config.toml`
//! (falling back to `~/.config/hackerone-mcp/config.toml`). Layers merge per
//! field: a later source only fills what earlier sources left unset.

use std::{
    io,
    path::{Path, PathBuf},
};

use serde::Deserialize;

/// Env var holding the config directory (XDG base directory spec).
const ENV_XDG_CONFIG_HOME: &str = "XDG_CONFIG_HOME";
/// Env var holding the home directory, used when `XDG_CONFIG_HOME` is unset.
const ENV_HOME: &str = "HOME";

/// Effective configuration after merging every source.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Config {
    pub api_username: Option<String>,
    pub api_token: Option<String>,
    /// Hacker-side token pair (ADR-0008); independent of the customer pair.
    pub hacker_api_username: Option<String>,
    pub hacker_api_token: Option<String>,
    pub program_handle: Option<String>,
    pub base_url: Option<String>,
    pub bind: Option<String>,
    pub http_token: Option<String>,
}

impl Config {
    /// Fill every field this config left `None` from `other` (lower precedence).
    fn fill_from(&mut self, other: Self) {
        macro_rules! fill {
            ($($field:ident),+ $(,)?) => {
                $(
                    if self.$field.is_none() {
                        self.$field = other.$field;
                    }
                )+
            };
        }
        fill!(
            api_username,
            api_token,
            hacker_api_username,
            hacker_api_token,
            program_handle,
            base_url,
            bind,
            http_token,
        );
    }
}

/// Command-line layer; `None` means the flag was not passed.
#[derive(Debug, Default, Clone)]
pub struct Flags {
    /// Explicit config file, highest precedence; missing/unreadable is fatal.
    pub config: Option<PathBuf>,
    pub api_username: Option<String>,
    pub api_token: Option<String>,
    pub program_handle: Option<String>,
    pub base_url: Option<String>,
    pub bind: Option<String>,
}

impl From<&Flags> for Config {
    fn from(flags: &Flags) -> Self {
        Self {
            api_username: flags.api_username.clone(),
            api_token: flags.api_token.clone(),
            hacker_api_username: None,
            hacker_api_token: None,
            program_handle: flags.program_handle.clone(),
            base_url: flags.base_url.clone(),
            bind: flags.bind.clone(),
            http_token: None,
        }
    }
}

/// Configuration failures. Server-local: the api crate's error enum does not
/// carry config causes.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("cannot read config file {}: {source}", path.display())]
    Read {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("invalid config file {}: {reason}", path.display())]
    Parse { path: PathBuf, reason: String },
    #[error(
        "refusing config file {}: mode {mode:03o} is group/world accessible; chmod 600 it",
        path.display()
    )]
    InsecurePermissions { path: PathBuf, mode: u32 },
}

/// Merge every source. `cwd` is a parameter (not the process cwd) so tests and
/// embedders can pin the `./hackerone-mcp.toml` lookup; `env` is the only env
/// view, so tests never touch the process environment.
pub fn load<F>(flags: &Flags, cwd: &Path, env: F) -> Result<Config, ConfigError>
where
    F: Fn(&str) -> Option<String>,
{
    let var = |key: &str| env(key).filter(|value| !value.is_empty());

    // Highest precedence first; each layer only fills what is still unset.
    let mut config = match &flags.config {
        Some(path) => read_file(path)?,
        None => Config::default(),
    };
    config.fill_from(Config::from(flags));
    config.fill_from(Config {
        api_username: var("HACKERONE_API_USERNAME"),
        api_token: var("HACKERONE_API_TOKEN"),
        hacker_api_username: var("HACKERONE_HACKER_API_USERNAME"),
        hacker_api_token: var("HACKERONE_HACKER_API_TOKEN"),
        program_handle: var("HACKERONE_PROGRAM_HANDLE"),
        base_url: var("HACKERONE_API_BASE_URL"),
        bind: None,
        http_token: var("HACKERONE_MCP_HTTP_TOKEN"),
    });
    merge_if_present(&mut config, cwd.join("hackerone-mcp.toml"))?;

    let config_home = var(ENV_XDG_CONFIG_HOME)
        .map(PathBuf::from)
        .or_else(|| var(ENV_HOME).map(|home| PathBuf::from(home).join(".config")));
    if let Some(dir) = config_home {
        merge_if_present(&mut config, dir.join("hackerone-mcp").join("config.toml"))?;
    }
    Ok(config)
}

/// Implicit config files are optional: only merge them when they exist.
fn merge_if_present(config: &mut Config, path: PathBuf) -> Result<(), ConfigError> {
    if path.is_file() {
        config.fill_from(read_file(&path)?);
    }
    Ok(())
}

fn read_file(path: &Path) -> Result<Config, ConfigError> {
    let text = std::fs::read_to_string(path).map_err(|source| ConfigError::Read {
        path: path.to_owned(),
        source,
    })?;
    let file: FileConfig = toml::from_str(&text).map_err(|err| ConfigError::Parse {
        path: path.to_owned(),
        reason: err.to_string(),
    })?;
    let config = file.into_config();
    if config.api_token.is_some()
        || config.hacker_api_token.is_some()
        || config.http_token.is_some()
    {
        check_secret_permissions(path)?;
    }
    Ok(config)
}

/// A config file that carries a token must not be readable by group or other.
#[cfg(unix)]
fn check_secret_permissions(path: &Path) -> Result<(), ConfigError> {
    use std::os::unix::fs::PermissionsExt;

    let mode = std::fs::metadata(path)
        .map_err(|source| ConfigError::Read {
            path: path.to_owned(),
            source,
        })?
        .permissions()
        .mode()
        & 0o777;
    if mode & 0o077 != 0 {
        return Err(ConfigError::InsecurePermissions {
            path: path.to_owned(),
            mode,
        });
    }
    Ok(())
}

#[cfg(not(unix))]
fn check_secret_permissions(_path: &Path) -> Result<(), ConfigError> {
    Ok(())
}

/// TOML shape: `[api]`, `[hacker_api]` and `[http]` only; typos fail instead
/// of falling back.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileConfig {
    #[serde(default)]
    api: ApiSection,
    #[serde(default)]
    hacker_api: HackerApiSection,
    #[serde(default)]
    http: HttpSection,
}

impl FileConfig {
    fn into_config(self) -> Config {
        Config {
            api_username: self.api.username,
            api_token: self.api.token,
            hacker_api_username: self.hacker_api.username,
            hacker_api_token: self.hacker_api.token,
            program_handle: self.api.program_handle,
            base_url: self.api.base_url,
            bind: self.http.bind,
            http_token: self.http.token,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct HackerApiSection {
    username: Option<String>,
    token: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct ApiSection {
    username: Option<String>,
    token: Option<String>,
    program_handle: Option<String>,
    base_url: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct HttpSection {
    bind: Option<String>,
    token: Option<String>,
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    /// Self-cleaning temp directory; avoids a dev-dependency.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(tag: &str) -> Self {
            let path = std::env::temp_dir()
                .join(format!("hackerone-mcp-config-{}-{tag}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).expect("create temp dir");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }

        fn write(&self, name: &str, body: &str) -> PathBuf {
            let path = self.0.join(name);
            std::fs::write(&path, body).expect("write fixture");
            path
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn env(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect()
    }

    fn load_with(
        dir: &TempDir,
        flags: Flags,
        vars: &HashMap<String, String>,
    ) -> Result<Config, ConfigError> {
        load(&flags, dir.path(), |key| vars.get(key).cloned())
    }

    const FULL: &str = r#"
[api]
username = "file-user"
token = "file-token"
program_handle = "file-program"
base_url = "https://file.example"

[http]
bind = "127.0.0.1:9000"
token = "file-http-token"

[hacker_api]
username = "file-hacker-user"
token = "file-hacker-token"
"#;

    fn chmod(path: &Path, mode: u32) {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).expect("chmod");
    }

    #[test]
    fn single_source_explicit_file_fills_every_field() {
        let dir = TempDir::new("single");
        let path = dir.write("hackerone.toml", FULL);
        chmod(&path, 0o600);

        let config = load_with(
            &dir,
            Flags {
                config: Some(path),
                ..Flags::default()
            },
            &HashMap::new(),
        )
        .expect("load explicit config");

        assert_eq!(config.api_username.as_deref(), Some("file-user"));
        assert_eq!(config.api_token.as_deref(), Some("file-token"));
        assert_eq!(
            config.hacker_api_username.as_deref(),
            Some("file-hacker-user")
        );
        assert_eq!(
            config.hacker_api_token.as_deref(),
            Some("file-hacker-token")
        );
        assert_eq!(config.program_handle.as_deref(), Some("file-program"));
        assert_eq!(config.base_url.as_deref(), Some("https://file.example"));
        assert_eq!(config.bind.as_deref(), Some("127.0.0.1:9000"));
        assert_eq!(config.http_token.as_deref(), Some("file-http-token"));
    }

    #[test]
    fn three_sources_merge_field_wise() {
        let dir = TempDir::new("merge");
        let xdg = dir.path().join("xdg");
        std::fs::create_dir_all(xdg.join("hackerone-mcp")).expect("create xdg dir");
        std::fs::write(
            xdg.join("hackerone-mcp/config.toml"),
            "[api]\nusername = \"xdg-user\"\nbase_url = \"https://xdg.example\"\n",
        )
        .expect("write xdg config");
        std::fs::write(
            dir.path().join("hackerone-mcp.toml"),
            "[api]\nprogram_handle = \"cwd-program\"\n",
        )
        .expect("write cwd config");

        let vars = env(&[
            ("XDG_CONFIG_HOME", xdg.to_str().expect("utf8 path")),
            ("HACKERONE_API_TOKEN", "env-token"),
        ]);
        let config = load_with(
            &dir,
            Flags {
                base_url: Some("https://flag.example".to_owned()),
                ..Flags::default()
            },
            &vars,
        )
        .expect("load merged config");

        assert_eq!(config.api_username.as_deref(), Some("xdg-user"));
        assert_eq!(config.program_handle.as_deref(), Some("cwd-program"));
        assert_eq!(config.api_token.as_deref(), Some("env-token"));
        assert_eq!(config.base_url.as_deref(), Some("https://flag.example"));
        assert_eq!(config.bind, None);
    }

    #[test]
    fn unknown_key_is_an_error_naming_the_key() {
        let dir = TempDir::new("unknown");
        let path = dir.write("hackerone.toml", "[api]\nusernam = \"typo\"\n");

        let err = load_with(
            &dir,
            Flags {
                config: Some(path),
                ..Flags::default()
            },
            &HashMap::new(),
        )
        .expect_err("unknown key must fail");

        let ConfigError::Parse { reason, .. } = &err else {
            panic!("expected parse error, got {err:?}");
        };
        assert!(reason.contains("usernam"), "error must name key: {reason}");
    }

    #[cfg(unix)]
    #[test]
    fn token_file_with_group_or_other_bits_is_refused() {
        let dir = TempDir::new("perms");
        let path = dir.write("hackerone.toml", "[api]\ntoken = \"secret\"\n");
        chmod(&path, 0o644);

        let err = load_with(
            &dir,
            Flags {
                config: Some(path.clone()),
                ..Flags::default()
            },
            &HashMap::new(),
        )
        .expect_err("world-readable secret must fail");
        assert!(matches!(
            err,
            ConfigError::InsecurePermissions { mode: 0o644, .. }
        ));

        chmod(&path, 0o600);
        let config = load_with(
            &dir,
            Flags {
                config: Some(path),
                ..Flags::default()
            },
            &HashMap::new(),
        )
        .expect("0600 secret is accepted");
        assert_eq!(config.api_token.as_deref(), Some("secret"));
    }

    #[test]
    fn missing_explicit_config_is_an_error() {
        let dir = TempDir::new("missing");
        let path = dir.path().join("absent.toml");

        let err = load_with(
            &dir,
            Flags {
                config: Some(path.clone()),
                ..Flags::default()
            },
            &HashMap::new(),
        )
        .expect_err("missing --config must fail");
        let ConfigError::Read { path: reported, .. } = &err else {
            panic!("expected read error, got {err:?}");
        };
        assert_eq!(reported, &path);
        assert!(err.to_string().contains("absent.toml"));
    }

    #[test]
    fn env_only_config_without_program_handle_loads() {
        let dir = TempDir::new("env-only");
        let vars = env(&[
            ("HACKERONE_API_USERNAME", "env-user"),
            ("HACKERONE_API_TOKEN", "env-token"),
        ]);

        let config = load_with(&dir, Flags::default(), &vars).expect("env-only config loads");
        assert_eq!(config.api_username.as_deref(), Some("env-user"));
        assert_eq!(config.api_token.as_deref(), Some("env-token"));
        assert_eq!(config.program_handle, None);
    }

    #[test]
    fn hacker_fields_layer_independently_of_customer_fields() {
        let dir = TempDir::new("hacker-merge");
        let path = dir.write(
            "hackerone.toml",
            "[api]\nusername = \"file-user\"\ntoken = \"file-token\"\n\
             [hacker_api]\nusername = \"file-hacker-user\"\n",
        );
        chmod(&path, 0o600);

        let vars = env(&[
            ("HACKERONE_HACKER_API_TOKEN", "env-hacker-token"),
            ("HACKERONE_API_BASE_URL", "https://env.example"),
        ]);
        let config = load_with(
            &dir,
            Flags {
                config: Some(path),
                ..Flags::default()
            },
            &vars,
        )
        .expect("load layered config");

        assert_eq!(config.api_username.as_deref(), Some("file-user"));
        assert_eq!(config.api_token.as_deref(), Some("file-token"));
        assert_eq!(
            config.hacker_api_username.as_deref(),
            Some("file-hacker-user")
        );
        assert_eq!(config.hacker_api_token.as_deref(), Some("env-hacker-token"));
        assert_eq!(config.base_url.as_deref(), Some("https://env.example"));
    }

    #[test]
    fn unknown_key_in_hacker_api_is_an_error() {
        let dir = TempDir::new("hacker-unknown");
        let path = dir.write("hackerone.toml", "[hacker_api]\ntokn = \"typo\"\n");

        let err = load_with(
            &dir,
            Flags {
                config: Some(path),
                ..Flags::default()
            },
            &HashMap::new(),
        )
        .expect_err("unknown key must fail");

        let ConfigError::Parse { reason, .. } = &err else {
            panic!("expected parse error, got {err:?}");
        };
        assert!(reason.contains("tokn"), "error must name key: {reason}");
    }

    #[cfg(unix)]
    #[test]
    fn hacker_token_file_with_group_or_other_bits_is_refused() {
        let dir = TempDir::new("hacker-perms");
        let path = dir.write(
            "hackerone.toml",
            "[hacker_api]\ntoken = \"hacker-secret\"\n",
        );
        chmod(&path, 0o644);

        let err = load_with(
            &dir,
            Flags {
                config: Some(path.clone()),
                ..Flags::default()
            },
            &HashMap::new(),
        )
        .expect_err("world-readable hacker secret must fail");
        assert!(matches!(
            err,
            ConfigError::InsecurePermissions { mode: 0o644, .. }
        ));
    }
}
