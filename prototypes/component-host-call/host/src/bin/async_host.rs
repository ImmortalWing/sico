use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use futures::executor::block_on;
use wasmtime::component::{Accessor, Component, HasData, Linker};
use wasmtime::{Config, Engine, Store};
use wit_component::ComponentEncoder;

wasmtime::component::bindgen!({
    path: "../async-wit",
    world: "demo",
    imports: { default: trappable },
});

use sico::component_async::runtime::{Host, HostWithStore};

#[derive(Default)]
struct State {
    calls: u32,
}

impl HasData for State {
    type Data<'a> = &'a mut State;
}

impl Host for State {}

impl<T: Send + 'static> HostWithStore<T> for State {
    async fn delayed_add(
        accessor: &Accessor<T, Self>,
        left: u32,
        right: u32,
    ) -> wasmtime::Result<u32> {
        accessor.with(|mut access| {
            let state = access.get();
            state.calls += 1;
            left.checked_add(right)
                .ok_or_else(|| wasmtime::Error::msg("delayed-add overflowed u32"))
        })
    }
}

fn componentize(core_path: &Path, component_path: &Path) -> Result<Vec<u8>> {
    let core = fs::read(core_path)
        .with_context(|| format!("failed to read core module {}", core_path.display()))?;
    let component = ComponentEncoder::default()
        .module(&core)
        .context("failed to decode async component metadata")?
        .validate(true)
        .encode()
        .context("failed to encode async WebAssembly Component")?;
    if let Some(parent) = component_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(component_path, &component)?;
    Ok(component)
}

async fn run(component: &[u8], input: u32) -> Result<(u32, u32)> {
    let mut config = Config::new();
    config.concurrency_support(true);
    let engine = Engine::new(&config).map_err(|error| anyhow!("engine setup failed: {error:#}"))?;
    let component = Component::new(&engine, component)
        .map_err(|error| anyhow!("Wasmtime rejected async component: {error:#}"))?;
    let mut linker = Linker::new(&engine);
    Demo::add_to_linker::<_, State>(&mut linker, |state| state)
        .map_err(|error| anyhow!("failed to link async imports: {error:#}"))?;
    let mut store = Store::new(&engine, State::default());
    let bindings = Demo::instantiate_async(&mut store, &component, &linker)
        .await
        .map_err(|error| anyhow!("failed to instantiate async component: {error:#}"))?;
    let guest = bindings.sico_component_async_app();
    let output = store
        .run_concurrent(async |accessor| guest.call_run(accessor, input).await)
        .await
        .map_err(|error| anyhow!("async scheduler failed: {error:#}"))?
        .map_err(|error| anyhow!("async guest run trapped: {error:#}"))?;
    Ok((output, store.data().calls))
}

fn parse_args() -> Result<(PathBuf, PathBuf, u32)> {
    let mut args = env::args_os().skip(1);
    let Some(core) = args.next() else {
        bail!("usage: sico-component-async-host <core.wasm> <component.wasm> [input]");
    };
    let Some(component) = args.next() else {
        bail!("missing <component.wasm>");
    };
    let input = match args.next() {
        Some(value) => value
            .to_string_lossy()
            .parse::<u32>()
            .context("input must be a u32")?,
        None => 35,
    };
    Ok((core.into(), component.into(), input))
}

fn main() -> Result<()> {
    let (core, component_path, input) = parse_args()?;
    let component = componentize(&core, &component_path)?;
    let (output, calls) = block_on(run(&component, input))?;
    if output != 42 || calls != 1 {
        bail!("unexpected async result: output={output}, calls={calls}");
    }
    println!("PASS component-async output={output} host-calls={calls}");
    Ok(())
}
