// expect-code: E0515
use sico_resource_async_prototype::{AffineResource, BorrowedResource, new_audit};

fn escaped() -> BorrowedResource<'static> {
    let resource = AffineResource::new(1, new_audit());
    resource.borrow()
}

fn main() {
    let _ = escaped();
}
