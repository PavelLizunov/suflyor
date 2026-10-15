//! Read-only tools a local model may call while it answers (ADR-003,
//! decision 6): the knowledge base and, when the user allows it, the web.
//! No tool writes anything or runs a program.
//!
//! Web search uses DuckDuckGo's lite page: it needs no key. Its full HTML
//! page answers scripted requests with a captcha, the lite page does not.

use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

static AGENT_TOOLS: AtomicBool = AtomicBool::new(false);
static WEB_TOOLS: AtomicBool = AtomicBool::new(false);

/// `enabled`: the local model may call tools. `web`: those tools include web
/// search and page reading (the query leaves the PC).
pub fn set_agent_tools(enabled: bool, web: bool) {
    AGENT_TOOLS.store(enabled, Ordering::Relaxed);
    WEB_TOOLS.store(enabled && web, Ordering::Relaxed);
}

pub(crate) fn agent_enabled() -> bool {
    AGENT_TOOLS.load(Ordering::Relaxed)
}

pub(crate) fn web_enabled() -> bool {
    WEB_TOOLS.load(Ordering::Relaxed)
}

pub const KB_SEARCH: &str = "kb_search";
pub const WEB_SEARCH: &str = "web_search";
pub const OPEN_URL: &str = "open_url";

/// The longest tool result handed back to the model, in characters.
const RESULT_CHARS: usize = 4_000;
const SEARCH_HITS: usize = 5;
const SNIPPET_CHARS: usize = 300;
/// Bytes read from a page before it is cut: enough for the text of an
/// article, small enough to keep a slow page from holding the answer.
const PAGE_BYTES: usize = 512 * 1024;
const WEB_TIMEOUT: Duration = Duration::from_secs(12);
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Suflyor";

/// Tool schemas in the OpenAI `tools` shape. The list is the same for every
/// request of a day, so the engine's prompt cache keeps the prefix.
pub(crate) fn definitions(web: bool, today: &str) -> Value {
    let mut tools = vec![json!({
        "type": "function",
        "function": {
            "name": KB_SEARCH,
            "description": "Search the user's knowledge base (glossary, commands, interview answer patterns) for a term. Returns matching entries.",
            "parameters": {
                "type": "object",
                "properties": { "query": { "type": "string", "description": "A term or short phrase" } },
                "required": ["query"]
            }
        }
    })];
    if web {
        tools.push(json!({
            "type": "function",
            "function": {
                "name": WEB_SEARCH,
                "description": format!("Search the web. Use it for fresh facts, news, prices, versions, or anything you are not sure about. Returns titles, links and short snippets; name the sources you use in the answer. Today is {today}."),
                "parameters": {
                    "type": "object",
                    "properties": { "query": { "type": "string", "description": "What to search for, in the language that fits the topic" } },
                    "required": ["query"]
                }
            }
        }));
        tools.push(json!({
            "type": "function",
            "function": {
                "name": OPEN_URL,
                "description": "Open a web page (a link from web_search) and return its text, up to about 4000 characters.",
                "parameters": {
                    "type": "object",
                    "properties": { "url": { "type": "string", "description": "An http or https link" } },
                    "required": ["url"]
                }
            }
        }));
    }
    Value::Array(tools)
}

/// The argument the UI shows next to a running tool: the query, or the host
/// of a page.
pub(crate) fn status_detail(name: &str, arguments: &str) -> String {
    let args: Value = serde_json::from_str(arguments).unwrap_or(Value::Null);
    if name == OPEN_URL {
        let url = args.get("url").and_then(Value::as_str).unwrap_or_default();
        return reqwest::Url::parse(url)
            .ok()
            .and_then(|u| u.host_str().map(str::to_owned))
            .unwrap_or_default();
    }
    cap_chars(
        args.get("query")
            .and_then(Value::as_str)
            .unwrap_or_default(),
        80,
    )
}

/// Run one tool call. Every failure comes back as text for the model, so it
/// can answer without the tool.
pub(crate) async fn run(name: &str, arguments: &str) -> String {
    let args: Value = match serde_json::from_str(arguments) {
        Ok(v) => v,
        Err(_) => return "error: the arguments must be a JSON object".to_string(),
    };
    let text_arg = |key: &str| {
        args.get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
    };
    let out = match name {
        KB_SEARCH => match text_arg("query") {
            Some(q) => kb_search(&q),
            None => "error: \"query\" is required".to_string(),
        },
        WEB_SEARCH if web_enabled() => match text_arg("query") {
            Some(q) => web_search(&q)
                .await
                .unwrap_or_else(|e| format!("error: web search failed ({e})")),
            None => "error: \"query\" is required".to_string(),
        },
        OPEN_URL if web_enabled() => match text_arg("url") {
            Some(u) => open_url(&u)
                .await
                .unwrap_or_else(|e| format!("error: the page could not be read ({e})")),
            None => "error: \"url\" is required".to_string(),
        },
        _ => format!("error: there is no tool named {name}"),
    };
    cap_chars(&out, RESULT_CHARS)
}

