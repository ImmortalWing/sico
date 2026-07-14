wit_bindgen::generate!({
    path: "../wit",
    world: "demo",
});

use exports::sico::component_host_call::app::{BigInt, DecimalValue, Guest};
use sico::component_host_call::runtime::{self, Counter};

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
}

export!(Component);
