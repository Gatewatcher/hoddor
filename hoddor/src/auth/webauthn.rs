use js_sys::{Array, ArrayBuffer, Object, Promise, Reflect, Uint8Array};
use wasm_bindgen::{JsCast, JsError, JsValue};
use web_sys::{
    AuthenticationExtensionsClientInputs, CredentialCreationOptions, CredentialRequestOptions,
    PublicKeyCredential, PublicKeyCredentialCreationOptions, PublicKeyCredentialDescriptor,
    PublicKeyCredentialParameters, PublicKeyCredentialRequestOptions, PublicKeyCredentialRpEntity,
    PublicKeyCredentialType, PublicKeyCredentialUserEntity, UserVerificationRequirement, Window,
};

use crate::console::*;

static SECURE_ALGORITHM: &[i32; 3] = &[-7, -257, -8];

pub fn webauthn_create(
    challenge: &Uint8Array,
    name: &str,
    rp_id: &str,
    prf_salt: &Uint8Array,
    cred_id: &Uint8Array,
) -> Result<Promise, JsError> {
    log(&format!("Create webauthn"));

    let pk_rp_entity = PublicKeyCredentialRpEntity::new(name);
    pk_rp_entity.set_id(rp_id);

    let pk_user = PublicKeyCredentialUserEntity::new(name, name, cred_id);

    let pk_options = PublicKeyCredentialCreationOptions::new(
        challenge,
        &SECURE_ALGORITHM
            .iter()
            .map(|alg| {
                PublicKeyCredentialParameters::new(alg.clone(), PublicKeyCredentialType::PublicKey)
            })
            .collect::<Array>(),
        &pk_rp_entity,
        &pk_user,
    );

    pk_options.set_extensions(&prf_extension_eval(&prf_salt.buffer()));

    let cred_options = CredentialCreationOptions::new();
    cred_options.set_public_key(&pk_options);

    Ok(window()
        .navigator()
        .credentials()
        .create_with_options(&cred_options)
        .unwrap())
}

pub fn webauthn_get(
    challenge: &Uint8Array,
    rp_id: &str,
    prf_salt: &Uint8Array,
    cred_id: Option<Uint8Array>,
) -> Result<Promise, JsError> {
    log(&format!("Get webauthn"));

    let pk_options = PublicKeyCredentialRequestOptions::new(&challenge);

    match cred_id {
        None => (),
        Some(cred_id) => {
            let allow_creds = Array::new();
            allow_creds.push(&PublicKeyCredentialDescriptor::new(
                &Uint8Array::from(cred_id),
                PublicKeyCredentialType::PublicKey,
            ));
            pk_options.set_allow_credentials(&allow_creds);
        }
    };

    pk_options.set_rp_id(&rp_id);
    pk_options.set_user_verification(UserVerificationRequirement::Required);

    pk_options.set_extensions(&prf_extension_eval(&prf_salt.buffer()));

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

pub fn prf_first_output(cred: &PublicKeyCredential) -> Result<Uint8Array, JsValue> {
    let extensions: Object = cred.get_client_extension_results().dyn_into()?;
    let prf_output: Result<Uint8Array, JsValue> = Reflect::get(&extensions, &"prf".into())
        .and_then(|prf| Reflect::get(&prf, &"results".into()))
        .and_then(|prf_results| Reflect::get(&prf_results, &"first".into()))
        .map(|first| Uint8Array::new(&first));
    prf_output
}

pub fn window() -> Window {
    web_sys::window().expect("Unable to retrieve window")
}
