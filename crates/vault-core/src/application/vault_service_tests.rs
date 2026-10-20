//! Tests de extremo a extremo de la fachada con adaptadores en memoria.

use chrono::Duration;

use crate::application::save_vault_item::{CustomFieldDraft, SaveVaultItemRequest};
use crate::application::search_vault_items::VaultItemFilter;
use crate::application::test_support::{
    FakeClock, STRONG_MASTER_PASSWORD, SharedTestStorage, TestVaultHarness, build_test_vault_service,
    build_test_vault_service_on,
};
use crate::domain::encryption_key::EncryptionKey;
use crate::domain::item_content::{ItemContent, LoginContent};
use crate::domain::key_hierarchy::derive_vault_subkeys;
use crate::domain::ports::authenticated_cipher::AuthenticatedCipher;
use crate::domain::ports::key_derivation_function::KeyDerivationFunction;
use crate::domain::ports::vault_exporter::ExportFormat;
use crate::domain::ports::vault_importer::ImportFormat;
use crate::domain::ports::vault_repository::VaultRepository;
use crate::domain::secret_text::SecretText;
use crate::domain::vault_error::VaultError;
use crate::domain::vault_file::vault_file_format::decode_vault_file;
use crate::domain::vault_item::{REDACTED_SECRET_PLACEHOLDER, RevealableField, VaultItem};
use crate::infrastructure::crypto::argon2id_key_derivation::Argon2idKeyDerivation;
use crate::infrastructure::crypto::xchacha20_poly1305_cipher::XChaCha20Poly1305Cipher;

fn master_password() -> SecretText {
    SecretText::new(STRONG_MASTER_PASSWORD)
}

fn login_request(title: &str, password: &str) -> SaveVaultItemRequest {
    SaveVaultItemRequest {
        item_id: None,
        title: title.to_string(),
        folder_id: None,
        tag_names: vec!["personal".into(), " personal ".into(), String::new()],
        is_favorite: false,
        notes: SecretText::default(),
        custom_fields: vec![],
        item_content: ItemContent::Login(LoginContent {
            username: "ana@correo.es".into(),
            password: SecretText::new(password),
            website_urls: vec!["https://correo.es".into()],
            totp_secret: None,
        }),
    }
}

fn harness_with_created_vault() -> TestVaultHarness {
    let mut test_harness = build_test_vault_service();
    test_harness.vault_service.create_vault(&master_password()).expect("crear bóveda");
    test_harness
}

#[test]
fn created_vault_survives_lock_and_unlock() {
    let mut test_harness = harness_with_created_vault();
    let saved_item = test_harness.vault_service.save_item(login_request("Correo", "x9#Lq2!vRt")).expect("guardar");

    test_harness.vault_service.lock_vault();
    assert!(matches!(test_harness.vault_service.get_item_details(saved_item.item_id), Err(VaultError::VaultLocked)));
    test_harness.vault_service.unlock_vault(&master_password()).expect("desbloquear");

    let reloaded_item = test_harness.vault_service.get_item_details(saved_item.item_id).expect("leer");
    assert_eq!(reloaded_item.title, "Correo");
    assert_eq!(reloaded_item.tag_names, vec!["personal".to_string()]);
}

#[test]
fn stored_file_does_not_contain_plaintext_secrets() {
    let mut test_harness = harness_with_created_vault();
    test_harness.vault_service.save_item(login_request("MiBancoSecreto", "contraseña-ultra-secreta")).expect("guardar");

    let stored_vault_bytes = test_harness.vault_repository.read_vault_bytes().expect("leer");
    let stored_text = String::from_utf8_lossy(&stored_vault_bytes);

    assert!(!stored_text.contains("MiBancoSecreto"));
    assert!(!stored_text.contains("ultra-secreta"));
}

