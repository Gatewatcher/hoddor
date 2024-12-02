extern crate console_error_panic_hook;
use std::panic;

use core::str;
use serde_wasm_bindgen::{from_value, to_value};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};
use web_sys::{
    js_sys::IteratorNext, window, FileSystemDirectoryHandle, FileSystemGetDirectoryOptions,
    FileSystemGetFileOptions,
};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[wasm_bindgen]
pub fn hash_password(password: JsValue) -> JsValue {
    let serde_password: String = from_value(password).unwrap();

    let argon = Argon2::default();
    let salt = SaltString::generate(&mut OsRng);

    let password_hash = argon
        .hash_password(serde_password.as_bytes(), &salt)
        .unwrap()
        .to_string();

    to_value(&password_hash).unwrap()
}

#[wasm_bindgen]
pub async fn create_opfs() {
    let options_directory = FileSystemGetDirectoryOptions::new();
    options_directory.set_create(true);

    let options_file = FileSystemGetFileOptions::new();
    options_file.set_create(true);

    let directory_handle = FileSystemDirectoryHandle::from(
        JsFuture::from(window().unwrap().navigator().storage().get_directory())
            .await
            .unwrap(),
    );
    let first_dir = FileSystemDirectoryHandle::from(
        JsFuture::from(
            directory_handle.get_directory_handle_with_options("first_dir", &options_directory),
        )
        .await
        .unwrap(),
    );
    JsFuture::from(first_dir.get_file_handle_with_options("first_file", &options_file))
        .await
        .unwrap();
    JsFuture::from(first_dir.get_file_handle_with_options("second_file", &options_file))
        .await
        .unwrap();

    let entries = first_dir.keys();

    while let Ok(i) = JsFuture::from(entries.next().unwrap()).await {
        if IteratorNext::from(i.clone()).done() {
            break;
        } else {
            log(&format!("{:?}", i));
        }
    }
}
