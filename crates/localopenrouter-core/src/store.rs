use chrono::Utc;
use std::env;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

use crate::crypto;
use crate::error::{LocalOpenRouterError, Result};
use crate::models::{
    Account, AccountInput, ApiProtocol, DeleteResponse, HealthResponse, LogQuery,
    ProviderDefinition, ProviderInput, RequestLog, RequestLogInput, ResolvedAccount,
    RevealedSecret, RouteBinding, RouteBindingInput, UnlockResponse, extract_session_id,
};
use crate::onboarding::{DEFAULT_PORT, guide_for_target};
use crate::sqlite::{Connection, Row, SqlValue};

const VAULT_SALT_KEY: &str = "vault_salt";
const VAULT_CHECK_NONCE_KEY: &str = "vault_check_nonce";
const VAULT_CHECK_CIPHERTEXT_KEY: &str = "vault_check_ciphertext";
const DATA_DIR_ENV: &str = "LOCALOPENROUTER_DATA_DIR";
const LEGACY_DATA_DIR_ENV: &str = "LOCALROUTER_DATA_DIR";
const APP_DIR_NAME: &str = "LocalOpenRouter";
const LEGACY_APP_DIR_NAME: &str = "LocalRouter";
const DATABASE_FILE_NAME: &str = "localopenrouter.sqlite3";
const LEGACY_DATABASE_FILE_NAME: &str = "localrouter.sqlite3";

#[derive(Debug, Clone)]
struct BuiltinProvider {
    slug: &'static str,
    display_name: &'static str,
    protocol: ApiProtocol,
    base_url: &'static str,
    proxy_path: &'static str,
    auth_header: &'static str,
    auth_prefix: Option<&'static str>,
    enabled: bool,
}

#[derive(Debug, Clone, Copy)]
struct BuiltinProviderRename {
    old_slug: &'static str,
    new_slug: &'static str,
    old_display_name: &'static str,
    new_display_name: &'static str,
    old_proxy_path: &'static str,
    new_proxy_path: &'static str,
}

const BUILTIN_PROVIDERS: &[BuiltinProvider] = &[
    BuiltinProvider {
        slug: "codex",
        display_name: "Codex",
        protocol: ApiProtocol::OpenAi,
        base_url: "https://api.openai.com",
        proxy_path: "codex",
        auth_header: "Authorization",
        auth_prefix: Some("Bearer"),
        enabled: true,
    },
    BuiltinProvider {
        slug: "claude-code",
        display_name: "Claude Code",
        protocol: ApiProtocol::Anthropic,
        base_url: "https://api.anthropic.com",
        proxy_path: "claude-code",
        auth_header: "x-api-key",
        auth_prefix: None,
        enabled: true,
    },
    BuiltinProvider {
        slug: "gemini",
        display_name: "Gemini",
        protocol: ApiProtocol::Generic,
        base_url: "https://generativelanguage.googleapis.com/v1beta",
        proxy_path: "gemini",
        auth_header: "x-goog-api-key",
        auth_prefix: None,
        enabled: true,
    },
];

const BUILTIN_PROVIDER_RENAMES: &[BuiltinProviderRename] = &[
    BuiltinProviderRename {
        old_slug: "openai",
        new_slug: "codex",
        old_display_name: "OpenAI",
        new_display_name: "Codex",
        old_proxy_path: "openai",
        new_proxy_path: "codex",
    },
    BuiltinProviderRename {
        old_slug: "anthropic",
        new_slug: "claude-code",
        old_display_name: "Anthropic",
        new_display_name: "Claude Code",
        old_proxy_path: "anthropic",
        new_proxy_path: "claude-code",
    },
];

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub root: PathBuf,
    pub database: PathBuf,
    pub logs: PathBuf,
}

impl AppPaths {
    pub fn discover() -> Result<Self> {
        let root = if let Some(override_dir) = discover_override_root() {
            override_dir
        } else {
            let local_data_dir = dirs::data_local_dir().ok_or_else(|| {
                LocalOpenRouterError::Io("failed to locate local data directory".into())
            })?;
            resolve_default_root(local_data_dir)
        };
        fs::create_dir_all(&root)?;
        let logs = root.join("logs");
        fs::create_dir_all(&logs)?;
        Ok(Self {
            database: database_path_for_root(&root),
            logs,
            root,
        })
    }

    pub fn from_root(root: PathBuf) -> Result<Self> {
        fs::create_dir_all(&root)?;
        let logs = root.join("logs");
        fs::create_dir_all(&logs)?;
        Ok(Self {
            database: database_path_for_root(&root),
            logs,
            root,
        })
    }
}

fn discover_override_root() -> Option<PathBuf> {
    env::var(DATA_DIR_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            env::var(LEGACY_DATA_DIR_ENV)
                .ok()
                .filter(|value| !value.trim().is_empty())
                .map(PathBuf::from)
        })
}

fn resolve_default_root(local_data_dir: PathBuf) -> PathBuf {
    let root = local_data_dir.join(APP_DIR_NAME);
    let legacy_root = local_data_dir.join(LEGACY_APP_DIR_NAME);
    if root.exists() || !legacy_root.exists() {
        root
    } else {
        legacy_root
    }
}

fn database_path_for_root(root: &Path) -> PathBuf {
    let database = root.join(DATABASE_FILE_NAME);
    let legacy_database = root.join(LEGACY_DATABASE_FILE_NAME);
    if database.exists() || !legacy_database.exists() {
        database
    } else {
        legacy_database
    }
}

pub struct Repository {
    paths: AppPaths,
    db: Mutex<Connection>,
    master_key: RwLock<Option<[u8; 32]>>,
    started_at: String,
    port: u16,
}

impl Repository {
    pub async fn new(port: u16) -> Result<Self> {
        let paths = AppPaths::discover()?;
        Self::new_with_paths(paths, port).await
    }

    pub async fn new_with_paths(paths: AppPaths, port: u16) -> Result<Self> {
        let database = Connection::open(&paths.database)?;
        database.execute_batch(SCHEMA)?;
        migrate_schema(&database, &paths.root)?;
        seed_builtin_providers(&database)?;
        Ok(Self {
            paths,
            db: Mutex::new(database),
            master_key: RwLock::new(None),
            started_at: timestamp(),
            port: if port == 0 { DEFAULT_PORT } else { port },
        })
    }

    pub async fn health(&self) -> Result<HealthResponse> {
        Ok(HealthResponse {
            version: env!("CARGO_PKG_VERSION").into(),
            started_at: self.started_at.clone(),
            db_path: self.paths.database.to_string_lossy().into_owned(),
            initialized: self.is_initialized().await?,
            unlocked: self.master_key.read().await.is_some(),
            port: self.port,
        })
    }

    pub async fn is_initialized(&self) -> Result<bool> {
        let db = self.db.lock().await;
        let salt = get_setting_blob(&db, VAULT_SALT_KEY)?;
        Ok(salt.is_some())
    }

    pub async fn unlock(&self, password: &str) -> Result<UnlockResponse> {
        if password.trim().is_empty() {
            return Err(LocalOpenRouterError::Validation(
                "master password must not be empty".into(),
            ));
        }

        let mut key_guard = self.master_key.write().await;
        let db = self.db.lock().await;
        let salt = get_setting_blob(&db, VAULT_SALT_KEY)?;
        let check_nonce = get_setting_blob(&db, VAULT_CHECK_NONCE_KEY)?;
        let check_ciphertext = get_setting_blob(&db, VAULT_CHECK_CIPHERTEXT_KEY)?;

        let response = match (salt, check_nonce, check_ciphertext) {
            (Some(_), Some(_), Some(_)) => {
                let key = master_key_from_password(&db, password)?;
                *key_guard = Some(key);
                UnlockResponse {
                    initialized: true,
                    unlocked: true,
                    message: "vault unlocked".into(),
                }
            }
            (None, None, None) => {
                let initialized = crypto::initialize_master_password(password)?;
                upsert_setting_blob(&db, VAULT_SALT_KEY, &initialized.salt)?;
                upsert_setting_blob(&db, VAULT_CHECK_NONCE_KEY, &initialized.check_nonce)?;
                upsert_setting_blob(
                    &db,
                    VAULT_CHECK_CIPHERTEXT_KEY,
                    &initialized.check_ciphertext,
                )?;
                *key_guard = Some(initialized.key);
                UnlockResponse {
                    initialized: true,
                    unlocked: true,
                    message: "vault initialized and unlocked".into(),
                }
            }
            _ => {
                return Err(LocalOpenRouterError::Sqlite(
                    "vault metadata is incomplete; delete the database to recover".into(),
                ));
            }
        };
        Ok(response)
    }

    pub async fn lock(&self) {
        let mut key_guard = self.master_key.write().await;
        *key_guard = None;
    }

    pub async fn list_providers(&self) -> Result<Vec<ProviderDefinition>> {
        let db = self.db.lock().await;
        let rows = db.query(
            "SELECT slug, display_name, protocol, base_url, proxy_path, auth_header,
             auth_prefix, enabled, is_builtin, created_at, updated_at
             FROM providers ORDER BY is_builtin DESC, display_name, slug",
            &[],
        )?;
        rows.into_iter().map(provider_from_row).collect()
    }

    pub async fn get_provider(&self, slug: &str) -> Result<ProviderDefinition> {
        let db = self.db.lock().await;
        fetch_provider_by_slug(&db, slug)?
            .ok_or_else(|| LocalOpenRouterError::NotFound(format!("provider `{slug}`")))
    }

