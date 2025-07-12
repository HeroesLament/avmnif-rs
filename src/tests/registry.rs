use crate::{nif_collection, Context, Term, NifResult};

fn test_init(_ctx: &mut Context) {}

fn add_fn(_ctx: &Context, _args: &[Term]) -> NifResult<Term> {
    Ok(Term::from_i64(42))
}

nif_collection!(
    example,
    init = test_init,
    nifs = [("add", 2, add_fn)]
);



#[test]
fn test_nif_is_registered() {
    extern "C" {
        fn example_get_nif(name: *const u8) -> *const core::ffi::c_void;
    }

    let name = b"add\0";
    let ptr = unsafe { example_get_nif(name.as_ptr()) };
    assert!(!ptr.is_null(), "Expected 'add/2' to be registered");
}
