wit_bindgen::generate!({
    path: "../async-wit",
    world: "demo",
});

use sico::component_async::runtime;

struct Component;

impl exports::sico::component_async::app::Guest for Component {
    async fn run(input: u32) -> u32 {
        runtime::delayed_add(input, 7).await
    }
}

export!(Component);