#[test]
fn wrong_password_fails_and_eventually_throttles() {
    let mut test_harness = harness_with_created_vault();
    test_harness.vault_service.lock_vault();
    let wrong_password = SecretText::new("totalmente-incorrecta-123");

    for _ in 0..3 {
        let unlock_result = test_harness.vault_service.unlock_vault(&wrong_password);
        assert!(matches!(unlock_result, Err(VaultError::InvalidMasterPasswordOrCorruptedVault)));
    }
    let throttled_result = test_harness.vault_service.unlock_vault(&master_password());

    assert!(matches!(throttled_result, Err(VaultError::UnlockTemporarilyBlocked { .. })));
    test_harness.fake_clock.advance_by(Duration::seconds(2));
    test_harness.vault_service.unlock_vault(&master_password()).expect("tras la espera se puede desbloquear");
}

#[test]
fn tampered_vault_file_is_rejected() {
    let mut test_harness = harness_with_created_vault();
    test_harness.vault_service.lock_vault();
    let mut stored_vault_bytes = test_harness.vault_repository.read_vault_bytes().expect("leer");
    let last_byte_position = stored_vault_bytes.len() - 1;
    stored_vault_bytes[last_byte_position] ^= 0xff;
    test_harness.vault_repository.write_vault_bytes(&stored_vault_bytes).expect("escribir");

    let unlock_result = test_harness.vault_service.unlock_vault(&master_password());

    assert!(matches!(unlock_result, Err(VaultError::InvalidMasterPasswordOrCorruptedVault)));
}

#[test]
fn weak_master_password_is_rejected_on_creation() {
    let mut test_harness = build_test_vault_service();

    let creation_result = test_harness.vault_service.create_vault(&SecretText::new("123456"));

    assert!(matches!(creation_result, Err(VaultError::WeakMasterPassword { .. })));
}

#[test]
fn creating_twice_is_rejected() {
    let mut test_harness = harness_with_created_vault();

    assert!(matches!(
        test_harness.vault_service.create_vault(&master_password()),
        Err(VaultError::VaultAlreadyExists)
    ));
}

#[test]
fn auto_lock_triggers_only_after_idle_timeout() {
    let mut test_harness = harness_with_created_vault();

    test_harness.fake_clock.advance_by(Duration::seconds(299));
    assert!(!test_harness.vault_service.lock_if_idle());
    test_harness.vault_service.register_user_activity().expect("actividad");
    test_harness.fake_clock.advance_by(Duration::seconds(299));
    assert!(!test_harness.vault_service.lock_if_idle());
    test_harness.fake_clock.advance_by(Duration::seconds(1));

    assert!(test_harness.vault_service.lock_if_idle());
    assert!(!test_harness.vault_service.vault_status().is_unlocked);
}

#[test]
fn changed_master_password_replaces_the_old_one() {
    let mut test_harness = harness_with_created_vault();
    let new_master_password = SecretText::new("nebula-quartz-falcon-ember-81");

    test_harness.vault_service.change_master_password(&master_password(), &new_master_password).expect("cambiar");
    test_harness.vault_service.lock_vault();

    assert!(test_harness.vault_service.unlock_vault(&master_password()).is_err());
    test_harness.vault_service.unlock_vault(&new_master_password).expect("desbloquear con la nueva");
}

#[test]
fn search_finds_by_title_username_and_respects_trash() {
    let mut test_harness = harness_with_created_vault();
    let correo_item = test_harness.vault_service.save_item(login_request("Correo", "pw-1")).expect("guardar");
    test_harness.vault_service.save_item(login_request("Banco", "pw-2")).expect("guardar");

    let text_filter = VaultItemFilter { search_text: Some("corr".into()), ..VaultItemFilter::default() };
    assert_eq!(test_harness.vault_service.search_items(&text_filter).expect("buscar").len(), 2);

    test_harness.vault_service.move_item_to_trash(correo_item.item_id).expect("papelera");
    let active_items = test_harness.vault_service.search_items(&VaultItemFilter::default()).expect("buscar");
    let trash_filter = VaultItemFilter { should_show_trash: true, ..VaultItemFilter::default() };
    let trashed_items = test_harness.vault_service.search_items(&trash_filter).expect("buscar");

    assert_eq!(active_items.len(), 1);
    assert_eq!(trashed_items.len(), 1);
    test_harness.vault_service.restore_item_from_trash(correo_item.item_id).expect("restaurar");
    assert_eq!(test_harness.vault_service.vault_overview().expect("resumen").total_item_count, 2);
}