fn kb_search(query: &str) -> String {
    let hits = crate::kb::search(query, 5);
    if hits.is_empty() {
        return "No entries found.".to_string();
    }
    hits.iter()
        .map(|e| format!("## {}\n{}", e.heading, cap_chars(e.body.trim(), 700)))
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Resolves names for the web tools and keeps only public addresses. The
/// connection uses the addresses checked here, so neither a redirect to a
/// name on the home network nor a DNS answer that changes between a check
/// and the connection reaches this PC or the LAN. Address literals never
/// reach a resolver: `public_url` checks those.
struct PublicOnlyResolver;

impl reqwest::dns::Resolve for PublicOnlyResolver {
    fn resolve(&self, name: reqwest::dns::Name) -> reqwest::dns::Resolving {
        let host = name.as_str().to_string();
        Box::pin(async move {
            let addrs = tokio::task::spawn_blocking(move || {
                use std::net::ToSocketAddrs;
                (host.as_str(), 0)
                    .to_socket_addrs()
                    .map(Iterator::collect::<Vec<_>>)
            })
            .await
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })??;
            let public: Vec<std::net::SocketAddr> =
                addrs.into_iter().filter(|a| public_ip(a.ip())).collect();
            if public.is_empty() {
                return Err("the name resolves only to private addresses".into());
            }
            let addrs: reqwest::dns::Addrs = Box::new(public.into_iter());
            Ok(addrs)
        })
    }
}

fn web_client() -> reqwest::Client {
    static CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
    CLIENT
        .get_or_init(|| {
            reqwest::Client::builder()
                .user_agent(USER_AGENT)
                .timeout(WEB_TIMEOUT)
                .dns_resolver(std::sync::Arc::new(PublicOnlyResolver))
                // A redirect may not lead to this PC or the home network.
                .redirect(reqwest::redirect::Policy::custom(|attempt| {
                    if attempt.previous().len() >= 5 || !public_url(attempt.url()) {
                        attempt.stop()
                    } else {
                        attempt.follow()
                    }
                }))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new())
        })
        .clone()
}

