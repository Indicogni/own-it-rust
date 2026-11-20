//@ expect: run
//@ why: the complete flow new -> to -> (cc) -> body -> (cc) -> send must work, with stage names, equal sizes in every stage, and the body moved in without a copy.
use m1_types::l07_sealed_typestate::{Envelope, HasRecipient, NeedsBody, NeedsTo, Ready, Stage};

/// Generic code may ask for "any stage" or for "a stage with a recipient".
fn stage_name<S: Stage>(envelope: &Envelope<S>) -> &'static str {
    envelope.stage()
}

fn copy_to_archive<S: HasRecipient>(envelope: Envelope<S>) -> Envelope<S> {
    envelope.cc("archive")
}

fn main() {
    let draft: Envelope<NeedsTo> = Envelope::new("ada");
    assert_eq!(draft.stage(), "needs-to");

    let addressed: Envelope<NeedsBody> = draft.to("grace");
    assert_eq!(stage_name(&addressed), "needs-body");

    let text = String::from("the compiler checked this message");
    let text_address = text.as_ptr();
    let ready: Envelope<Ready> = addressed.cc("alan").body(text);
    assert_eq!(ready.stage(), "ready");
    assert_eq!(
        ready.text().as_ptr(),
        text_address,
        "body() must move the String into the envelope, not copy it"
    );

    let ready = copy_to_archive(ready);
    assert_eq!(
        ready.send(),
        "ada -> grace, alan, archive: the compiler checked this message"
    );

    assert_eq!(
        Envelope::new("a").to("b").body(String::from("c")).send(),
        "a -> b: c"
    );

    assert_eq!(
        (NeedsTo::NAME, NeedsBody::NAME, Ready::NAME),
        ("needs-to", "needs-body", "ready")
    );
    assert_eq!(
        (
            size_of::<NeedsTo>(),
            size_of::<NeedsBody>(),
            size_of::<Ready>()
        ),
        (0, 0, 0),
        "the stage types must be zero-sized"
    );
    assert_eq!(
        size_of::<Envelope<NeedsTo>>(),
        size_of::<Envelope<Ready>>(),
        "the stage must not be stored in the envelope"
    );
}
