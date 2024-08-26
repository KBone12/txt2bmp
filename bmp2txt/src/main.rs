use std::{
    fs::File,
    io::{BufReader, BufWriter, Read, Write},
    path::PathBuf,
};

use clap::{Parser, ValueEnum};

#[derive(Clone, Copy, Eq, PartialEq, ValueEnum)]
enum BitmapKind {
    Windows,
    Os2,
}

#[derive(Parser)]
#[command(version, about)]
struct Args {
    /// A path to the bmp file
    input: PathBuf,

    /// A path to the text file
    output: Option<PathBuf>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let mut input = BufReader::new(File::open(args.input)?);
    let output: &mut dyn Write = if let Some(path) = args.output {
        &mut BufWriter::new(File::create(path)?)
    } else {
        &mut std::io::stdout()
    };

    let mut file_header = [0; 14];
    input.read_exact(&mut file_header)?;
    let offset = u32::from_le_bytes(file_header[10..14].try_into().unwrap()); // must be safe
    let mut info_header = vec![0; offset as usize - 14];
    input.read_exact(&mut info_header)?; // info header
    let header_size = u32::from_le_bytes(info_header[0..4].try_into().unwrap()); // must be safe
    let width = match header_size {
        12 => u16::from_le_bytes(info_header[4..6].try_into().unwrap()) as usize, // must be safe
        40 => u32::from_le_bytes(info_header[4..8].try_into().unwrap()) as usize, // must be safe
        _ => unimplemented!(
            "BMP header size must be 40 (for Windows Bitmap) or 12 (for OS/2 Bitmap)"
        ),
    };
    let padded_width = ((width + 3) / 4) * 4;
    let mut data = Vec::new();
    input.read_to_end(&mut data)?;
    let text = data
        .chunks(padded_width)
        .flat_map(|chunk| chunk.iter().take(width).map(|byte| *byte as char))
        .collect::<String>();
    write!(output, "{}", text)?;
    output.flush()?;

    Ok(())
}
