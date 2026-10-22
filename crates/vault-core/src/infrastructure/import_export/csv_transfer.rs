//! CSV de Bitwarden y CSV genérico de navegadores (Chrome, Edge, Firefox, Safari...).
//!
//! Las columnas se localizan por nombre mediante tablas de alias, así un mismo
//! lector sirve para varios orígenes sin cadenas de `if` por proveedor.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use uuid::Uuid;
use zeroize::Zeroizing;

use crate::domain::folder::Folder;
use crate::domain::item_content::{ItemContent, LoginContent, SecureNoteContent};
use crate::domain::ports::vault_exporter::{ExportFormat, VaultExporter};
use crate::domain::ports::vault_importer::{ImportFormat, ImportedVaultData, VaultImporter};
use crate::domain::secret_serialization::write_secret_bytes;
use crate::domain::secret_text::SecretText;
use crate::domain::vault_contents::VaultContents;
use crate::domain::vault_error::{VaultError, VaultResult};
use crate::domain::vault_item::{CustomField, VaultItem};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum CsvColumn {
    Title,
    WebsiteUrl,
    Username,
    Password,
    Notes,
    TotpSecret,
    FolderName,
    IsFavorite,
    ItemType,
    CustomFields,
}

const COLUMN_ALIASES: &[(CsvColumn, &[&str])] = &[
    (CsvColumn::Title, &["name", "title"]),
    (CsvColumn::WebsiteUrl, &["login_uri", "url", "website", "uri"]),
    (CsvColumn::Username, &["login_username", "username", "user", "email"]),
    (CsvColumn::Password, &["login_password", "password"]),
    (CsvColumn::Notes, &["notes", "note", "extra", "comments"]),
    (CsvColumn::TotpSecret, &["login_totp", "totp", "otpauth"]),
    (CsvColumn::FolderName, &["folder", "grouping"]),
    (CsvColumn::IsFavorite, &["favorite", "fav"]),
    (CsvColumn::ItemType, &["type"]),
    (CsvColumn::CustomFields, &["fields"]),
];

const BITWARDEN_CSV_HEADERS: [&str; 11] = [
    "folder",
    "favorite",
    "type",
    "name",
    "notes",
    "fields",
    "reprompt",
    "login_uri",
    "login_username",
    "login_password",
    "login_totp",
];

/// Posición de cada columna conocida dentro del archivo concreto.
struct CsvColumnLayout {
    column_positions: HashMap<CsvColumn, usize>,
}

impl CsvColumnLayout {
    fn from_headers(csv_headers: &csv::StringRecord) -> Self {
        let normalized_headers: Vec<String> =
            csv_headers.iter().map(|header_name| header_name.trim().to_ascii_lowercase()).collect();
        let column_positions = COLUMN_ALIASES
            .iter()
            .filter_map(|(csv_column, header_aliases)| {
                header_aliases
                    .iter()
                    .find_map(|header_alias| normalized_headers.iter().position(|header_name| header_name == header_alias))
                    .map(|column_position| (*csv_column, column_position))
            })
            .collect();
        Self { column_positions }
    }

    fn read<'record>(&self, csv_record: &'record csv::StringRecord, csv_column: CsvColumn) -> &'record str {
        self.column_positions
            .get(&csv_column)
            .and_then(|column_position| csv_record.get(*column_position))
            .map_or("", str::trim)
    }

    fn has_column(&self, csv_column: CsvColumn) -> bool {
        self.column_positions.contains_key(&csv_column)
    }
}

/// Lee todas las filas y las convierte con `build_item`.
fn parse_csv_rows(
    file_bytes: &[u8],
    build_item: impl Fn(&CsvColumnLayout, &csv::StringRecord, &mut FolderCollector, DateTime<Utc>) -> Option<VaultItem>,
) -> VaultResult<ImportedVaultData> {
    let mut csv_reader = csv::ReaderBuilder::new().flexible(true).from_reader(strip_utf8_bom(file_bytes));
    let column_layout = CsvColumnLayout::from_headers(csv_reader.headers().map_err(VaultError::import_failed)?);
    if !column_layout.has_column(CsvColumn::Password) && !column_layout.has_column(CsvColumn::Notes) {
        return Err(VaultError::import_failed("no se reconocen las columnas del CSV"));
    }

    let now = Utc::now();
    let mut folder_collector = FolderCollector::default();
    let mut vault_items = Vec::new();
    for csv_record_result in csv_reader.records() {
        let csv_record = csv_record_result.map_err(VaultError::import_failed)?;
        vault_items.extend(build_item(&column_layout, &csv_record, &mut folder_collector, now));
    }
    Ok(ImportedVaultData { folders: folder_collector.folders, vault_items })
}

fn strip_utf8_bom(file_bytes: &[u8]) -> &[u8] {
    file_bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(file_bytes)
}

