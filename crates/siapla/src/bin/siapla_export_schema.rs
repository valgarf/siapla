use std::fs;

use siapla::gql::schema;

pub fn main() -> anyhow::Result<()> {
    fs::write("./schema.gql", schema().as_sdl())?;
    Ok(())
}
