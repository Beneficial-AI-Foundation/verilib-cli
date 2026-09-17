//! Repo tracking reuses /v2/repo/logs/{id}; no broker credentials or new job API.
#[cfg(test)]
#[path = "repo_tests.rs"]
mod tests;
use anyhow::{Context, Result};
use reqwest::{Client, StatusCode};
use serde_json::{json, Value};
use std::{path::Path, time::Duration};
use tokio::time::Instant;

use super::{init::save_config, metadata, status::get_stored_api_key};
use crate::{
    cli::{RepoCreate, RepoTarget, WaitOptions},
    config::ProjectConfig,
    constants::{auth_required_msg, resolve_base_url},
};

pub fn validate_id(id: &str) -> Result<()> {
    if id.is_empty()
        || !id.bytes().all(|b| b.is_ascii_digit())
        || id.parse::<u64>().unwrap_or(0) == 0
    {
        anyhow::bail!("Repository ID must be a positive integer");
    }
    Ok(())
}

pub fn browser_url(base: &str, id: &str) -> String {
    format!("{}/repobrowser?id={id}", base.trim_end_matches('/'))
}

pub fn render(value: &Value, json_output: bool) -> Result<()> {
    if json_output {
        println!("{}", serde_json::to_string(value)?);
    } else {
        println!("{}", serde_json::to_string_pretty(value)?);
    }
    Ok(())
}

pub fn target(target: RepoTarget) -> Result<(String, String)> {
    let config = ProjectConfig::load(Path::new("."))?;
    let base = resolve_base_url(target.url, config.repo.as_ref().map(|r| r.url.as_str()));
    let id = target
        .id
        .or_else(|| config.repo.map(|r| r.id))
        .context("No repository configured. Use --id or run init --id first")?;
    validate_id(&id)?;
    Ok((id, base))
}

// API debug errors can contain stack traces/request arguments; never relay those to agents.
fn safe_error(body: &Value) -> Value {
    json!({"code":body["data"]["code"], "message":body["data"]["message"]})
}

pub struct Api {
    client: Client,
    base: String,
    key: String,
}

impl Api {
    pub fn new(base: &str, key: String) -> Result<Self> {
        let parsed = reqwest::Url::parse(base).context("Invalid API base URL")?;
        if !matches!(parsed.scheme(), "http" | "https")
            || parsed.host_str().is_none()
            || !parsed.username().is_empty()
            || parsed.password().is_some()
            || parsed.query().is_some()
            || parsed.fragment().is_some()
        {
            anyhow::bail!("API base URL must be HTTP(S) without credentials, query or fragment");
        }
        Ok(Self {
            client: Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .build()?,
            base: base.trim_end_matches('/').to_owned(),
            key,
        })
    }

    pub fn authenticated(base: &str) -> Result<Self> {
        Self::new(base, get_stored_api_key().context(auth_required_msg())?)
    }

