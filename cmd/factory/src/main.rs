use anyhow::{bail, Context, Result};
use base64::Engine;
use clap::{Parser, Subcommand};
use kube::Config;
use secrecy::ExposeSecret;
use serde_json::json;
use std::env;

/// Factory — a k8s-native AI software factory
#[derive(Parser, Debug)]
#[command(name = "factory", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,

    /// Kubeconfig path
    #[arg(long, global = true)]
    kubeconfig: Option<String>,

    /// Namespace (default: factory)
    #[arg(long, global = true, default_value = "factory")]
    namespace: String,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Submit a new factory job
    Submit {
        /// Task description
        #[arg(long)]
        task: String,

        /// Repository URL (git@... or file://...)
        #[arg(long)]
        repo: String,

        /// Git branch (default: main)
        #[arg(long, default_value = "main")]
        branch: Option<String>,

        /// Git ref (commit SHA, tag, or PR ref) — overrides branch
        #[arg(long, default_value = "")]
        git_ref: String,

        /// Model name
        #[arg(long)]
        model: String,

        /// Model API key (or prompt if not provided)
        #[arg(long)]
        model_api_key: Option<String>,

        /// Model endpoint (or prompt if not provided)
        #[arg(long)]
        model_endpoint: Option<String>,

        /// Stage names to run (comma-separated, default: plan,code,test,review)
        #[arg(long, default_value = "plan,code,test,review")]
        stages: String,
    },

    /// Show the status of a factory job
    Status {
        /// Factory job name
        name: String,
    },

    /// Install the factory (CRD + controller)
    Install,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Wire up --kubeconfig: set KUBECONFIG env var before loading config
    if let Some(ref path) = cli.kubeconfig {
        env::set_var("KUBECONFIG", path);
    }

    match cli.command {
        Command::Submit {
            task,
            repo,
            branch,
            git_ref,
            model,
            model_api_key,
            model_endpoint,
            stages,
        } => {
            let branch = branch.unwrap_or_else(|| String::from("main"));
            cmd_submit(
                &cli.namespace,
                &task,
                &repo,
                &branch,
                &git_ref,
                &model,
                &model_api_key,
                &model_endpoint,
                &stages,
            )
            .await
        }
        Command::Status { name } => cmd_status(&cli.namespace, &name).await,
        Command::Install => cmd_install().await,
    }
}

async fn cmd_submit(
    namespace: &str,
    task: &str,
    repo: &str,
    branch: &str,
    git_ref: &str,
    model: &str,
    model_api_key: &Option<String>,
    model_endpoint: &Option<String>,
    stages: &str,
) -> Result<()> {
    let (cluster_url, auth_header) = get_k8s_info().await?;

    // Get or prompt for secrets
    let api_key = get_or_prompt(model_api_key, "Model API key")?;
    let endpoint = get_or_prompt(model_endpoint, "Model endpoint")?;

    // Create model secret
    let secret_name = format!("factory-model-{}", uuid::Uuid::new_v4());
    let secret_body = json!({
        "apiVersion": "v1",
        "kind": "Secret",
        "metadata": {
            "name": secret_name,
            "namespace": namespace
        },
        "type": "Opaque",
        "data": {
            "api-key": base64::engine::general_purpose::STANDARD.encode(&api_key),
            "endpoint": base64::engine::general_purpose::STANDARD.encode(&endpoint)
        }
    });
    k8s_apply(&cluster_url, &auth_header, "v1", "Secret", None, &secret_name, &secret_body).await?;

    // Parse stages
    let stage_list: Vec<String> = stages.split(',').map(|s| s.trim().to_string()).collect();

    // Create Factory CR
    let factory_name = format!("factory-{}", uuid::Uuid::new_v4());

    let stages_json: Vec<serde_json::Value> = stage_list.iter().map(|s| json!(s)).collect();

    // Build repo spec — include ref if provided
    let mut repo_obj = json!({
        "url": repo,
        "branch": branch
    });
    if !git_ref.is_empty() {
        repo_obj["ref"] = json!(git_ref);
    }

    let body = json!({
        "apiVersion": "factory.earendil.works/v1",
        "kind": "Factory",
        "metadata": {
            "name": factory_name.clone(),
            "namespace": namespace
        },
        "spec": {
            "model": {
                "apiKeySecretRef": {
                    "name": secret_name,
                    "key": "api-key"
                },
                "endpointSecretRef": {
                    "name": secret_name,
                    "key": "endpoint"
                },
                "model": model
            },
            "repo": repo_obj,
            "task": task,
            "stages": stages_json,
            "resources": {
                "cpu": "1",
                "memory": "4Gi"
            }
        }
    });

    k8s_apply(
        &cluster_url,
        &auth_header,
        "factory.earendil.works",
        "Factory",
        Some(namespace),
        &factory_name,
        &body,
    )
    .await?;

    println!("Factory submitted: {}", factory_name);
    println!("Phase: Pending");
    println!("Run 'factory status {}' to check progress.", factory_name);

    Ok(())
}

