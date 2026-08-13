//! LaVision UltraMicroscope stage positions on real acquisition files.
//!
//! These assert an **intentional deviation from Java Bio-Formats**: Java reports
//! no stage position for these files (its `OMETiffReader` trusts an OME-XML that
//! has no `<Plane>` and no `<StageLabel>`), while we recover the positions from
//! the vendor block inside `<Description>`. See `src/tiff/lavision.rs`.
//!
//! Opt-in, because the fixtures are multi-GB acquisition files that live outside
//! the repository:
//!
//! ```bash
//! BIOFORMATS_RS_LAVISION=1 cargo test --test lavision_stage_test
//! ```

use bioformats::ImageReader;
use std::path::PathBuf;

const TILE_DIR: &str = "/husky/henriksson/clearmap-img/inputs/osf-sa3x8";

fn tile_path(tile: &str) -> PathBuf {
    PathBuf::from(format!(
        "{TILE_DIR}/14-16-41_tricocktail_UltraII[{tile}]_C00_UltraII Filter0000.ome.tif"
    ))
}

fn enabled() -> bool {
    if std::env::var("BIOFORMATS_RS_LAVISION").as_deref() != Ok("1") {
        eprintln!("SKIP lavision stage tests (set BIOFORMATS_RS_LAVISION=1)");
        return false;
    }
    true
}

fn stage_value(tile: &str, key: &str) -> Option<f64> {
    let reader = ImageReader::open(&tile_path(tile)).expect("open UltraII tile");
    reader
        .metadata()
        .series_metadata
        .get(key)
        .map(|v| v.to_string().parse().expect("numeric stage value"))
}

/// The exact values Java Bio-Formats cannot return. Verified against
/// `bioformats_package.jar`: `getPlanePositionX(0, 0)` throws
/// `IndexOutOfBoundsException` and `getStageLabelX(0)` is null for both tiles.
#[test]
fn recovers_stage_offsets_java_reports_as_absent() {
    if !enabled() {
        return;
    }
    assert_eq!(
        stage_value("05 x 05", "LaVision StagePositionX"),
        Some(-1861.449219)
    );
    assert_eq!(
        stage_value("05 x 06", "LaVision StagePositionX"),
        Some(-1270.599365)
    );
    // Adjacent in X, so Y is shared.
    assert_eq!(
        stage_value("05 x 05", "LaVision StagePositionY"),
        Some(-5300.662598)
    );
    assert_eq!(
        stage_value("05 x 06", "LaVision StagePositionY"),
        Some(-5300.662598)
    );
}

/// The reason this matters: the recovered offsets give the tile overlap, which
/// is what a stitcher needs and what `overlap='auto'` otherwise cannot compute.
#[test]
fn recovered_offsets_give_the_tile_overlap() {
    if !enabled() {
        return;
    }
    let a = stage_value("05 x 05", "LaVision StagePositionX").unwrap();
    let b = stage_value("05 x 06", "LaVision StagePositionX").unwrap();

    let reader = ImageReader::open(&tile_path("05 x 05")).expect("open");
    let width_px = f64::from(reader.metadata().size_x);
    assert_eq!(
        width_px, 404.0,
        "tile width changed; overlap maths assumes it"
    );

    let tile_um = width_px * 1.625; // PhysicalSizeX from the OME <Pixels>
    let overlap = (tile_um - (b - a).abs()) / tile_um;
    assert!(
        (overlap - 0.10).abs() < 0.005,
        "expected ~10% overlap between adjacent tiles, got {overlap}"
    );
}

/// The acquisition-wide layout is repeated in every file of the set, so any one
/// tile can reconstruct the whole mosaic.
#[test]
fn every_tile_carries_the_whole_acquisition_layout() {
    if !enabled() {
        return;
    }
    for tile in ["05 x 05", "05 x 06"] {
        let reader = ImageReader::open(&tile_path(tile)).expect("open");
        let count = reader
            .metadata()
            .series_metadata
            .get("LaVision TileCount")
            .map(|v| v.to_string())
            .expect("TileCount present");
        assert!(
            count.parse::<i64>().unwrap() > 1,
            "tile {tile} should list the full mosaic, got {count}"
        );
    }
}