async fn web_search(query: &str) -> anyhow::Result<String> {
    let url = reqwest::Url::parse_with_params(
        "https://lite.duckduckgo.com/lite/",
        &[("q", cap_chars(query, 200))],
    )?;
    let resp = web_client().get(url).send().await?;
    if !resp.status().is_success() {
        anyhow::bail!("HTTP {}", resp.status().as_u16());
    }
    let html = resp.text().await?;
    let hits = parse_ddg_lite(&html);
    if hits.is_empty() {
        if html.contains("bots use DuckDuckGo") {
            anyhow::bail!("the search engine asked for a captcha");
        }
        return Ok("No results.".to_string());
    }
    Ok(hits
        .iter()
        .enumerate()
        .map(|(i, h)| format!("{}. {}\n{}\n{}", i + 1, h.title, h.url, h.snippet))
        .collect::<Vec<_>>()
        .join("\n\n"))
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct SearchHit {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

/// Results of DuckDuckGo's lite page: each is an `<a class='result-link'>`
/// followed by a `<td class='result-snippet'>`. Links go through a
/// `/l/?uddg=<target>` redirect; ads go through `y.js` and are skipped.
pub(crate) fn parse_ddg_lite(html: &str) -> Vec<SearchHit> {
    let mut hits = Vec::new();
    let mut rest = html;
    while let Some(pos) = rest.find("class='result-link'") {
        let tag_start = rest[..pos].rfind("<a ").unwrap_or(pos);
        let tag = &rest[tag_start..pos];
        let after = &rest[pos..];
        let Some(gt) = after.find('>') else { break };
        let Some(end) = after.find("</a>") else { break };
        let title = clean_text(&after[gt + 1..end]);
        let href = attr(tag, "href").unwrap_or_default();
        rest = &after[end..];
        let next_link = rest.find("class='result-link'").unwrap_or(rest.len());
        let snippet = rest[..next_link]
            .find("class='result-snippet'")
            .and_then(|s| {
                let cell = &rest[s..next_link];
                let open = cell.find('>')?;
                let close = cell.find("</td>").unwrap_or(cell.len());
                Some(clean_text(&cell[open + 1..close]))
            })
            .unwrap_or_default();
        if href.contains("duckduckgo.com/y.js") {
            continue;
        }
        let url = redirect_target(&href);
        if url.is_empty() || title.is_empty() {
            continue;
        }
        hits.push(SearchHit {
            title,
            url,
            snippet: cap_chars(&snippet, SNIPPET_CHARS),
        });
        if hits.len() == SEARCH_HITS {
            break;
        }
    }
    hits
}

fn attr(tag: &str, name: &str) -> Option<String> {
    for quote in ['"', '\''] {
        let key = format!("{name}={quote}");
        if let Some(s) = tag.find(&key) {
            let v = &tag[s + key.len()..];
            let e = v.find(quote)?;
            return Some(decode_entities(&v[..e]));
        }
    }
    None
}

fn redirect_target(href: &str) -> String {
    let absolute = if href.starts_with("//") {
        format!("https:{href}")
    } else {
        href.to_string()
    };
    let Ok(url) = reqwest::Url::parse(&absolute) else {
        return String::new();
    };
    if url
        .host_str()
        .is_some_and(|h| h.ends_with("duckduckgo.com"))
    {
        return url
            .query_pairs()
            .find(|(k, _)| k == "uddg")
            .map(|(_, v)| v.into_owned())
            .unwrap_or_default();
    }
    absolute
}

async fn open_url(raw: &str) -> anyhow::Result<String> {
    let url = reqwest::Url::parse(raw)?;
    if !public_url(&url) {
        anyhow::bail!("only public http and https pages can be opened");
    }
    // Names are checked by the client's resolver (`PublicOnlyResolver`).
    let mut resp = web_client().get(url).send().await?;
    if !resp.status().is_success() {
        anyhow::bail!("HTTP {}", resp.status().as_u16());
    }
    let kind = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("text/html")
        .to_ascii_lowercase();
    let is_html = kind.contains("html");
    if !(is_html || kind.starts_with("text/") || kind.contains("json") || kind.contains("xml")) {
        anyhow::bail!("not a text page ({kind})");
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = resp.chunk().await? {
        bytes.extend_from_slice(&chunk);
        if bytes.len() >= PAGE_BYTES {
            bytes.truncate(PAGE_BYTES);
            break;
        }
    }
    let body = String::from_utf8_lossy(&bytes);
    let text = if is_html {
        html_to_text(&body)
    } else {
        collapse_ws(&body)
    };
    Ok(if text.is_empty() {
        "The page has no readable text.".to_string()
    } else {
        text
    })
}

/// http(s) to a public host name or address.
pub(crate) fn public_url(url: &reqwest::Url) -> bool {
    if !matches!(url.scheme(), "http" | "https") {
        return false;
    }
    let Some(host) = url.host_str() else {
        return false;
    };
    // The URL parser writes IPv6 hosts in brackets and IPv4 in dotted form.
    if let Ok(ip) = host
        .trim_start_matches('[')
        .trim_end_matches(']')
        .parse::<std::net::IpAddr>()
    {
        return public_ip(ip);
    }
    let d = host.trim_end_matches('.').to_ascii_lowercase();
    !(d == "localhost"
        || d.ends_with(".localhost")
        || d.ends_with(".local")
        || d.ends_with(".lan")
        || d.ends_with(".internal")
        || d.ends_with(".home.arpa")
        || !d.contains('.'))
}

fn public_ip(ip: std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(v4) => {
            let o = v4.octets();
            !(v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                || v4.is_unspecified()
                || v4.is_broadcast()
                // carrier-grade NAT, also Tailscale's range
                || (o[0] == 100 && (64..128).contains(&o[1])))
        }
        std::net::IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                return public_ip(std::net::IpAddr::V4(v4));
            }
            let first = v6.segments()[0];
            !(v6.is_loopback()
                || v6.is_unspecified()
                || (first & 0xfe00) == 0xfc00
                || (first & 0xffc0) == 0xfe80)
        }
    }
}

/// Where a tag that starts at `lt` ends (after its `>`), skipping `>`
/// inside quoted attribute values.
fn tag_end(html: &str, lt: usize) -> usize {
    let mut quote: Option<u8> = None;
    for (i, b) in html.bytes().enumerate().skip(lt + 1) {
        match quote {
            Some(q) if b == q => quote = None,
            Some(_) => {}
            None if b == b'"' || b == b'\'' => quote = Some(b),
            None if b == b'>' => return i + 1,
            None => {}
        }
    }
    html.len()
}

