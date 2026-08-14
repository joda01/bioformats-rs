/// Focused reader profiler for slow real-world fixtures.
///
/// Usage: bench_profile_rust <path> <planes_per_series> <max_w> <max_h>

use std::path::Path;
use std::time::Instant;

use bioformats::{ImageReader, MetadataValue};

fn parse_arg<T: std::str::FromStr>(args: &[String], index: usize, name: &str) -> T {
    args.get(index)
        .unwrap_or_else(|| panic!("missing {name}"))
        .parse()
        .unwrap_or_else(|_| panic!("invalid {name}"))
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 5 {
        eprintln!("usage: bench_profile_rust <path> <planes_per_series> <max_w> <max_h>");
        std::process::exit(2);
    }
    let path = Path::new(&args[1]);
    let planes_per_series: u32 = parse_arg(&args, 2, "planes_per_series");
    let max_region_width: u32 = parse_arg(&args, 3, "max_region_width");
    let max_region_height: u32 = parse_arg(&args, 4, "max_region_height");

    let t0 = Instant::now();
    let mut reader = match ImageReader::open(path) {
        Ok(reader) => reader,
        Err(err) => {
            println!("status=error");
            println!("phase=open");
            println!("error={err}");
            std::process::exit(1);
        }
    };
    println!("status=opened");
    println!("open_ns={}", t0.elapsed().as_nanos());
    println!("series_count={}", reader.series_count());

    for series in 0..reader.series_count() {
        let t_series = Instant::now();
        if let Err(err) = reader.set_series(series) {
            println!("series={series} set_series_error={err}");
            std::process::exit(1);
        }
        let meta = reader.metadata();
        let image_count = meta.image_count;
        let size_x = meta.size_x;
        let size_y = meta.size_y;
        let w = size_x.min(max_region_width).max(1);
        let h = size_y.min(max_region_height).max(1);
        let x = (size_x - w) / 2;
        let y = (size_y - h) / 2;
        let planes = image_count.min(planes_per_series);
        println!(
            "series={series} size={}x{} planes={planes} metadata_keys={} set_series_ns={}",
            size_x,
            size_y,
            meta.series_metadata.len(),
            t_series.elapsed().as_nanos()
        );
        for (key, value) in meta
            .series_metadata
            .iter()
            .filter(|(key, _)| key.starts_with("nd2_"))
        {
            if matches!(
                key.as_str(),
                "nd2_is_lossless"
                    | "nd2_split_channels"
                    | "nd2_image_data_first_encoding"
                    | "nd2_image_data_payload_offsets"
                    | "nd2_loop_series_handling"
                    | "nd2_series_index"
                    | "nd2_chunks"
                    | "nd2_image_data_chunks"
                    | "nd2_image_metadata_seq_chunks"
                    | "nd2_image_metadata_seq_matches_images"
            ) {
                match value {
                    MetadataValue::String(value) => println!("series={series} {key}={value}"),
                    MetadataValue::Int(value) => println!("series={series} {key}={value}"),
                    MetadataValue::Float(value) => println!("series={series} {key}={value}"),
                    MetadataValue::Bool(value) => println!("series={series} {key}={value}"),
                    MetadataValue::Bytes(value) => {
                        println!("series={series} {key}=<{} bytes>", value.len())
                    }
                }
            }
        }
        for plane in 0..planes {
            let t_read = Instant::now();
            match reader.open_bytes_region(plane, x, y, w, h) {
                Ok(bytes) => println!(
                    "series={series} plane={plane} read_ns={} bytes={}",
                    t_read.elapsed().as_nanos(),
                    bytes.len()
                ),
                Err(err) => {
                    println!("series={series} plane={plane} read_error={err}");
                    std::process::exit(1);
                }
            }
        }
    }
    println!("status=ok");
}
