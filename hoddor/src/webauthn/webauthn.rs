use js_sys::{Array, Promise, Uint8Array};
use wasm_bindgen::JsValue;
use web_sys::{
    AuthenticationExtensionsClientInputs, AuthenticationExtensionsPrfInputs,
    AuthenticatorAttachment, AuthenticatorSelectionCriteria, CredentialCreationOptions,
    CredentialRequestOptions, PublicKeyCredentialCreationOptions, PublicKeyCredentialDescriptor,
    PublicKeyCredentialParameters, PublicKeyCredentialRequestOptions, PublicKeyCredentialRpEntity,
    PublicKeyCredentialType, PublicKeyCredentialUserEntity, UserVerificationRequirement,
};

use crate::{adapters::logger, crypto::prf_inputs, global::window};
use sha2::{Digest, Sha256};

/// Secure algorithms recommendation:
/// -8: Ed25519
/// -7: ES256
/// -257: RS256
///
/// https://www.iana.org/assignments/cose/cose.xhtml#algorithms
static SECURE_ALGORITHM: &[i32; 3] = &[-7, -257, -8];

pub fn webauthn_create(
    challenge: &Uint8Array,
    name: &str,
    prf_salt: &Uint8Array,
) -> Result<Promise, JsValue> {
    logger().log(&"Create webauthn".to_string());

    let pk_rp_entity = PublicKeyCredentialRpEntity::new(name);

    let mut hasher = Sha256::new();
    hasher.update(name.as_bytes());
    let result = hasher.finalize();
    let user_id = &result[0..8];

    let pk_user = PublicKeyCredentialUserEntity::new(name, name, &Uint8Array::from(user_id));

    let pk_options = PublicKeyCredentialCreationOptions::new(
        challenge,
        &SECURE_ALGORITHM
            .iter()
            .map(|alg| {
                PublicKeyCredentialParameters::new(*alg, PublicKeyCredentialType::PublicKey)
            })
            .collect::<Array>(),
        &pk_rp_entity,
        &pk_user,
    );

    let authenticator_selection = AuthenticatorSelectionCriteria::new();
    authenticator_selection.set_authenticator_attachment(AuthenticatorAttachment::CrossPlatform);
    pk_options.set_authenticator_selection(&authenticator_selection);

    let extensions = prf_extension_eval(prf_salt)?;
    pk_options.set_extensions(&extensions);

    let cred_options = CredentialCreationOptions::new();
    cred_options.set_public_key(&pk_options);

    window()
        .navigator()
        .credentials()
        .create_with_options(&cred_options)
}

pub fn webauthn_get(
    challenge: &Uint8Array,
    prf_salt: &Uint8Array,
    credential_id: Uint8Array,
) -> Result<Promise, JsValue> {
    let opts_obj = js_sys::Object::new();

    let pk_options = PublicKeyCredentialRequestOptions::new(&opts_obj);

    pk_options.set_challenge(challenge);

    let allow_creds = Array::new();
    let descriptor = PublicKeyCredentialDescriptor::new(
        &credential_id,
        PublicKeyCredentialType::PublicKey,
    );
    allow_creds.push(&descriptor);
    pk_options.set_allow_credentials(&allow_creds);

    let extensions = prf_extension_eval(prf_salt)?;
    pk_options.set_extensions(&extensions);

    pk_options.set_user_verification(UserVerificationRequirement::Required);

    let cred_options = CredentialRequestOptions::new();
    cred_options.set_public_key(&pk_options);

    window()
        .navigator()
        .credentials()
        .get_with_options(&cred_options)
        .map_err(|e| JsValue::from_str(&format!("WebAuthn error: {:?}", e)))
}

pub fn prf_extension_eval(
    salt: &Uint8Array,
) -> Result<AuthenticationExtensionsClientInputs, JsValue> {
    let prf_eval_inputs = prf_inputs(salt);

    let prf_extension = AuthenticationExtensionsPrfInputs::new();

    prf_extension.set_eval(&prf_eval_inputs);

    let extensions = AuthenticationExtensionsClientInputs::new();
    extensions.set_prf(&prf_extension);

    Ok(extensions)
}
