use std::{env, fs, path::PathBuf, rc::Rc};

use serde::Serialize;
use sico_resource_async_prototype::{
    AffineResource, BoundedStream, Pull, TaskScope, new_audit, one_shot,
};

#[derive(Serialize)]
struct Report {
    schema: &'static str,
    os: &'static str,
    arch: &'static str,
    resource_closes: u32,
    resource_drops: u32,
    task_completions: u32,
    cancellations: u32,
    stream_max_in_flight: usize,
    backpressure: u32,
    events: Vec<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = output_path()?;
    let audit = new_audit();

    AffineResource::new(1, Rc::clone(&audit)).close();
    {
        let _implicit = AffineResource::new(2, Rc::clone(&audit));
    }

    {
        let scope = TaskScope::new(Rc::clone(&audit));
        assert_eq!(scope.spawn("complete", 42).finish(), Ok(42));
        let pending = scope.spawn("cancel", 7);
        scope.cancel_all();
        assert!(pending.finish().is_err());
    }

    let (producer, reader) = one_shot(Rc::clone(&audit));
    producer.complete(9)?;
    assert_eq!(reader.read()?, 9);

    let mut stream = BoundedStream::new(2, Rc::clone(&audit));
    stream.push(1)?;
    stream.push(2)?;
    assert!(stream.push(3).is_err());
    assert_eq!(stream.pull(), Pull::Item(1));
    stream.push(3)?;
    stream.close();
    while !matches!(stream.pull(), Pull::End) {}

    let snapshot = audit.borrow().clone();
    let report = Report {
        schema: "sico.resource-async-probe/0",
        os: env::consts::OS,
        arch: env::consts::ARCH,
        resource_closes: snapshot.closes,
        resource_drops: snapshot.drops,
        task_completions: snapshot.completions,
        cancellations: snapshot.cancellations,
        stream_max_in_flight: snapshot.max_in_flight,
        backpressure: snapshot.backpressure,
        events: snapshot.events,
    };
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
