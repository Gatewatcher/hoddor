use js_sys::{Array, ArrayBuffer, Object, Promise, Uint8Array};
use wasm_bindgen::{JsCast, JsError, JsValue};
use web_sys::{
    AuthenticationExtensionsClientInputs, CredentialCreationOptions, CredentialRequestOptions,
    PublicKeyCredentialCreationOptions, PublicKeyCredentialDescriptor,
    PublicKeyCredentialParameters, PublicKeyCredentialRequestOptions, PublicKeyCredentialRpEntity,
    PublicKeyCredentialType, PublicKeyCredentialUserEntity, UserVerificationRequirement, Window,
};

use crate::console::*;

pub fn webauthn_create(
    challenge: &[u8],
    user: PublicKeyCredentialUserEntity,
    rp_entity: PublicKeyCredentialRpEntity,
    prf_salt: [u8; 32],
) -> Result<Promise, JsError> {
    log(&format!("Create webauthn"));

    let pk_options = PublicKeyCredentialCreationOptions::new(
        &Uint8Array::from(challenge),
        &Array::of1(&PublicKeyCredentialParameters::new(
            -7,
            PublicKeyCredentialType::PublicKey,
        )),
        &rp_entity,
        &user,
    );

    pk_options.set_extensions(&prf_extension_eval(
        &Uint8Array::from(prf_salt.as_slice()).buffer(),
    ));

    let cred_options = CredentialCreationOptions::new();
    cred_options.set_public_key(&pk_options);

    Ok(window()
        .navigator()
        .credentials()
        .create_with_options(&cred_options)
        .unwrap())
}

pub fn webauthn_get(
    challenge: &[u8],
    user_uuid: &str,
    rp_id: &str,
    prf_salt: [u8; 32],
) -> Result<Promise, JsError> {
    log(&format!("Get webauthn"));

    let pk_options = PublicKeyCredentialRequestOptions::new(&Uint8Array::from(challenge));
    pk_options.set_allow_credentials(
        &Some(PublicKeyCredentialDescriptor::new(
            &Uint8Array::from(user_uuid.as_bytes().to_vec().as_slice()),
            PublicKeyCredentialType::PublicKey,
        ))
        .into_iter()
        .collect::<Array>(),
    );
    pk_options.set_rp_id(&rp_id);
    pk_options.set_user_verification(UserVerificationRequirement::Required);

    pk_options.set_extensions(&prf_extension_eval(
        &Uint8Array::from(prf_salt.as_slice()).buffer(),
    ));

    let cred_options = CredentialRequestOptions::new();
    cred_options.set_public_key(&pk_options);

    Ok(window()
        .navigator()
        .credentials()
        .get_with_options(&cred_options)
        .unwrap())
}

pub fn prf_extension_eval(salt: &ArrayBuffer) -> AuthenticationExtensionsClientInputs {
    AuthenticationExtensionsClientInputs::from(
        Object::from_entries(&Array::of1(&Array::of2(
            &"prf".into(),
            &Object::from_entries(&Array::of1(&Array::of2(
                &"eval".into(),
                &Object::from_entries(&Array::of1(&Array::of2(&"first".into(), salt))).unwrap(),
            )))
            .unwrap(),
        )))
        .unwrap()
        .dyn_into::<JsValue>()
        .unwrap(),
    )
}

pub fn window() -> Window {
    web_sys::window().expect("Unable to retrieve window")
}
