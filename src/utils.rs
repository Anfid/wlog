use anyhow::Result;
use console::Term;
use owo_colors::OwoColorize;
use std::str::FromStr;

const MAX_ATTEMPTS: u32 = 3;

pub const TABLE_STYLE: &str = "┃┃━━┣━┿┫│─┼┠┨┯┷┏┓┗┛";

pub fn yn_prompt(msg: &str) -> Result<bool> {
    eprintln!("{msg} [y/N]");
    let term = Term::stdout();
    let mut attempt = 1;
    loop {
        let answer = term.read_char()?;
        match answer {
            'y' | 'Y' => break Ok(true),
            'n' | 'N' | '\n' => break Ok(false),
            unknown => eprintln!(
                "{} {}, press {} to confirm or {} to cancel",
                "Unknown option:".yellow().bold(),
                format!("'{unknown}'").red(),
                "'y'".green(),
                "'n'".green()
            ),
        }
        attempt += 1;
        if attempt > MAX_ATTEMPTS {
            anyhow::bail!("Unable to parse response in {MAX_ATTEMPTS} attempts");
        }
    }
}

pub fn prompt_opt<T>(msg: &str) -> Result<Option<T>>
where
    T: FromStr,
    T::Err: Into<anyhow::Error>,
{
    let stdin = std::io::stdin();

    eprintln!("{msg} (leave empty for none):");
    let mut buffer = String::new();
    stdin.read_line(&mut buffer)?;
    let str = buffer.trim();
    if str.is_empty() {
        Ok(None)
    } else {
        str.parse().map(Some).map_err(Into::into)
    }
}

pub fn prompt<T>(msg: &str) -> Result<T>
where
    T: FromStr,
    T::Err: Into<anyhow::Error>,
{
    let stdin = std::io::stdin();

    let mut buffer = String::new();
    let mut attempt = 1;
    loop {
        eprintln!("{msg}:");
        stdin.read_line(&mut buffer)?;
        let str = buffer.trim();
        if str.is_empty() {
            eprintln!(
                "{} This field can't be empty and must be initialized",
                "Note:".cyan()
            );
        } else {
            match str.parse().map_err(Into::into) {
                Ok(v) => break Ok(v),
                Err(e) => eprintln!("{} Unable to parse: {e}", "Error:".red().bold()),
            }
        }
        attempt += 1;
        if attempt > 3 {
            anyhow::bail!("Unable to parse response in {MAX_ATTEMPTS} attempts");
        }
        eprintln!("{} Attempt {attempt}/{MAX_ATTEMPTS}", "Info:".cyan())
    }
}
