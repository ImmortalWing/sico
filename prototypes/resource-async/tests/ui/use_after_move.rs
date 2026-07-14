// expect-code: E0382
use sico_resource_async_prototype::{AffineResource, new_audit};

fn main() {
    let resource = AffineResource::new(1, new_audit());
    resource.close();
    let _ = resource.borrow();
}
