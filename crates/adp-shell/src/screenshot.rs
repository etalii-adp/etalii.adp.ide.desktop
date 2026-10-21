//! A headless frame as a PNG, for the tests that look at what the shell draws (the canvas, task T055, and the
//! table, task T065): every mesh egui tessellates from a frame's shapes, rasterised over a background with the
//! frame's textures, the font atlas included. Written under `target/screenshots`.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use eframe::egui;

/// The textures every frame so far set, kept as egui hands them out: whole, or a patch of one already set.
pub fn take_textures(textures: &mut HashMap<egui::TextureId, egui::ColorImage>, output: &mut egui::FullOutput) {
    for (id, delta) in std::mem::take(&mut output.textures_delta.set)
        .into_iter()
        .flat_map(|(id, deltas)| deltas.into_iter().map(move |d| (id, d)))
    {
        let egui::ImageData::Color(image) = delta.image;
        match (delta.pos, textures.get_mut(&id)) {
            (Some([x, y]), Some(texture)) => {
                for row in 0..image.size[1] {
                    for column in 0..image.size[0] {
                        let at = (y + row) * texture.size[0] + x + column;
                        texture.pixels[at] = image.pixels[row * image.size[0] + column];
                    }
                }
            }
            _ => {
                textures.insert(id, (*image).clone());
            }
        }
    }
    output.textures_delta.clear();
}

/// Writes the frame of `shapes` as a PNG of `size` points at `path`, over `background`.
pub fn write(
    ctx: &egui::Context,
    shapes: &[egui::epaint::ClippedShape],
    textures: &HashMap<egui::TextureId, egui::ColorImage>,
    size: egui::Vec2,
    background: egui::Color32,
    path: &Path,
) {
    let (width, height) = (size.x as usize, size.y as usize);
    let mut pixels = vec![[background.r() as f32, background.g() as f32, background.b() as f32]; width * height];
    for clipped in ctx.tessellate(shapes.to_vec(), 1.0) {
        let egui::epaint::Primitive::Mesh(mesh) = clipped.primitive else {
            continue;
        };
        let texture = textures.get(&mesh.texture_id);
        let clip = clipped.clip_rect;
        for triangle in mesh.indices.as_chunks::<3>().0 {
            let [a, b, c] = [0, 1, 2].map(|i| mesh.vertices[triangle[i] as usize]);
            let area = (b.pos - a.pos).x * (c.pos - a.pos).y - (b.pos - a.pos).y * (c.pos - a.pos).x;
            if area.abs() < f32::EPSILON {
                continue;
            }
            let span = |v: [f32; 3], low: f32, high: f32, limit: usize| {
                let from = v[0].min(v[1]).min(v[2]).max(low).max(0.0).floor() as usize;
                let to = (v[0].max(v[1]).max(v[2]).min(high).ceil().max(0.0) as usize).min(limit);
                from..to
            };
            for y in span([a.pos.y, b.pos.y, c.pos.y], clip.min.y, clip.max.y, height) {
                for x in span([a.pos.x, b.pos.x, c.pos.x], clip.min.x, clip.max.x, width) {
                    let p = egui::pos2(x as f32 + 0.5, y as f32 + 0.5);
                    let edge = |u: egui::Pos2, v: egui::Pos2| (v - u).x * (p - u).y - (v - u).y * (p - u).x;
                    let weights = [
                        edge(b.pos, c.pos) / area,
                        edge(c.pos, a.pos) / area,
                        edge(a.pos, b.pos) / area,
                    ];
                    if weights.iter().any(|w| *w < 0.0) {
                        continue;
                    }
                    let mix = |channel: fn(&egui::Color32) -> u8| {
                        weights[0] * channel(&a.color) as f32
                            + weights[1] * channel(&b.color) as f32
                            + weights[2] * channel(&c.color) as f32
                    };
                    let texel = texture.map_or([1.0; 4], |image| {
                        let uv =
                            a.uv.to_vec2() * weights[0] + b.uv.to_vec2() * weights[1] + c.uv.to_vec2() * weights[2];
                        let tx = ((uv.x * image.size[0] as f32) as usize).min(image.size[0] - 1);
                        let ty = ((uv.y * image.size[1] as f32) as usize).min(image.size[1] - 1);
                        let t = image.pixels[ty * image.size[0] + tx];
                        [t.r(), t.g(), t.b(), t.a()].map(|v| v as f32 / 255.0)
                    });
                    let source = [
                        mix(egui::Color32::r) * texel[0],
                        mix(egui::Color32::g) * texel[1],
                        mix(egui::Color32::b) * texel[2],
                    ];
                    let alpha = mix(egui::Color32::a) / 255.0 * texel[3];
                    let pixel = &mut pixels[y * width + x];
                    for channel in 0..3 {
                        // Premultiplied colour over what is there.
                        pixel[channel] = source[channel] + pixel[channel] * (1.0 - alpha);
                    }
                }
            }
        }
    }
    fs::create_dir_all(path.parent().expect("a folder")).expect("the folder is made");
    let file = fs::File::create(path).expect("the file is made");
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), width as u32, height as u32);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let bytes: Vec<u8> = pixels
        .iter()
        .flat_map(|p| p.map(|v| v.clamp(0.0, 255.0) as u8))
        .collect();
    encoder
        .write_header()
        .expect("a PNG header")
        .write_image_data(&bytes)
        .expect("the PNG is written");
}