    pub async fn find_provider_by_proxy_path(
        &self,
        proxy_path: &str,
    ) -> Result<Option<ProviderDefinition>> {
        let db = self.db.lock().await;
        fetch_provider_by_proxy_path(&db, proxy_path)
    }

    pub async fn upsert_provider(&self, input: ProviderInput) -> Result<ProviderDefinition> {
        let normalized_slug = normalize_slug(&input.slug)?;
        let display_name = normalize_non_empty("display name", &input.display_name)?;
        let base_url = normalize_base_url(&input.base_url)?;
        let proxy_path = normalize_proxy_path(&input.proxy_path)?;
        let auth_header = normalize_header_name(&input.auth_header)?;
        let auth_prefix = normalize_optional(input.auth_prefix);
        let now = timestamp();

        let db = self.db.lock().await;
        let existing = fetch_provider_by_slug(&db, &normalized_slug)?;
        if let Some(other) = fetch_provider_by_proxy_path(&db, &proxy_path)? {
            if other.slug != normalized_slug {
                return Err(LocalOpenRouterError::Validation(format!(
                    "proxy path `{proxy_path}` is already used by provider `{}`",
                    other.slug
                )));
            }
        }

        let (is_builtin, created_at, protocol) = match existing {
            Some(existing) => (
                existing.is_builtin,
                existing.created_at,
                if existing.is_builtin {
                    existing.protocol
                } else {
                    input.protocol
                },
            ),
            None => (false, now.clone(), input.protocol),
        };

        db.execute(
            "INSERT INTO providers (
               slug, display_name, protocol, base_url, proxy_path, auth_header, auth_prefix,
               enabled, is_builtin, created_at, updated_at
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(slug) DO UPDATE SET
               display_name = excluded.display_name,
               protocol = excluded.protocol,
               base_url = excluded.base_url,
               proxy_path = excluded.proxy_path,
               auth_header = excluded.auth_header,
               auth_prefix = excluded.auth_prefix,
               enabled = excluded.enabled,
               updated_at = excluded.updated_at",
            &[
                SqlValue::Text(normalized_slug.clone()),
                SqlValue::Text(display_name),
                SqlValue::Text(protocol.as_str().into()),
                SqlValue::Text(base_url),
                SqlValue::Text(proxy_path),
                SqlValue::Text(auth_header),
                auth_prefix
                    .clone()
                    .map(SqlValue::Text)
                    .unwrap_or(SqlValue::Null),
                SqlValue::Integer(i64::from(input.enabled)),
                SqlValue::Integer(i64::from(is_builtin)),
                SqlValue::Text(created_at),
                SqlValue::Text(now),
            ],
        )?;

        drop(db);
        self.get_provider(&normalized_slug).await
    }

    pub async fn delete_provider(&self, slug: &str) -> Result<DeleteResponse> {
        let db = self.db.lock().await;
        let provider = fetch_provider_by_slug(&db, slug)?
            .ok_or_else(|| LocalOpenRouterError::NotFound(format!("provider `{slug}`")))?;
        if provider.is_builtin {
            return Err(LocalOpenRouterError::Validation(format!(
                "built-in provider `{slug}` cannot be deleted"
            )));
        }

        let account_count = db
            .query(
                "SELECT COUNT(*) AS count FROM accounts WHERE provider = ?",
                &[SqlValue::Text(slug.into())],
            )?
            .into_iter()
            .next()
            .ok_or_else(|| LocalOpenRouterError::Sqlite("failed to count accounts".into()))?
            .get_i64("count")?;
        if account_count > 0 {
            return Err(LocalOpenRouterError::Validation(format!(
                "provider `{slug}` still has {account_count} account(s)"
            )));
        }

        let route_count = db
            .query(
                "SELECT COUNT(*) AS count FROM route_bindings WHERE provider = ?",
                &[SqlValue::Text(slug.into())],
            )?
            .into_iter()
            .next()
            .ok_or_else(|| LocalOpenRouterError::Sqlite("failed to count routes".into()))?
            .get_i64("count")?;
        if route_count > 0 {
            return Err(LocalOpenRouterError::Validation(format!(
                "provider `{slug}` still has {route_count} route(s)"
            )));
        }

        db.execute(
            "DELETE FROM providers WHERE slug = ?",
            &[SqlValue::Text(slug.into())],
        )?;
        Ok(DeleteResponse { success: true })
    }

    pub async fn list_accounts(&self) -> Result<Vec<Account>> {
        let db = self.db.lock().await;
        let rows = db.query(
            "SELECT a.id, a.provider, a.name, a.base_url, a.enabled, a.note,
             EXISTS(SELECT 1 FROM encrypted_secrets s WHERE s.account_id = a.id) AS has_secret,
             a.created_at, a.updated_at
             FROM accounts a ORDER BY a.provider, a.name",
            &[],
        )?;
        rows.into_iter().map(account_from_row).collect()
    }

    pub async fn upsert_account(&self, input: AccountInput) -> Result<Account> {
        validate_account_input(&input)?;
        let key = self.require_master_key().await?;
        let provider_slug = normalize_slug(&input.provider)?;
        let id = input.id.unwrap_or_else(|| Uuid::new_v4().to_string());
        let now = timestamp();
        let base_url = normalize_optional_base_url(input.base_url)?;
        let note = normalize_optional(input.note);

        let db = self.db.lock().await;
        let provider = fetch_provider_by_slug(&db, &provider_slug)?
            .ok_or_else(|| LocalOpenRouterError::NotFound(format!("provider `{provider_slug}`")))?;
        if !provider.enabled {
            return Err(LocalOpenRouterError::Validation(format!(
                "provider `{provider_slug}` is disabled"
            )));
        }

        let existing = db.query(
            "SELECT id, created_at FROM accounts WHERE id = ?",
            &[SqlValue::Text(id.clone())],
        )?;
        let created_at = existing
            .first()
            .map(|row| row.get_text("created_at"))
            .transpose()?
            .unwrap_or_else(|| now.clone());

        db.execute(
            "INSERT INTO accounts (id, provider, name, base_url, enabled, note, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET
               provider = excluded.provider,
               name = excluded.name,
               base_url = excluded.base_url,
               enabled = excluded.enabled,
               note = excluded.note,
               updated_at = excluded.updated_at",
            &[
                SqlValue::Text(id.clone()),
                SqlValue::Text(provider_slug.clone()),
                SqlValue::Text(input.name.trim().into()),
                base_url.clone().map(SqlValue::Text).unwrap_or(SqlValue::Null),
                SqlValue::Integer(i64::from(input.enabled)),
                note.clone().map(SqlValue::Text).unwrap_or(SqlValue::Null),
                SqlValue::Text(created_at),
                SqlValue::Text(now.clone()),
            ],
        )?;

        match input.api_key {
            Some(api_key) => {
                let api_key = api_key.trim();
                if api_key.is_empty() {
                    return Err(LocalOpenRouterError::Validation(
                        "API key must not be empty when provided".into(),
                    ));
                }
                let (nonce, ciphertext) = crypto::encrypt_secret(&key, api_key)?;
                db.execute(
                    "INSERT INTO encrypted_secrets (account_id, nonce, ciphertext, updated_at)
                     VALUES (?, ?, ?, ?)
                     ON CONFLICT(account_id) DO UPDATE SET
                       nonce = excluded.nonce,
                       ciphertext = excluded.ciphertext,
                       updated_at = excluded.updated_at",
                    &[
                        SqlValue::Text(id.clone()),
                        SqlValue::Blob(nonce),
                        SqlValue::Blob(ciphertext),
                        SqlValue::Text(now.clone()),
                    ],
                )?;
            }
            None if existing.is_empty() => {
                return Err(LocalOpenRouterError::Validation(
                    "new accounts require an API key".into(),
                ));
            }
            None => {}
        }

        ensure_default_route(&db, &provider_slug, &id)?;
        drop(db);
        self.get_account(&id).await
    }

    pub async fn reveal_account_secret(
        &self,
        account_id: &str,
        password: &str,
    ) -> Result<RevealedSecret> {
        if password.trim().is_empty() {
            return Err(LocalOpenRouterError::Validation(
                "master password must not be empty".into(),
            ));
        }

        let db = self.db.lock().await;
        let row = db
            .query(
                "SELECT id FROM accounts WHERE id = ?",
                &[SqlValue::Text(account_id.into())],
            )?
            .into_iter()
            .next()
            .ok_or_else(|| LocalOpenRouterError::NotFound(format!("account `{account_id}`")))?;
        let account_id = row.get_text("id")?;
        let key = master_key_from_password(&db, password)?;
        let api_key = decrypt_account_secret(&db, &account_id, &key)?;
        Ok(RevealedSecret {
            account_id,
            api_key,
        })
    }

    pub async fn get_account(&self, id: &str) -> Result<Account> {
        let db = self.db.lock().await;
        let row = db
            .query(
                "SELECT a.id, a.provider, a.name, a.base_url, a.enabled, a.note,
                 EXISTS(SELECT 1 FROM encrypted_secrets s WHERE s.account_id = a.id) AS has_secret,
                 a.created_at, a.updated_at
                 FROM accounts a WHERE a.id = ?",
                &[SqlValue::Text(id.into())],
            )?
            .into_iter()
            .next()
            .ok_or_else(|| LocalOpenRouterError::NotFound(format!("account `{id}`")))?;
        account_from_row(row)
    }

    pub async fn disable_account(&self, id: &str) -> Result<Account> {
        let db = self.db.lock().await;
        db.execute(
            "UPDATE accounts SET enabled = 0, updated_at = ? WHERE id = ?",
            &[SqlValue::Text(timestamp()), SqlValue::Text(id.into())],
        )?;
        drop(db);
        self.get_account(id).await
    }