async fn cmd_status(namespace: &str, name: &str) -> Result<()> {
    let (cluster_url, auth_header) = get_k8s_info().await?;

    let url = format!(
        "{}/apis/factory.earendil.works/v1/namespaces/{}/factories/{}",
        cluster_url, namespace, name
    );

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .header("Authorization", &auth_header)
        .header("Content-Type", "application/json")
        .send()
        .await
        .context("Failed to send request to k8s API")?;

    let status_code = response.status();
    let body = response
        .text()
        .await
        .context("Failed to read response body")?;

    if !status_code.is_success() {
        bail!("Failed to get factory status: {} — {}", status_code, body);
    }

    let factory: serde_json::Value =
        serde_json::from_str(&body).context("Failed to parse factory response")?;

    let phase = factory
        .get("status")
        .and_then(|s| s.get("phase"))
        .and_then(|p| p.as_str())
        .unwrap_or("Unknown");

    println!("NAME              PHASE     AGE");
    println!(
        "{}\t{}\t{}",
        name,
        phase,
        factory
            .get("metadata")
            .and_then(|m| m.get("creationTimestamp"))
            .and_then(|t| t.as_str())
            .unwrap_or("?")
    );

    // Show stage statuses
    if let Some(stages) = factory
        .get("status")
        .and_then(|s| s.get("stages"))
        .and_then(|s| s.as_array())
    {
        println!("\nSTAGES:");
        for stage in stages {
            let stage_name = stage["name"].as_str().unwrap_or("?");
            let stage_phase = stage["phase"].as_str().unwrap_or("?");
            let error = stage.get("error").and_then(|e| e.as_str()).unwrap_or("");
            println!("  {} — {}", stage_name, stage_phase);
            if !error.is_empty() {
                println!("    Error: {}", error);
            }
        }
    }

    Ok(())
}

async fn cmd_install() -> Result<()> {
    let (cluster_url, auth_header) = get_k8s_info().await?;

    // Create namespace
    let ns_body = json!({
        "apiVersion": "v1",
        "kind": "Namespace",
        "metadata": { "name": "factory" }
    });
    match k8s_apply(&cluster_url, &auth_header, "v1", "Namespace", None, "factory", &ns_body).await {
        Ok(_) => println!("Namespace 'factory' created/updated"),
        Err(e) if e.to_string().contains("AlreadyExists") => {
            println!("Namespace 'factory' already exists");
        }
        Err(e) => bail!("Failed to create namespace: {}", e),
    }

    // Apply CRD
    let crd_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../deploy/crd.yaml");
    let crd_yaml = std::fs::read_to_string(crd_path).context("Failed to read CRD YAML")?;

    let crd: serde_json::Value =
        serde_yaml::from_str(&crd_yaml).context("Failed to parse CRD YAML")?;
    let crd_name = crd["metadata"]["name"]
        .as_str()
        .context("CRD missing metadata.name")?;

    match k8s_apply(
        &cluster_url,
        &auth_header,
        "apiextensions.k8s.io",
        "CustomResourceDefinition",
        None,
        crd_name,
        &crd,
    )
    .await
    {
        Ok(_) => println!("CRD applied: {}", crd_name),
        Err(e) if e.to_string().contains("AlreadyExists") => {
            println!("CRD already exists: {}", crd_name);
        }
        Err(e) => bail!("Failed to apply CRD: {}", e),
    }

    // Apply controller
    let controller_path =
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../deploy/k8s/controller.yaml");
    let controller_yaml = std::fs::read_to_string(controller_path)
        .context("Failed to read controller YAML. Deploy manually first.")?;

    let resources: Vec<serde_yaml::Value> =
        serde_yaml::from_str(&controller_yaml).context("Failed to parse controller YAML")?;

    for resource in resources.iter() {
        let kind = resource["kind"].as_str().unwrap_or("Unknown");
        let name = resource["metadata"]["name"].as_str().unwrap_or("unknown");
        let ns = resource["metadata"]["namespace"].as_str().unwrap_or("factory");

        match k8s_apply(
            &cluster_url,
            &auth_header,
            "apps/v1",
            kind,
            Some(ns),
            name,
            &serde_json::to_value(&resource)?,
        )
        .await
        {
            Ok(_) => println!("Applied: {}/{}", kind, name),
            Err(e) if e.to_string().contains("AlreadyExists") => {
                println!("Already exists: {}/{}", kind, name);
            }
            Err(e) => bail!("Failed to apply {}/{}: {}", kind, name, e),
        }
    }

    println!(
        "\nFactory installed. Run 'kubectl -n factory logs -l app=factory-ctrl' to watch."
    );
    Ok(())
}

