use std::env;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let dest = Path::new(&out_dir).join("iata_set.rs");

    let data_path = Path::new(&manifest_dir).join("data/iata_codes.txt");
    let data = fs::File::open(&data_path).expect("data/iata_codes.txt not found");
    let reader = BufReader::new(data);

    let mut set = phf_codegen::Set::new();
    let mut count = 0u32;
    for line in reader.lines() {
        let code = line.expect("failed to read line");
        let code = code.trim().to_string();
        if code.is_empty() {
            continue;
        }
        set.entry(code);
        count += 1;
    }

    let mut out = fs::File::create(&dest).expect("failed to create iata_set.rs");
    writeln!(
        out,
        "/// Auto-generated from data/iata_codes.txt ({count} codes).\nstatic VALID_IATA_CODES: phf::Set<&'static str> = {};",
        set.build()
    )
    .unwrap();

    println!("cargo::rerun-if-changed=data/iata_codes.txt");
}
