#[derive(Debug, Default)]
pub(crate) struct TagFields {
    pub artist: Option<String>,
    pub album_artist: Option<String>,
    pub album: Option<String>,
    pub title: Option<String>,
    pub track_no: Option<u32>,
    pub disc_no: Option<u32>,
    pub year: Option<u32>,
    pub genre: Option<String>,
}

pub(crate) fn read_tags(path: &std::path::Path) -> Option<TagFields> {
    use audiotags::Tag;

    let tag = Tag::new().read_from_path(path).ok()?;

    let mut tf = TagFields::default();
    tf.title = tag.title().map(str::to_string);
    tf.album = tag.album_title().map(str::to_string);
    tf.artist = tag.artist().map(str::to_string);
    tf.album_artist = tag.album_artist().map(str::to_string);
    tf.genre = tag.genre().map(str::to_string);
    tf.track_no = tag.track_number().map(|n| n as u32);
    tf.disc_no = tag.disc_number().map(|n| n as u32);
    tf.year = tag.year().and_then(|y| if y >= 0 {
        Some(y as u32)
    } else {
        None
    });

    Some(tf)
}
