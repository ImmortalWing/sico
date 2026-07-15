use std::pin::Pin;
use std::task::{Context, Poll};

use futures::executor::block_on;
use sha2::{Digest, Sha256};
use wasmtime::component::{Component, FutureProducer, FutureReader, Linker, StreamReader};
use wasmtime::{Config, Engine, Store, StoreContextMut};

type ProbeResult<T> = std::result::Result<T, wasmtime::Error>;

const IDENTITY_COMPONENT: &str = r#"
(component
  (type $f (future u32))
  (type $s (stream u32))
  (core module $m
    (func (export "identity") (param i32) (result i32) local.get 0)
  )
  (core instance $i (instantiate $m))
  (func (export "future") (param "value" $f) (result $f)
    (canon lift (core func $i "identity")))
  (func (export "stream") (param "value" $s) (result $s)
    (canon lift (core func $i "identity")))
)
"#;

const FUTURE_CANCEL_COMPONENT: &str = r#"
(component
  (core module $libc (memory (export "memory") 1))
  (core instance $libc (instantiate $libc))
  (core module $m
    (import "" "future.read" (func $future.read (param i32 i32) (result i32)))
    (import "" "future.cancel-read" (func $future.cancel-read (param i32) (result i32)))
    (memory (export "memory") 1)
    (func (export "run") (param i32)
      (call $future.read (local.get 0) (i32.const 100))
      i32.const -1
      i32.ne
      if unreachable end
      (call $future.cancel-read (local.get 0))
      i32.const 2
      i32.ne
      if unreachable end
    )
  )
  (type $f (future u32))
  (core func $future.read (canon future.read $f async (memory $libc "memory")))
  (core func $future.cancel-read (canon future.cancel-read $f))
  (core instance $i (instantiate $m
    (with "" (instance
      (export "future.read" (func $future.read))
      (export "future.cancel-read" (func $future.cancel-read))
    ))
  ))
  (func (export "run") async (param "future" $f)
    (canon lift (core func $i "run") (memory $libc "memory")))
)
"#;

const STREAM_CAPACITY_COMPONENT: &str = r#"
(component
  (core module $libc (memory (export "memory") 1))
  (core instance $libc (instantiate $libc))
  (core module $m
    (import "" "memory" (memory 1))
    (import "" "stream.read" (func $stream.read (param i32 i32 i32) (result i32)))
    (func (export "read") (param i32 i32) (result i32)
      (call $stream.read (local.get 0) (i32.const 0) (local.get 1))
    )
  )
  (type $s (stream u8))
  (core func $stream.read (canon stream.read $s async (memory $libc "memory")))
  (core instance $i (instantiate $m
    (with "" (instance
      (export "memory" (memory $libc "memory"))
      (export "stream.read" (func $stream.read))
    ))
  ))
  (func (export "read") (param "stream" (stream u8)) (param "capacity" u32) (result u32)
    (canon lift (core func $i "read")))
)
"#;

struct PendingFuture;

impl FutureProducer<()> for PendingFuture {
    type Item = u32;

    fn poll_produce(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _store: StoreContextMut<()>,
        finish: bool,
    ) -> Poll<ProbeResult<Option<Self::Item>>> {
        if finish {
            Poll::Ready(Ok(None))
        } else {
            Poll::Pending
        }
    }
}

fn component_bytes(text: &str) -> ProbeResult<Vec<u8>> {
    wat::parse_str(text)
        .map_err(|error| wasmtime::Error::msg(format!("Component WAT failed: {error}")))
}

fn component(engine: &Engine, bytes: &[u8]) -> ProbeResult<Component> {
    Component::new(engine, bytes)
}

async fn run() -> ProbeResult<()> {
    let mut config = Config::new();
    config.concurrency_support(true);
    let engine = Engine::new(&config)?;
    let linker = Linker::new(&engine);
    let identity_bytes = component_bytes(IDENTITY_COMPONENT)?;
    let cancel_bytes = component_bytes(FUTURE_CANCEL_COMPONENT)?;
    let capacity_bytes = component_bytes(STREAM_CAPACITY_COMPONENT)?;
    let mut artifact_hash = Sha256::new();
    artifact_hash.update(&identity_bytes);
    artifact_hash.update(&cancel_bytes);
    artifact_hash.update(&capacity_bytes);
    let artifact_hash = format!("{:x}", artifact_hash.finalize());

    let mut store = Store::new(&engine, ());
    let identity = linker
        .instantiate_async(&mut store, &component(&engine, &identity_bytes)?)
        .await?;
    let future_identity = identity
        .get_typed_func::<(FutureReader<u32>,), (FutureReader<u32>,)>(&mut store, "future")?;
    let stream_identity = identity
        .get_typed_func::<(StreamReader<u32>,), (StreamReader<u32>,)>(&mut store, "stream")?;
    let future = FutureReader::new(&mut store, async { wasmtime::error::Ok(10_u32) })?;
    let (mut future,) = future_identity.call_async(&mut store, (future,)).await?;
    future.close(&mut store)?;
    let stream = StreamReader::new(&mut store, vec![1_u32, 2, 3])?;
    let (mut stream,) = stream_identity.call_async(&mut store, (stream,)).await?;
    stream.close(&mut store)?;

    let cancel = linker
        .instantiate_async(&mut store, &component(&engine, &cancel_bytes)?)
        .await?;
    let cancel_run = cancel.get_typed_func::<(FutureReader<u32>,), ()>(&mut store, "run")?;
    let pending = FutureReader::new(&mut store, PendingFuture)?;
    cancel_run.call_async(&mut store, (pending,)).await?;

    let capacity = linker
        .instantiate_async(&mut store, &component(&engine, &capacity_bytes)?)
        .await?;
    let read = capacity.get_typed_func::<(StreamReader<u8>, u32), (u32,)>(&mut store, "read")?;
    let one = StreamReader::new(&mut store, b"hello".to_vec())?;
    let (one_code,) = read.call_async(&mut store, (one, 1)).await?;
    if one_code != 1 << 4 {
        return Err(wasmtime::Error::msg(format!(
            "bounded read ignored capacity: {one_code}"
        )));
    }
    let all = StreamReader::new(&mut store, b"hello".to_vec())?;
    let (all_code,) = read.call_async(&mut store, (all, 100)).await?;
    if all_code != (5 << 4) | 1 {
        return Err(wasmtime::Error::msg(format!(
            "stream completion code changed: {all_code}"
        )));
    }

    println!(
        "FUTURE_STREAM_OK future_roundtrip=closed stream_roundtrip=closed future_cancel=acknowledged stream_capacity=1/5 artifacts=3 sha256={artifact_hash}"
    );
    Ok(())
}

fn main() -> ProbeResult<()> {
    block_on(run())
}
