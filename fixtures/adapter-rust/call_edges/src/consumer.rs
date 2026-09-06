use crate::services::{format_name as fmt, Outcome, UserService, Wrapper};

pub fn run() -> String {
    // An associated function: the edge names the method that runs, not the
    // type. `new` returns Self only by convention, so calling this an
    // instantiation would be reading meaning into a name.
    let service = UserService::new("Ada".to_string());

    // A struct literal is Rust's actual construction syntax.
    let direct = UserService {
        name: "Grace".to_string(),
    };

    // A tuple struct constructor reads as a plain call; the resolved target's
    // kind is what makes it an instantiation.
    let wrapped = Wrapper(1);

    // A tuple enum variant is construction spelled as a call. Nothing calls
    // a variant, so the edge must not say `Calls`.
    let _outcome = Outcome::Ready("ok".to_string());

    // Call through a renamed import, resolving to the canonical symbol.
    let name = fmt(service.handle(), direct.handle());

    // A method call is recorded as a property access on the receiver, never
    // as a call edge to the type itself.
    let _ = wrapped.0;

    // A prelude name and a macro invocation: neither names a symbol in the
    // graph, so neither records an edge.
    drop(direct);
    println!("{name}");

    // A fully-qualified path used inline without a `use`: resolving it would
    // mean guessing which `services` was meant, so it records no edge. This is
    // a documented limit, pinned here so it cannot widen by accident.
    crate::services::unused_helper();

    name
}
