use std::borrow::Cow;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Target<'a> {
    Symbol(Literal<'a>),
    Content(Literal<'a>),
}

#[derive(Debug, PartialEq, Eq, Clone, Hash, serde::Serialize, serde::Deserialize)]
pub enum Literal<'a> {
    Plain(Cow<'a, str>),
    Regex(Cow<'a, str>),
}

#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub struct Query<'a> {
    pub open: Option<bool>,
    pub case_sensitive: Option<bool>,
    pub global_regex: Option<bool>,

    pub org: Option<Literal<'a>>,
    pub repo: Option<Literal<'a>>,
    pub path: Option<Literal<'a>>,
    pub lang: Option<Cow<'a, str>>,
    pub branch: Option<Literal<'a>>,
    pub target: Option<Target<'a>>,
}
