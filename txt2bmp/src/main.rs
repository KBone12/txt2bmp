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
    /// A path to the text file ('-' for stdin)
    input: PathBuf,

    /// A path to the bmp file
    output: PathBuf,

    #[arg(long)]
    kind: Option<BitmapKind>,

    /// Image width
    #[arg(long)]
    width: Option<u32>,

    /// Upside down
    #[arg(long)]
    inverted: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let input: &mut dyn Read = if args.input.to_str().map(|path| path.trim() == "-") == Some(true) {
        &mut std::io::stdin()
    } else {
        &mut BufReader::new(File::open(args.input)?)
    };
    let mut output = BufWriter::new(File::create(args.output)?);

    let (data, width, height) = {
        let mut buf = Vec::new();
        input.read_to_end(&mut buf)?;
        let (width, height) = match args.width {
            Some(0) | None => (buf.len() as _, 1),
            Some(width) => (width, (buf.len() as u32 + width - 1) / width),
        };
        if (buf.len() as u32) < width * height {
            buf.resize_with((width * height) as _, || 0);
        }
        (buf, width, height)
    };
    if height > u16::max_value() as u32 {
        return Err("The height must be smaller than 2^16")?;
    }
    if (width > u16::max_value() as u32 || height > u16::max_value() as u32)
        && args.kind == Some(BitmapKind::Os2)
    {
        return Err("The 32bit width/height is not supported in OS/2 Bitmap")?;
    }
    let kind = args.kind.unwrap_or(
        if width > u16::max_value() as u32 || height > u16::max_value() as u32 {
            BitmapKind::Windows
        } else {
            BitmapKind::Os2
        },
    );
    let offset = 14
        + match kind {
            BitmapKind::Windows => 40 + 4,
            BitmapKind::Os2 => 12 + 3,
        };
    let size = offset + ((width + 3) / 4) * 4 * height;
    output.write(&[0x42, 0x4D])?; // "BM"
    output.write(&size.to_le_bytes())?;
    output.write(&[0; 4])?; // reserved
    output.write(&offset.to_le_bytes())?;
    match kind {
        BitmapKind::Windows => {
            output.write(&40u32.to_le_bytes())?;
            output.write(&width.to_le_bytes())?;
            output.write(
                &(if args.inverted {
                    -(height as i32)
                } else {
                    height as i32
                })
                .to_le_bytes(),
            )?;
            output.write(&1u16.to_le_bytes())?; // planes
            output.write(&1u16.to_le_bytes())?; // data size per pixel
            output.write(&0u32.to_le_bytes())?; // compression == no
            output.write(&0u32.to_le_bytes())?; // image data size (ignored)
            output.write(&0u32.to_le_bytes())?; // image horizontal resolution (ignored)
            output.write(&0u32.to_le_bytes())?; // image vertical resolution (ignored)
            output.write(&1u32.to_le_bytes())?; // the number of palettes == 1
            output.write(&0u32.to_le_bytes())?; // important palette index == 0
            output.write(&[0xFF, 0xFF, 0xFF, 0x00])?; // palette
        }
        BitmapKind::Os2 => {
            output.write(&12u32.to_le_bytes())?;
            output.write(&(width as u16).to_le_bytes())?;
            output.write(
                &(if args.inverted {
                    -(height as i16)
                } else {
                    height as i16
                })
                .to_le_bytes(),
            )?;
            output.write(&1u16.to_le_bytes())?; // planes
            output.write(&1u16.to_le_bytes())?; // data size per pixel
            output.write(&[0xFF, 0xFF, 0xFF])?; // palette
        }
    }
    let pad = if width % 4 == 0 {
        None
    } else {
        Some(vec![0; 4 - (width as usize % 4)])
    };
    data.chunks_exact(width as _)
        .map(|chunk| {
            output.write(chunk)?;
            if let Some(pad) = pad.as_ref() {
                output.write(pad)?;
            }
            Ok(())
        })
        .collect::<std::io::Result<()>>()?;

    Ok(())
}
