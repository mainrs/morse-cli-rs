use std::{process::exit, ffi::OsStr};

enum Data {
    Short,
    Long,
}

impl TryFrom<char> for Data {
    type Error = String;

    fn try_from(value: char) -> Result<Self, Self::Error> {
        use Data::*;

        match value {
            '.' => Ok(Short),
            '-' => Ok(Long),
            _ => Err("unknown morse code character: only".to_owned()),
        }
    }
}

type MorseCode = Vec<Data>;

fn main() {
    let args = std::env::args_os();
    if args.len() != 2 {
        eprintln!("Usage: morse-cli <morse-code>");
        exit(1);
    }

    // SAFETY: if-statement above.
    let morse_code = args.into_iter().nth(1).unwrap();
    let morse_code = morse_code.to_string_lossy();
    let morse_code = parser(&morse_code);

    println!("Done.");
}

fn parser(data: &str) -> MorseCode {
    data.chars()
        .into_iter()
        .filter(|c| *c == '-' || *c == '.')
        .map(|c| Data::try_from(c))
        // SAFETY: we filter beforehand and can simply flatten 
        // the whole tree.
        .flatten()
        .collect()
}
