use std::{env, fs, path::PathBuf};

fn main() {
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let svg_path = PathBuf::from("assets/icon.svg");

    println!("cargo:rerun-if-changed=assets/icon.svg");

    let svg_data = fs::read_to_string(&svg_path)
        .expect("assets/icon.svg not found");

    render_svg_to_png(&svg_data, 32, &out.join("icon32.png"));
    render_svg_to_png(&svg_data, 64, &out.join("icon64.png"));
}

fn render_svg_to_png(svg: &str, size: u32, dest: &PathBuf) {
    use resvg::{tiny_skia, usvg};

    let opts = usvg::Options::default();
    let tree = usvg::Tree::from_str(svg, &opts).expect("invalid SVG");

    let mut pixmap = tiny_skia::Pixmap::new(size, size).expect("pixmap alloc failed");

    let scale = size as f32 / 64.0;
    let transform = tiny_skia::Transform::from_scale(scale, scale);

    resvg::render(&tree, transform, &mut pixmap.as_mut());

    pixmap.save_png(dest).expect("failed to write PNG");
}