    pub async fn logs(&self, id: &str, deadline: Instant) -> Result<(Value, Option<u64>)> {
        validate_id(id)?;
        let endpoint = format!("{}/v2/repo/logs/{id}", self.base);
        let future = async {
            let response = self
                .client
                .get(&endpoint)
                .header("Authorization", format!("ApiKey {}", self.key))
                .header("Accept", "application/json")
                .send()
                .await?;
            let status = response.status();
            let retry = response
                .headers()
                .get("Retry-After")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok());
            let body: Value = response
                .json()
                .await
                .context("Logs API did not return JSON")?;
            if !status.is_success() || body["error"] == true {
                anyhow::bail!(
                    "{}",
                    json!({"code":"STATUS_REQUEST_FAILED", "http_status":status.as_u16(),
                    "repo_id":id, "repository_url":browser_url(&self.base,id),
                    "message":"Cannot read repository logs. Check key validity, permissions and whether this endpoint enables API-key authentication (ApiAuthMiddleware); session-only endpoints cannot serve CLI wait.", "details":safe_error(&body)})
                );
            }
            let raw = body
                .get("data")
                .filter(|v| v.is_object())
                .context("Logs API missing data object")?;
            let label = raw["repo_status"]
                .as_str()
                .unwrap_or("unknown")
                .to_lowercase();
            let state = match label.as_str() {
                "initial" | "submitted" | "processing" => "processing",
                "approved" | "verified" => "ready",
                "rejected" => "rejected",
                _ => "unknown",
            };
            Ok((
                json!({"repo_id":id, "state":state, "repo_status":label,
                "repository_url":browser_url(&self.base, id), "status_url":endpoint,
                "source":{"origin":"remote_git", "commit":null, "note":"Commit is not exposed by the logs API"},
                "metadata_deployment":{"state":"unknown", "origin":"local .verilib"},
                "upload":raw["upload_response"], "atomization":raw["livelog"], "queue_info":raw["queue_info"]}),
                retry,
            ))
        };
        tokio::time::timeout_at(deadline, future)
            .await
            .context("Status request deadline exceeded")?
    }

    pub async fn wait(&self, id: &str, options: &WaitOptions, deadline: Instant) -> Result<Value> {
        let mut last = Value::Null;
        loop {
            if Instant::now() >= deadline {
                return Err(self.wait_error("WAIT_TIMEOUT", id, last));
            }
            let (snapshot, retry) = match self.logs(id, deadline).await {
                Ok(result) => result,
                Err(_) if Instant::now() >= deadline => {
                    return Err(self.wait_error("WAIT_TIMEOUT", id, last))
                }
                Err(error) => return Err(error),
            };
            last = snapshot;
            match last["state"].as_str() {
                Some("ready") => return Ok(last),
                Some("rejected") => return Err(self.wait_error("REPO_REJECTED", id, last)),
                Some("unknown") => return Err(self.wait_error("UNKNOWN_REPO_STATE", id, last)),
                _ => eprintln!(
                    "Repository {id}: {} (source processing; .verilib metadata not deployed)",
                    last["repo_status"]
                ),
            }
            let delay = Duration::from_secs(
                retry
                    .unwrap_or(options.poll_interval)
                    .clamp(1, options.timeout),
            );
            tokio::time::sleep_until((Instant::now() + delay).min(deadline)).await;
        }
    }

    fn wait_error(&self, code: &str, id: &str, last: Value) -> anyhow::Error {
        anyhow::anyhow!(
            "{}",
            json!({"code":code, "repo_id":id,
            "repository_url":browser_url(&self.base,id), "last_status":last,
            "message":"Waiting stopped; no deployment or cancellation was performed. Inspect repo status/logs; reuse this repo ID, do not create another repository."})
        )
    }

    pub async fn deploy(
        &self,
        id: Option<&str>,
        payload: &Value,
        deadline: Instant,
    ) -> Result<super::types::DeployResponse> {
        let path = match id {
            Some(id) => {
                validate_id(id)?;
                format!("/v2/repo/deploy/{id}")
            }
            None => "/v2/repo/deploy".into(),
        };
        if Instant::now() >= deadline {
            anyhow::bail!("Deployment deadline exceeded before sending; no request sent");
        }
        let response = self
            .client
            .post(format!("{}{path}", self.base))
            .header("Authorization", format!("ApiKey {}", self.key))
            .header("Accept", "application/json")
            .json(payload)
            .timeout(deadline.saturating_duration_since(Instant::now()))
            .send()
            .await
            .context(
                "Deploy outcome unknown after request failure; inspect repository before retrying",
            )?;
        let status = response.status();
        let body: Value = response
            .json()
            .await
            .context("Deploy response unreadable; outcome unknown, inspect before retrying")?;
        if status == StatusCode::ACCEPTED {
            anyhow::bail!(
                "{}",
                json!({"code":"DEPLOY_PENDING", "repo_id":id,
                "message":"Deploy accepted, NOT confirmed complete. Inspect status before retrying; do not resend blindly"})
            );
        }
        // Only the explicit legacy guard guarantees the mutation was not applied.
        if status == StatusCode::SERVICE_UNAVAILABLE
            && body["error"] == true
            && body["data"]["message"] == "Atomization in progress"
        {
            anyhow::bail!(
                "{}",
                json!({"code":"REPO_NOT_READY", "repo_id":id, "retryable":true,
                "message":"Deploy blocked by atomization. Run wait-for-ready, then retry deploy. Do not create another repository."})
            );
        }
        if !status.is_success() || body["error"] == true {
            anyhow::bail!(
                "{}",
                json!({"code":"DEPLOY_FAILED", "http_status":status.as_u16(), "details":safe_error(&body)})
            );
        }
        serde_json::from_value(body)
            .context("Invalid deploy response; inspect server state before retrying")
    }

    pub async fn create(&self, payload: &Value, timeout: u64) -> Result<Value> {
        let response = self.client.post(format!("{}/v2/repo/create", self.base))
            .header("Authorization", format!("ApiKey {}", self.key))
            .header("Accept", "application/json").json(payload)
            .timeout(Duration::from_secs(timeout)).send().await
            .context("Create outcome unknown after request failure; inspect existing repositories before retrying")?;
        let status = response.status();
        let body: Value = response
            .json()
            .await
            .context("Create response unreadable; outcome unknown, do not blindly retry")?;
        if !status.is_success() || body["error"] == true {
            anyhow::bail!(
                "{}",
                json!({"code":"CREATE_FAILED", "http_status":status.as_u16(), "details":safe_error(&body)})
            );
        }
        let id = body["data"]["id"]
            .as_u64()
            .map(|v| v.to_string())
            .or_else(|| body["data"]["id"].as_str().map(str::to_owned))
            .context(
                "Create response missing ID; outcome unknown, inspect repositories before retrying",
            )?;
        validate_id(&id)?;
        let mut link = browser_url(&self.base, &id);
        if let (Some(owner), Some(slug)) = (
            body["data"]["owner_username"].as_str(),
            body["data"]["slug"].as_str(),
        ) {
            if !owner.is_empty() && !slug.is_empty() {
                use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
                link = format!(
                    "{}/r/{}/{}",
                    self.base,
                    utf8_percent_encode(owner, NON_ALPHANUMERIC),
                    utf8_percent_encode(slug, NON_ALPHANUMERIC)
                );
            }
        }
        Ok(
            json!({"repo_id":id, "state":"created", "repository_url":link,
            "status_url":format!("{}/v2/repo/logs/{id}", self.base),
            "source":{"origin":"remote_git", "url":payload["url"]},
            "metadata_deployment":{"state":"not_deployed", "origin":"local .verilib"},
            "http_status":status.as_u16(), "accepted_async":status == StatusCode::ACCEPTED,
            "next_action":"Run wait-for-ready before deploy; creation does not mean atomization is complete"}),
        )
    }
}

