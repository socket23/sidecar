use std::ops::Deref;

use anyhow::Result;
use once_cell::sync::Lazy;
use regex::Regex;
use reqwest::header::HeaderMap;
use reqwest::header::USER_AGENT;
use reqwest::redirect::Policy;
use reqwest::IntoUrl;
use select::document::Document;
use select::node::Descendants;
use select::node::Node;
use select::predicate::Attr;
use select::predicate::Name;
use select::predicate::Predicate;
use url::Url;

include!(concat!(env!("OUT_DIR"), "/languages.rs"));

// Different websites have different ways of storing the article content.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Language {
    Arabic,
    Russian,
    Dutch,
    German,
    #[default]
    English,
    Spanish,
    French,
    Hebrew,
    Italian,
    Korean,
    Norwegian,
    Persian,
    Polish,
    Portuguese,
    Swedish,
    Hungarian,
    Finnish,
    Danish,
    Chinese,
    Indonesian,
    Vietnamese,
    Swahili,
    Turkish,
    Greek,
    Ukrainian,
    Other(String),
}

pub struct Article {
    pub url: Url,
    pub doc: Document,
    pub content: ArticleContent,
    pub language: Language,
}

#[derive(Debug, Clone)]
pub struct ArticleContent {
    pub title: Option<String>,
    pub icon: Option<String>,
    pub description: Option<String>,
    pub text: Option<String>,
    pub language: Option<Language>,
}

impl ArticleContent {
    fn builder<'a>() -> ArticleContentBuilder<'a> {
        ArticleContentBuilder::default()
    }

    fn into_owned(self) -> ArticleContent {
        ArticleContent {
            title: self.title.into(),
            icon: self.icon.into(),
            description: self.description.into(),
            text: self.text.into(),
            language: self.language,
        }
    }
}

#[derive(Debug, Default)]
struct ArticleContentBuilder<'a> {
    title: Option<&'a str>,
    icon: Option<&'a str>,
    description: Option<&'a str>,
    text: Option<&'a str>,
    language: Option<Language>,
}

impl<'a> ArticleContentBuilder<'a> {
    fn title(mut self, title: &'a str) -> Self {
        self.title = Some(title);
        self
    }

    fn icon(mut self, icon: &'a str) -> Self {
        self.icon = Some(icon);
        self
    }

    fn text(mut self, text: &'a str) -> Self {
        self.text = Some(text);
        self
    }

    fn language(mut self, language: Language) -> Self {
        self.language = Some(language);
        self
    }

    fn description(mut self, description: &'a str) -> Self {
        self.description = Some(description);
        self
    }

    fn build(self) -> ArticleContent {
        ArticleContent {
            title: self.title.map(|s| s.to_owned()),
            icon: self.icon.map(|s| s.to_owned()),
            description: self.description.map(|s| s.to_owned()),
            text: self.text.map(|s| s.to_owned()),
            language: self.language,
        }
    }
}

static RE_BAD_NODES_ATTR: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r###"(?mi)^side$|combx|retweet|mediaarticlerelated|menucontainer|navbar|storytopbar-bucket|utility-bar|inline-share-tools|comment|PopularQuestions|contact|foot(er|note)?|cnn_strycaptiontxt|cnn_html_slideshow|cnn_strylftcntnt|links|meta$|shoutbox|sponsor|tags|socialnetworking|socialNetworking|cnnStryHghLght|cnn_stryspcvbx|^inset$|pagetools|post-attributes|welcome_form|contentTools2|the_answers|communitypromo|runaroundLeft|subscribe|vcard|articleheadings|date|^print$|popup|author-dropdown|socialtools|byline|konafilter|breadcrumbs|^fn$|wp-caption-text|legende|ajoutVideo|timestamp|js_replies|[^-]facebook(-broadcasting)?|google|[^-]twitter|styln-briefing-block|read-more-link|js-body-read-more"###).unwrap()
});
const PUNCTUATION: &str = r#",."'!?&-/:;()#$%*+<=>@[\]^_`{|}~"#;
const ARTICLE_BODY_ATTR: &[(&str, &str); 3] = &[
    ("itemprop", "articleBody"),
    ("data-testid", "article-body"),
    ("name", "articleBody"),
];
const BAD_NODE_NAMES: &[&str; 9] = &[
    "nav",
    "script",
    "style",
    "figcaption",
    "figure",
    "button",
    "summary",
    "aside",
    // astro components - the top level astro-island should suffice
    "astro-island",
];
const ATTR_TO_CHECK: [&str; 3] = ["id", "class", "name"];

