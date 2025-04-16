use ed25519_dalek::Signature;
use proptest::prelude::*;

use crate::signature::Eddsa;

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
    fn signature_encode_agrees_with_decode(sig: Eddsa) {
        let sig = Signature::from_components(sig.0.0, sig.1.0);
        let encoded = crate::signature::encode(&sig);
        let res = crate::signature::decode(&encoded);
        prop_assert!(res.is_ok());
        let decoded = res.unwrap();
        prop_assert_eq!(decoded, sig);
    }

    // TODO :: test verifying key decode/encode
}
