wit_bindgen::generate!({
    path: "../../../wit/boundary-probe-v0",
    world: "boundary-probe",
});

use exports::sico::boundary_probe::app::{BigInt, BoundaryError, DecimalValue, Guest};
use sico::boundary_probe::runtime::{self, Counter};

struct Component;

impl Guest for Component {
    fn run(input: u32) -> u32 {
        let counter = Counter::new(input);
        counter.add(7);
        let subtotal = counter.value();
        runtime::log(&format!("guest subtotal={subtotal}"));
        drop(counter);
        runtime::host_add(subtotal, 8)
    }

    fn roundtrip_int(value: BigInt) -> BigInt {
        value
    }

    fn roundtrip_decimal(value: DecimalValue) -> DecimalValue {
        value
    }

    fn roundtrip_result(value: Result<u32, BoundaryError>) -> Result<u32, BoundaryError> {
        value
    }
}

export!(Component);