    pub async fn delete_account(&self, id: &str) -> Result<DeleteResponse> {
        let db = self.db.lock().await;
        db.execute(
            "DELETE FROM route_bindings WHERE account_id = ?",
            &[SqlValue::Text(id.into())],
        )?;
        db.execute(
            "DELETE FROM encrypted_secrets WHERE account_id = ?",
            &[SqlValue::Text(id.into())],
        )?;
        db.execute(
            "DELETE FROM accounts WHERE id = ?",
            &[SqlValue::Text(id.into())],
        )?;
        Ok(DeleteResponse { success: true })
    }

    pub async fn list_routes(&self) -> Result<Vec<RouteBinding>> {
        let db = self.db.lock().await;
        let rows = db.query(
            "SELECT id, provider, model_prefix, account_id, updated_at
             FROM route_bindings ORDER BY provider, model_prefix IS NULL DESC, model_prefix",
            &[],
        )?;
        rows.into_iter().map(route_from_row).collect()
    }

    pub async fn set_route_binding(&self, input: RouteBindingInput) -> Result<RouteBinding> {
        let provider_slug = normalize_slug(&input.provider)?;
        let model_prefix = normalize_optional(input.model_prefix);
        let db = self.db.lock().await;
        validate_route_account(&db, &provider_slug, &input.account_id)?;
        let id = route_binding_id(&provider_slug, model_prefix.as_deref());
        delete_noncanonical_route_bindings(&db, &provider_slug, model_prefix.as_deref(), &id)?;
        let now = timestamp();
        db.execute(
            "INSERT INTO route_bindings (id, provider, model_prefix, account_id, updated_at)
             VALUES (?, ?, ?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET
               account_id = excluded.account_id,
               updated_at = excluded.updated_at",
            &[
                SqlValue::Text(id.clone()),
                SqlValue::Text(provider_slug.clone()),
                model_prefix
                    .clone()
                    .map(SqlValue::Text)
                    .unwrap_or(SqlValue::Null),
                SqlValue::Text(input.account_id.clone()),
                SqlValue::Text(now.clone()),
            ],
        )?;
        Ok(RouteBinding {
            id,
            provider: provider_slug,
            model_prefix,
            account_id: input.account_id,
            updated_at: now,
        })
    }

    pub async fn delete_route_binding(&self, id: &str) -> Result<DeleteResponse> {
        let db = self.db.lock().await;
        db.execute(
            "DELETE FROM route_bindings WHERE id = ?",
            &[SqlValue::Text(id.into())],
        )?;
        Ok(DeleteResponse { success: true })
    }

    pub async fn query_logs(&self, query: LogQuery) -> Result<Vec<RequestLog>> {
        let mut sql = String::from(
            "SELECT id, created_at, provider, session_id, model, account_id, method, path, status_code,
             duration_ms, error_text, '' AS request_headers, '' AS request_body,
             '' AS response_headers, '' AS response_body, log_file_path, streamed
             FROM request_logs WHERE 1 = 1",
        );
        let mut params = Vec::new();
        if let Some(provider) = query.provider {
            sql.push_str(" AND provider = ?");
            params.push(SqlValue::Text(provider));
        }
        if let Some(account_id) = query.account_id {
            sql.push_str(" AND account_id = ?");
            params.push(SqlValue::Text(account_id));
        }
        if let Some(session_id) = query.session_id {
            sql.push_str(" AND session_id = ?");
            params.push(SqlValue::Text(session_id));
        }
        if let Some(status_code) = query.status_code {
            sql.push_str(" AND status_code = ?");
            params.push(SqlValue::Integer(i64::from(status_code)));
        }
        sql.push_str(" ORDER BY created_at DESC LIMIT ?");
        params.push(SqlValue::Integer(i64::from(query.limit.unwrap_or(50))));

        let db = self.db.lock().await;
        let rows = db.query(sql.as_str(), &params)?;
        rows.into_iter().map(log_from_row).collect()
    }

    pub async fn get_log(&self, id: &str) -> Result<RequestLog> {
        let db = self.db.lock().await;
        let row = db
            .query(
                "SELECT id, created_at, provider, session_id, model, account_id, method, path, status_code,
                 duration_ms, error_text, request_headers, request_body, response_headers,
                 response_body, log_file_path, streamed, request_headers_path, request_body_path,
                 response_headers_path, response_body_path
                 FROM request_logs WHERE id = ?",
                &[SqlValue::Text(id.into())],
            )?
            .into_iter()
            .next()
            .ok_or_else(|| LocalOpenRouterError::NotFound(format!("log `{id}`")))?;
        log_detail_from_row(row, &self.paths.root)
    }

    pub async fn insert_log(&self, input: RequestLogInput) -> Result<RequestLog> {
        let id = Uuid::new_v4().to_string();
        let created_at = timestamp();
        let status_code = input.status_code.map(i64::from);
        let session_id = extract_session_id(
            &input.request_headers,
            &input.request_body,
            &input.response_headers,
            &input.response_body,
        );
        let record = StoredLogRecord::from_input(&id, &created_at, session_id.clone(), &input);
        let log_file_path = append_log_record(&self.paths.root, &record)?;
        let db = self.db.lock().await;
        db.execute(
            "INSERT INTO request_logs (
               id, created_at, provider, session_id, model, account_id, method, path,
               status_code, duration_ms, error_text, request_headers, request_body,
               response_headers, response_body, request_headers_path, request_body_path,
               response_headers_path, response_body_path, log_file_path, streamed
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            &[
                SqlValue::Text(id.clone()),
                SqlValue::Text(created_at.clone()),
                SqlValue::Text(input.provider.clone()),
                session_id
                    .clone()
                    .map(SqlValue::Text)
                    .unwrap_or(SqlValue::Null),
                input
                    .model
                    .clone()
                    .map(SqlValue::Text)
                    .unwrap_or(SqlValue::Null),
                input
                    .account_id
                    .clone()
                    .map(SqlValue::Text)
                    .unwrap_or(SqlValue::Null),
                SqlValue::Text(input.method.clone()),
                SqlValue::Text(input.path.clone()),
                status_code.map(SqlValue::Integer).unwrap_or(SqlValue::Null),
                SqlValue::Integer(input.duration_ms as i64),
                input
                    .error_text
                    .clone()
                    .map(SqlValue::Text)
                    .unwrap_or(SqlValue::Null),
                SqlValue::Text(String::new()),
                SqlValue::Text(String::new()),
                SqlValue::Text(String::new()),
                SqlValue::Text(String::new()),
                SqlValue::Null,
                SqlValue::Null,
                SqlValue::Null,
                SqlValue::Null,
                SqlValue::Text(log_file_path.clone()),
                SqlValue::Integer(i64::from(input.streamed)),
            ],
        )?;
        Ok(RequestLog {
            id,
            created_at,
            provider: input.provider,
            session_id,
            model: input.model,
            account_id: input.account_id,
            method: input.method,
            path: input.path,
            status_code: input.status_code,
            duration_ms: input.duration_ms,
            error_text: input.error_text,
            request_headers: input.request_headers,
            request_body: input.request_body,
            response_headers: input.response_headers,
            response_body: input.response_body,
            log_file_path: Some(log_file_path),
            streamed: input.streamed,
        })
    }

    pub async fn resolve_account(
        &self,
        provider_slug: &str,
        model: Option<&str>,
    ) -> Result<ResolvedAccount> {
        let key = self.require_master_key().await?;
        let db = self.db.lock().await;
        let provider = fetch_provider_by_slug(&db, provider_slug)?
            .ok_or_else(|| LocalOpenRouterError::NotFound(format!("provider `{provider_slug}`")))?;
        if !provider.enabled {
            return Err(LocalOpenRouterError::Validation(format!(
                "provider `{provider_slug}` is disabled"
            )));
        }

        let routes = db.query(
            "SELECT id, provider, model_prefix, account_id, updated_at
             FROM route_bindings WHERE provider = ?",
            &[SqlValue::Text(provider_slug.into())],
        )?;
        let selected = pick_route(provider_slug, model, routes)?;
        let account = db
            .query(
                "SELECT a.id, a.provider, a.name, a.base_url, a.enabled, a.note,
                 EXISTS(SELECT 1 FROM encrypted_secrets s WHERE s.account_id = a.id) AS has_secret,
                 a.created_at, a.updated_at
                 FROM accounts a WHERE a.id = ?",
                &[SqlValue::Text(selected.account_id.clone())],
            )?
            .into_iter()
            .next()
            .ok_or_else(|| {
                LocalOpenRouterError::NotFound(format!(
                    "selected account `{}` for provider `{provider_slug}`",
                    selected.account_id
                ))
            })?;
        let account = account_from_row(account)?;
        if !account.enabled {
            return Err(LocalOpenRouterError::Validation(format!(
                "selected account `{}` is disabled",
                account.name
            )));
        }
        let api_key = decrypt_account_secret(&db, &account.id, &key)?;
        Ok(ResolvedAccount {
            upstream_base_url: account
                .base_url
                .clone()
                .unwrap_or_else(|| provider.base_url.clone()),
            provider,
            account,
            api_key,
        })
    }

    pub async fn onboarding(&self, target: &str) -> Result<crate::models::OnboardingGuide> {
        let provider_slug = match target {
            "codex" => "codex",
            "claude-code" => "claude-code",
            other => {
                return Err(LocalOpenRouterError::NotFound(format!(
                    "unsupported onboarding target `{other}`"
                )));
            }
        };
        let provider = self.get_provider(provider_slug).await?;
        guide_for_target(target, self.port, &provider)
    }

    async fn require_master_key(&self) -> Result<[u8; 32]> {
        self.master_key
            .read()
            .await
            .as_ref()
            .copied()
            .ok_or(LocalOpenRouterError::Locked)
    }
}

fn validate_account_input(input: &AccountInput) -> Result<()> {
    if input.name.trim().is_empty() {
        return Err(LocalOpenRouterError::Validation(
            "account name must not be empty".into(),
        ));
    }
    normalize_slug(&input.provider)?;
    normalize_optional_base_url(input.base_url.clone())?;
    Ok(())
}

fn validate_route_account(db: &Connection, provider_slug: &str, account_id: &str) -> Result<()> {
    let row = db
        .query(
            "SELECT provider, enabled FROM accounts WHERE id = ?",
            &[SqlValue::Text(account_id.into())],
        )?
        .into_iter()
        .next()
        .ok_or_else(|| LocalOpenRouterError::NotFound(format!("account `{account_id}`")))?;
    if row.get_text("provider")? != provider_slug {
        return Err(LocalOpenRouterError::Validation(format!(
            "account `{account_id}` does not belong to provider `{provider_slug}`"
        )));
    }
    if row.get_i64("enabled")? == 0 {
        return Err(LocalOpenRouterError::Validation(format!(
            "account `{account_id}` is disabled"
        )));
    }
    Ok(())
}

fn ensure_default_route(db: &Connection, provider_slug: &str, account_id: &str) -> Result<()> {
    let route_id = route_binding_id(provider_slug, None);
    let rows = db.query(
        "SELECT id FROM route_bindings WHERE id = ?",
        &[SqlValue::Text(route_id.clone())],
    )?;
    if rows.is_empty() {
        db.execute(
            "INSERT INTO route_bindings (id, provider, model_prefix, account_id, updated_at)
             VALUES (?, ?, NULL, ?, ?)",
            &[
                SqlValue::Text(route_id),
                SqlValue::Text(provider_slug.into()),
                SqlValue::Text(account_id.into()),
                SqlValue::Text(timestamp()),
            ],
        )?;
    }
    Ok(())
}

fn delete_noncanonical_route_bindings(
    db: &Connection,
    provider_slug: &str,
    model_prefix: Option<&str>,
    canonical_id: &str,
) -> Result<()> {
    match model_prefix {
        Some(model_prefix) => db.execute(
            "DELETE FROM route_bindings
             WHERE provider = ? AND model_prefix = ? AND id <> ?",
            &[
                SqlValue::Text(provider_slug.into()),
                SqlValue::Text(model_prefix.into()),
                SqlValue::Text(canonical_id.into()),
            ],
        )?,
        None => db.execute(
            "DELETE FROM route_bindings
             WHERE provider = ? AND model_prefix IS NULL AND id <> ?",
            &[
                SqlValue::Text(provider_slug.into()),
                SqlValue::Text(canonical_id.into()),
            ],
        )?,
    }
    Ok(())
}

fn route_binding_id(provider_slug: &str, model_prefix: Option<&str>) -> String {
    match model_prefix
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(prefix) => format!("{provider_slug}::{prefix}"),
        None => format!("{provider_slug}::*"),
    }
}

