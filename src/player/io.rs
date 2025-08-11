use anyhow::{
    anyhow,
    Result,
    Context,
};
use rodio::Decoder;
use std::{
    fs::File,
    path::Path,
    io::BufReader,
};

pub(super) fn open_decoder(path: &Path) -> Result<Decoder<BufReader<File>>> {
    let f = File::open(path)
        .with_context(|| format!("Failed to open file: {}", path.display()))?;
    let reader = BufReader::new(f);
    Decoder::new(reader).map_err(|e| anyhow!("rodio::Decoder error for {}: {e}", path.display()))
}

