use std::ffi::OsStr;
use std::path::{
    Path,
    PathBuf
};

#[inline]
pub fn is_audio_file(p: &Path) -> bool {
    let Some(ext) = p.extension().and_then(OsStr::to_str) else {
        return false;
    };
    matches!(ext, _ if ext.eq_ignore_ascii_case("mp3")
        || ext.eq_ignore_ascii_case("flac")
        || ext.eq_ignore_ascii_case("ogg")
        || ext.eq_ignore_ascii_case("opus")
        || ext.eq_ignore_ascii_case("wav")
        || ext.eq_ignore_ascii_case("aiff")
        || ext.eq_ignore_ascii_case("aif")
        || ext.eq_ignore_ascii_case("alac")
        || ext.eq_ignore_ascii_case("m4a")
        || ext.eq_ignore_ascii_case("m4b")
        || ext.eq_ignore_ascii_case("mp4")
        || ext.eq_ignore_ascii_case("aac")
        || ext.eq_ignore_ascii_case("webm"))
}

pub fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"))
}
