use anyhow::Result;
use rodio::{OutputStream, Source};
use std::{
    fs::File,
    path::PathBuf,
    thread::sleep,
    time::Duration, f32::consts::PI,
};

struct Args {
    frequency: f32,
    unit: f32,
    morse_code: String,
    outfile: Option<PathBuf>,
}

#[derive(Debug)]
enum MorseCode {
    Dah,
    Dit,
}

#[derive(Debug)]
enum Instruction {
    Morse(MorseCode),
    SymbolSpace,
    LetterSpace,
    WordSpace,
}

impl From<MorseCode> for Instruction {
    fn from(value: MorseCode) -> Self {
        Instruction::Morse(value)
    }
}

impl TryFrom<char> for MorseCode {
    type Error = String;

    fn try_from(value: char) -> Result<Self, Self::Error> {
        use MorseCode::*;

        match value {
            '-' => Ok(Dah),
            '.' => Ok(Dit),
            _ => Err("unprocessable code points".to_owned()),
        }
    }
}

fn main() {
    let args = parse_args().unwrap();
    let morse_code = parse_morse_code(&args.morse_code);

    if args.outfile.is_some() {
        render_audio(&args, &morse_code);
    } else {
        play_audio(&args, &morse_code).expect("failed to render morse code");
    }
}

fn parse_args() -> Result<Args> {
    let mut pargs = pico_args::Arguments::from_env();
    let args = Args {
        frequency: pargs
            .value_from_str(["-f", "--frequency"])
            .unwrap_or_else(|_| 440.0),
        unit: pargs
            .value_from_str(["-u", "--unit"])
            .unwrap_or_else(|_| 0.3),
        morse_code: pargs.free_from_str()?,
        outfile: pargs.opt_value_from_str(["-o", "--outfile"])?,
    };

    let remaining = pargs.finish();
    if !remaining.is_empty() {
        eprintln!("warning: dangling arguments: {:?}", remaining);
    }

    Ok(args)
}

fn parse_morse_code(code: &str) -> Vec<Instruction> {
    let letters = code.split(' '); // [.-..-, .--.-]
    let letters: Vec<Vec<Instruction>> = letters
        .into_iter()
        .map(|word| {
            word.chars()
                .into_iter()
                .map(TryFrom::try_from)
                .flatten()
                .map(Instruction::Morse)
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let number_of_letters = letters.len() - 1;
    let mut res = Vec::new();

    // Iterate over each letter the user inputs.
    for (index, letter) in letters.into_iter().enumerate() {
        // Iterate over each morse code char of said letter.
        // Insert the morse code instruction followed by a
        // symbol pause if another morse code instruction
        // follows.
        let letter_code_len = letter.len() - 1;
        for (index_inner, morse_code) in letter.into_iter().enumerate() {
            res.push(morse_code);

            if index_inner < letter_code_len {
                res.push(Instruction::SymbolSpace);
            }
        }

        // We finished encoding an ASCII letter. Insert a
        // letter space instruction if another morse code
        // instruction follows.
        if index < number_of_letters {
            res.push(Instruction::LetterSpace);
        }
    }

    res
}

fn play_audio(args: &Args, ins: &[Instruction]) -> Result<()> {
    let (_stream, stream_handle) = OutputStream::try_default()?;
    let sink = rodio::Sink::try_new(&stream_handle)?;

    let tone = rodio::source::SineWave::new(args.frequency);
    let dot_duration = Duration::from_millis((args.unit * 1000.) as u64);
    let dot = tone.clone().take_duration(dot_duration);
    let dash = tone.take_duration(dot_duration * 3);

    for is in ins {
        use Instruction::*;

        match is {
            Morse(c) => match c {
                MorseCode::Dit => {
                    sink.append(dot.clone());
                    sink.sleep_until_end();
                }
                MorseCode::Dah => {
                    sink.append(dash.clone());
                    sink.sleep_until_end();
                }
            },
            SymbolSpace => sleep(dot_duration),
            LetterSpace => sleep(dot_duration * 3),
            _ => unreachable!(),
        }
    }

    Ok(())
}

fn render_audio(args: &Args, ins: &[Instruction]) {
    fn wav_sleep(writer: &mut hound::WavWriter<std::io::BufWriter<File>>, samples: u64) {
        for _ in (0..samples).map(|x| x as f32 / 44100.0) {
            writer.write_sample(0).unwrap();
        }
    }
    
    let path = args.outfile.as_deref().unwrap();
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 44100,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).unwrap();

    let dit_samples = spec.sample_rate as f32 / args.unit;
    let dit_samples = dit_samples as u64;
    let dah_samples = dit_samples * 3;

    for is in ins {
        use Instruction::*;

        match is {
            Morse(c) => match c {
                MorseCode::Dit => {
                    for t in (0..dit_samples).map(|x| x as f32 / 44100.0) {
                        let sample = (t * args.frequency * 2.0 * PI).sin();
                        let amplitude = i16::MAX as f32;
                        writer.write_sample((sample * amplitude) as i16).unwrap();
                    }
                }
                MorseCode::Dah => {
                    for t in (0..dah_samples).map(|x| x as f32 / 44100.0) {
                        let sample = (t * args.frequency * 2.0 * PI).sin();
                        let amplitude = i16::MAX as f32;
                        writer.write_sample((sample * amplitude) as i16).unwrap();
                    }
                }
            },
            SymbolSpace => wav_sleep(&mut writer, dit_samples),
            LetterSpace => wav_sleep(&mut writer, dah_samples),
            _ => unreachable!(),
        }
    }
}
