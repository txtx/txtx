use flate2::read::GzDecoder;
use reqwest::blocking::Client;
use reqwest::StatusCode;
use serde::Deserialize;
use std::error::Error;
use std::fs;
use std::io::Cursor;
use std::io::Error as IoError;
use std::path::{Path, PathBuf};
use tar::Archive;

const RELEASES_API_BASE: &str =
    "https://api.github.com/repos/solana-foundation/txtx-supervisor-ui/releases";
const RELEASE_ASSET_NAME: &str = "txtx-supervisor-ui-dist.tar.gz";
const VERSION_ENV_VAR: &str = "TXTX_SUPERVISOR_UI_VERSION";

fn main() {
    #[cfg(not(feature = "bypass_supervisor_build"))]
    {
        if let Err(err) = run() {
            panic!("Supervisor build failed: {err}");
        }
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR")?);
    let bundled_dist_dir = out_dir.join("supervisor");
    let cache_root = out_dir.join("supervisor-release-cache");

    println!("cargo:warning=------------ Supervisor Build Script ------------");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed={VERSION_ENV_VAR}");
    println!("cargo:warning=Supervisor build script output dir: {:?}", bundled_dist_dir);

    let requested_version = std::env::var(VERSION_ENV_VAR).ok();
    let client = build_http_client()?;
    let release = resolve_release(&client, requested_version.as_deref())?;
    let release_dir = cache_root.join(release.tag_name.replace('/', "_"));

    if release_dir.exists() {
        println!("cargo:warning=Using cached supervisor UI release {}", release.tag_name);
    } else {
        println!("cargo:warning=Downloading supervisor UI release {}", release.tag_name);
        download_and_extract_release(&client, &release, &release_dir)?;
    }

    if bundled_dist_dir.exists() {
        fs::remove_dir_all(&bundled_dist_dir)?;
    }
    fs::create_dir_all(&bundled_dist_dir)?;
    copy_dir_recursive(&release_dir, &bundled_dist_dir)?;

    println!("cargo:warning=Prepared supervisor UI assets from {}", release.tag_name);

    Ok(())
}

fn build_http_client() -> Result<Client, Box<dyn Error>> {
    Ok(Client::builder()
        .user_agent(concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION")))
        .build()?)
}

fn resolve_release(
    client: &Client,
    requested_version: Option<&str>,
) -> Result<GitHubRelease, Box<dyn Error>> {
    if let Some(version) = requested_version.map(str::trim).filter(|value| !value.is_empty()) {
        return fetch_release_by_tag(client, version);
    }

    fetch_release(client, format!("{RELEASES_API_BASE}/latest"))
}

fn fetch_release_by_tag(client: &Client, version: &str) -> Result<GitHubRelease, Box<dyn Error>> {
    let candidates = if version.starts_with('v') {
        vec![version.to_string()]
    } else {
        vec![version.to_string(), format!("v{version}")]
    };

    let mut last_error = None;

    for tag in candidates {
        match fetch_release(client, format!("{RELEASES_API_BASE}/tags/{tag}")) {
            Ok(release) => return Ok(release),
            Err(err) => last_error = Some(err),
        }
    }

    Err(last_error.unwrap_or_else(|| {
        IoError::other(format!("failed to resolve release for requested version {version}")).into()
    }))
}

fn fetch_release(client: &Client, url: String) -> Result<GitHubRelease, Box<dyn Error>> {
    let response = client.get(url).send()?;
    let status = response.status();

    if status == StatusCode::NOT_FOUND {
        return Err(IoError::new(std::io::ErrorKind::NotFound, "release not found").into());
    }

    let response = response.error_for_status()?;
    Ok(response.json()?)
}

fn download_and_extract_release(
    client: &Client,
    release: &GitHubRelease,
    destination_dir: &Path,
) -> Result<(), Box<dyn Error>> {
    let asset = release
        .assets
        .iter()
        .find(|asset| asset.name == RELEASE_ASSET_NAME)
        .or_else(|| {
            release.assets.iter().find(|asset| {
                asset.name.ends_with(".tar.gz") && asset.name.contains("supervisor-ui-dist")
            })
        })
        .ok_or_else(|| {
            IoError::other(format!(
                "release {} does not contain expected asset {}",
                release.tag_name, RELEASE_ASSET_NAME
            ))
        })?;

    let archive_bytes =
        client.get(&asset.browser_download_url).send()?.error_for_status()?.bytes()?;

    let tmp_dir = destination_dir.with_extension("tmp");
    if tmp_dir.exists() {
        fs::remove_dir_all(&tmp_dir)?;
    }
    fs::create_dir_all(&tmp_dir)?;

    let decoder = GzDecoder::new(Cursor::new(archive_bytes));
    let mut archive = Archive::new(decoder);
    archive.unpack(&tmp_dir)?;

    if destination_dir.exists() {
        fs::remove_dir_all(destination_dir)?;
    }
    fs::rename(&tmp_dir, destination_dir)?;

    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    assets: Vec<GitHubAsset>,
}

#[derive(Debug, Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}
