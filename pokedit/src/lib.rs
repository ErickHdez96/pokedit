use std::path::PathBuf;

#[derive(Debug)]
pub struct BinaryConfig {
    pub help: &'static str,
}

impl BinaryConfig {
    fn bail(&self, exit_code: i32) -> ! {
        if exit_code == 0 {
            println!("{}", self.help);
        } else {
            eprintln!("{}", self.help);
        }
        std::process::exit(exit_code)
    }
}

#[derive(Debug)]
pub struct Args {
    pub input: Option<PathBuf>,
}

pub fn parse_args(config: BinaryConfig) -> Args {
    let mut args = Args { input: None };
    let mut env_args = std::env::args_os().skip(1);

    while let Some(arg) = env_args.next() {
        if arg.as_encoded_bytes().starts_with(&[b'-']) {
            let arg = arg.into_string().unwrap_or_else(|_| config.bail(1));
            if arg.starts_with("--") {
                match arg.as_str() {
                    "--help" => {
                        config.bail(0);
                    }
                    _ => config.bail(1),
                }
            } else {
                match arg.as_str() {
                    "-h" => {
                        config.bail(0);
                    }
                    _ => config.bail(1),
                }
            }
        } else {
            if args.input.is_some() {
                config.bail(1);
            }
            args.input = Some(arg.into());
        }
    }

    args
}
