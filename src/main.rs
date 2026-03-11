use std::{cmp::Reverse, fs, path::PathBuf};

use anyhow::Context;
use chrono::{DateTime, Duration, Utc};
use maud::{Render, html};
use reqwest::Url;

const MIN_ENTRIES: usize = 5;
const MIN_PUBLISHED_TIME: Duration = Duration::days(7);

struct Entry {
    title: String,
    url: Url,
    dt: DateTime<Utc>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let out_dir = PathBuf::from(
        std::env::args()
            .nth(1)
            .context("missing output dir argument")?,
    );

    let urls = include_str!("./feeds.txt").trim().lines();
    let entries = get_all_entries(urls).await?;

    let html = html! {
        (maud::DOCTYPE)
        html lang="en" {
            head {
                meta charset="UTF-8";
                meta name="viewport" content="width=device-width, initial-scale=1.0";
                link rel="stylesheet" href="style.css";
                script src="script.js" defer {}
                title { "Blogroll" }
            }

            body { main {
                ul {
                    @for entry in entries {
                        li { (entry) }
                    }
                }
            }}
        }
    };

    fs::create_dir_all(&out_dir)?;
    fs::write(out_dir.join("index.html"), html.into_string())?;
    fs::write(out_dir.join("style.css"), include_str!("./style.css"))?;
    fs::write(out_dir.join("script.js"), include_str!("./script.js"))?;

    Ok(())
}

async fn get_all_entries(urls: impl IntoIterator<Item = &str>) -> anyhow::Result<Vec<Entry>> {
    let client = reqwest::Client::new();
    let mut feeds = futures::future::try_join_all(urls.into_iter().map(async |url| {
        get_url_entries(&client, url)
            .await
            .context(format!("failed at {url}"))
    }))
    .await?
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();
    feeds.sort_by_key(|entry| Reverse(entry.dt));
    Ok(feeds)
}

async fn get_url_entries(
    client: &reqwest::Client,
    url: &str,
) -> anyhow::Result<impl Iterator<Item = Entry>> {
    let xml = client.get(url).send().await?.bytes().await?;
    let rss = feed_rs::parser::parse(&*xml)?;
    let mut entries: Vec<_> = rss
        .entries
        .into_iter()
        .filter_map(|entry| {
            Some(Entry {
                title: entry.title?.content,
                url: Url::parse(&entry.links.first()?.href).ok()?,
                dt: entry.published.or(entry.updated)?,
            })
        })
        .collect();

    entries.sort_by_key(|entry| Reverse(entry.dt));

    let recent_entries = entries
        .into_iter()
        .enumerate()
        .take_while(|(i, entry)| *i < MIN_ENTRIES || Utc::now() - entry.dt < MIN_PUBLISHED_TIME)
        .map(|(_i, entry)| entry);

    Ok(recent_entries)
}

impl Render for Entry {
    fn render(&self) -> maud::Markup {
        // The date shown will be converted to the user's local timezone
        // using a js script.

        // 7 Mar 2026
        let utc_format = self.dt.date_naive().format("%-d %b %Y");
        html! {
            div.eyebrow {
                time datetime=(self.dt.to_rfc3339()) { (utc_format) }
                ", " (self.url.domain().unwrap())
            }
            a.title href=(self.url) {
                (self.title)
            }
        }
    }
}
