pub const ADDRESS: &str = "127.0.0.1:8643";
pub const USAGE: &str = "usage: voidwright-server [--bind <address>] [--seed <number>]";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Options {
    pub address: String,
    pub seed: u64,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            address: ADDRESS.to_owned(),
            seed: 1,
        }
    }
}

impl Options {
    pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut options = Self::default();
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--bind" => options.address = args.next().ok_or("--bind needs an address")?,
                "--seed" => {
                    options.seed = args
                        .next()
                        .and_then(|seed| seed.parse().ok())
                        .ok_or("--seed needs a number")?;
                }
                _ => return Err(format!("unknown argument {arg:?}")),
            }
        }
        Ok(options)
    }
}

#[cfg(test)]
mod tests {
    use super::{ADDRESS, Options};

    fn parse(args: &[&str]) -> Result<Options, String> {
        Options::parse(args.iter().map(ToString::to_string))
    }

    #[test]
    fn no_arguments_listen_on_the_local_default_port() {
        assert_eq!(parse(&[]), Ok(Options::default()));
        assert_eq!(Options::default().address, ADDRESS);
        assert_eq!(Options::default().seed, 1);
    }

    #[test]
    fn bind_takes_an_address() {
        let options = parse(&["--bind", "0.0.0.0:9000"]).unwrap();
        assert_eq!(options.address, "0.0.0.0:9000");
    }

    #[test]
    fn seed_takes_a_number() {
        assert_eq!(parse(&["--seed", "42"]).map(|options| options.seed), Ok(42));
        assert!(parse(&["--seed", "many"]).is_err());
        assert!(parse(&["--seed"]).is_err());
    }

    #[test]
    fn an_unknown_argument_is_an_error() {
        assert!(parse(&["--fast"]).is_err());
        assert!(parse(&["--bind"]).is_err());
    }
}