/// `<` starts markup only before a letter, `/`, `!` or `?`; otherwise it is
/// text, as in `x < 5`.
fn starts_markup(rest: &str) -> bool {
    rest.as_bytes()
        .get(1)
        .is_some_and(|b| b.is_ascii_alphabetic() || matches!(b, b'/' | b'!' | b'?'))
}

/// Visible text of an HTML page: scripts, styles and the head are dropped,
/// tags become spaces, entities are decoded and whitespace is collapsed.
pub(crate) fn html_to_text(html: &str) -> String {
    // ASCII lowercasing keeps byte offsets equal between the two strings.
    let lower = html.to_ascii_lowercase();
    let mut out = String::with_capacity(html.len() / 4);
    let mut i = 0;
    while i < html.len() {
        let Some(lt) = lower[i..].find('<').map(|p| p + i) else {
            out.push_str(&html[i..]);
            break;
        };
        out.push_str(&html[i..lt]);
        if !starts_markup(&lower[lt..]) {
            out.push('<');
            i = lt + 1;
            continue;
        }
        if lower[lt..].starts_with("<!--") {
            i = lower[lt..]
                .find("-->")
                .map_or(html.len(), |p| lt + p + 3);
            continue;
        }
        let raw = ["script", "style", "noscript", "svg", "head", "template"]
            .into_iter()
            .find(|tag| {
                lower[lt + 1..].starts_with(tag)
                    && lower[lt + 1 + tag.len()..]
                        .chars()
                        .next()
                        .is_some_and(|c| c == '>' || c == '/' || c.is_ascii_whitespace())
            });
        if let Some(tag) = raw {
            // Everything up to the closing tag is not page text.
            let close = format!("</{tag}");
            i = match lower[lt..].find(&close) {
                Some(p) => tag_end(&lower, lt + p),
                None => html.len(),
            };
            out.push(' ');
            continue;
        }
        let block = [
            "<p", "<br", "<div", "<li", "<h1", "<h2", "<h3", "<h4", "<tr",
        ]
        .iter()
        .any(|t| lower[lt..].starts_with(t));
        i = tag_end(&lower, lt);
        out.push(if block { '\n' } else { ' ' });
    }
    collapse_ws(&decode_entities(&out))
}

fn clean_text(fragment: &str) -> String {
    html_to_text(fragment)
}