fn pick_route(provider_slug: &str, model: Option<&str>, routes: Vec<Row>) -> Result<RouteBinding> {
    let bindings: Vec<RouteBinding> = routes
        .into_iter()
        .map(route_from_row)
        .collect::<Result<_>>()?;
    if let Some(model) = model {
        let mut matches: Vec<RouteBinding> = bindings
            .iter()
            .filter(|binding| {
                binding
                    .model_prefix
                    .as_deref()
                    .map(|prefix| model.starts_with(prefix))
                    .unwrap_or(false)
            })
            .cloned()
            .collect();
        matches.sort_by_key(|binding| {
            std::cmp::Reverse(binding.model_prefix.as_deref().unwrap_or_default().len())
        });
        if let Some(binding) = matches.into_iter().next() {
            return Ok(binding);
        }
    }
    bindings
        .into_iter()
        .find(|binding| binding.model_prefix.is_none())
        .ok_or_else(|| {
            LocalOpenRouterError::NotFound(format!("default route for provider `{provider_slug}`"))
        })
}

fn provider_from_row(row: Row) -> Result<ProviderDefinition> {
    Ok(ProviderDefinition {
        slug: row.get_text("slug")?,
        display_name: row.get_text("display_name")?,
        protocol: row.get_text("protocol")?.parse()?,
        base_url: row.get_text("base_url")?,
        proxy_path: row.get_text("proxy_path")?,
        auth_header: row.get_text("auth_header")?,
        auth_prefix: row.get_optional_text("auth_prefix")?,
        enabled: row.get_i64("enabled")? != 0,
        is_builtin: row.get_i64("is_builtin")? != 0,
        created_at: row.get_text("created_at")?,
        updated_at: row.get_text("updated_at")?,
    })
}

fn account_from_row(row: Row) -> Result<Account> {
    Ok(Account {
        id: row.get_text("id")?,
        provider: row.get_text("provider")?,
        name: row.get_text("name")?,
        base_url: normalize_optional(row.get_optional_text("base_url")?),
        enabled: row.get_i64("enabled")? != 0,
        note: row.get_optional_text("note")?,
        has_secret: row.get_i64("has_secret")? != 0,
        created_at: row.get_text("created_at")?,
        updated_at: row.get_text("updated_at")?,
    })
}

fn route_from_row(row: Row) -> Result<RouteBinding> {
    Ok(RouteBinding {
        id: row.get_text("id")?,
        provider: row.get_text("provider")?,
        model_prefix: normalize_optional(row.get_optional_text("model_prefix")?),
        account_id: row.get_text("account_id")?,
        updated_at: row.get_text("updated_at")?,
    })
}

fn log_from_row(row: Row) -> Result<RequestLog> {
    Ok(RequestLog {
        id: row.get_text("id")?,
        created_at: row.get_text("created_at")?,
        provider: row.get_text("provider")?,
        session_id: row.get_optional_text("session_id")?,
        model: row.get_optional_text("model")?,
        account_id: row.get_optional_text("account_id")?,
        method: row.get_text("method")?,
        path: row.get_text("path")?,
        status_code: row
            .get_optional_i64("status_code")?
            .map(|value| value as u16),
        duration_ms: row.get_i64("duration_ms")? as u64,
        error_text: row.get_optional_text("error_text")?,
        request_headers: row.get_text("request_headers")?,
        request_body: row.get_text("request_body")?,
        response_headers: row.get_text("response_headers")?,
        response_body: row.get_text("response_body")?,
        log_file_path: row.get_optional_text("log_file_path")?,
        streamed: row.get_i64("streamed")? != 0,
    })
}

fn log_detail_from_row(row: Row, root: &Path) -> Result<RequestLog> {
    let id = row.get_text("id")?;
    let created_at = row.get_text("created_at")?;
    let provider = row.get_text("provider")?;
    let session_id = row.get_optional_text("session_id")?;
    let model = row.get_optional_text("model")?;
    let account_id = row.get_optional_text("account_id")?;
    let method = row.get_text("method")?;
    let path = row.get_text("path")?;
    let status_code = row
        .get_optional_i64("status_code")?
        .map(|value| value as u16);
    let duration_ms = row.get_i64("duration_ms")? as u64;
    let error_text = row.get_optional_text("error_text")?;
    let streamed = row.get_i64("streamed")? != 0;
    let log_file_path = normalize_optional(row.get_optional_text("log_file_path")?);

    if let Some(file_path) = log_file_path.clone() {
        if let Some(log) = read_log_record(root, &file_path, &id)? {
            return Ok(log);
        }
    }

    let request_headers = read_log_artifact(
        root,
        row.get_optional_text("request_headers_path")?,
        row.get_text("request_headers")?,
    )?;
    let request_body = read_log_artifact(
        root,
        row.get_optional_text("request_body_path")?,
        row.get_text("request_body")?,
    )?;
    let response_headers = read_log_artifact(
        root,
        row.get_optional_text("response_headers_path")?,
        row.get_text("response_headers")?,
    )?;
    let response_body = read_log_artifact(
        root,
        row.get_optional_text("response_body_path")?,
        row.get_text("response_body")?,
    )?;

    Ok(RequestLog {
        id,
        created_at,
        provider,
        session_id,
        model,
        account_id,
        method,
        path,
        status_code,
        duration_ms,
        error_text,
        request_headers,
        request_body,
        response_headers,
        response_body,
        log_file_path,
        streamed,
    })
}

fn fetch_provider_by_slug(db: &Connection, slug: &str) -> Result<Option<ProviderDefinition>> {
    db.query(
        "SELECT slug, display_name, protocol, base_url, proxy_path, auth_header,
         auth_prefix, enabled, is_builtin, created_at, updated_at
         FROM providers WHERE slug = ?",
        &[SqlValue::Text(slug.into())],
    )?
    .into_iter()
    .next()
    .map(provider_from_row)
    .transpose()
}

