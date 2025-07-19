use std::error::Error;
use std::fs;

use clap::Parser;

fn curl_image(url: &str, file_name: &str) -> Result<(), Box<dyn Error>> {
    std::process::Command::new("curl")
        .arg("-L")
        .arg(url)
        .arg("-o")
        .arg(file_name)
        .output()?;

    Ok(())
}

fn fetch_url(url: &str) -> Result<reqwest::blocking::Response, reqwest::Error> {
    // The user agent
    let user_agent = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36 Edg/134.0.0.0";
    let client = reqwest::blocking::ClientBuilder::new()
        .user_agent(user_agent)
        .build()?;
    let resp = client.get(url).send()?;
    Ok(resp)
}

fn save_text_to_file(file_name: &str, content: &str) -> Result<(), Box<dyn Error>> {
    fs::write(file_name, content)?;
    Ok(())
}

fn get_html(
    url: &str,
    return_html: bool,
    file_name: Option<&str>,
) -> Result<String, Box<dyn Error>> {
    let text = fetch_url(url)?.text()?;

    println!("{}", file_name.unwrap());
    if let Some(file_name) = file_name {
        save_text_to_file(file_name, &text)?;
    }

    if return_html {
        Ok(text)
    } else {
        Ok("ok".to_string())
    }
}

fn get_info(
    info_file_path: &str,
    html: &str,
    url: &str,
    num_img: usize,
    info_re: &regex::Regex,
) -> Result<(), Box<dyn Error>> {
    let mut info = String::new();
    info.push_str(format!("URL: {url}\n\n").as_str());

    if let Some(caps) = info_re.captures(html) {
        info.push_str(format!("Info: {}\n\n", &caps[2]).as_str());
    }

    info.push_str(format!("Number of images found: {num_img}").as_str());

    fs::write(info_file_path, info)?;

    Ok(())
}

fn get_links(re: &regex::Regex, html: &str) -> Vec<String> {
    re.captures_iter(html)
        .map(|c| c[0].to_string())
        .collect::<Vec<String>>()
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Output directory <output>/<name>
    #[arg(short, long, default_value = "houses")]
    output: String,

    #[arg(short, long)]
    name: String,

    #[arg(long)]
    url: String,
}

fn main() {
    let args = Args::parse();

    let html_file_path: &str = &format!("{}/{}/www.html", args.output, args.name);
    let info_file_path = &format!("{}/{}/info.txt", args.output, args.name);
    let base_dir = format!("{}/{}", args.output, args.name);

    if !std::path::Path::new(&format!("{base_dir}/images")).exists() {
        std::fs::create_dir_all(format!("{base_dir}/images")).expect("Unable to create directory");
    }

    let html = if args.url.contains("http") {
        match get_html(&args.url, true, Some(html_file_path)) {
            Ok(html) => html,
            Err(e) => {
                println!("Unable to get html: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        match fs::read_to_string(&args.url) {
            Ok(html) => {
                std::fs::rename(&args.url, html_file_path).expect("Unable to rename file");
                html
            }
            Err(e) => {
                println!("Unable to read html file: {}", e);
                std::process::exit(1);
            }
        }
    };

    lazy_static::lazy_static! {
        static ref COMPASS_LINKS_RE: regex::Regex = regex::Regex::new(r"[a-zA-Z/\d_\.:]*origin\.webp").unwrap();
        static ref ZILLOW_LINKS_RE: regex::Regex = regex::Regex::new(r"https://photos.zillowstatic.com/fp/[\w\d]*-uncropped_scaled_within_1536_1152\.jpg").unwrap();
    }

    let links = if args.url.contains("compass") {
        get_links(&COMPASS_LINKS_RE, &html)
    } else if args.url.contains("zillow") {
        get_links(&ZILLOW_LINKS_RE, &html)
    } else {
        println!("Unknown website");
        std::process::exit(1);
    };

    if args.url.contains("compass") {
        let info_re =
            regex::Regex::new(r"(</span>\.\.\.<span class=.[\s\w-]*.>)(.*)(</span></div><button)")
                .unwrap();
        get_info(info_file_path, &html, &args.url, links.len(), &info_re)
            .expect("Unable to get info");
    } else if args.url.contains("zillow") {
        let info_re =
            regex::Regex::new(r"(\\.description\\.:\\.)(.*)(\\.,\\.whatILove\\.)").unwrap();
        get_info(info_file_path, &html, &args.url, links.len() / 2, &info_re)
            .expect("Unable to get info");
    } else {
        println!("Unknown website");
        std::process::exit(1);
    };

    let mut images_seen: Vec<String> = Vec::new();
    let mut i = 1;
    let mut rng = rand::rng();
    for link in links {
        if images_seen.contains(&link) {
            continue;
        } else {
            println!("Downloading image: {}", link);
            images_seen.push(link.clone());
        }

        let file_path = format!("{}/images/{}-{}.jpg", base_dir, args.name, i);

        if std::path::Path::new(&file_path).exists() {
            i += 1;
            continue;
        }

        curl_image(&link, &file_path).expect("Unable to download image");

        let sleep_time = rand::Rng::random_range(&mut rng, 2..7);
        std::thread::sleep(std::time::Duration::from_secs(sleep_time));
        i += 1;
    }
}