#[derive(Default)]
struct FolderCollector {
    folders: Vec<Folder>,
}

impl FolderCollector {
    fn folder_id_for_name(&mut self, folder_name: &str) -> Option<Uuid> {
        if folder_name.is_empty() {
            return None;
        }
        let existing_folder_id = self
            .folders
            .iter()
            .find(|folder| folder.folder_name == folder_name)
            .map(|folder| folder.folder_id);
        Some(existing_folder_id.unwrap_or_else(|| {
            let new_folder = Folder { folder_id: Uuid::new_v4(), folder_name: folder_name.to_string() };
            let new_folder_id = new_folder.folder_id;
            self.folders.push(new_folder);
            new_folder_id
        }))
    }
}

fn build_login_content(column_layout: &CsvColumnLayout, csv_record: &csv::StringRecord) -> LoginContent {
    let totp_secret = column_layout.read(csv_record, CsvColumn::TotpSecret);
    LoginContent {
        username: column_layout.read(csv_record, CsvColumn::Username).to_string(),
        password: SecretText::new(column_layout.read(csv_record, CsvColumn::Password)),
        website_urls: column_layout
            .read(csv_record, CsvColumn::WebsiteUrl)
            .split(',')
            .map(str::trim)
            .filter(|website_url| !website_url.is_empty())
            .map(str::to_string)
            .collect(),
        totp_secret: (!totp_secret.is_empty()).then(|| SecretText::new(totp_secret)),
    }
}

/// Si no hay título, se usa el dominio de la URL (más útil que "Sin título").
fn derive_title(column_layout: &CsvColumnLayout, csv_record: &csv::StringRecord) -> String {
    let explicit_title = column_layout.read(csv_record, CsvColumn::Title);
    if !explicit_title.is_empty() {
        return explicit_title.to_string();
    }
    let website_url = column_layout.read(csv_record, CsvColumn::WebsiteUrl);
    url::Url::parse(website_url)
        .ok()
        .and_then(|parsed_url| parsed_url.host_str().map(str::to_string))
        .unwrap_or_else(|| "Sin título".to_string())
}

fn new_item_from_row(
    title: String,
    item_content: ItemContent,
    notes: &str,
    folder_id: Option<Uuid>,
    now: DateTime<Utc>,
) -> VaultItem {
    let has_password = item_content.login_password().is_some();
    VaultItem {
        item_id: Uuid::new_v4(),
        title,
        folder_id,
        tag_names: Vec::new(),
        is_favorite: false,
        notes: SecretText::new(notes),
        custom_fields: Vec::new(),
        item_content,
        password_history: Vec::new(),
        attachments: Vec::new(),
        created_at: now,
        updated_at: now,
        password_changed_at: has_password.then_some(now),
        moved_to_trash_at: None,
    }
}

// ── CSV de navegadores ───────────────────────────────────────────────────────

pub struct BrowserCsvImporter;

impl VaultImporter for BrowserCsvImporter {
    fn import_format(&self) -> ImportFormat {
        ImportFormat::BrowserCsv
    }

    fn parse_file(&self, file_bytes: &[u8], _file_password: Option<&SecretText>) -> VaultResult<ImportedVaultData> {
        parse_csv_rows(file_bytes, |column_layout, csv_record, _folder_collector, now| {
            let login_content = build_login_content(column_layout, csv_record);
            let has_any_data = !login_content.password.is_empty() || !login_content.username.is_empty();
            has_any_data.then(|| {
                new_item_from_row(
                    derive_title(column_layout, csv_record),
                    ItemContent::Login(login_content),
                    column_layout.read(csv_record, CsvColumn::Notes),
                    None,
                    now,
                )
            })
        })
    }
}

// ── CSV de Bitwarden ─────────────────────────────────────────────────────────

pub struct BitwardenCsvImporter;

impl VaultImporter for BitwardenCsvImporter {
    fn import_format(&self) -> ImportFormat {
        ImportFormat::BitwardenCsv
    }

    fn parse_file(&self, file_bytes: &[u8], _file_password: Option<&SecretText>) -> VaultResult<ImportedVaultData> {
        parse_csv_rows(file_bytes, |column_layout, csv_record, folder_collector, now| {
            let item_content = match column_layout.read(csv_record, CsvColumn::ItemType) {
                "note" => ItemContent::SecureNote(SecureNoteContent {}),
                _ => ItemContent::Login(build_login_content(column_layout, csv_record)),
            };
            let folder_id = folder_collector.folder_id_for_name(column_layout.read(csv_record, CsvColumn::FolderName));
            let mut vault_item = new_item_from_row(
                derive_title(column_layout, csv_record),
                item_content,
                column_layout.read(csv_record, CsvColumn::Notes),
                folder_id,
                now,
            );
            vault_item.is_favorite = column_layout.read(csv_record, CsvColumn::IsFavorite) == "1";
            vault_item.custom_fields = parse_bitwarden_custom_fields(column_layout.read(csv_record, CsvColumn::CustomFields));
            Some(vault_item)
        })
    }
}