fn fetch_provider_by_proxy_path(
    db: &Connection,
    proxy_path: &str,
) -> Result<Option<ProviderDefinition>> {
    db.query(
        "SELECT slug, display_name, protocol, base_url, proxy_path, auth_header,
         auth_prefix, enabled, is_builtin, created_at, updated_at
         FROM providers WHERE proxy_path = ?",
        &[SqlValue::Text(proxy_path.into())],
    )?
    .into_iter()
    .next()
    .map(provider_from_row)
    .transpose()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredLogPart {
    headers: serde_json::Value,
    body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredLogRecord {
    id: String,
    created_at: String,
    provider: String,
    session_id: Option<String>,
    model: Option<String>,
    account_id: Option<String>,
    method: String,
    path: String,
    status_code: Option<u16>,
    duration_ms: u64,
    error_text: Option<String>,
    request: StoredLogPart,
    response: StoredLogPart,
    streamed: bool,
}

impl StoredLogRecord {
    fn from_input(
        id: &str,
        created_at: &str,
        session_id: Option<String>,
        input: &RequestLogInput,
    ) -> Self {
        Self {
            id: id.to_owned(),
            created_at: created_at.to_owned(),
            provider: input.provider.clone(),
            session_id,
            model: input.model.clone(),
            account_id: input.account_id.clone(),
            method: input.method.clone(),
            path: input.path.clone(),
            status_code: input.status_code,
            duration_ms: input.duration_ms,
            error_text: input.error_text.clone(),
            request: StoredLogPart {
                headers: headers_to_json_value(&input.request_headers),
                body: input.request_body.clone(),
            },
            response: StoredLogPart {
                headers: headers_to_json_value(&input.response_headers),
                body: input.response_body.clone(),
            },
            streamed: input.streamed,
        }
    }

    fn into_request_log(self, log_file_path: String) -> RequestLog {
        let request_headers = headers_to_text(&self.request.headers);
        let response_headers = headers_to_text(&self.response.headers);
        RequestLog {
            id: self.id,
            created_at: self.created_at,
            provider: self.provider,
            session_id: self.session_id,
            model: self.model,
            account_id: self.account_id,
            method: self.method,
            path: self.path,
            status_code: self.status_code,
            duration_ms: self.duration_ms,
            error_text: self.error_text,
            request_headers,
            request_body: self.request.body,
            response_headers,
            response_body: self.response.body,
            log_file_path: Some(log_file_path),
            streamed: self.streamed,
        }
    }
}

fn migrate_schema(db: &Connection, root: &Path) -> Result<()> {
    ensure_column(db, "accounts", "base_url", "TEXT")?;
    ensure_column(db, "request_logs", "session_id", "TEXT")?;
    ensure_column(db, "request_logs", "log_file_path", "TEXT")?;
    ensure_column(db, "request_logs", "request_headers_path", "TEXT")?;
    ensure_column(db, "request_logs", "request_body_path", "TEXT")?;
    ensure_column(db, "request_logs", "response_headers_path", "TEXT")?;
    ensure_column(db, "request_logs", "response_body_path", "TEXT")?;
    db.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_request_logs_session_id ON request_logs(session_id);",
    )?;
    migrate_builtin_provider_catalog(db)?;
    retire_legacy_builtin_custom_provider(db)?;
    migrate_request_logs(db, root)?;
    Ok(())
}

fn ensure_column(db: &Connection, table: &str, column: &str, definition: &str) -> Result<()> {
    let pragma = format!("PRAGMA table_info({table})");
    let exists = db.query(pragma.as_str(), &[])?.into_iter().any(|row| {
        row.get_text("name")
            .map(|name| name == column)
            .unwrap_or(false)
    });
    if !exists {
        let sql = format!("ALTER TABLE {table} ADD COLUMN {column} {definition}");
        db.execute_batch(sql.as_str())?;
    }
    Ok(())
}

fn migrate_builtin_provider_catalog(db: &Connection) -> Result<()> {
    for rename in BUILTIN_PROVIDER_RENAMES {
        migrate_builtin_provider_slug(db, *rename)?;
    }
    Ok(())
}

fn retire_legacy_builtin_custom_provider(db: &Connection) -> Result<()> {
    let Some(provider) = fetch_provider_by_slug(db, "custom")? else {
        return Ok(());
    };
    if !provider.is_builtin {
        return Ok(());
    }

    let account_count = db
        .query(
            "SELECT COUNT(*) AS count FROM accounts WHERE provider = ?",
            &[SqlValue::Text("custom".into())],
        )?
        .into_iter()
        .next()
        .ok_or_else(|| LocalOpenRouterError::Sqlite("failed to count custom accounts".into()))?
        .get_i64("count")?;
    let route_count = db
        .query(
            "SELECT COUNT(*) AS count FROM route_bindings WHERE provider = ?",
            &[SqlValue::Text("custom".into())],
        )?
        .into_iter()
        .next()
        .ok_or_else(|| LocalOpenRouterError::Sqlite("failed to count custom routes".into()))?
        .get_i64("count")?;

    let is_default_stub = provider.display_name == "Custom"
        && provider.protocol == ApiProtocol::Generic
        && provider.base_url == "https://example.com"
        && provider.proxy_path == "custom"
        && provider.auth_header == "Authorization"
        && provider.auth_prefix.as_deref() == Some("Bearer")
        && !provider.enabled;

    if is_default_stub && account_count == 0 && route_count == 0 {
        db.execute(
            "DELETE FROM providers WHERE slug = ?",
            &[SqlValue::Text("custom".into())],
        )?;
    } else {
        db.execute(
            "UPDATE providers SET is_builtin = 0, updated_at = ? WHERE slug = ?",
            &[SqlValue::Text(timestamp()), SqlValue::Text("custom".into())],
        )?;
    }
    Ok(())
}

fn migrate_builtin_provider_slug(db: &Connection, rename: BuiltinProviderRename) -> Result<()> {
    let Some(provider) = fetch_provider_by_slug(db, rename.old_slug)? else {
        return Ok(());
    };
    if !provider.is_builtin || fetch_provider_by_slug(db, rename.new_slug)?.is_some() {
        return Ok(());
    }

    let display_name = if provider.display_name == rename.old_display_name {
        rename.new_display_name.to_owned()
    } else {
        provider.display_name.clone()
    };
    let proxy_path = if provider.proxy_path == rename.old_proxy_path {
        match fetch_provider_by_proxy_path(db, rename.new_proxy_path)? {
            Some(conflict) if conflict.slug != rename.old_slug => provider.proxy_path.clone(),
            _ => rename.new_proxy_path.to_owned(),
        }
    } else {
        provider.proxy_path.clone()
    };
    let now = timestamp();

    db.execute(
        "UPDATE providers
         SET slug = ?, display_name = ?, proxy_path = ?, updated_at = ?
         WHERE slug = ?",
        &[
            SqlValue::Text(rename.new_slug.into()),
            SqlValue::Text(display_name),
            SqlValue::Text(proxy_path),
            SqlValue::Text(now.clone()),
            SqlValue::Text(rename.old_slug.into()),
        ],
    )?;
    db.execute(
        "UPDATE accounts SET provider = ?, updated_at = ? WHERE provider = ?",
        &[
            SqlValue::Text(rename.new_slug.into()),
            SqlValue::Text(now.clone()),
            SqlValue::Text(rename.old_slug.into()),
        ],
    )?;

    let route_rows = db.query(
        "SELECT id, model_prefix FROM route_bindings WHERE provider = ? ORDER BY id",
        &[SqlValue::Text(rename.old_slug.into())],
    )?;
    for row in route_rows {
        let old_id = row.get_text("id")?;
        let model_prefix = normalize_optional(row.get_optional_text("model_prefix")?);
        let new_id = route_binding_id(rename.new_slug, model_prefix.as_deref());
        db.execute(
            "UPDATE route_bindings SET id = ?, provider = ?, updated_at = ? WHERE id = ?",
            &[
                SqlValue::Text(new_id),
                SqlValue::Text(rename.new_slug.into()),
                SqlValue::Text(now.clone()),
                SqlValue::Text(old_id),
            ],
        )?;
    }

    db.execute(
        "UPDATE request_logs SET provider = ? WHERE provider = ?",
        &[
            SqlValue::Text(rename.new_slug.into()),
            SqlValue::Text(rename.old_slug.into()),
        ],
    )?;
    Ok(())
}

fn append_log_record(root: &Path, record: &StoredLogRecord) -> Result<String> {
    let relative_path = daily_log_relative_path(&record.created_at);
    let absolute_path = root.join(&relative_path);
    if let Some(parent) = absolute_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&absolute_path)?;
    serde_json::to_writer(&mut file, record).map_err(|error| {
        LocalOpenRouterError::Io(format!("failed to serialize log record: {error}"))
    })?;
    file.write_all(b"\n")?;
    Ok(relative_path)
}