#[test]
fn editing_password_records_history() {
    let mut test_harness = harness_with_created_vault();
    let saved_item = test_harness.vault_service.save_item(login_request("Correo", "antigua")).expect("guardar");

    let mut edit_request = login_request("Correo", "nueva");
    edit_request.item_id = Some(saved_item.item_id);
    let edited_item = test_harness.vault_service.save_item(edit_request).expect("editar");

    assert_eq!(edited_item.password_history[0].previous_password.expose_secret(), "antigua");
    assert_eq!(edited_item.created_at, saved_item.created_at);
}

#[test]
fn copying_password_uses_preference_delay() {
    let mut test_harness = harness_with_created_vault();
    let saved_item = test_harness.vault_service.save_item(login_request("Correo", "copiable")).expect("guardar");

    test_harness
        .vault_service
        .copy_item_field_to_clipboard(saved_item.item_id, RevealableField::Password)
        .expect("copiar");

    let last_copied_value = test_harness.fake_clipboard.last_copied_value.lock().expect("mutex").clone();
    let (copied_secret, clear_after) = last_copied_value.expect("algo copiado");
    assert_eq!(copied_secret.expose_secret(), "copiable");
    assert_eq!(clear_after.as_secs(), 30);
}

#[test]
fn attachments_are_encrypted_and_size_limited() {
    let mut test_harness = harness_with_created_vault();
    let saved_item = test_harness.vault_service.save_item(login_request("Correo", "pw")).expect("guardar");

    let attachment_reference = test_harness
        .vault_service
        .add_attachment(saved_item.item_id, "../../etc/contrato.pdf", b"contenido del pdf")
        .expect("adjuntar");
    let decrypted_attachment = test_harness
        .vault_service
        .read_attachment(saved_item.item_id, attachment_reference.attachment_id)
        .expect("leer adjunto");
    let oversized_result = test_harness.vault_service.add_attachment(saved_item.item_id, "grande.bin", &[0u8; 2048]);

    assert_eq!(attachment_reference.file_name, "contrato.pdf");
    assert_eq!(decrypted_attachment.file_bytes.as_slice(), b"contenido del pdf");
    assert!(matches!(oversized_result, Err(VaultError::AttachmentTooLarge { .. })));
}

#[test]
fn folders_can_be_created_renamed_and_deleted_without_losing_items() {
    let mut test_harness = harness_with_created_vault();
    let work_folder = test_harness.vault_service.create_folder("Trabajo").expect("crear carpeta");
    let mut item_request = login_request("Jira", "pw");
    item_request.folder_id = Some(work_folder.folder_id);
    let saved_item = test_harness.vault_service.save_item(item_request).expect("guardar");

    assert!(test_harness.vault_service.create_folder(" trabajo ").is_err());
    test_harness.vault_service.rename_folder(work_folder.folder_id, "Oficina").expect("renombrar");
    test_harness.vault_service.delete_folder(work_folder.folder_id).expect("borrar carpeta");

    assert_eq!(test_harness.vault_service.get_item_details(saved_item.item_id).expect("leer").folder_id, None);
}

#[test]
fn health_report_flags_reused_and_weak_passwords() {
    let mut test_harness = harness_with_created_vault();
    test_harness.vault_service.save_item(login_request("Uno", "123456")).expect("guardar");
    test_harness.vault_service.save_item(login_request("Dos", "123456")).expect("guardar");
    test_harness.vault_service.save_item(login_request("Tres", "q7#Vz!m2Lp$9wX4r")).expect("guardar");

    let health_report = test_harness.vault_service.password_health_report().expect("informe");

    assert_eq!(health_report.analyzed_login_count, 3);
    assert_eq!(health_report.healthy_login_count, 1);
    assert_eq!(health_report.overall_health_score, 33);
}