/// Bitwarden guarda los campos personalizados como líneas `nombre: valor`.
fn parse_bitwarden_custom_fields(serialized_fields: &str) -> Vec<CustomField> {
    serialized_fields
        .lines()
        .filter_map(|field_line| field_line.split_once(": "))
        .map(|(field_label, field_value)| CustomField {
            field_label: field_label.to_string(),
            field_value: SecretText::new(field_value),
            is_hidden: false,
        })
        .collect()
}

/// Solo exporta inicios de sesión y notas (limitación del propio formato CSV de Bitwarden).
pub struct BitwardenCsvExporter;

impl VaultExporter for BitwardenCsvExporter {
    fn export_format(&self) -> ExportFormat {
        ExportFormat::BitwardenCsv
    }

    fn suggested_file_extension(&self) -> &'static str {
        "csv"
    }

    fn serialize_vault(&self, vault_contents: &VaultContents, _file_password: Option<&SecretText>) -> VaultResult<Zeroizing<Vec<u8>>> {
        let folder_name_by_id: HashMap<Uuid, &str> = vault_contents
            .folders
            .iter()
            .map(|folder| (folder.folder_id, folder.folder_name.as_str()))
            .collect();
        let csv_rows: Vec<[Zeroizing<String>; BITWARDEN_CSV_HEADERS.len()]> = vault_contents
            .vault_items
            .iter()
            .filter_map(|vault_item| bitwarden_csv_row(vault_item, &folder_name_by_id))
            .map(|csv_row| csv_row.map(Zeroizing::new))
            .collect();
        write_secret_bytes(|output| {
            let mut csv_writer = csv::Writer::from_writer(output);
            csv_writer.write_record(BITWARDEN_CSV_HEADERS).map_err(VaultError::storage)?;
            for csv_row in &csv_rows {
                csv_writer.write_record(csv_row.iter().map(|csv_cell| csv_cell.as_bytes())).map_err(VaultError::storage)?;
            }
            csv_writer.flush().map_err(VaultError::storage)
        })
    }
}

/// Las celdas llevan contraseñas en claro: quien las recibe las guarda en `Zeroizing`.
fn bitwarden_csv_row(vault_item: &VaultItem, folder_name_by_id: &HashMap<Uuid, &str>) -> Option<[String; BITWARDEN_CSV_HEADERS.len()]> {
    let (item_type, login_content) = match &vault_item.item_content {
        ItemContent::Login(login_content) => ("login", Some(login_content)),
        ItemContent::SecureNote(_) => ("note", None),
        ItemContent::PaymentCard(_) | ItemContent::Identity(_) => return None,
    };
    let folder_name = vault_item
        .folder_id
        .and_then(|folder_id| folder_name_by_id.get(&folder_id).copied())
        .unwrap_or_default();
    let field_lines: Vec<Zeroizing<String>> = vault_item
        .custom_fields
        .iter()
        .map(|custom_field| Zeroizing::new(format!("{}: {}", custom_field.field_label, custom_field.field_value.expose_secret())))
        .collect();
    let serialized_fields = Zeroizing::new(field_lines.iter().map(|field_line| field_line.as_str()).collect::<Vec<_>>().join("\n"));

    Some([
        neutralize_spreadsheet_formula(folder_name),
        if vault_item.is_favorite { "1" } else { "" }.to_string(),
        item_type.to_string(),
        neutralize_spreadsheet_formula(&vault_item.title),
        neutralize_spreadsheet_formula(vault_item.notes.expose_secret()),
        neutralize_spreadsheet_formula(&serialized_fields),
        "0".to_string(),
        neutralize_spreadsheet_formula(&login_content.map(|login| login.website_urls.join(",")).unwrap_or_default()),
        neutralize_spreadsheet_formula(&login_content.map(|login| login.username.clone()).unwrap_or_default()),
        // Contraseña y TOTP se exportan tal cual: alterarlos rompería el login al reimportar
        // en otro gestor, y no vienen de terceros sino del propio usuario.
        login_content.map(|login| login.password.expose_secret().to_string()).unwrap_or_default(),
        login_content
            .and_then(|login| login.totp_secret.as_ref())
            .map(|totp_secret| totp_secret.expose_secret().to_string())
            .unwrap_or_default(),
    ])
}

/// Caracteres con los que `Excel` / `LibreOffice` interpretan una celda como fórmula.
const SPREADSHEET_FORMULA_PREFIXES: [char; 6] = ['=', '+', '-', '@', '\t', '\r'];