fn read_log_artifact(root: &Path, path: Option<String>, fallback: String) -> Result<String> {
    if let Some(path) = normalize_optional(path) {
        match fs::read_to_string(root.join(path)) {
            Ok(value) => return Ok(value),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(fallback)
}

fn read_log_record(root: &Path, log_file_path: &str, id: &str) -> Result<Option<RequestLog>> {
    let file = match fs::File::open(root.join(log_file_path)) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let reader = BufReader::new(file);
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let record: StoredLogRecord = match serde_json::from_str(&line) {
            Ok(record) => record,
            Err(_) => continue,
        };
        if record.id == id {
            return Ok(Some(record.into_request_log(log_file_path.to_owned())));
        }
    }
    Ok(None)
}

fn migrate_request_logs(db: &Connection, root: &Path) -> Result<()> {
    let rows = db.query(
        "SELECT id, created_at, provider, session_id, model, account_id, method, path, status_code,
         duration_ms, error_text, request_headers, request_body, response_headers, response_body,
         log_file_path, request_headers_path, request_body_path, response_headers_path,
         response_body_path, streamed
         FROM request_logs
         WHERE log_file_path IS NULL",
        &[],
    )?;

    for row in rows {
        let id = row.get_text("id")?;
        let created_at = row.get_text("created_at")?;
        let provider = row.get_text("provider")?;
        let model = row.get_optional_text("model")?;
        let account_id = row.get_optional_text("account_id")?;
        let method = row.get_text("method")?;
        let path = row.get_text("path")?;
        let status_code = row
            .get_optional_i64("status_code")?
            .map(|value| value as u16);
        let duration_ms = row.get_i64("duration_ms")? as u64;
        let error_text = row.get_optional_text("error_text")?;
        let request_headers = row.get_text("request_headers")?;
        let request_body = row.get_text("request_body")?;
        let response_headers = row.get_text("response_headers")?;
        let response_body = row.get_text("response_body")?;
        let request_headers = read_log_artifact(
            root,
            row.get_optional_text("request_headers_path")?,
            request_headers,
        )?;
        let request_body = read_log_artifact(
            root,
            row.get_optional_text("request_body_path")?,
            request_body,
        )?;
        let response_headers = read_log_artifact(
            root,
            row.get_optional_text("response_headers_path")?,
            response_headers,
        )?;
        let response_body = read_log_artifact(
            root,
            row.get_optional_text("response_body_path")?,
            response_body,
        )?;
        let session_id = row.get_optional_text("session_id")?.or_else(|| {
            extract_session_id(
                &request_headers,
                &request_body,
                &response_headers,
                &response_body,
            )
        });
        let record = StoredLogRecord {
            id: id.clone(),
            created_at,
            provider,
            session_id: session_id.clone(),
            model,
            account_id,
            method,
            path,
            status_code,
            duration_ms,
            error_text,
            request: StoredLogPart {
                headers: headers_to_json_value(&request_headers),
                body: request_body,
            },
            response: StoredLogPart {
                headers: headers_to_json_value(&response_headers),
                body: response_body,
            },
            streamed: row.get_i64("streamed")? != 0,
        };
        let log_file_path = append_log_record(root, &record)?;
        db.execute(
            "UPDATE request_logs
             SET session_id = ?,
                 request_headers = '',
                 request_body = '',
                 response_headers = '',
                 response_body = '',
                 log_file_path = ?
             WHERE id = ?",
            &[
                session_id.map(SqlValue::Text).unwrap_or(SqlValue::Null),
                SqlValue::Text(log_file_path),
                SqlValue::Text(id),
            ],
        )?;
    }

    Ok(())
}

fn daily_log_relative_path(created_at: &str) -> String {
    let day = created_at.get(..10).unwrap_or("undated");
    PathBuf::from("logs")
        .join(format!("{day}.jsonl"))
        .to_string_lossy()
        .into_owned()
}

fn headers_to_json_value(headers: &str) -> serde_json::Value {
    serde_json::from_str(headers).unwrap_or_else(|_| serde_json::Value::String(headers.to_owned()))
}

fn headers_to_text(headers: &serde_json::Value) -> String {
    match headers {
        serde_json::Value::String(value) => value.clone(),
        _ => serde_json::to_string_pretty(headers).unwrap_or_else(|_| "{}".into()),
    }
}

fn seed_builtin_providers(db: &Connection) -> Result<()> {
    for builtin in BUILTIN_PROVIDERS {
        let now = timestamp();
        db.execute(
            "INSERT INTO providers (
               slug, display_name, protocol, base_url, proxy_path, auth_header, auth_prefix,
               enabled, is_builtin, created_at, updated_at
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, 1, ?, ?)
             ON CONFLICT(slug) DO NOTHING",
            &[
                SqlValue::Text(builtin.slug.into()),
                SqlValue::Text(builtin.display_name.into()),
                SqlValue::Text(builtin.protocol.as_str().into()),
                SqlValue::Text(builtin.base_url.into()),
                SqlValue::Text(builtin.proxy_path.into()),
                SqlValue::Text(builtin.auth_header.into()),
                builtin
                    .auth_prefix
                    .map(|value| SqlValue::Text(value.into()))
                    .unwrap_or(SqlValue::Null),
                SqlValue::Integer(i64::from(builtin.enabled)),
                SqlValue::Text(now.clone()),
                SqlValue::Text(now),
            ],
        )?;
    }
    Ok(())
}

fn get_setting_blob(db: &Connection, key: &str) -> Result<Option<Vec<u8>>> {
    Ok(db
        .query(
            "SELECT value_blob FROM app_settings WHERE key = ?",
            &[SqlValue::Text(key.into())],
        )?
        .into_iter()
        .next()
        .map(|row| row.get_blob("value_blob"))
        .transpose()?)
}

fn upsert_setting_blob(db: &Connection, key: &str, value: &[u8]) -> Result<()> {
    db.execute(
        "INSERT INTO app_settings (key, value_text, value_blob, updated_at)
         VALUES (?, NULL, ?, ?)
         ON CONFLICT(key) DO UPDATE SET
           value_blob = excluded.value_blob,
           updated_at = excluded.updated_at",
        &[
            SqlValue::Text(key.into()),
            SqlValue::Blob(value.to_vec()),
            SqlValue::Text(timestamp()),
        ],
    )
}

fn master_key_from_password(db: &Connection, password: &str) -> Result<[u8; 32]> {
    let salt = get_setting_blob(db, VAULT_SALT_KEY)?;
    let check_nonce = get_setting_blob(db, VAULT_CHECK_NONCE_KEY)?;
    let check_ciphertext = get_setting_blob(db, VAULT_CHECK_CIPHERTEXT_KEY)?;

    match (salt, check_nonce, check_ciphertext) {
        (Some(salt), Some(check_nonce), Some(check_ciphertext)) => {
            crypto::unlock_master_password(password, &salt, &check_nonce, &check_ciphertext)
        }
        (None, None, None) => Err(LocalOpenRouterError::Validation(
            "vault is not initialized".into(),
        )),
        _ => Err(LocalOpenRouterError::Sqlite(
            "vault metadata is incomplete; delete the database to recover".into(),
        )),
    }
}

fn decrypt_account_secret(db: &Connection, account_id: &str, key: &[u8; 32]) -> Result<String> {
    let secret = db
        .query(
            "SELECT nonce, ciphertext FROM encrypted_secrets WHERE account_id = ?",
            &[SqlValue::Text(account_id.into())],
        )?
        .into_iter()
        .next()
        .ok_or_else(|| {
            LocalOpenRouterError::NotFound(format!("secret missing for account `{account_id}`"))
        })?;
    crypto::decrypt_secret(
        key,
        &secret.get_blob("nonce")?,
        &secret.get_blob("ciphertext")?,
    )
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn normalize_non_empty(field: &str, value: &str) -> Result<String> {
    let normalized = value.trim();
    if normalized.is_empty() {
        return Err(LocalOpenRouterError::Validation(format!(
            "{field} must not be empty"
        )));
    }
    Ok(normalized.to_owned())
}

fn normalize_slug(value: &str) -> Result<String> {
    let normalized = value.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return Err(LocalOpenRouterError::Validation(
            "provider slug must not be empty".into(),
        ));
    }
    if normalized.starts_with("admin") {
        return Err(LocalOpenRouterError::Validation(
            "provider slug must not start with `admin`".into(),
        ));
    }
    if !normalized
        .chars()
        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
    {
        return Err(LocalOpenRouterError::Validation(
            "provider slug may only contain lowercase letters, digits, and dashes".into(),
        ));
    }
    Ok(normalized)
}

fn normalize_proxy_path(value: &str) -> Result<String> {
    let normalized = value.trim().trim_matches('/').to_ascii_lowercase();
    if normalized.is_empty() {
        return Err(LocalOpenRouterError::Validation(
            "proxy path must not be empty".into(),
        ));
    }
    if normalized.contains('/') {
        return Err(LocalOpenRouterError::Validation(
            "proxy path must be a single path segment".into(),
        ));
    }
    if !normalized
        .chars()
        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
    {
        return Err(LocalOpenRouterError::Validation(
            "proxy path may only contain lowercase letters, digits, and dashes".into(),
        ));
    }
    Ok(normalized)
}

fn normalize_base_url(value: &str) -> Result<String> {
    let normalized = value.trim().trim_end_matches('/').to_owned();
    if normalized.is_empty() {
        return Err(LocalOpenRouterError::Validation(
            "base URL must not be empty".into(),
        ));
    }
    if !(normalized.starts_with("http://") || normalized.starts_with("https://")) {
        return Err(LocalOpenRouterError::Validation(
            "base URL must start with http:// or https://".into(),
        ));
    }
    Ok(normalized)
}

fn normalize_optional_base_url(value: Option<String>) -> Result<Option<String>> {
    match normalize_optional(value) {
        Some(value) => Ok(Some(normalize_base_url(&value)?)),
        None => Ok(None),
    }
}

fn normalize_header_name(value: &str) -> Result<String> {
    let normalized = value.trim();
    if normalized.is_empty() {
        return Err(LocalOpenRouterError::Validation(
            "auth header must not be empty".into(),
        ));
    }
    if normalized.contains(char::is_whitespace) {
        return Err(LocalOpenRouterError::Validation(
            "auth header must not contain whitespace".into(),
        ));
    }
    Ok(normalized.to_owned())
}