fn get_or_prompt(value: &Option<String>, prompt: &str) -> Result<String> {
    if let Some(v) = value {
        return Ok(v.clone());
    }
    if let Ok(env_val) = env::var(prompt.to_lowercase().replace(' ', "_")) {
        return Ok(env_val);
    }
    println!("{} (leave empty to skip):", prompt);
    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .context("Failed to read input")?;
    let input = input.trim().to_string();
    if input.is_empty() {
        bail!("{} is required", prompt);
    }
    Ok(input)
}

async fn get_k8s_info() -> Result<(String, String)> {
    let config = Config::infer()
        .await
        .context("Failed to read kubeconfig (try setting KUBECONFIG env var)")?;

    let cluster_url = config.cluster_url.to_string();

    // Get auth header
    let auth_header = if let Some(token) = &config.auth_info.token {
        format!("Bearer {}", token.expose_secret())
    } else if config.auth_info.client_certificate_data.is_some() {
        // Client cert auth — store cert for later use
        // For now, warn and use token auth as fallback
        eprintln!(
            "Warning: client certificate auth detected but not yet supported. \
             Using token auth fallback."
        );
        "Bearer placeholder".to_string()
    } else {
        // Exec auth or other auth method — need to handle in production
        eprintln!(
            "Warning: exec/auth plugin detected but not yet supported. \
             Ensure your kubectl config uses token auth."
        );
        "Bearer placeholder".to_string()
    };

    Ok((cluster_url, auth_header))
}

/// k8s_apply either creates (POST) or applies (PATCH) a resource.
/// On AlreadyExists, returns an error containing "AlreadyExists" so callers can handle it.
async fn k8s_apply(
    cluster_url: &str,
    auth_header: &str,
    api_version: &str,
    kind: &str,
    namespace: Option<&str>,
    name: &str,
    body: &serde_json::Value,
) -> Result<()> {
    let url = match namespace {
        Some(ns) => format!(
            "{}/apis/{}/{}/{}",
            cluster_url, api_version, ns, name
        ),
        None => format!(
            "{}/apis/{}/{}/{}",
            cluster_url, api_version, kind, name
        ),
    };

    let client = reqwest::Client::new();
    let response = client
        .patch(&url)
        .header("Authorization", auth_header)
        .header("Content-Type", "application/strategic-merge-patch+json")
        .json(&body)
        .send()
        .await
        .context("Failed to send request to k8s API")?;

    let status = response.status();
    let body = response
        .text()
        .await
        .context("Failed to read response body")?;

    if status.is_success() {
        Ok(())
    } else if status == 409 {
        bail!("AlreadyExists: {}/{}", kind, name);
    } else {
        bail!(
            "Failed to apply {}/{}: {} — {}",
            kind,
            name,
            status,
            body
        );
    }
}
