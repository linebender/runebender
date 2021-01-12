//! Build script to generate our glyph name lookup table.

use std::convert::TryFrom;
use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::str::FromStr;

const OUT_FILE: &str = "glyph_names_codegen.rs";

fn main() {
    let path = Path::new(&env::var("OUT_DIR").unwrap()).join(OUT_FILE);
    let mut file = BufWriter::new(File::create(&path).unwrap());
    let names = include_str!("../resources/aglfn.txt");

    let mut entries = names
        .lines()
        .filter(|l| !l.starts_with('#'))
        .map(NameEntry::from_str)
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    entries.sort_by(|a, b| a.chr.cmp(&b.chr));
    let formatted = entries
        .iter()
        .map(|e| format!("('{}', \"{}\")", e.chr.escape_default(), e.postscript_name))
        .collect::<Vec<_>>();
    writeln!(
        &mut file,
        "static GLYPH_NAMES: &[(char, &str)] = &[\n{}];\n",
        formatted.join(",\n")
    )
    .unwrap();
}

struct NameEntry {
    chr: char,
    postscript_name: String,
}

impl FromStr for NameEntry {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut split = s.split(';');
        match (split.next(), split.next(), split.next(), split.next()) {
            (Some(cpoint), Some(postscript_name), Some(_unic_name), None) => {
                let chr = u32::from_str_radix(cpoint, 16).unwrap();
                let chr = char::try_from(chr).unwrap();
                let postscript_name = postscript_name.to_string();
                Ok(NameEntry {
                    chr,
                    postscript_name,
                })
            }
            _ => Err(s.to_string()),
        }
    }
}
