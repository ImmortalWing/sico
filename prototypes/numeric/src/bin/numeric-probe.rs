use std::{env, fs, hint::black_box, path::PathBuf, time::Instant};

use num_bigint::BigInt;
use num_traits::Signed;
use serde::Serialize;
use sico_numeric_prototype::{
    CanonicalInt, DecimalWire, ExactDecimal, NumericLimits, checked_int_add, checked_int_mul,
};

#[derive(Serialize)]
struct ProbeReport {
    schema: &'static str,
    crate_version: &'static str,
    os: &'static str,
    arch: &'static str,
    iterations: u32,
    int: IntResult,
    decimal: DecimalResult,
    deterministic: bool,
}

#[derive(Serialize)]
struct IntResult {
    operand_bits: u64,
    result_bits: u64,
    wire_bytes: usize,
    checksum_fnv1a64: String,
    add_elapsed_ns: u128,
    mul_elapsed_ns: u128,
    over_limit_rejected: bool,
}

#[derive(Serialize)]
struct DecimalResult {
    input_digits: usize,
    result_digits: usize,
    result_scale: u32,
    canonical: String,
    wire_bytes: usize,
    checksum_fnv1a64: String,
    add_elapsed_ns: u128,
    mul_elapsed_ns: u128,
    non_canonical_rejected: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = output_path()?;
    let report = run_probe()?;
    let json = serde_json::to_string_pretty(&report)? + "\n";
    if let Some(path) = output {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, &json)?;
    }
    print!("{json}");
    Ok(())
}

fn output_path() -> Result<Option<PathBuf>, String> {
    let mut args = env::args().skip(1);
    match args.next() {
        None => Ok(None),
        Some(flag) if flag == "--output" => args
            .next()
            .map(PathBuf::from)
            .map(Some)
            .ok_or_else(|| "--output requires a path".to_owned()),
        Some(flag) => Err(format!("unknown argument: {flag}")),
    }
}

fn run_probe() -> Result<ProbeReport, Box<dyn std::error::Error>> {
    const ITERATIONS: u32 = 2_000;
    let limits = NumericLimits::prototype_default();
    let int_left = (BigInt::from(1u8) << 4096) - 1u8;
    let int_right = BigInt::from(1u8);
    let int_result = checked_int_add(&int_left, &int_right, &limits)?;
    let int_wire = CanonicalInt::from_bigint(&int_result, &limits)?;

    let started = Instant::now();
    for _ in 0..ITERATIONS {
        black_box(checked_int_add(&int_left, &int_right, &limits)?);
    }
    let int_add_elapsed = started.elapsed().as_nanos();
    let started = Instant::now();
    for _ in 0..ITERATIONS {
        black_box(checked_int_mul(&int_left, &int_left, &limits)?);
    }
    let int_mul_elapsed = started.elapsed().as_nanos();

    let over_limit = NumericLimits {
        max_int_bytes: 8,
        ..limits
    };
    let over_limit_rejected = checked_int_add(&int_left, &int_right, &over_limit).is_err();

    let decimal_left = ExactDecimal::parse("12345678901234567890.123456789", &limits)?;
    let decimal_right = ExactDecimal::parse("0.000000011", &limits)?;
    let decimal_result = decimal_left.add(&decimal_right, &limits)?;
    decimal_left.mul(&decimal_right, &limits)?;
    let decimal_wire = decimal_result.to_wire(&limits)?;

    let started = Instant::now();
    for _ in 0..ITERATIONS {
        black_box(decimal_left.add(&decimal_right, &limits)?);
    }
    let decimal_add_elapsed = started.elapsed().as_nanos();
    let started = Instant::now();
    for _ in 0..ITERATIONS {
        black_box(decimal_left.mul(&decimal_right, &limits)?);
    }
    let decimal_mul_elapsed = started.elapsed().as_nanos();

    let non_canonical_rejected = {
        let invalid = DecimalWire {
            coefficient: CanonicalInt::from_bigint(
                &(decimal_result.coefficient() * 10u8),
                &limits,
            )?,
            scale: decimal_result.scale() + 1,
        };
        invalid.to_decimal(&limits).is_err()
    };

    let int_hash = fnv1a64(&int_wire.deterministic_bytes());
    let decimal_hash = fnv1a64(&decimal_wire.deterministic_bytes());
    let deterministic = int_hash == fnv1a64(&int_wire.deterministic_bytes())
        && decimal_hash == fnv1a64(&decimal_wire.deterministic_bytes());
    if !over_limit_rejected || !non_canonical_rejected || !deterministic {
        return Err("numeric probe invariant failed".into());
    }

    Ok(ProbeReport {
        schema: "sico.numeric-probe/0",
        crate_version: env!("CARGO_PKG_VERSION"),
        os: env::consts::OS,
        arch: env::consts::ARCH,
        iterations: ITERATIONS,
        int: IntResult {
            operand_bits: int_left.bits(),
            result_bits: int_result.bits(),
            wire_bytes: int_wire.encoded_len(),
            checksum_fnv1a64: format!("{int_hash:016x}"),
            add_elapsed_ns: int_add_elapsed,
            mul_elapsed_ns: int_mul_elapsed,
            over_limit_rejected,
        },
        decimal: DecimalResult {
            input_digits: decimal_left.coefficient().abs().to_str_radix(10).len(),
            result_digits: decimal_result.coefficient().abs().to_str_radix(10).len(),
            result_scale: decimal_result.scale(),
            canonical: decimal_result.to_canonical_string(),
            wire_bytes: decimal_wire.encoded_len(),
            checksum_fnv1a64: format!("{decimal_hash:016x}"),
            add_elapsed_ns: decimal_add_elapsed,
            mul_elapsed_ns: decimal_mul_elapsed,
            non_canonical_rejected,
        },
        deterministic,
    })
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}