fn timestamp() -> String {
    Utc::now().to_rfc3339()
}

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS providers (
  slug TEXT PRIMARY KEY,
  display_name TEXT NOT NULL,
  protocol TEXT NOT NULL,
  base_url TEXT NOT NULL,
  proxy_path TEXT NOT NULL UNIQUE,
  auth_header TEXT NOT NULL,
  auth_prefix TEXT,
  enabled INTEGER NOT NULL,
  is_builtin INTEGER NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS accounts (
  id TEXT PRIMARY KEY,
  provider TEXT NOT NULL,
  name TEXT NOT NULL,
  enabled INTEGER NOT NULL,
  note TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS encrypted_secrets (
  account_id TEXT PRIMARY KEY REFERENCES accounts(id) ON DELETE CASCADE,
  nonce BLOB NOT NULL,
  ciphertext BLOB NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS route_bindings (
  id TEXT PRIMARY KEY,
  provider TEXT NOT NULL,
  model_prefix TEXT,
  account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS request_logs (
  id TEXT PRIMARY KEY,
  created_at TEXT NOT NULL,
  provider TEXT NOT NULL,
  session_id TEXT,
  model TEXT,
  account_id TEXT,
  method TEXT NOT NULL,
  path TEXT NOT NULL,
  status_code INTEGER,
  duration_ms INTEGER NOT NULL,
  error_text TEXT,
  request_headers TEXT NOT NULL,
  request_body TEXT NOT NULL,
  response_headers TEXT NOT NULL,
  response_body TEXT NOT NULL,
  request_headers_path TEXT,
  request_body_path TEXT,
  response_headers_path TEXT,
  response_body_path TEXT,
  log_file_path TEXT,
  streamed INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS app_settings (
  key TEXT PRIMARY KEY,
  value_text TEXT,
  value_blob BLOB,
  updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_accounts_provider ON accounts(provider);
CREATE INDEX IF NOT EXISTS idx_route_bindings_provider ON route_bindings(provider);
CREATE INDEX IF NOT EXISTS idx_request_logs_provider ON request_logs(provider);

INSERT OR IGNORE INTO schema_migrations (version, applied_at) VALUES (1, CURRENT_TIMESTAMP);
INSERT OR IGNORE INTO schema_migrations (version, applied_at) VALUES (2, CURRENT_TIMESTAMP);
INSERT OR IGNORE INTO schema_migrations (version, applied_at) VALUES (3, CURRENT_TIMESTAMP);
"#;

#[cfg(test)]
mod tests {
    use super::{AppPaths, Repository, daily_log_relative_path};
    use crate::models::{
        AccountInput, ApiProtocol, ProviderInput, RequestLogInput, RouteBindingInput,
    };
    use crate::sqlite::Connection;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(1);

    fn unique_root() -> std::path::PathBuf {
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!("localopenrouter-test-{id}"))
    }

    async fn repo() -> Repository {
        let root = unique_root();
        let _ = std::fs::remove_dir_all(&root);
        Repository::new_with_paths(AppPaths::from_root(root).unwrap(), 7331)
            .await
            .unwrap()
    }

    fn init_legacy_db(root: &std::path::Path) {
        let paths = AppPaths::from_root(root.to_path_buf()).unwrap();
        let db = Connection::open(&paths.database).unwrap();
        db.execute_batch(
            r#"
CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS providers (
  slug TEXT PRIMARY KEY,
  display_name TEXT NOT NULL,
  protocol TEXT NOT NULL,
  base_url TEXT NOT NULL,
  proxy_path TEXT NOT NULL UNIQUE,
  auth_header TEXT NOT NULL,
  auth_prefix TEXT,
  enabled INTEGER NOT NULL,
  is_builtin INTEGER NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS accounts (
  id TEXT PRIMARY KEY,
  provider TEXT NOT NULL,
  name TEXT NOT NULL,
  enabled INTEGER NOT NULL,
  note TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS encrypted_secrets (
  account_id TEXT PRIMARY KEY REFERENCES accounts(id) ON DELETE CASCADE,
  nonce BLOB NOT NULL,
  ciphertext BLOB NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS route_bindings (
  id TEXT PRIMARY KEY,
  provider TEXT NOT NULL,
  model_prefix TEXT,
  account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS request_logs (
  id TEXT PRIMARY KEY,
  created_at TEXT NOT NULL,
  provider TEXT NOT NULL,
  model TEXT,
  account_id TEXT,
  method TEXT NOT NULL,
  path TEXT NOT NULL,
  status_code INTEGER,
  duration_ms INTEGER NOT NULL,
  error_text TEXT,
  request_headers TEXT NOT NULL,
  request_body TEXT NOT NULL,
  response_headers TEXT NOT NULL,
  response_body TEXT NOT NULL,
  request_headers_path TEXT,
  request_body_path TEXT,
  response_headers_path TEXT,
  response_body_path TEXT,
  streamed INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS app_settings (
  key TEXT PRIMARY KEY,
  value_text TEXT,
  value_blob BLOB,
  updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_accounts_provider ON accounts(provider);
CREATE INDEX IF NOT EXISTS idx_route_bindings_provider ON route_bindings(provider);
CREATE INDEX IF NOT EXISTS idx_request_logs_provider ON request_logs(provider);
"#,
        )
        .unwrap();
    }

    #[test]
    fn app_paths_reuse_legacy_database_name_when_present() {
        let root = unique_root();
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::File::create(root.join(super::LEGACY_DATABASE_FILE_NAME)).unwrap();

        let paths = AppPaths::from_root(root).unwrap();
        assert_eq!(
            paths.database.file_name().and_then(|value| value.to_str()),
            Some(super::LEGACY_DATABASE_FILE_NAME)
        );
    }

    #[tokio::test]
    async fn legacy_builtin_providers_are_migrated_to_client_named_catalog() {
        let root = unique_root();
        let _ = std::fs::remove_dir_all(&root);
        init_legacy_db(&root);
        let paths = AppPaths::from_root(root).unwrap();
        let db = Connection::open(&paths.database).unwrap();
        let now = super::timestamp();
        db.execute(
            "INSERT INTO providers (
               slug, display_name, protocol, base_url, proxy_path, auth_header, auth_prefix,
               enabled, is_builtin, created_at, updated_at
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            &[
                crate::sqlite::SqlValue::Text("openai".into()),
                crate::sqlite::SqlValue::Text("OpenAI".into()),
                crate::sqlite::SqlValue::Text("openai".into()),
                crate::sqlite::SqlValue::Text("https://api.openai.com".into()),
                crate::sqlite::SqlValue::Text("openai".into()),
                crate::sqlite::SqlValue::Text("Authorization".into()),
                crate::sqlite::SqlValue::Text("Bearer".into()),
                crate::sqlite::SqlValue::Integer(1),
                crate::sqlite::SqlValue::Integer(1),
                crate::sqlite::SqlValue::Text(now.clone()),
                crate::sqlite::SqlValue::Text(now.clone()),
            ],
        )
        .unwrap();
        db.execute(
            "INSERT INTO accounts (id, provider, name, enabled, note, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            &[
                crate::sqlite::SqlValue::Text("acct_legacy".into()),
                crate::sqlite::SqlValue::Text("openai".into()),
                crate::sqlite::SqlValue::Text("Legacy Primary".into()),
                crate::sqlite::SqlValue::Integer(1),
                crate::sqlite::SqlValue::Null,
                crate::sqlite::SqlValue::Text(now.clone()),
                crate::sqlite::SqlValue::Text(now.clone()),
            ],
        )
        .unwrap();
        db.execute(
            "INSERT INTO route_bindings (id, provider, model_prefix, account_id, updated_at)
             VALUES (?, ?, NULL, ?, ?)",
            &[
                crate::sqlite::SqlValue::Text("openai::*".into()),
                crate::sqlite::SqlValue::Text("openai".into()),
                crate::sqlite::SqlValue::Text("acct_legacy".into()),
                crate::sqlite::SqlValue::Text(now),
            ],
        )
        .unwrap();

        let repo = Repository::new_with_paths(paths, 7331).await.unwrap();
        let codex = repo.get_provider("codex").await.unwrap();
        assert_eq!(codex.display_name, "Codex");
        assert_eq!(codex.proxy_path, "codex");
        assert!(repo.get_provider("openai").await.is_err());

        let accounts = repo.list_accounts().await.unwrap();
        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].provider, "codex");

        let routes = repo.list_routes().await.unwrap();
        assert_eq!(routes.len(), 1);
        assert_eq!(routes[0].provider, "codex");
        assert_eq!(routes[0].id, "codex::*");

        let providers = repo.list_providers().await.unwrap();
        assert!(
            providers
                .iter()
                .any(|provider| provider.slug == "claude-code")
        );
        assert!(providers.iter().any(|provider| provider.slug == "gemini"));
        assert!(!providers.iter().any(|provider| provider.slug == "custom"));
    }

    #[tokio::test]
    async fn legacy_builtin_custom_provider_is_demoted_when_in_use() {
        let root = unique_root();
        let _ = std::fs::remove_dir_all(&root);
        init_legacy_db(&root);
        let paths = AppPaths::from_root(root).unwrap();
        let db = Connection::open(&paths.database).unwrap();
        let now = super::timestamp();
        db.execute(
            "INSERT INTO providers (
               slug, display_name, protocol, base_url, proxy_path, auth_header, auth_prefix,
               enabled, is_builtin, created_at, updated_at
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            &[
                crate::sqlite::SqlValue::Text("custom".into()),
                crate::sqlite::SqlValue::Text("Custom".into()),
                crate::sqlite::SqlValue::Text("generic".into()),
                crate::sqlite::SqlValue::Text("https://example.com".into()),
                crate::sqlite::SqlValue::Text("custom".into()),
                crate::sqlite::SqlValue::Text("Authorization".into()),
                crate::sqlite::SqlValue::Text("Bearer".into()),
                crate::sqlite::SqlValue::Integer(0),
                crate::sqlite::SqlValue::Integer(1),
                crate::sqlite::SqlValue::Text(now.clone()),
                crate::sqlite::SqlValue::Text(now.clone()),
            ],
        )
        .unwrap();
        db.execute(
            "INSERT INTO accounts (id, provider, name, enabled, note, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            &[
                crate::sqlite::SqlValue::Text("acct_custom".into()),
                crate::sqlite::SqlValue::Text("custom".into()),
                crate::sqlite::SqlValue::Text("Custom Primary".into()),
                crate::sqlite::SqlValue::Integer(1),
                crate::sqlite::SqlValue::Null,
                crate::sqlite::SqlValue::Text(now.clone()),
                crate::sqlite::SqlValue::Text(now),
            ],
        )
        .unwrap();

        let repo = Repository::new_with_paths(paths, 7331).await.unwrap();
        let provider = repo.get_provider("custom").await.unwrap();
        assert!(!provider.is_builtin);
    }

    #[tokio::test]
    async fn route_precedence_prefers_model_match() {
        let repo = repo().await;
        repo.unlock("password").await.unwrap();
        let codex = repo
            .upsert_account(AccountInput {
                id: None,
                provider: "codex".into(),
                name: "Primary".into(),
                base_url: None,
                api_key: Some("sk-primary".into()),
                note: None,
                enabled: true,
            })
            .await
            .unwrap();
        let backup = repo
            .upsert_account(AccountInput {
                id: None,
                provider: "codex".into(),
                name: "GPT-5".into(),
                base_url: None,
                api_key: Some("sk-gpt5".into()),
                note: None,
                enabled: true,
            })
            .await
            .unwrap();
        repo.set_route_binding(RouteBindingInput {
            provider: "codex".into(),
            model_prefix: Some("gpt-5".into()),
            account_id: backup.id.clone(),
        })
        .await
        .unwrap();
        let resolved = repo
            .resolve_account("codex", Some("gpt-5-codex"))
            .await
            .unwrap();
        assert_eq!(resolved.account.id, backup.id);
        let resolved_default = repo
            .resolve_account("codex", Some("gpt-4.1"))
            .await
            .unwrap();
        assert_eq!(resolved_default.account.id, codex.id);
    }

    #[tokio::test]
    async fn set_route_binding_replaces_legacy_default_route_rows() {
        let repo = repo().await;
        repo.unlock("password").await.unwrap();
        let primary = repo
            .upsert_account(AccountInput {
                id: None,
                provider: "codex".into(),
                name: "Primary".into(),
                base_url: None,
                api_key: Some("sk-primary".into()),
                note: None,
                enabled: true,
            })
            .await
            .unwrap();
        let backup = repo
            .upsert_account(AccountInput {
                id: None,
                provider: "codex".into(),
                name: "Backup".into(),
                base_url: None,
                api_key: Some("sk-backup".into()),
                note: None,
                enabled: true,
            })
            .await
            .unwrap();

        {
            let db = repo.db.lock().await;
            db.execute(
                "DELETE FROM route_bindings WHERE provider = ? AND model_prefix IS NULL",
                &[crate::sqlite::SqlValue::Text("codex".into())],
            )
            .unwrap();
            db.execute(
                "INSERT INTO route_bindings (id, provider, model_prefix, account_id, updated_at)
                 VALUES (?, ?, NULL, ?, ?)",
                &[
                    crate::sqlite::SqlValue::Text("legacy-default-route".into()),
                    crate::sqlite::SqlValue::Text("codex".into()),
                    crate::sqlite::SqlValue::Text(primary.id.clone()),
                    crate::sqlite::SqlValue::Text(super::timestamp()),
                ],
            )
            .unwrap();
        }

        let updated = repo
            .set_route_binding(RouteBindingInput {
                provider: "codex".into(),
                model_prefix: None,
                account_id: backup.id.clone(),
            })
            .await
            .unwrap();
        assert_eq!(updated.id, "codex::*");
        assert_eq!(updated.account_id, backup.id);

        let routes = repo.list_routes().await.unwrap();
        let default_routes: Vec<_> = routes
            .into_iter()
            .filter(|route| route.provider == "codex" && route.model_prefix.is_none())
            .collect();
        assert_eq!(default_routes.len(), 1);
        assert_eq!(default_routes[0].id, "codex::*");
        assert_eq!(default_routes[0].account_id, backup.id);
    }

    #[tokio::test]
    async fn custom_provider_can_be_created_and_resolved() {
        let repo = repo().await;
        repo.unlock("password").await.unwrap();
        let provider = repo
            .upsert_provider(ProviderInput {
                slug: "openrouter".into(),
                display_name: "OpenRouter".into(),
                protocol: ApiProtocol::OpenAi,
                base_url: "https://openrouter.ai/api/v1".into(),
                proxy_path: "openrouter".into(),
                auth_header: "Authorization".into(),
                auth_prefix: Some("Bearer".into()),
                enabled: true,
            })
            .await
            .unwrap();
        let account = repo
            .upsert_account(AccountInput {
                id: None,
                provider: provider.slug.clone(),
                name: "OpenRouter Primary".into(),
                base_url: None,
                api_key: Some("sk-openrouter".into()),
                note: None,
                enabled: true,
            })
            .await
            .unwrap();
        let resolved = repo
            .resolve_account("openrouter", Some("gpt-4o-mini"))
            .await
            .unwrap();
        assert_eq!(resolved.provider.proxy_path, "openrouter");
        assert_eq!(resolved.account.id, account.id);
        assert_eq!(resolved.provider.protocol, ApiProtocol::OpenAi);
    }

    #[tokio::test]
    async fn account_base_url_override_wins_over_provider_base_url() {
        let repo = repo().await;
        repo.unlock("password").await.unwrap();
        let provider = repo
            .upsert_provider(ProviderInput {
                slug: "openrouter".into(),
                display_name: "OpenRouter".into(),
                protocol: ApiProtocol::OpenAi,
                base_url: "https://openrouter.ai/api/v1".into(),
                proxy_path: "openrouter".into(),
                auth_header: "Authorization".into(),
                auth_prefix: Some("Bearer".into()),
                enabled: true,
            })
            .await
            .unwrap();
        let account = repo
            .upsert_account(AccountInput {
                id: None,
                provider: provider.slug.clone(),
                name: "Regional Override".into(),
                base_url: Some("https://regional.example.com/v1/".into()),
                api_key: Some("sk-regional".into()),
                note: None,
                enabled: true,
            })
            .await
            .unwrap();

        let resolved = repo
            .resolve_account(&provider.slug, Some("gpt-4.1"))
            .await
            .unwrap();
        assert_eq!(resolved.account.id, account.id);
        assert_eq!(
            resolved.account.base_url.as_deref(),
            Some("https://regional.example.com/v1")
        );
        assert_eq!(
            resolved.upstream_base_url,
            "https://regional.example.com/v1"
        );
    }

    #[tokio::test]
    async fn reveal_account_secret_requires_password_without_unlocking_vault() {
        let repo = repo().await;
        repo.unlock("password").await.unwrap();
        let account = repo
            .upsert_account(AccountInput {
                id: None,
                provider: "codex".into(),
                name: "Primary".into(),
                base_url: None,
                api_key: Some("sk-primary".into()),
                note: None,
                enabled: true,
            })
            .await
            .unwrap();

        repo.lock().await;
        assert!(!repo.health().await.unwrap().unlocked);

        let revealed = repo
            .reveal_account_secret(&account.id, "password")
            .await
            .unwrap();
        assert_eq!(revealed.account_id, account.id);
        assert_eq!(revealed.api_key, "sk-primary");
        assert!(!repo.health().await.unwrap().unlocked);

        let error = repo
            .reveal_account_secret(&account.id, "wrong-password")
            .await
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            "crypto error: master password is invalid"
        );
        assert!(!repo.health().await.unwrap().unlocked);
    }

    #[tokio::test]
    async fn logs_are_indexed_in_sqlite_and_written_to_daily_jsonl() {
        let repo = repo().await;
        let stored = repo
            .insert_log(RequestLogInput {
                provider: "codex".into(),
                model: Some("gpt-5-codex".into()),
                account_id: Some("acct_primary".into()),
                method: "POST".into(),
                path: "/codex/responses".into(),
                status_code: Some(200),
                duration_ms: 42,
                error_text: None,
                request_headers: r#"{"x-session-id":"sess_demo"}"#.into(),
                request_body: r#"{"metadata":{"session_id":"sess_demo"}}"#.into(),
                response_headers: "{}".into(),
                response_body: r#"{"ok":true}"#.into(),
                streamed: false,
            })
            .await
            .unwrap();

        assert_eq!(stored.session_id.as_deref(), Some("sess_demo"));
        let expected_path = daily_log_relative_path(&stored.created_at);
        assert_eq!(
            stored.log_file_path.as_deref(),
            Some(expected_path.as_str())
        );

        let queried = repo
            .query_logs(crate::models::LogQuery {
                limit: Some(5),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(queried.len(), 1);
        assert_eq!(queried[0].session_id.as_deref(), Some("sess_demo"));
        assert_eq!(queried[0].request_body, "");

        let detailed = repo.get_log(&stored.id).await.unwrap();
        assert_eq!(
            detailed.request_body,
            r#"{"metadata":{"session_id":"sess_demo"}}"#
        );
        assert_eq!(detailed.response_body, r#"{"ok":true}"#);
        assert_eq!(detailed.session_id.as_deref(), Some("sess_demo"));
    }

    #[tokio::test]
    async fn repository_migrates_legacy_request_logs_without_session_column() {
        let root = unique_root();
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        init_legacy_db(&root);

        let repo = Repository::new_with_paths(AppPaths::from_root(root).unwrap(), 7331)
            .await
            .unwrap();
        let health = repo.health().await.unwrap();
        assert!(health.db_path.ends_with(super::DATABASE_FILE_NAME));
    }
}
