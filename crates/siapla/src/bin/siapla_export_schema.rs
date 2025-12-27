use std::fs;

use siapla::gql::schema;

pub fn main() -> anyhow::Result<()> {
    fs::write("./schema.graphql", schema().as_sdl())?;
    Ok(())
}
