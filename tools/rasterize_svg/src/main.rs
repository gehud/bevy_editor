//! Rasterizes SVG icons into PNGs that Bevy can load.
//!
//! Usage: `cargo run -p rasterize_svg -- [--mask] [--scale N] <in.svg> <out.png> [<in.svg> <out.png> ...]`
//!
//! * `--scale N` renders at N times the SVG's intrinsic size (default 4), so the icon stays
//!   crisp when displayed at its design size on high-DPI screens.
//! * `--mask` turns every pixel white while keeping its alpha. Use it for single-color icons:
//!   the color is then applied at runtime as a tint (as `bevy_feathers` does for its icons).

use resvg::{tiny_skia, usvg};
use std::{env, fs, process::ExitCode};

fn main() -> ExitCode {
    let mut mask = false;
    let mut scale = 4.0_f32;
    let mut paths = Vec::new();
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--mask" => mask = true,
            "--no-mask" => mask = false,
            "--scale" => {
                scale = args
                    .next()
                    .and_then(|s| s.parse().ok())
                    .expect("--scale needs a number")
            }
            _ => paths.push(arg),
        }
    }
    if paths.is_empty() || paths.len() % 2 != 0 {
        eprintln!("usage: rasterize_svg [--mask] [--scale N] <in.svg> <out.png> ...");
        return ExitCode::FAILURE;
    }

    for pair in paths.chunks(2) {
        let (input, output) = (&pair[0], &pair[1]);
        if let Err(err) = rasterize(input, output, scale, mask) {
            eprintln!("{input}: {err}");
            return ExitCode::FAILURE;
        }
        println!("{input} -> {output}");
    }
    ExitCode::SUCCESS
}

fn rasterize(input: &str, output: &str, scale: f32, mask: bool) -> Result<(), String> {
    let data = fs::read(input).map_err(|e| e.to_string())?;
    let tree =
        usvg::Tree::from_data(&data, &usvg::Options::default()).map_err(|e| e.to_string())?;
    let size = tree.size();
    let width = (size.width() * scale).ceil() as u32;
    let height = (size.height() * scale).ceil() as u32;
    let mut pixmap = tiny_skia::Pixmap::new(width, height).ok_or("empty image")?;
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );

    if mask {
        // Pixmap data is premultiplied RGBA: a white pixel with alpha `a` is (a, a, a, a).
        for px in pixmap.data_mut().chunks_exact_mut(4) {
            let a = px[3];
            px[0] = a;
            px[1] = a;
            px[2] = a;
        }
    }
    pixmap.save_png(output).map_err(|e| e.to_string())
}
