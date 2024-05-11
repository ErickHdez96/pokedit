use anyhow::Result;

fn main() -> Result<()> {
    simple_logger::init_with_env().unwrap();

    let mut args = std::env::args();
    let mut bytes = std::fs::read(args.nth(1).unwrap_or(
        "./savs/Pokemon - savs/Pokemon - Emerald Version (USA, Europe).sav".to_string(),
    ))?;
    let game = pokedit_lib::gen3::Game::new(&mut bytes)?;
    println!("Gender: {}", game.trainer().gender()?);
    Ok(())
}