pub async fn handle_status(
    target_args: RepoTarget,
    options: WaitOptions,
    force_wait: bool,
    json_output: bool,
) -> Result<()> {
    let (id, base) = target(target_args)?;
    let api = Api::authenticated(&base)?;
    let deadline = Instant::now() + Duration::from_secs(options.timeout);
    let value = if options.wait || force_wait {
        api.wait(&id, &options, deadline).await?
    } else {
        api.logs(&id, deadline).await?.0
    };
    render(&value, json_output)
}

pub async fn handle_create(args: RepoCreate, json_output: bool) -> Result<()> {
    metadata::validate(&args.summary, args.description.as_deref())?;
    let git = reqwest::Url::parse(&args.git_url).context("Git URL must be an HTTP(S) URL")?;
    if !matches!(git.scheme(), "http" | "https")
        || git.host_str().is_none()
        || !git.username().is_empty()
        || git.password().is_some()
    {
        anyhow::bail!("Git URL must be HTTP(S) without embedded credentials");
    }
    let config = ProjectConfig::load(Path::new("."))?;
    if config.repo.is_some() {
        anyhow::bail!("This directory already has a repository. Reuse its ID or choose a new directory; refusing duplicate creation");
    }
    let base = resolve_base_url(args.url, None);
    let api = Api::authenticated(&base)?;
    let payload = json!({"url":args.git_url, "summary":args.summary, "description":args.description,
        "language_id":args.language_id, "prooflanguage_id":args.prooflanguage_id,
        "type_id":args.type_id, "verifierversion_id":args.verifierversion_id});
    let deadline = Instant::now() + Duration::from_secs(args.options.timeout);
    let mut result = api.create(&payload, args.options.timeout).await?;
    let id = result["repo_id"].as_str().unwrap().to_owned();
    eprintln!("Repository created: {id}. {}. Source is cloned remotely; .verilib metadata has not been deployed.", result["repository_url"]);
    save_config(&id, &base, true, args.execution_mode.into())
        .with_context(|| format!("Repository {id} was created but config could not be saved. Rebind with init --id {id}; do not recreate"))?;
    if args.options.wait {
        result["readiness"] = api.wait(&id, &args.options, deadline).await?;
    }
    render(&result, json_output)
}