/// Evita la "CSV injection": un título importado como `=HYPERLINK(...)` se ejecutaría
/// como fórmula al abrir la exportación en una hoja de cálculo. El apóstrofo inicial
/// hace que se muestre como texto (recomendación de OWASP).
fn neutralize_spreadsheet_formula(cell_text: &str) -> String {
    if cell_text.starts_with(SPREADSHEET_FORMULA_PREFIXES) {
        return format!("'{cell_text}");
    }
    cell_text.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::vault_preferences::VaultPreferences;

    #[test]
    fn exported_formulas_are_neutralized_but_passwords_are_kept() {
        let bitwarden_csv = "folder,favorite,type,name,notes,fields,reprompt,login_uri,login_username,login_password,login_totp\n\
            ,,login,\"=HYPERLINK(\"\"https://malo.example\"\")\",@SUM(1),,0,https://ok.example,+ana,=pw-que-empieza-por-igual,\n";
        let imported_vault_data = BitwardenCsvImporter.parse_file(bitwarden_csv.as_bytes(), None).expect("importar");
        let vault_contents = VaultContents {
            vault_items: imported_vault_data.vault_items,
            folders: Vec::new(),
            preferences: VaultPreferences::default(),
            vault_id: None,
            revision_number: 0,
        };

        let exported_bytes = BitwardenCsvExporter.serialize_vault(&vault_contents, None).expect("exportar");
        let exported_text = String::from_utf8(exported_bytes.to_vec()).expect("utf-8");

        assert!(exported_text.contains("'=HYPERLINK"));
        assert!(exported_text.contains("'@SUM(1)"));
        assert!(exported_text.contains("'+ana"));
        assert!(exported_text.contains(",=pw-que-empieza-por-igual,"));
        assert!(exported_text.contains(",https://ok.example,"));
    }

    #[test]
    fn chrome_csv_is_imported_with_title_fallback_to_host() {
        let chrome_csv = "name,url,username,password,note\n,https://mail.example.com/login,ana,pw1,\nBanco,https://banco.es,ana,pw2,pin aparte\n";

        let imported_vault_data = BrowserCsvImporter.parse_file(chrome_csv.as_bytes(), None).expect("importar");

        assert_eq!(imported_vault_data.vault_items.len(), 2);
        assert_eq!(imported_vault_data.vault_items[0].title, "mail.example.com");
        assert_eq!(imported_vault_data.vault_items[1].notes.expose_secret(), "pin aparte");
    }

    #[test]
    fn firefox_csv_column_order_is_supported() {
        let firefox_csv = "\u{feff}\"url\",\"username\",\"password\",\"httpRealm\"\n\"https://foro.org\",\"luis\",\"clave\",\"\"\n";

        let imported_vault_data = BrowserCsvImporter.parse_file(firefox_csv.as_bytes(), None).expect("importar");

        let ItemContent::Login(login_content) = &imported_vault_data.vault_items[0].item_content else {
            panic!("se esperaba un login");
        };
        assert_eq!(login_content.username, "luis");
        assert_eq!(login_content.password.expose_secret(), "clave");
    }

    #[test]
    fn bitwarden_csv_round_trips_folders_favorites_and_fields() {
        let bitwarden_csv = "folder,favorite,type,name,notes,fields,reprompt,login_uri,login_username,login_password,login_totp\n\
            Trabajo,1,login,GitLab,,\"PIN: 99\",0,https://gitlab.com,ana,pw,\n\
            ,,note,Receta,ingredientes,,0,,,,\n";

        let imported_vault_data = BitwardenCsvImporter.parse_file(bitwarden_csv.as_bytes(), None).expect("importar");
        let vault_contents = VaultContents {
            vault_items: imported_vault_data.vault_items,
            folders: imported_vault_data.folders,
            preferences: VaultPreferences::default(),
            vault_id: None,
            revision_number: 0,
        };
        let exported_bytes = BitwardenCsvExporter.serialize_vault(&vault_contents, None).expect("exportar");
        let reimported_vault_data = BitwardenCsvImporter.parse_file(&exported_bytes, None).expect("reimportar");

        assert_eq!(reimported_vault_data.folders.len(), 1);
        assert_eq!(reimported_vault_data.vault_items.len(), 2);
        assert!(reimported_vault_data.vault_items[0].is_favorite);
        assert_eq!(reimported_vault_data.vault_items[0].custom_fields[0].field_value.expose_secret(), "99");
        assert!(matches!(reimported_vault_data.vault_items[1].item_content, ItemContent::SecureNote(_)));
    }

    #[test]
    fn unrelated_csv_is_rejected() {
        let unrelated_csv = "producto,precio\nmanzana,1\n";

        assert!(BrowserCsvImporter.parse_file(unrelated_csv.as_bytes(), None).is_err());
    }
}
