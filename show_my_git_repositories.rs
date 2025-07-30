#!/usr/bin/env -S cargo +nightly -q -Zscript run --release --manifest-path
---
[dependencies]
clap = { version = "4.5.41", features = ["derive"] }
tokio = { version = "1.47.0", features = ["full"] }
reqwest = { version = "0.12.22", features = ["json"] }
serde = { version = "1.0.219", features = ["derive"] }
serde_json = "1.0.141"
dotenv = "0.15.0"
dialoguer = "0.11.0"
anyhow = "1.0.98"
---

use clap::{Parser, ValueEnum};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use anyhow::bail;

#[derive(ValueEnum, Debug, Clone, Copy)]
enum OwnershipType {
    All,
    Owner,
    Member,
}

#[derive(Parser)]
#[command(name = "show_my_git_repositories")]
#[command(about = "GitHubユーザーのリポジトリ一覧を表示")]
struct Args {
    /// ユーザー名
    /// 
    /// 指定しない場合は .env ファイルを参照する
    /// .env もない場合はプロンプトで聞く
    #[arg(long, short)]
    user_name: Option<String>,

    /// Personal Access Token
    /// 
    /// 指定しない場合は .env ファイルを参照する
    /// .env もない場合はプロンプトで聞く
    #[arg(long, short)]
    personal_access_token: Option<String>,

    /// 取得するリポジトリの最大数
    #[arg(long, default_value = "100")]
    max_repos: u32,

    /// プライベートリポジトリも含める
    #[arg(long, default_value = "false")]
    include_private: bool,

    /// 所有タイプ
    #[arg(long, value_enum, default_value_t = OwnershipType::Owner)]
    ownership_type: OwnershipType,
}

#[derive(Debug, Deserialize, Serialize)]
struct Repository {
    name: String,
    html_url: String,
    description: Option<String>,
    private: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // .envファイルを読み込み
    dotenv::dotenv().ok();

    // ユーザー名を取得
    let username = get_username(args.user_name.as_ref())?;
    
    // Personal Access Tokenを取得
    let token = get_token(args.personal_access_token.as_ref())?;

    // GitHubからリポジトリを取得
    let repositories = fetch_repositories(&username, &token, args).await?;

    // 結果を表示
    display_repositories(&repositories);

    Ok(())
}

fn get_username(arg_username: Option<&String>) -> anyhow::Result<String> {
    if let Some(username) = arg_username {
        return Ok(username.to_string());
    }

    if let Ok(username) = env::var("GITHUB_USERNAME") {
        return Ok(username);
    }

    let input = dialoguer::Input::<String>::new()
        .with_prompt("GitHubユーザー名を入力してください")
        .interact_text()?;
    Ok(input)
}

fn get_token(arg_token: Option<&String>) -> anyhow::Result<String> {
    if let Some(token) = arg_token {
        return Ok(token.to_string());
    }

    if let Ok(token) = env::var("GITHUB_TOKEN") {
        return Ok(token);
    }

    let input = dialoguer::Input::<String>::new()
        .with_prompt("GitHub Personal Access Tokenを入力してください")
        .interact_text()?;
    Ok(input)
}

async fn fetch_repositories(
    username: &str,
    token: &str,
    Args {
        max_repos,
        include_private,
        ownership_type,
        ..
    }: Args,
) -> anyhow::Result<Vec<Repository>> {
    let client = Client::new();
    let mut repositories = Vec::new();
    let mut page = 1;
    let per_page = 100;

    loop {
        let url = format!(
            "https://api.github.com/users/{}/repos",
            username,
        );

        let response = client
            .get(&url)
            .query(&[
                ("type", match ownership_type {
                    OwnershipType::All => "all",
                    OwnershipType::Owner => "owner",
                    OwnershipType::Member => "member",
                }),
                ("sort", "updated"),
                ("direction", "desc"),
                ("page", page.to_string().as_str()),
                ("per_page", per_page.to_string().as_str())
            ])
            .header("Accept", "application/vnd.github.v3+json")
            .header("Authorization", format!("token {}", token))
            .header("User-Agent", "show_my_git_repositories")
            .send()
            .await?;

        if !response.status().is_success() {
            bail!("GitHub API request failed: {}", response.status());
        }

        let repos: Vec<Repository> = response.json().await?;
        
        if repos.is_empty() {
            break;
        }

        for repo in repos {
            if !include_private && repo.private {
                continue;
            }
            repositories.push(repo);
            if repositories.len() >= max_repos as usize {
                break;
            }
        }

        if repositories.len() >= max_repos as usize {
            break;
        }

        page += 1;
    }

    Ok(repositories)
}

fn display_repositories(repositories: &[Repository]) {
    for repo in repositories {
        let description = repo.description
            .as_ref()
            .map(|d| d.as_str())
            .unwrap_or("説明なし");
        
        println!("- ⚙️ [{}]({})", repo.name, repo.html_url);
        println!("    - {}", description);
    }
}
