//! Runner-boundary fail-closed coverage for the RFC-0039 §2.3/§2.4 package
//! **consumer** path (STEP-0147 bridge), which the real `sico run` CLI depends
//! on but no other test exercises: every other package test
//! (`package_builder.rs`/`vision_package.rs`/`image_codec_package.rs`)
//! instantiates a package with a raw empty linker and never runs a Program
//! that *imports* one through [`Runner`]'s link + limits machinery.
//!
//! This fixture proves the three fail-closed launch refusals that make the
//! consumer path safe:
//!
//! 1. a guest imports a `sico:user/…` instance no prepared package exports;
//! 2. a guest import function's arity does not match the prepared package's
//!    export (the bridge forwards dynamic `Val`s, so a wrong arity is unsafe);
//! 3. a package that itself imports something is refused at preparation
//!    (v0 packages are pure — an import can only narrow, and a package has no
//!    grants to narrow from).
//!
//! These are `contract-verified` at the runner boundary via real wasmtime
//! component compilation, not rendered/package-runtime feature claims, and
//! they move no M24 §3 gate.

use sico_codegen_wasm::package_builder::csv_package;
use sico_runner::{FsGrants, NetGrants, PackageBinary, RunOutcome, Runner};
use wasm_encoder::{
    ComponentBuilder, ComponentTypeRef, ComponentValType, InstanceType, PrimitiveValType,
};

const CSV_INTERFACE: &str = "sico:user/csv@1.0.0";

/// A Program component that imports `sico:user/csv@1.0.0` and declares its
/// `parse-line` func with `params` core-visible string parameters and exactly
/// one result. The runner matches this declaration against the prepared
/// package's export by name + arity, so `params` is the knob under test
/// (`1` is the real `csv` arity; anything else must be a fail-closed refusal).
fn guest_importing_csv(params: usize) -> Vec<u8> {
    let names = ["p0", "p1", "p2", "p3"];
    let declared: Vec<(&str, ComponentValType)> = names[..params]
        .iter()
        .map(|name| (*name, ComponentValType::Primitive(PrimitiveValType::String)))
        .collect();

    let mut instance = InstanceType::new();
    let mut function = instance.ty().function();
    function.params(declared);
    function.result(Some(ComponentValType::Primitive(PrimitiveValType::String)));
    instance.export("parse-line", ComponentTypeRef::Func(0));

    let mut builder = ComponentBuilder::default();
    let instance_ty = builder.type_instance(Some("csv"), &instance);
    builder.import(CSV_INTERFACE, ComponentTypeRef::Instance(instance_ty));
    builder.finish()
}

/// A bare Program component that imports nothing (a valid component surface
/// the package-bridge step simply skips; the failure in the purity test comes
/// from the *package* argument, not the program).
fn guest_importing_nothing() -> Vec<u8> {
    ComponentBuilder::default().finish()
}

fn launch_message(outcome: Result<sico_runner::PreparedProgram, RunOutcome>) -> String {
    match outcome {
        Ok(_) => panic!("expected a fail-closed Launch refusal, got a prepared program"),
        Err(RunOutcome::Launch(message)) => message,
        Err(other) => panic!("expected a Launch refusal, got {other:?}"),
    }
}

#[test]
fn guest_importing_an_unprepared_package_fails_closed() {
    let runner = Runner::new().expect("runner builds");
    let message = launch_message(runner.prepare_program_with_packages(
        &guest_importing_csv(1),
        &FsGrants::default(),
        &NetGrants::default(),
        &[],
    ));
    assert!(
        message.contains("no prepared package exports it"),
        "an import with no matching package must be a typed launch refusal, got: {message}"
    );
}

#[test]
fn guest_package_import_arity_mismatch_fails_closed() {
    // The bridge forwards dynamically typed `Val`s straight to the package
    // export, so an arity mismatch is unsafe and must be refused at link time
    // rather than reaching a partial/incorrect call.
    let runner = Runner::new().expect("runner builds");
    let packages = [PackageBinary {
        bytes: csv_package(),
    }];
    let message = launch_message(runner.prepare_program_with_packages(
        &guest_importing_csv(2),
        &FsGrants::default(),
        &NetGrants::default(),
        &packages,
    ));
    assert!(
        message.contains("arity mismatch"),
        "a guest import whose arity differs from the package export must fail closed, got: {message}"
    );
}

#[test]
fn impure_package_is_refused_at_preparation() {
    // A "package" that itself imports something is not a pure v0 package;
    // preparation must refuse it before it is ever linked into a run, so it
    // can never smuggle ambient authority into the consumer's Store.
    let runner = Runner::new().expect("runner builds");
    let packages = [PackageBinary {
        bytes: guest_importing_csv(1),
    }];
    let message = launch_message(runner.prepare_program_with_packages(
        &guest_importing_nothing(),
        &FsGrants::default(),
        &NetGrants::default(),
        &packages,
    ));
    assert!(
        message.contains("must not import"),
        "a package with its own imports must be refused as impure, got: {message}"
    );
}
