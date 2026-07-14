use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result as AppResult, anyhow, bail};
use serde::Serialize;
use sha2::{Digest, Sha256};
use wasmtime::component::{Component, HasSelf, Linker, Resource, ResourceTable};
use wasmtime::{Engine, Store};
use wit_component::ComponentEncoder;

wasmtime::component::bindgen!({
    path: "../wit",
    world: "demo",
    with: {
        "sico:component-host-call/runtime@0.1.0.counter": HostCounter,
    },
    imports: { default: trappable },
    additional_derives: [PartialEq],
});

use exports::sico::component_host_call::app::{BigInt, DecimalValue, IntegerSign};
use sico::component_host_call::runtime::{Host, HostCounter as HostCounterResource};

#[derive(Debug)]
pub struct HostCounter {
    value: u32,
}

#[derive(Default)]
struct State {
    table: ResourceTable,
    events: Vec<String>,
}

impl Host for State {
    fn log(&mut self, message: String) -> wasmtime::Result<()> {
        self.events.push(format!("log:{message}"));
        Ok(())
    }

    fn host_add(&mut self, left: u32, right: u32) -> wasmtime::Result<u32> {
        let result = left
            .checked_add(right)
            .ok_or_else(|| wasmtime::Error::msg("host-add overflowed u32"))?;
        self.events
            .push(format!("host-add:{left}+{right}={result}"));
        Ok(result)
    }
}

impl HostCounterResource for State {
    fn new(&mut self, seed: u32) -> wasmtime::Result<Resource<HostCounter>> {
        self.events.push(format!("counter.new:{seed}"));
        Ok(self.table.push(HostCounter { value: seed })?)
    }

    fn add(&mut self, counter: Resource<HostCounter>, delta: u32) -> wasmtime::Result<()> {
        debug_assert!(!counter.owned());
        let counter = self.table.get_mut(&counter)?;
        counter.value = counter
            .value
            .checked_add(delta)
            .ok_or_else(|| wasmtime::Error::msg("counter.add overflowed u32"))?;
        self.events
            .push(format!("counter.add:{delta}->{}", counter.value));
        Ok(())
    }

    fn value(&mut self, counter: Resource<HostCounter>) -> wasmtime::Result<u32> {
        debug_assert!(!counter.owned());
        let value = self.table.get(&counter)?.value;
        self.events.push(format!("counter.value:{value}"));
        Ok(value)
    }

    fn drop(&mut self, counter: Resource<HostCounter>) -> wasmtime::Result<()> {
        debug_assert!(counter.owned());
        let counter = self.table.delete(counter)?;
        self.events.push(format!("counter.drop:{}", counter.value));
        Ok(())
    }
}

#[derive(Serialize)]
struct RunResult {
    schema: &'static str,
    input: u32,
    output: u32,
    bigint_roundtrip_bytes: usize,
    decimal_roundtrip: bool,
    component_sha256: String,
    events: Vec<String>,
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn componentize(core_path: &Path, component_path: &Path) -> AppResult<Vec<u8>> {
    let core = fs::read(core_path)
        .with_context(|| format!("failed to read core module {}", core_path.display()))?;
    let component = ComponentEncoder::default()
        .module(&core)
        .context("failed to decode embedded component metadata")?
        .validate(true)
        .encode()
        .context("failed to encode WebAssembly Component")?;
    if let Some(parent) = component_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(component_path, &component)
        .with_context(|| format!("failed to write component {}", component_path.display()))?;
    Ok(component)
}

fn run(component: &[u8], input: u32) -> AppResult<RunResult> {
    let engine = Engine::default();
    let component = Component::new(&engine, component)
        .map_err(|error| anyhow!("Wasmtime rejected component: {error:#}"))?;
    let mut linker = Linker::new(&engine);
    Demo::add_to_linker::<_, HasSelf<_>>(&mut linker, |state| state)
        .map_err(|error| anyhow!("failed to link host imports: {error:#}"))?;
    let mut store = Store::new(&engine, State::default());
    let bindings = Demo::instantiate(&mut store, &component, &linker)
        .map_err(|error| anyhow!("failed to instantiate component: {error:#}"))?;
    let output = bindings
        .sico_component_host_call_app()
        .call_run(&mut store, input)
        .map_err(|error| anyhow!("guest run trapped: {error:#}"))?;
    let bigint = BigInt {
        sign: IntegerSign::Positive,
        magnitude_be: vec![0xff; 512],
    };
    let bigint_returned = bindings
        .sico_component_host_call_app()
        .call_roundtrip_int(&mut store, &bigint)
        .map_err(|error| anyhow!("big-int roundtrip trapped: {error:#}"))?;
    if bigint_returned != bigint {
        bail!("big-int roundtrip changed the value");
    }
    let decimal = DecimalValue {
        coefficient: BigInt {
            sign: IntegerSign::Negative,
            magnitude_be: vec![0x04, 0xd2],
        },
        scale: 2,
    };
    let decimal_returned = bindings
        .sico_component_host_call_app()
        .call_roundtrip_decimal(&mut store, &decimal)
        .map_err(|error| anyhow!("decimal roundtrip trapped: {error:#}"))?;
    let decimal_roundtrip = decimal_returned == decimal;
    if !decimal_roundtrip {
        bail!("decimal roundtrip changed the value");
    }
    let events = std::mem::take(&mut store.data_mut().events);
    Ok(RunResult {
        schema: "sico.component-host-call.result.v1",
        input,
        output,
        bigint_roundtrip_bytes: bigint_returned.magnitude_be.len(),
        decimal_roundtrip,
        component_sha256: String::new(),
        events,
    })
}

fn parse_args() -> AppResult<(PathBuf, PathBuf, PathBuf, u32)> {
    let mut args = env::args_os().skip(1);
    let Some(core) = args.next() else {
        bail!("usage: sico-component-host <core.wasm> <component.wasm> <result.json> [input]");
    };
    let Some(component) = args.next() else {
        bail!("missing <component.wasm>");
    };
    let Some(result) = args.next() else {
        bail!("missing <result.json>");
    };
    let input = match args.next() {
        Some(value) => value
            .to_string_lossy()
            .parse::<u32>()
            .context("input must be a u32")?,
        None => 35,
    };
    if args.next().is_some() {
        bail!("too many arguments");
    }
    Ok((core.into(), component.into(), result.into(), input))
}

fn main() -> AppResult<()> {
    let (core_path, component_path, result_path, input) = parse_args()?;
    let component = componentize(&core_path, &component_path)?;
    let mut result = run(&component, input)?;
    result.component_sha256 = sha256(&component);
    if result.output != 50 {
        bail!("unexpected output: expected 50, got {}", result.output);
    }
    if let Some(parent) = result_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(&result)? + "\n";
    fs::write(&result_path, json.as_bytes())?;
    print!("{json}");
    Ok(())
}
