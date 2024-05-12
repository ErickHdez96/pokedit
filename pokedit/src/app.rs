use std::path::{Path, PathBuf};

use embedded_graphics::{
    draw_target::DrawTarget,
    geometry::Point,
    mono_font::{iso_8859_14::FONT_10X20, MonoTextStyle},
    pixelcolor::{Rgb888, RgbColor},
    text::Text,
    Drawable,
};
use log::info;
use pokedit_lib::gen3::Game;

use crate::app::input::{Key, KeyEvent};

pub mod input;

pub trait Platform: DrawTarget {
    fn display_width(&self) -> u32;
    fn display_height(&self) -> u32;
    fn flush(&mut self);
    async fn poll(&mut self) -> input::KeyEvent;
}

#[derive(Debug, Default)]
pub struct AppState {
    save_file: PathBuf,
    game: Option<Game<'static>>,
}

#[derive(Debug)]
pub struct App<P> {
    platform: P,
    state: AppState,
}

impl<P> App<P> {
    pub fn new(platform: P) -> Self {
        Self {
            platform,
            state: AppState::default(),
        }
    }

    pub fn open(&mut self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        let path = path.as_ref();
        self.state.save_file = path.into();
        let file = std::fs::read(path)?;
        self.state.game = Some(pokedit_lib::gen3::Game::new_vec(file)?);
        Ok(())
    }

    fn quit(&mut self) -> anyhow::Result<()> {
        if let Some(game) = &mut self.state.game {
            info!("Saving game");
            game.save(&self.state.save_file)?;
        }
        Ok(())
    }
}

impl<P> App<P>
where
    P: Platform + DrawTarget<Color = Rgb888, Error: 'static + Send + Sync + std::error::Error>,
{
    pub async fn run_event_loop(&mut self) -> anyhow::Result<()> {
        'main_loop: loop {
            self.draw()?;

            let event = self.platform.poll().await;
            info!("event: {:?}", event);
            match event {
                KeyEvent::Pressed(Key::Quit) => {
                    break 'main_loop;
                }
                KeyEvent::Pressed(Key::Up) | KeyEvent::Autorepeat(Key::Up) => {
                    if let Some(game) = &mut self.state.game {
                        info!("Increasing money!");
                        let money = game.team_items().money();
                        game.team_items_mut().set_money(money.saturating_add(1));
                    }
                }
                KeyEvent::Pressed(Key::Down) | KeyEvent::Autorepeat(Key::Down) => {
                    if let Some(game) = &mut self.state.game {
                        info!("Increasing money!");
                        let money = game.team_items().money();
                        game.team_items_mut().set_money(money.saturating_sub(1));
                    }
                }
                _ => {}
            }
        }

        self.quit()
    }

    fn draw(&mut self) -> anyhow::Result<()> {
        let width = self.platform.display_width() as i32;
        let height = self.platform.display_height() as i32;
        self.platform.clear(Rgb888::WHITE)?;

        if let Some(game) = &self.state.game {
            let money = game.team_items().money().to_string();
            let text = Text::with_alignment(
                &money,
                Point::new(width / 2, height / 2),
                MonoTextStyle::new(&FONT_10X20, Rgb888::BLACK),
                embedded_graphics::text::Alignment::Center,
            );
            text.draw(&mut self.platform)?;
        }

        self.platform.flush();
        Ok(())
    }
}