struct DefaultDocumentCleaner {
    url: Url,
}

impl DocumentCleaner for DefaultDocumentCleaner {
    fn url(&self) -> &Url {
        &self.url
    }
}

trait DocumentCleaner {
    fn clean_node_text(&self, node: Node) -> String {
        fn recur_text<T: DocumentCleaner + ?Sized>(
            node: Node,
            text: &mut String,
            cleaner: &T,
            mut classes: Vec<String>,
        ) -> bool {
            if cleaner.is_bad_node_name(node) {
                return false;
            }

            // maintain a heirarchy of classes
            classes.extend(extract_language_classes(node));

            let mut text_added = false;
            if cleaner.is_good_node(node) {
                for child in node.children() {
                    if child.is(header()) {
                        let header_level = child
                            .name()
                            .and_then(|tag| tag.strip_prefix('h'))
                            .and_then(|level| level.parse::<usize>().ok())
                            .unwrap_or(1);
                        text.push('\n');
                        text.push('\n');
                        for _ in 0..header_level {
                            text.push('#');
                        }
                        text.push(' ');
                        text.push_str(
                            child
                                .text()
                                .chars()
                                .filter(|c| c.is_ascii() && *c != '\n')
                                .collect::<String>()
                                .trim(),
                        );
                        text.push('\n');
                        text_added |= true;
                    } else if child.is(pre()) {
                        let child_classes = extract_language_classes(child)
                            .chain(
                                child
                                    .children()
                                    .filter(|c| c.is(code()))
                                    .flat_map(extract_language_classes),
                            )
                            .collect::<Vec<_>>();
                        let language = EXT_MAP
                            .keys()
                            .chain(PROPER_CASE_MAP.keys())
                            .find(|&k| child_classes.iter().chain(classes.iter()).any(|c| c == k));
                        text.push_str("```\n");
                        if let Some(language) = language {
                            text.push_str(language);
                        }
                        text.push('\n');
                        text.push_str(&child.text());
                        if !child.text().ends_with('\n') {
                            text.push('\n');
                        }
                        text.push_str("```\n");
                        text_added |= true;
                    } else if child.is(link()) {
                        let link_text = child.text();

                        // check if this link is an anchor link, typically used to share permalinks to headers
                        if link_text.chars().count() == 1 {
                            text_added |= false;
                        } else {
                            let link_href = child.attr("href");
                            if !link_text.trim().is_empty() {
                                if let Some(href) = link_href {
                                    let absolute_href = match Url::parse(href) {
                                        Err(url::ParseError::RelativeUrlWithoutBase) => {
                                            cleaner.url().join(href).ok()
                                        }
                                        _ => None,
                                    };
                                    if let Some(ah) = absolute_href {
                                        text.push_str(&format!(
                                            "[{}]({})",
                                            link_text.trim(),
                                            ah.as_str()
                                        ));
                                    } else {
                                        text.push_str(&link_text);
                                    }
                                } else {
                                    text.push_str(&link_text);
                                }
                            }
                            text_added |= true;
                        }
                    } else if child.is(code()) {
                        text.push('`');
                        text.push_str(&child.text());
                        text.push('`');
                        text_added |= true;
                    } else if child.is(Name("td")) {
                        text.push_str(&child.text());
                        text.push(';');
                        text.push_str(&child.text());
                        text_added |= true;
                    } else if child.is(list()) {
                        text.push('-');
                        text.push(' ');
                        text.push_str(&child.text());
                        text_added |= true;
                    } else {
                        let mut a = String::new();
                        if recur_text(child, &mut a, cleaner, classes.clone()) {
                            text.push_str(&a);
                            text_added |= true;
                        } else if !cleaner.is_bad_node_name(child) {
                            text.push_str(&child.text());
                            text_added |= true;
                        }
                    }

                    if child.is(para()) {
                        text.push('\n');
                    }
                }
            }
            text_added
        }
        
        let mut text = String::new();
        let classes = Vec::new();
        recur_text(node, &mut text, self, classes);
        text
    }