/// Spaces collapse to one; line breaks survive as single newlines.
fn collapse_ws(s: &str) -> String {
    s.lines()
        .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn decode_entities(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        let tail = &rest[amp..];
        let window = tail.char_indices().nth(12).map_or(tail.len(), |(i, _)| i);
        let semi = tail[..window].find(';');
        let decoded = semi.and_then(|semi| {
            let name = &tail[1..semi];
            let ch = match name {
                "amp" => Some('&'),
                "lt" => Some('<'),
                "gt" => Some('>'),
                "quot" => Some('"'),
                "apos" | "#39" => Some('\''),
                "nbsp" => Some(' '),
                "mdash" => Some('—'),
                "ndash" => Some('–'),
                "laquo" => Some('«'),
                "raquo" => Some('»'),
                _ => name
                    .strip_prefix("#x")
                    .or_else(|| name.strip_prefix("#X"))
                    .and_then(|h| u32::from_str_radix(h, 16).ok())
                    .or_else(|| name.strip_prefix('#').and_then(|d| d.parse().ok()))
                    .and_then(char::from_u32),
            };
            ch.map(|c| (c, semi))
        });
        match decoded {
            Some((c, semi)) => {
                out.push(c);
                rest = &tail[semi + 1..];
            }
            None => {
                out.push('&');
                rest = &tail[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

fn cap_chars(s: &str, max: usize) -> String {
    match s.char_indices().nth(max) {
        Some((cut, _)) => format!("{}…", &s[..cut]),
        None => s.to_string(),
    }
}

/// Today's UTC date as YYYY-MM-DD, so the model knows what "latest" means.
pub(crate) fn today_utc() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (y, m, d) = civil_from_days((secs / 86_400) as i64);
    format!("{y:04}-{m:02}-{d:02}")
}

/// Days since 1970-01-01 to a proleptic Gregorian date (H. Hinnant).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    const LITE: &str = r#"
<tr><td valign="top">1.&nbsp;</td><td>
  <a rel="nofollow" href="//duckduckgo.com/l/?uddg=https%3A%2F%2Ftokio.rs%2F&amp;rut=52" class='result-link'>Tokio - An asynchronous <b>Rust</b> runtime</a>
</td></tr>
<tr><td>&nbsp;&nbsp;&nbsp;</td><td class='result-snippet'>
  <b>Tokio</b> is a library for writing fast &amp; reliable network applications.
</td></tr>
<tr><td><a rel="nofollow" href="https://duckduckgo.com/y.js?ad_domain=x" class='result-link'>An ad</a></td></tr>
<tr><td class='result-snippet'>Buy now</td></tr>
<tr><td>2.&nbsp;</td><td>
  <a rel="nofollow" href="//duckduckgo.com/l/?uddg=https%3A%2F%2Fdocs.rs%2Ftokio%2F&amp;rut=5f" class='result-link'>tokio - Rust - Docs.rs</a>
</td></tr>
"#;

    #[test]
    fn parses_lite_results_and_skips_ads() {
        let hits = parse_ddg_lite(LITE);
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].title, "Tokio - An asynchronous Rust runtime");
        assert_eq!(hits[0].url, "https://tokio.rs/");
        assert_eq!(
            hits[0].snippet,
            "Tokio is a library for writing fast & reliable network applications."
        );
        assert_eq!(hits[1].url, "https://docs.rs/tokio/");
        assert_eq!(hits[1].snippet, "");
    }

    #[test]
    fn html_to_text_drops_scripts_and_keeps_paragraphs() {
        let html = "<html><head><title>T</title><style>p{}</style></head><body>\
            <script>var x = '<p>no</p>';</script><h1>Title</h1><p>One &amp; two</p>\
            <!-- hidden --><p>Цена: 5&nbsp;₽</p></body></html>";
        assert_eq!(html_to_text(html), "Title\nOne & two\nЦена: 5 ₽");
    }

    #[test]
    fn quoted_angle_brackets_and_text_comparisons_survive() {
        assert_eq!(html_to_text(r#"<p title="a > b">Hello</p>"#), "Hello");
        assert_eq!(html_to_text("<p>if x < 5 then y</p>"), "if x < 5 then y");
        assert_eq!(html_to_text("<div x-data='{ a: 1 > 0 }'>ok</div>"), "ok");
        assert_eq!(html_to_text("a<script>if (a > b) {}</script >b"), "a b");
    }

    #[tokio::test]
    async fn the_web_resolver_refuses_private_names() {
        use reqwest::dns::Resolve;
        let name: reqwest::dns::Name = "localhost".parse().unwrap();
        assert!(PublicOnlyResolver.resolve(name).await.is_err());
    }

    #[test]
    fn only_public_urls_open() {
        let ok = |s: &str| public_url(&reqwest::Url::parse(s).unwrap());
        assert!(ok("https://example.com/page"));
        assert!(ok("http://8.8.8.8/"));
        assert!(!ok("http://127.0.0.1:8080/v1"));
        assert!(!ok("http://localhost/"));
        assert!(!ok("http://192.168.1.10/"));
        assert!(!ok("http://100.77.14.34/"));
        assert!(!ok("http://[::1]/"));
        assert!(!ok("http://router/"));
        assert!(!ok("http://nas.local/"));
        assert!(!ok("file:///C:/Windows/win.ini"));
    }

    #[test]
    fn web_tools_are_offered_only_when_allowed() {
        let names = |v: Value| {
            v.as_array()
                .unwrap()
                .iter()
                .map(|t| t["function"]["name"].as_str().unwrap().to_string())
                .collect::<Vec<_>>()
        };
        assert_eq!(names(definitions(false, "2026-10-15")), [KB_SEARCH]);
        assert_eq!(
            names(definitions(true, "2026-10-15")),
            [KB_SEARCH, WEB_SEARCH, OPEN_URL]
        );
    }

    #[test]
    fn status_detail_shows_query_or_host() {
        assert_eq!(status_detail(WEB_SEARCH, r#"{"query":"курс"}"#), "курс");
        assert_eq!(
            status_detail(OPEN_URL, r#"{"url":"https://docs.rs/tokio"}"#),
            "docs.rs"
        );
        assert_eq!(status_detail(WEB_SEARCH, "not json"), "");
    }

    #[tokio::test]
    async fn bad_arguments_come_back_as_text() {
        assert!(run(KB_SEARCH, "{").await.starts_with("error:"));
        assert!(run(KB_SEARCH, "{}").await.starts_with("error:"));
        assert!(run("shell", r#"{"query":"x"}"#).await.contains("no tool"));
    }

    #[test]
    fn dates_and_caps() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(20_741), (2026, 10, 15));
        assert_eq!(cap_chars("абвгд", 3), "абв…");
        assert_eq!(
            decode_entities("a &#x41; &#66; &bogus; &"),
            "a A B &bogus; &"
        );
    }
}