#[test]
fn bitwarden_json_export_reimports_into_a_new_vault() {
    let mut source_harness = harness_with_created_vault();
    source_harness.vault_service.save_item(login_request("Correo", "pw-exportable")).expect("guardar");
    let exported_file = source_harness.vault_service.export_vault(&master_password(), ExportFormat::BitwardenJson, None).expect("exportar");

    let mut target_harness = harness_with_created_vault();
    let import_summary = target_harness
        .vault_service
        .import_file(ImportFormat::BitwardenJson, &exported_file.file_bytes, None)
        .expect("importar");

    assert_eq!(import_summary.imported_item_count, 1);
    assert_eq!(exported_file.suggested_file_extension, "json");
}

#[test]
fn encrypted_export_requires_a_password_and_round_trips() {
    let mut source_harness = harness_with_created_vault();
    source_harness.vault_service.save_item(login_request("Correo", "pw-cifrada")).expect("guardar");
    let export_password = SecretText::new("timber-cobalt-lantern-prism-44");

    let missing_password_result = source_harness.vault_service.export_vault(&master_password(), ExportFormat::EncryptedVaultExport, None);
    let exported_file = source_harness
        .vault_service
        .export_vault(&master_password(), ExportFormat::EncryptedVaultExport, Some(&export_password))
        .expect("exportar");
    let mut target_harness = harness_with_created_vault();
    let wrong_password_result = target_harness.vault_service.import_file(
        ImportFormat::EncryptedVaultExport,
        &exported_file.file_bytes,
        Some(&SecretText::new("otra-clave-distinta-99")),
    );
    let import_summary = target_harness
        .vault_service
        .import_file(ImportFormat::EncryptedVaultExport, &exported_file.file_bytes, Some(&export_password))
        .expect("importar");

    assert!(matches!(missing_password_result, Err(VaultError::TransferPasswordRequired)));
    assert!(wrong_password_result.is_err());
    assert_eq!(import_summary.imported_item_count, 1);
    assert!(!String::from_utf8_lossy(&exported_file.file_bytes).contains("pw-cifrada"));
}

#[test]
fn invalid_totp_secret_is_rejected_on_save() {
    let mut test_harness = harness_with_created_vault();
    let mut item_request = login_request("Correo", "pw");
    if let ItemContent::Login(login_content) = &mut item_request.item_content {
        login_content.totp_secret = Some(SecretText::new("¡¡no válido!!"));
    }

    assert!(matches!(test_harness.vault_service.save_item(item_request), Err(VaultError::InvalidTotpSecret)));
}

// ── Cambio de contraseña con rotación de clave ───────────────────────────────

/// Lo que haría un atacante con una copia vieja del archivo y la contraseña antigua.
fn vault_key_recovered_from(vault_file_bytes: &[u8], master_password: &SecretText) -> EncryptionKey {
    let decoded_vault_file = decode_vault_file(vault_file_bytes).expect("decodificar");
    let vault_header = decoded_vault_file.vault_header;
    let key_encryption_key = Argon2idKeyDerivation
        .derive_key_encryption_key(master_password, &vault_header.master_key_salt, &vault_header.key_derivation_parameters)
        .expect("derivar");
    let vault_key_bytes = XChaCha20Poly1305Cipher
        .open(&key_encryption_key, &vault_header.wrapped_vault_key, &vault_header.key_wrapping_associated_data())
        .expect("la contraseña antigua abre la copia antigua");
    EncryptionKey::copy_from_slice(&vault_key_bytes).expect("clave")
}

#[test]
fn changing_master_password_rotates_the_vault_key() {
    let mut test_harness = harness_with_created_vault();
    test_harness.vault_service.save_item(login_request("Correo", "pw-rotada")).expect("guardar");
    let stolen_old_vault_file = test_harness.vault_repository.read_vault_bytes().expect("leer");
    let new_master_password = SecretText::new("harbor-quartz-meadow-falcon-81");

    test_harness.vault_service.change_master_password(&master_password(), &new_master_password).expect("cambiar");

    let old_vault_key = vault_key_recovered_from(&stolen_old_vault_file, &master_password());
    let old_contents_key = derive_vault_subkeys(&old_vault_key).expect("subclaves").vault_contents_key;
    let current_vault_file = decode_vault_file(&test_harness.vault_repository.read_vault_bytes().expect("leer")).expect("decodificar");
    let open_with_old_key = XChaCha20Poly1305Cipher.open(
        &old_contents_key,
        &current_vault_file.encrypted_contents,
        &current_vault_file.encoded_header_bytes,
    );
    assert!(open_with_old_key.is_err(), "la clave antigua no debe abrir la bóveda nueva");
}

