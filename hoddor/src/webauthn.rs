use base64urlsafedata::Base64UrlSafeData;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::Window;
use rand::prelude::*;
use crate::console::*;

use serde::{Deserialize, Serialize};

use crate::errors::AuthError;

fn random_fill(buffer: &mut [u8]) {
    let mut random = rand::thread_rng();
    random.fill_bytes(buffer);
}

pub fn random_vec(len: usize) -> Vec<u8> {
    let mut data = vec![0u8; len];
    random_fill(&mut data);
    data
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelyingParty {
    pub name: String,
    pub id: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: Base64UrlSafeData,
    pub name: String,
    pub display_name: String,
}

#[allow(non_camel_case_types)]
#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
#[repr(i32)]
pub enum COSEAlgorithm {
    /// Identifies this key as ECDSA (recommended SECP256R1) with SHA256 hashing
    #[serde(alias = "ECDSA_SHA256")]
    ES256 = -7, // recommends curve SECP256R1
    /// Identifies this key as ECDSA (recommended SECP384R1) with SHA384 hashing
    #[serde(alias = "ECDSA_SHA384")]
    ES384 = -35, // recommends curve SECP384R1
    /// Identifies this key as ECDSA (recommended SECP521R1) with SHA512 hashing
    #[serde(alias = "ECDSA_SHA512")]
    ES512 = -36, // recommends curve SECP521R1
    /// Identifies this key as RS256 aka RSASSA-PKCS1-v1_5 w/ SHA-256
    RS256 = -257,
    /// Identifies this key as RS384 aka RSASSA-PKCS1-v1_5 w/ SHA-384
    RS384 = -258,
    /// Identifies this key as RS512 aka RSASSA-PKCS1-v1_5 w/ SHA-512
    RS512 = -259,
    /// Identifies this key as PS256 aka RSASSA-PSS w/ SHA-256
    PS256 = -37,
    /// Identifies this key as PS384 aka RSASSA-PSS w/ SHA-384
    PS384 = -38,
    /// Identifies this key as PS512 aka RSASSA-PSS w/ SHA-512
    PS512 = -39,
    /// Identifies this key as EdDSA (likely curve ed25519)
    EDDSA = -8,
    /// Identifies this as an INSECURE RS1 aka RSASSA-PKCS1-v1_5 using SHA-1. This is not
    /// used by validators, but can exist in some windows hello tpm's
    INSECURE_RS1 = -65535,
    /// Identifies this key as the protocol used for [PIN/UV Auth Protocol One](https://fidoalliance.org/specs/fido-v2.1-ps-20210615/fido-client-to-authenticator-protocol-v2.1-ps-20210615.html#pinProto1)
    ///
    /// This reports as algorithm `-25`, but it is a lie. Don't include this in any algorithm lists.
    PinUvProtocol,
}

impl COSEAlgorithm {
    pub fn secure_algs() -> Vec<Self> {
        vec![
            COSEAlgorithm::ES256,
            // COSEAlgorithm::ES384,
            // COSEAlgorithm::ES512,
            COSEAlgorithm::RS256,
            // COSEAlgorithm::RS384,
            // COSEAlgorithm::RS512
            // -- Testing required
            // COSEAlgorithm::EDDSA,
        ]
    }

    pub fn all_possible_algs() -> Vec<Self> {
        vec![
            COSEAlgorithm::ES256,
            COSEAlgorithm::ES384,
            COSEAlgorithm::ES512,
            COSEAlgorithm::RS256,
            COSEAlgorithm::RS384,
            COSEAlgorithm::RS512,
            COSEAlgorithm::PS256,
            COSEAlgorithm::PS384,
            COSEAlgorithm::PS512,
            COSEAlgorithm::EDDSA,
            COSEAlgorithm::INSECURE_RS1,
        ]
    }
}

impl TryFrom<i128> for COSEAlgorithm {
    type Error = ();

    fn try_from(i: i128) -> Result<Self, Self::Error> {
        match i {
            -7 => Ok(COSEAlgorithm::ES256),
            -35 => Ok(COSEAlgorithm::ES384),
            -36 => Ok(COSEAlgorithm::ES512),
            -257 => Ok(COSEAlgorithm::RS256),
            -258 => Ok(COSEAlgorithm::RS384),
            -259 => Ok(COSEAlgorithm::RS512),
            -37 => Ok(COSEAlgorithm::PS256),
            -38 => Ok(COSEAlgorithm::PS384),
            -39 => Ok(COSEAlgorithm::PS512),
            -8 => Ok(COSEAlgorithm::EDDSA),
            -65535 => Ok(COSEAlgorithm::INSECURE_RS1),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PubKeyCredParams {
    #[serde(rename = "type")]
    pub type_: String,
    pub alg: i64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UserVerificationPolicy {
    Required,
    Preferred,
    // Unsafe to use it
    Discouraged,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthenticatorSelectionCriteria {
    pub user_verification: UserVerificationPolicy,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthenticationExtensionsPrfValues {
    pub first: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthenticationExtensionsPrfInputs {
    pub eval: AuthenticationExtensionsPrfValues,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthenticationExtensionsClientInputs {
    pub prf: AuthenticationExtensionsPrfInputs,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicKeyCredentialCreationOptions {
    pub rp: RelyingParty,
    pub user: User,
    pub challenge: Base64UrlSafeData,
    pub pub_key_cred_params: Vec<PubKeyCredParams>,
    pub authenticator_selection: AuthenticatorSelectionCriteria,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreationChallengeResponse {
    pub public_key: PublicKeyCredentialCreationOptions,
}

impl From<CreationChallengeResponse> for web_sys::CredentialCreationOptions {
    fn from(ccr: CreationChallengeResponse) -> Self {
        use js_sys::Uint8Array;

        let chal = Uint8Array::from(ccr.public_key.challenge.as_slice());
        let userid = Uint8Array::from(ccr.public_key.user.id.as_slice());
        
        let jsv = serde_wasm_bindgen::to_value(&ccr).unwrap();
		
        let pkcco = js_sys::Reflect::get(&jsv, &"publicKey".into()).unwrap();
        js_sys::Reflect::set(&pkcco, &"challenge".into(), &chal).unwrap();
		
        let user = js_sys::Reflect::get(&pkcco, &"user".into()).unwrap();
        js_sys::Reflect::set(&user, &"id".into(), &userid).unwrap();
        
        web_sys::CredentialCreationOptions::from(jsv)
    }
}

static UUID_TEST: &str = "dwadawadw.awdawd.awdawd.dawdaw";

#[wasm_bindgen]
pub async fn register(
    username: &str,
) -> Result<(), JsValue> {
	if username.is_empty() {
		return Err(AuthError::UsernameRequired.into());
	}

	let challenge: Base64UrlSafeData = random_vec(32).into();

	let options = CreationChallengeResponse {
		public_key: PublicKeyCredentialCreationOptions {
			rp: RelyingParty {
				name: String::from("Vault webauthn"),
				id: String::from("localhost"),
			},
			user: User {
				id: UUID_TEST.as_bytes().to_vec().into(),
				name: String::from(username),
				display_name: String::from(username),
			},
			challenge: challenge.clone(),
			pub_key_cred_params: COSEAlgorithm::secure_algs()
				.iter()
				.map(|alg| PubKeyCredParams {
					type_: "public-key".to_string(),
					alg: *alg as i64,
				})
				.collect(),
				authenticator_selection: AuthenticatorSelectionCriteria {
					user_verification: UserVerificationPolicy::Required,
				},
		},
	};

	let c_options: web_sys::CredentialCreationOptions = options.into();
    
	let _credentials = JsFuture::from(window()
        .navigator()
        .credentials()
        .create_with_options(&c_options)
        .expect_throw("Unable to create promise")).await.unwrap();

    let _credentials: web_sys::PublicKeyCredential = _credentials.into();

    log(&format!("{:?}", _credentials.get_client_extension_results()));

	Ok(())
}

/// A descriptor of a credential that can be used.
#[derive(Debug, Serialize, Deserialize)]
pub struct AllowCredentials {
    #[serde(rename = "type")]
    pub type_: String,
    pub id: Base64UrlSafeData,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicKeyCredentialRequestOptions {
    pub challenge: Base64UrlSafeData,
    pub rp_id: String,
    pub allow_credentials: Vec<AllowCredentials>,
    pub user_verification: UserVerificationPolicy,
    pub extensions: AuthenticationExtensionsClientInputs,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestChallengeResponse {
    pub public_key: PublicKeyCredentialRequestOptions,
}

impl From<RequestChallengeResponse> for web_sys::CredentialRequestOptions {
    fn from(rcr: RequestChallengeResponse) -> Self {
        use js_sys::Uint8Array;
        use js_sys::{Array, Object};

        let chal = Uint8Array::from(rcr.public_key.challenge.as_slice());
        
        let jsv = serde_wasm_bindgen::to_value(&rcr).unwrap();
		
        let pkcco = js_sys::Reflect::get(&jsv, &"publicKey".into()).unwrap();
        js_sys::Reflect::set(&pkcco, &"challenge".into(), &chal).unwrap();

        let allow_credentials: Array = rcr
            .public_key
            .allow_credentials
            .iter()
            .map(|allow_credential| {
                let userid = Uint8Array::from(allow_credential.id.as_slice());
                let type_ = JsValue::from_str(allow_credential.type_.as_str());
                let obj = Object::new();

                js_sys::Reflect::set(&obj, &"type".into(), &type_)
                    .unwrap();
                js_sys::Reflect::set(&obj, &"id".into(), &userid)
                    .unwrap();

                obj
            })
            .collect();

        js_sys::Reflect::set(&pkcco, &"allowCredentials".into(), &allow_credentials).unwrap();

        let first = Uint8Array::from(rcr.public_key.extensions.prf.eval.first.as_slice());
        
        let extensions = js_sys::Reflect::get(&pkcco, &"extensions".into()).unwrap();
        let prf = js_sys::Reflect::get(&extensions, &"prf".into()).unwrap();
        let eval = js_sys::Reflect::get(&prf, &"eval".into()).unwrap();
        js_sys::Reflect::set(&eval, &"first".into(), &first).unwrap();
        
        web_sys::CredentialRequestOptions::from(jsv)
    }
}

#[wasm_bindgen]
pub async fn authenticate(
    username: &str,
) -> Result<(), JsValue> {
    if username.is_empty() {
		return Err(AuthError::UsernameRequired.into());
	}

	let challenge: Base64UrlSafeData = random_vec(32).into();

    let mut allow_credentials: Vec<AllowCredentials> = Vec::new();
    allow_credentials.push(AllowCredentials{
        id: UUID_TEST.as_bytes().to_vec().into(),
        type_: String::from("public_key")
    });

	let options = RequestChallengeResponse {
		public_key: PublicKeyCredentialRequestOptions {
			rp_id: String::from("localhost"),
			challenge: challenge.clone(),
            allow_credentials,
            user_verification: UserVerificationPolicy::Required,
			extensions: AuthenticationExtensionsClientInputs {
                prf: AuthenticationExtensionsPrfInputs {
                    eval: AuthenticationExtensionsPrfValues {
						first: random_vec(128),
					},
                }
            },
		},
	};

	let c_options: web_sys::CredentialRequestOptions = options.into();
    
	let _credentials = JsFuture::from(window()
        .navigator()
        .credentials()
        .get_with_options(&c_options)
        .expect_throw("Unable to create promise")).await.unwrap();

    let _credentials: web_sys::PublicKeyCredential = _credentials.into();

    log(&format!("{:?}", _credentials.get_client_extension_results()));

	Ok(())

}

pub fn window() -> Window {
    web_sys::window().expect("Unable to retrieve window")
}