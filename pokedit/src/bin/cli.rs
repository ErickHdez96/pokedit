use std::path::PathBuf;

use anyhow::Result;
use pokedit::{parse_args, BinaryConfig};

const HELP_STR: &str = "
A pokemon save file editor

Usage: pokedit [OPTIONS] FILE

Arguments:
    FILE\tSave file to edit.
";

fn main() -> Result<()> {
    simple_logger::init_with_env().unwrap();

    let args = parse_args(BinaryConfig {
        help: HELP_STR.trim(),
    });
    let save_file_path = args.input.unwrap_or_else(|| {
        PathBuf::from("./savs/Pokemon - Emerald Version (USA, Europe).sav".to_string())
    });
    let mut bytes = std::fs::read(save_file_path)?;
    let game = pokedit_lib::gen3::Game::new_bytes(&mut bytes)?;
    println!("Gender: {}", game.trainer().gender()?);
    println!("Public TrainerId: {}", game.trainer().trainer_id().public);
    println!("Private TrainerId: {}", game.trainer().trainer_id().private);
    println!("Time played: {:?}", game.trainer().time_played());
    println!("Security code: 0x{:08X}", game.trainer().security_key()?);
    println!("Money: {}", game.team_items().money());
    //println!("Setting Money");
    //{
    //    let mut team_items = game.team_items_mut();
    //    let money = team_items.as_data().money() + 1;
    //    team_items.set_money(money);
    //}
    //println!("Money: {}", game.team_items().money());
    //game.save(save_file_path)?;
    Ok(())
}