#[test]
fn changing_master_password_discards_the_backup_and_keeps_data() {
    let mut test_harness = harness_with_created_vault();
    let saved_item = test_harness.vault_service.save_item(login_request("Correo", "pw-conservada")).expect("guardar");
    assert!(test_harness.vault_repository.backup_vault_bytes.lock().expect("mutex").is_some());
    let new_master_password = SecretText::new("harbor-quartz-meadow-falcon-81");

    test_harness.vault_service.change_master_password(&master_password(), &new_master_password).expect("cambiar");
    test_harness.vault_service.lock_vault();
    test_harness.vault_service.unlock_vault(&new_master_password).expect("desbloquear con la nueva");

    assert!(test_harness.vault_repository.backup_vault_bytes.lock().expect("mutex").is_none());
    let revealed_password = test_harness
        .vault_service
        .reveal_item_field(saved_item.item_id, RevealableField::Password)
        .expect("revelar");
    assert_eq!(revealed_password.expose_secret(), "pw-conservada");
}

#[test]
fn attachments_are_reencrypted_and_old_copies_removed_on_password_change() {
    let mut test_harness = harness_with_created_vault();
    let saved_item = test_harness.vault_service.save_item(login_request("Correo", "pw")).expect("guardar");
    let original_attachment =
        test_harness.vault_service.add_attachment(saved_item.item_id, "contrato.pdf", b"contenido").expect("adjuntar");
    let new_master_password = SecretText::new("harbor-quartz-meadow-falcon-81");

    test_harness.vault_service.change_master_password(&master_password(), &new_master_password).expect("cambiar");

    let rotated_item = test_harness.vault_service.get_item_details(saved_item.item_id).expect("leer");
    let rotated_attachment_id = rotated_item.attachments[0].attachment_id;
    let stored_attachment_ids: Vec<_> =
        test_harness.attachment_storage.encrypted_attachments.lock().expect("mutex").keys().copied().collect();
    assert_ne!(rotated_attachment_id, original_attachment.attachment_id);
    assert_eq!(stored_attachment_ids, vec![rotated_attachment_id]);
    let decrypted_attachment =
        test_harness.vault_service.read_attachment(saved_item.item_id, rotated_attachment_id).expect("leer adjunto");
    assert_eq!(decrypted_attachment.file_bytes.as_slice(), b"contenido");
}

#[test]
fn failed_attachment_reencryption_leaves_the_vault_untouched() {
    let mut test_harness = harness_with_created_vault();
    let saved_item = test_harness.vault_service.save_item(login_request("Correo", "pw")).expect("guardar");
    let attachment_reference =
        test_harness.vault_service.add_attachment(saved_item.item_id, "contrato.pdf", b"contenido").expect("adjuntar");
    test_harness.attachment_storage.encrypted_attachments.lock().expect("mutex").remove(&attachment_reference.attachment_id);
    let vault_file_before = test_harness.vault_repository.read_vault_bytes().expect("leer");

    let change_result = test_harness
        .vault_service
        .change_master_password(&master_password(), &SecretText::new("harbor-quartz-meadow-falcon-81"));

    assert!(matches!(change_result, Err(VaultError::AttachmentNotFound { .. })));
    assert_eq!(test_harness.vault_repository.read_vault_bytes().expect("leer"), vault_file_before);
    test_harness.vault_service.lock_vault();
    test_harness.vault_service.unlock_vault(&master_password()).expect("la contraseña antigua sigue valiendo");
}

