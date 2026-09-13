//! M17 (RFC-0043): image-vision@1 package content contracts — grey8,
//! threshold, occupancy over BGRA8 buffers, asserted at byte level
//! (element contents, not counts — the STEP-0131 lesson).

use sico_codegen_wasm::package_builder::image_vision_package;
use wasmtime::component::{Component, Linker, Val};
use wasmtime::{Config, Engine, Store};

fn engine() -> Engine {
    let mut config = Config::new();
    config.wasm_component_model(true);
    Engine::new(&config).unwrap()
}

/// Instantiates the package and calls one export with dynamic values.
fn call(instance_bytes: &[u8], func: &str, args: &[Val]) -> Vec<Val> {
    let engine = engine();
    let component = Component::new(&engine, instance_bytes)
        .map_err(|error| panic!("component loads: {error:#}"))
        .expect("compiles");
    let mut store = Store::new(&engine, ());
    let linker = Linker::new(&engine);
    let instance = linker
        .instantiate(&mut store, &component)
        .expect("instantiates");
    // Two-level export resolution: instance identity, then function name.
    let inner = instance
        .get_export_index(&mut store, None, "sico:user/image-vision@1.0.0")
        .expect("interface instance exported");
    let func_index = instance
        .get_export_index(&mut store, Some(&inner), func)
        .unwrap_or_else(|| panic!("export {func} missing"));
    let func = instance
        .get_func(&mut store, func_index)
        .expect("function resolves");
    // Every v0 vision op returns `list<u8>`: exactly one result value.
    let mut results = vec![Val::List(Vec::new()); 1];
    func.call(&mut store, args, &mut results)
        .expect("call succeeds");
    results
}

fn bytes_val(bytes: &[u8]) -> Val {
    Val::List(bytes.iter().map(|b| Val::U8(*b)).collect())
}

fn result_bytes(value: &Val) -> Vec<u8> {
    match value {
        Val::List(items) => items
            .iter()
            .map(|item| match item {
                Val::U8(b) => *b,
                other => panic!("unexpected list element {other:?}"),
            })
            .collect(),
        other => panic!("unexpected result {other:?}"),
    }
}

/// One deterministic BGRA pixel (b, g, r, 255).
fn pixel(b: u8, g: u8, r: u8) -> [u8; 4] {
    [b, g, r, 255]
}

#[test]
fn image_vision_exports_and_is_deterministic() {
    let package = image_vision_package();
    assert_eq!(image_vision_package(), package, "two builds identical");
    // Structure: the interface instance exports the three functions.
    let engine = engine();
    let component = Component::new(&engine, &package)
        .map_err(|error| panic!("compile: {error:#}"))
        .unwrap();
    let ty = component.component_type();
    let mut exports = 0;
    for (name, item) in ty.exports(&engine) {
        assert!(name.starts_with("sico:user/image-vision@"), "{name}");
        assert!(matches!(
            item.ty,
            wasmtime::component::types::ComponentItem::ComponentInstance(_)
        ));
        exports += 1;
    }
    assert_eq!(exports, 1);
}

#[test]
fn to_grey8_bt601_luma_byte_exact() {
    let package = image_vision_package();
    // BT.601 on a 2x1 buffer: pixel A = (b=0,g=0,r=255) → floor(299*255/1024)
    // = 74; pixel B = (b=255,g=255,r=0) → floor((114+587)*255/1024) = 174.
    let pixels = [pixel(0, 0, 255), pixel(255, 255, 0)].concat();
    let results = call(
        &package,
        "to-grey8",
        &[bytes_val(&pixels), Val::U64(2), Val::U64(1)],
    );
    let grey = result_bytes(&results[0]);
    assert_eq!(grey, vec![74, 174], "byte-exact BT.601 truncation");
}

#[test]
fn threshold_classifies_against_threshold_slot() {
    let package = image_vision_package();
    // Threshold contract: param 2 carries the threshold, param 3 the pixel
    // count. With threshold = 128: white (luma 255) → 255, cyan
    // (b=g=255 → luma 174) → 255, dark (luma 0) → 0.
    let pixels = [pixel(255, 255, 255), pixel(255, 255, 0), pixel(0, 0, 0)].concat();
    let results = call(
        &package,
        "threshold",
        &[bytes_val(&pixels), Val::U64(128), Val::U64(3)],
    );
    assert_eq!(result_bytes(&results[0]), vec![255, 255, 0]);
}

#[test]
fn occupancy_lights_columns_with_bright_pixels() {
    let package = image_vision_package();
    // 3x2 buffer: columns 0 and 2 hold a bright pixel (white, luma 255),
    // column 1 stays dark (luma 0).
    let pixels = [
        pixel(255, 255, 255),
        pixel(0, 0, 0),
        pixel(255, 255, 255),
        pixel(0, 0, 0),
        pixel(0, 0, 0),
        pixel(0, 0, 0),
    ]
    .concat();
    let results = call(
        &package,
        "occupancy",
        &[bytes_val(&pixels), Val::U64(3), Val::U64(2)],
    );
    assert_eq!(result_bytes(&results[0]), vec![255, 0, 255]);
}

#[test]
fn tetris_column_detector_finds_occupied_slots() {
    // The tetris recognition primitive: an 8-wide board region where the
    // occupied cells (white blocks → luma 255) sit at columns 1, 4, 4, 6.
    let width = 8_u64;
    let height = 2_u64;
    let mut pixels = Vec::new();
    let occupied: [(usize, usize); 4] = [(1, 0), (4, 0), (4, 1), (6, 1)];
    for y in 0..height as usize {
        for x in 0..width as usize {
            if occupied.contains(&(x, y)) {
                pixels.extend_from_slice(&pixel(255, 255, 255));
            } else {
                pixels.extend_from_slice(&pixel(0, 0, 0));
            }
        }
    }
    let package = image_vision_package();
    let results = call(
        &package,
        "occupancy",
        &[bytes_val(&pixels), Val::U64(width), Val::U64(height)],
    );
    assert_eq!(
        result_bytes(&results[0]),
        vec![0, 255, 0, 0, 255, 0, 255, 0]
    );
}
