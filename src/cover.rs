use std::{fs,
    path::{
        Path,
        PathBuf
    }
};

pub fn find_cover_image(dir: &Path) -> Option<PathBuf> {
    const NAMES: &[&str] = &[
        "cover.jpg", "cover.png", "cover.jpeg", "cover.webp",
        "folder.jpg", "folder.png", "folder.jpeg", "folder.webp",
        "front.jpg", "front.png", "front.jpeg", "front.webp",
    ];
    for n in NAMES {
        let p = dir.join(n);
        if p.exists() && p.is_file() {
            return Some(p);
        }
    }
    if let Ok(read) = fs::read_dir(dir) {
        for e in read.flatten() {
            let p = e.path();
            if let Some(ext) = p.extension().and_then(|e| e.to_str()).map(|s| s.to_lowercase()) {
                if matches!(ext.as_str(), "jpg" | "jpeg" | "png" | "webp" | "bmp" | "gif" | "tiff") {
                    return Some(p);
                }
            }
        }
    }
    None
}