// ── Retardo de desbloqueo persistente ────────────────────────────────────────

#[test]
fn unlock_throttle_survives_an_app_restart() {
    let shared_storage = SharedTestStorage::default();
    let fake_clock = FakeClock::starting_now();
    let mut first_session = build_test_vault_service_on(shared_storage.clone(), fake_clock.clone());
    first_session.vault_service.create_vault(&master_password()).expect("crear");
    first_session.vault_service.lock_vault();
    for _ in 0..4 {
        let _ = first_session.vault_service.unlock_vault(&SecretText::new("incorrecta"));
    }

    let mut restarted_session = build_test_vault_service_on(shared_storage, fake_clock);
    let unlock_result = restarted_session.vault_service.unlock_vault(&master_password());

    assert!(matches!(unlock_result, Err(VaultError::UnlockTemporarilyBlocked { .. })));
}

// ── Portapapeles ─────────────────────────────────────────────────────────────

#[test]
fn locking_clears_the_copied_secret_from_clipboard() {
    let mut test_harness = harness_with_created_vault();
    let saved_item = test_harness.vault_service.save_item(login_request("Correo", "pw-copiada")).expect("guardar");
    test_harness.vault_service.copy_item_field_to_clipboard(saved_item.item_id, RevealableField::Password).expect("copiar");
    assert!(test_harness.fake_clipboard.last_copied_value.lock().expect("mutex").is_some());

    test_harness.vault_service.lock_vault();

    assert!(test_harness.fake_clipboard.last_copied_value.lock().expect("mutex").is_none());
}

// ── Editor con secretos enmascarados ─────────────────────────────────────────

/// Simula lo que hace la UI: carga el elemento enmascarado y lo devuelve como borrador.
fn editing_request_from(redacted_item: &VaultItem) -> SaveVaultItemRequest {
    SaveVaultItemRequest {
        item_id: Some(redacted_item.item_id),
        title: redacted_item.title.clone(),
        folder_id: redacted_item.folder_id,
        tag_names: redacted_item.tag_names.clone(),
        is_favorite: redacted_item.is_favorite,
        notes: redacted_item.notes.clone(),
        custom_fields: redacted_item
            .custom_fields
            .iter()
            .enumerate()
            .map(|(field_position, custom_field)| CustomFieldDraft {
                field_label: custom_field.field_label.clone(),
                field_value: custom_field.field_value.clone(),
                is_hidden: custom_field.is_hidden,
                source_field_index: Some(field_position),
            })
            .collect(),
        item_content: redacted_item.item_content.clone(),
    }
}

fn hidden_field_draft(field_label: &str, field_value: &str) -> CustomFieldDraft {
    CustomFieldDraft {
        field_label: field_label.into(),
        field_value: SecretText::new(field_value),
        is_hidden: true,
        source_field_index: None,
    }
}

#[test]
fn editor_never_receives_secrets_and_untouched_ones_are_kept() {
    let mut test_harness = harness_with_created_vault();
    let mut item_request = login_request("Correo", "pw-original");
    if let ItemContent::Login(login_content) = &mut item_request.item_content {
        login_content.totp_secret = Some(SecretText::new("JBSWY3DPEHPK3PXP"));
    }
    let saved_item = test_harness.vault_service.save_item(item_request).expect("guardar");

    let redacted_item = test_harness.vault_service.get_item_details(saved_item.item_id).expect("leer");
    let ItemContent::Login(redacted_login) = &redacted_item.item_content else { panic!("se esperaba un login") };
    assert_eq!(redacted_login.password.expose_secret(), REDACTED_SECRET_PLACEHOLDER);
    let mut editing_request = editing_request_from(&redacted_item);
    editing_request.title = "Correo personal".into();
    test_harness.vault_service.save_item(editing_request).expect("guardar sin tocar secretos");

    let revealed_password =
        test_harness.vault_service.reveal_item_field(saved_item.item_id, RevealableField::Password).expect("revelar");
    let revealed_totp =
        test_harness.vault_service.reveal_item_field(saved_item.item_id, RevealableField::TotpSecret).expect("revelar");
    assert_eq!(revealed_password.expose_secret(), "pw-original");
    assert_eq!(revealed_totp.expose_secret(), "JBSWY3DPEHPK3PXP");
    let reloaded_item = test_harness.vault_service.get_item_details(saved_item.item_id).expect("leer");
    assert!(reloaded_item.password_history.is_empty(), "no cambió la contraseña");
}

