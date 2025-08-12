use regex::Regex;
use std::borrow::Cow;
use once_cell::sync::Lazy;

static RE_WS: Lazy<Regex> = Lazy::new(|| Regex::new(r"\s+").unwrap());
static RE_YEAR_PREFIX1: Lazy<Regex> = Lazy::new(|| Regex::new(r"^\s*\d{4}\s*[-_.]\s*").unwrap());
static RE_YEAR_PREFIX2: Lazy<Regex> = Lazy::new(|| Regex::new(r"^\s*[\(\[]\s*\d{4}\s*[\)\]]\s*[-_.]?\s*").unwrap());
static RE_YEAR_SUFFIX: Lazy<Regex> = Lazy::new(|| Regex::new(r"\s*[\(\[]\s*\d{4}\s*[\)\]]\s*$").unwrap());
static RE_TECH: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?x)\s*[\(\[][^)\]]*(?:kHz|Hz|bit|kbps|VBR|CBR|FLAC|ALAC|MP3|AAC|OGG|OPUS|DSD|mono|stereo)[^)\]]*[\)\]]"#).unwrap()
});
static RE_CATALOG_SUFFIX: Lazy<Regex> = Lazy::new(|| Regex::new(r#"(?xi)\s*\[(?:[A-Z]{1,5}\s*)?\d[\d\s\-]*\]\s*$"#).unwrap());

#[inline]
pub(crate) fn normalize_ws(s: &str) -> Cow<'_, str> {
    if !s.contains(['_', ' ', '\t', '\n', '\r']) {
        return Cow::Borrowed(s);
    }
    let tmp = s.replace('_', " ");
    Cow::Owned(RE_WS.replace_all(&tmp, " ").trim().to_string())
}

#[inline]
pub(crate) fn strip_year_prefix(s: &str) -> Cow<'_, str> {
    let t = RE_YEAR_PREFIX1.replace(s, "");
    let u = RE_YEAR_PREFIX2.replace(&t, "");
    let out = u.trim();
    if std::ptr::eq(out, s) {
        Cow::Borrowed(s)
    } else {
        Cow::Owned(out.to_string())
    }
}

#[inline]
pub(crate) fn strip_year_suffix(s: &str) -> Cow<'_, str> {
    let t = RE_YEAR_SUFFIX.replace(s, "");
    let out = t.trim();
    if std::ptr::eq(out, s) {
        Cow::Borrowed(s)
    } else {
        Cow::Owned(out.to_string())
    }
}

#[inline]
pub(crate) fn strip_tech_brackets(s: &str) -> Cow<'_, str> {
    let mut cur = Cow::Borrowed(s);
    loop {
        let replaced = RE_TECH.replace(cur.as_ref(), "");
        if replaced.as_ref() == cur.as_ref() {
            break;
        }
        cur = Cow::Owned(replaced.to_string());
    }
    Cow::Owned(cur.trim().to_string())
}

#[inline]
pub(crate) fn strip_catalog_suffix(s: &str) -> Cow<'_, str> {
    let mut cur = Cow::Borrowed(s);
    loop {
        let replaced = RE_CATALOG_SUFFIX.replace(cur.as_ref(), "");
        if replaced.as_ref() == cur.as_ref() {
            break;
        }
        cur = Cow::Owned(replaced.to_string());
    }
    Cow::Owned(cur.trim().to_string())
}

#[inline]
pub(crate) fn strip_artist_prefix<'a>(album: &'a str, artist: &str) -> Cow<'a, str> {
    for sep in [" - ", " – ", " — "] {
        let pref = format!("{artist}{sep}");
        if album.len() >= pref.len()
            && album[..pref.len()].eq_ignore_ascii_case(&pref) {
            return Cow::Owned(album[pref.len()..].to_string());
        }
    }
    Cow::Borrowed(album)
}

#[inline]
pub(crate) fn clean_album_for_display(raw_album: &str, artist: &str) -> String {
    let s = raw_album.trim();
    let s = strip_year_prefix(s);
    let s = strip_year_suffix(&s);
    let s = strip_tech_brackets(&s);
    let s = strip_catalog_suffix(&s);
    let s = strip_artist_prefix(&s, artist);
    normalize_ws(&s).into_owned()
}
