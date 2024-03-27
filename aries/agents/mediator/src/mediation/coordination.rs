// Copyright 2023 Naian G.
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use log::info;
use messages::{decorators::thread::Thread, msg_fields::protocols::coordinate_mediation::{
    keylist::KeylistItem,
    keylist_update::{KeylistUpdateItem, KeylistUpdateItemAction},
    keylist_update_response::{KeylistUpdateItemResult, KeylistUpdateResponseItem},
    CoordinateMediation, Keylist, KeylistContent, KeylistDecorators, KeylistQueryContent,
    KeylistUpdateContent, KeylistUpdateResponse, KeylistUpdateResponseContent,
    KeylistUpdateResponseDecorators, MediateDeny, MediateDenyContent, MediateDenyDecorators,
    MediateGrant, MediateGrantContent, MediateGrantDecorators,
}, msg_parts::MsgParts};
use public_key::Key;
use uuid::Uuid;

use crate::persistence::MediatorPersistence;

pub async fn handle_coord_authenticated(
    storage: Arc<impl MediatorPersistence>,
    message: CoordinateMediation,
    auth_pubkey: &str,
) -> CoordinateMediation {
    match message {
        CoordinateMediation::MediateRequest(_mediate_request) => {
            panic!(
                "Use handle_mediate_request directly. This handler is for preregistered clients."
            );
        }
        CoordinateMediation::KeylistUpdate(keylist_update) => {
            handle_keylist_update(storage, keylist_update, auth_pubkey).await
        }
        CoordinateMediation::KeylistQuery(keylist_query) => {
            handle_keylist_query(storage, keylist_query.content, auth_pubkey).await
        }
        _ => handle_unimplemented().await,
    }
}

pub async fn handle_unimplemented() -> CoordinateMediation {
    todo!("This error should ideally be handled on outer layer. Panicking for now.")
}

pub async fn handle_mediate_request<T: MediatorPersistence>(
    storage: Arc<T>,
    auth_pubkey: &str,
    did_doc: &str,
    our_signing_key: &str,
    grant_content: MediateGrantContent,
) -> CoordinateMediation {
    match storage
        .create_account(auth_pubkey, our_signing_key, did_doc)
        .await
    {
        Ok(()) => {
            let mediate_grant_msg = MediateGrant::builder()
                .content(grant_content)
                .decorators(MediateGrantDecorators::default())
                .id(Uuid::new_v4().to_string())
                .build();
            CoordinateMediation::MediateGrant(mediate_grant_msg)
        }
        Err(_msg) => {
            let mediate_deny_msg = MediateDeny::builder()
                .content(MediateDenyContent::default())
                .decorators(MediateDenyDecorators::default())
                .id(Uuid::new_v4().to_string())
                .build();
            CoordinateMediation::MediateDeny(mediate_deny_msg)
        }
    }
}

pub async fn handle_keylist_query<T: MediatorPersistence>(
    storage: Arc<T>,
    //todo: use the limits mentioned in the KeylistQueryData to modify response
    _keylist_query_data: KeylistQueryContent,
    auth_pubkey: &str,
) -> CoordinateMediation {
    let keylist_items: Vec<KeylistItem> = match storage.list_recipient_keys(auth_pubkey).await {
        Ok(recipient_keys) => recipient_keys
            .into_iter()
            .map(|recipient_key| KeylistItem { recipient_key })
            .collect(),
        Err(err) => todo!(
            "This error should ideally be handled on outer layer. Panicking for now{}",
            err
        ),
    };
    let keylist = Keylist::builder()
        .content(KeylistContent {
            keys: keylist_items,
            pagination: None,
        })
        .decorators(KeylistDecorators::default())
        .id(Uuid::new_v4().to_string())
        .build();
    CoordinateMediation::Keylist(keylist)
}

pub async fn handle_keylist_update<T: MediatorPersistence>(
    storage: Arc<T>,
    keylist_update_data: MsgParts<KeylistUpdateContent>,
    auth_pubkey: &str,
) -> CoordinateMediation {
    let updates: Vec<KeylistUpdateItem> = keylist_update_data.content.updates;
    let mut updated: Vec<KeylistUpdateResponseItem> = Vec::new();
    for update_item in updates.into_iter() {
        let result = match &update_item.action {
            KeylistUpdateItemAction::Add => {
            let key = match update_item.recipient_key.strip_prefix("did:key:") {
                Some(key_fingerprint) => {
                    let key_result = Key::from_fingerprint(key_fingerprint);
                    match key_result {
                        Ok(key) => {
                            let base58_key = Key::base58(&key);
                            base58_key
                        },
                        Err(err) => {
                            info!("Error creating key from fingerprint: {:?}", err);
                            update_item.recipient_key.clone() 
                        }
                    }
                },
                None => update_item.recipient_key.clone(), 
            };

 
                storage
                    .add_recipient(auth_pubkey, &key)
                    .await
            }
            KeylistUpdateItemAction::Remove => {
                let key = match update_item.recipient_key.strip_prefix("did:key:") {
                    Some(key_fingerprint) => {
                        let key_result = Key::from_fingerprint(key_fingerprint);
                        match key_result {
                            Ok(key) => {
                                let base58_key = Key::base58(&key);
                                base58_key
                            },
                            Err(err) => {
                                info!("Error creating key from fingerprint: {:?}", err);
                                update_item.recipient_key.clone() 
                            }
                        }
                    },
                    None => update_item.recipient_key.clone(), 
                };
     

                storage
                    .remove_recipient(auth_pubkey, &key)
                    .await
            }
        };
        let update_item_result = match result {
            Ok(()) => KeylistUpdateItemResult::Success,
            Err(_msg) => KeylistUpdateItemResult::ServerError,
        };
        updated.push(KeylistUpdateResponseItem {
            recipient_key: update_item.recipient_key,
            action: update_item.action,
            result: update_item_result,
        });
    }

    let decorators = KeylistUpdateResponseDecorators::builder()
        .thread(
            Thread::builder()
                .thid(
                    keylist_update_data.id
                )
                .build(),
        )
        .build();

    let keylist_update_response = KeylistUpdateResponse::builder()
        .content(KeylistUpdateResponseContent { updated })
        .decorators(decorators)
        .id(Uuid::new_v4().to_string())
        .build();
    CoordinateMediation::KeylistUpdateResponse(keylist_update_response)
}
