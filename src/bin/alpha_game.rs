use alpha::{Engine, Game};

fn main() -> anyhow::Result<()> {
    let mut engine = Engine::<Game>::init()?;
    engine.run()?;

    Ok(())
}