    fn is_bad_node_name(&self, node: Node) -> bool {
        is_bad_node(node)
    }

    fn is_good_node(&self, node: Node) -> bool {
        !has_bad_attr(node)
    }

    fn url(&self) -> &Url;
}

fn is_bad_node(node: Node) -> bool {
    if let Some(n) = node.name() {
        BAD_NODE_NAMES.contains(&n)
    } else {
        false
    }
}

fn has_bad_attr(node: Node) -> bool {
    for attr in ATTR_TO_CHECK.iter() {
        if let Some(id) = node.attr(attr) {
            if RE_BAD_NODES_ATTR.is_match(id) {
                return true;
            }
        }
    }
    false
}

fn extract_language_classes(node: Node) -> impl Iterator<Item = String> + '_ {
    node.attr("class")
        .map(|s| s.split(' '))
        .into_iter()
        .flatten()
        .map(|s| {
            // some heuristic prefixes & suffixes to remove
            s.replace("language", "")
                .replace("source", "")
                .replace("highlight", "")
                .replace('-', "")
        })
}

fn para() -> impl Predicate {
    Name("blockquote")
        .or(Name("dl"))
        .or(Name("div"))
        .or(Name("img"))
        .or(Name("ol"))
        .or(Name("p"))
        .or(Name("pre"))
        .or(Name("table"))
        .or(Name("tr"))
        .or(Name("thead"))
        .or(Name("ul"))
}

fn header() -> impl Predicate {
    Name("h1")
        .or(Name("h2"))
        .or(Name("h3"))
        .or(Name("h4"))
        .or(Name("h5"))
        .or(Name("h6"))
}

fn pre() -> impl Predicate {
    Name("pre")
}

fn code() -> impl Predicate {
    Name("code")
}

fn link() -> impl Predicate {
    Name("a")
}

fn list() -> impl Predicate {
    Name("li")
}

#[derive(Debug, Clone)]
struct ArticleTextNode<'a> {
    inner: Node<'a>,
}

impl<'a> ArticleTextNode<'a> {
    fn new(inner: Node<'a>) -> Self {
        Self { inner }
    }

    fn clean_text(&self, url: &Url) -> String {
        DefaultDocumentCleaner { url: url.clone() }.clean_node_text(self.inner)
    }
}

impl<'a> Deref for ArticleTextNode<'a> {
    type Target = Node<'a>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

// struct ArticleTextNodeExtractor;

// impl ArticleTextNodeExtractor {
//     const MINIMUM_STOPWORD_COUNT: usize = 5;
//     const MAX_DISTANCE_FROM_NODE: usize = 3;

//     fn article_body_predicate() -> for<'r, 's> fn(&'r Node<'s>) -> bool {
//         |node| {
//             for (k, v) in ARTICLE_BODY_ATTR.iter().cloned() {
//                 if Attr(k, v).matches(node) {
//                     return true;
//                 }
//             }
//             false
//         }
//     }

//     fn calculate_best_node(doc: &Document, lang: Language) -> Option<ArticleTextNode> {
//         let mut starting_boost = 1.0;

//         let mut common_best_node = doc.find(
//             Name("article")
//                 .or(Name("main"))
//                 .or(Attr("id", "main"))
//                 .or(Attr("id", "content"))
//                 .or(Attr("id", "doc-content"))
//                 .or(Attr("id", "contents"))
//                 .or(Attr("class", "book-body")),
//         );
//     }
// }