#[test]
fn edited_secret_replaces_the_stored_one() {
    let mut test_harness = harness_with_created_vault();
    let saved_item = test_harness.vault_service.save_item(login_request("Correo", "pw-original")).expect("guardar");
    let redacted_item = test_harness.vault_service.get_item_details(saved_item.item_id).expect("leer");
    let mut editing_request = editing_request_from(&redacted_item);
    if let ItemContent::Login(login_content) = &mut editing_request.item_content {
        login_content.password = SecretText::new("pw-nueva");
    }

    test_harness.vault_service.save_item(editing_request).expect("guardar");

    let revealed_password =
        test_harness.vault_service.reveal_item_field(saved_item.item_id, RevealableField::Password).expect("revelar");
    assert_eq!(revealed_password.expose_secret(), "pw-nueva");
}

#[test]
fn hidden_custom_field_keeps_its_value_after_removing_an_earlier_field() {
    let mut test_harness = harness_with_created_vault();
    let mut item_request = login_request("Banco", "pw");
    item_request.custom_fields = vec![hidden_field_draft("PIN", "1234"), hidden_field_draft("Clave de firma", "firma-9")];
    let saved_item = test_harness.vault_service.save_item(item_request).expect("guardar");

    let redacted_item = test_harness.vault_service.get_item_details(saved_item.item_id).expect("leer");
    let mut editing_request = editing_request_from(&redacted_item);
    editing_request.custom_fields.remove(0);
    test_harness.vault_service.save_item(editing_request).expect("guardar");

    let remaining_value = test_harness
        .vault_service
        .reveal_item_field(saved_item.item_id, RevealableField::CustomField { field_index: 0 })
        .expect("revelar");
    assert_eq!(remaining_value.expose_secret(), "firma-9");
}

#[test]
fn placeholder_typed_in_a_new_item_is_stored_literally() {
    let mut test_harness = harness_with_created_vault();
    let saved_item = test_harness
        .vault_service
        .save_item(login_request("Raro", REDACTED_SECRET_PLACEHOLDER))
        .expect("guardar");

    let revealed_password =
        test_harness.vault_service.reveal_item_field(saved_item.item_id, RevealableField::Password).expect("revelar");
    assert_eq!(revealed_password.expose_secret(), REDACTED_SECRET_PLACEHOLDER);
}

// ── Reautenticación ──────────────────────────────────────────────────────────

#[test]
fn export_requires_the_master_password() {
    let mut test_harness = harness_with_created_vault();
    test_harness.vault_service.save_item(login_request("Correo", "pw")).expect("guardar");

    let export_result =
        test_harness.vault_service.export_vault(&SecretText::new("no-es-la-maestra"), ExportFormat::BitwardenJson, None);

    assert!(matches!(export_result, Err(VaultError::InvalidMasterPasswordOrCorruptedVault)));
}

#[test]
fn wrong_passwords_while_unlocked_are_throttled_too() {
    let mut test_harness = harness_with_created_vault();
    let wrong_password = SecretText::new("no-es-la-maestra");
    let new_master_password = SecretText::new("harbor-quartz-meadow-falcon-81");
    for _ in 0..3 {
        let _ = test_harness.vault_service.change_master_password(&wrong_password, &new_master_password);
    }

    let export_result = test_harness.vault_service.export_vault(&master_password(), ExportFormat::BitwardenJson, None);
    let change_result = test_harness.vault_service.change_master_password(&master_password(), &new_master_password);

    assert!(matches!(export_result, Err(VaultError::UnlockTemporarilyBlocked { .. })));
    assert!(matches!(change_result, Err(VaultError::UnlockTemporarilyBlocked { .. })));
}
