// Copyright 2019 the Runebender authors.

//! Font data for editing.

use std::fs::File;
use std::io::Read;

use kurbo::BezPath;

pub struct Font {
    pub glyphs: Vec<Option<BezPath>>,
}

impl Font {
    pub fn load_from_file(filename: &str) -> Font {
        let mut f = File::open(&filename).expect("error opening font file");
        let mut data = Vec::new();
        f.read_to_end(&mut data).expect("error reading font file");

        let font = font_rs::font::parse(&data).expect("error parsing font file");
        let mut glyphs = Vec::new();
        for glyph_ix in 0..font.num_glyphs() {
            glyphs.push(font.get_glyph_path(glyph_ix));
        }
        Font { glyphs }
    }
}
