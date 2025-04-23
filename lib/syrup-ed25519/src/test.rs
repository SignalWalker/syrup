use ed25519_dalek::{ed25519::ComponentBytes, Signature};
use proptest::prelude::*;

fn signature_strategy() -> impl Strategy<Value = Signature> {
    <(ComponentBytes, ComponentBytes)>::arbitrary()
        .prop_map(|(r, s)| Signature::from_components(r, s))
}

// #[test]
// fn decodes_signatures() {
//     let mut rng = StdRng::seed_from_u64(0);
//     let key = RefCell::new(SigningKey::generate(&mut rng));
//     proptest!(|(sig in <Vec<u8>>::arbitrary().prop_map(|msg| key.borrow_mut().sign(&msg)))| {
//         let mut encoded = Vec::new();
//         encoded.extend_from_slice(b"<'eddsa<'r");
//         encoded.extend_from_slice(&sig.r_bytes().encode_bytes());
//         encoded.extend_from_slice(b"><s'");
//         encoded.extend_from_slice(&sig.s_bytes().encode_bytes());
//         encoded.extend_from_slice(b">>");
//     });
// }

proptest! {
    #[test]
    fn signature_encode_agrees_with_decode(sig in signature_strategy()) {
        let encoded = crate::signature::encode::<&[u8]>(&sig);
        let res = crate::signature::decode(&encoded);
        prop_assert!(res.is_ok());
        let decoded = res.unwrap();
        prop_assert_eq!(decoded, sig);
    }

    // TODO :: test verifying key decode/encode
}
