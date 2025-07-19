#![warn(
    clippy::all,
    unsafe_code,
    unused_extern_crates,
    unused_import_braces,
    unused_qualifications,
    clippy::pedantic,
    missing_debug_implementations,
    trivial_casts,
    trivial_numeric_casts
)]

use anyhow::{Context, Result};
use clap::Parser;
use once_cell::sync::Lazy;
use regex::Regex;
use reqwest::Client;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::fs;

static IMAGE_LINK_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#""(https://[^"]*?origin\.webp)""#).unwrap());
static INFO_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?s)<span>Description</span>.*?<div class="[^"]*">(.*?)</div>"#).unwrap()
});

/// Command-line arguments structure.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The URL of the page to scrape.
    #[arg(long)]
    url: String,

    /// The name to use for the output subdirectory.
    #[arg(short, long)]
    name: String,

    /// Base output directory.
    #[arg(short, long, default_value = "scraped_data")]
    output: String,

    /// Skip downloading images.
    #[arg(long)]
    skip_images: bool,

    /// Delay in seconds between each download.
    #[arg(long, default_value_t = 2)]
    delay: u64,
}

/// Fetches HTML from a URL or reads it from a local file.
async fn get_html(client: &Client, url_or_path: &str) -> Result<String> {
    if url_or_path.starts_with("http") {
        client
            .get(url_or_path)
            .send()
            .await
            .context("Failed to send request")?
            .text()
            .await
            .context("Failed to read response text")
    } else {
        fs::read_to_string(url_or_path)
            .await
            .context("Failed to read HTML from file")
    }
}

/// Extracts unique image links from the HTML content.
fn extract_unique_image_links(html: &str) -> Vec<String> {
    IMAGE_LINK_RE
        .captures_iter(html)
        .map(|cap| cap[1].to_string())
        .collect::<HashSet<_>>() // Use a HashSet to automatically handle duplicates
        .into_iter()
        .collect()
}

/// Downloads a single file from a URL to a specified path.
async fn download_file(client: &Client, url: &str, path: &Path) -> Result<()> {
    println!("Downloading {url}...");
    let response = client.get(url).send().await?.error_for_status()?;
    let content = response.bytes().await?;
    fs::write(path, &content)
        .await
        .with_context(|| format!("Failed to write to {}", path.display()))?;
    println!(" -> Saved to {}", path.display());
    Ok(())
}

/// Saves extracted metadata to an `info.txt` file.
async fn save_metadata(html: &str, url: &str, num_images: usize, out_dir: &Path) -> Result<()> {
    let mut info = format!("URL: {url}\n\n");
    if let Some(caps) = INFO_RE.captures(html) {
        if let Some(desc) = caps.get(1) {
            info.push_str(&format!("Info: {}\n\n", desc.as_str().trim()));
        }
    }
    info.push_str(&format!("Number of unique images found: {num_images}"));

    let info_path = out_dir.join("info.txt");
    fs::write(&info_path, info)
        .await
        .context("Failed to write info file")?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let base_dir = PathBuf::from(&args.output).join(&args.name);
    let images_dir = base_dir.join("images");
    fs::create_dir_all(&images_dir)
        .await
        .context("Failed to create output directories")?;

    let client = Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36")
        .timeout(Duration::from_secs(30))
        .build()?;

    println!("Fetching HTML from {}...", &args.url);
    let html = get_html(&client, &args.url).await?;
    let html_path = base_dir.join("page.html");
    fs::write(&html_path, &html)
        .await
        .context("Failed to save HTML file")?;

    let image_links = extract_unique_image_links(&html);
    save_metadata(&html, &args.url, image_links.len(), &base_dir).await?;
    println!("Found {} unique images.", image_links.len());

    if args.skip_images {
        println!("--skip-images flag is set, skipping download.");
    } else if !image_links.is_empty() {
        println!("Downloading images sequentially...");
        let total_links = image_links.len();
        for (i, link) in image_links.iter().enumerate() {
            let file_path = images_dir.join(format!("{}-{}.webp", args.name, i + 1));
            if let Err(e) = download_file(&client, link, &file_path).await {
                eprintln!("Error downloading {link}: {e:?}");
            }

            if i < total_links - 1 {
                println!("Waiting for {} seconds... ⏳", args.delay);
                tokio::time::sleep(Duration::from_secs(args.delay)).await;
            }
        }
    }

    println!(
        "\nScraping complete! ✨\nData saved in: {}",
        base_dir.display()
    );
    Ok(())
}
