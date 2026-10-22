use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::item_content::ItemContentType;
use crate::domain::vault_contents::VaultContents;
use crate::domain::vault_item::VaultItem;

/// Vista de lista: nunca incluye contraseñas ni otros secretos.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultItemSummary {
    pub item_id: Uuid,
    pub title: String,
    pub subtitle: String,
    pub content_type: ItemContentType,
    pub folder_id: Option<Uuid>,
    pub tag_names: Vec<String>,
    pub is_favorite: bool,
    pub has_password: bool,
    pub has_totp: bool,
    pub primary_website_url: Option<String>,
    pub attachment_count: usize,
    pub updated_at: DateTime<Utc>,
    pub is_in_trash: bool,
}

impl From<&VaultItem> for VaultItemSummary {
    fn from(vault_item: &VaultItem) -> Self {
        Self {
            item_id: vault_item.item_id,
            title: vault_item.title.clone(),
            subtitle: vault_item.item_content.non_sensitive_subtitle(),
            content_type: vault_item.item_content.content_type(),
            folder_id: vault_item.folder_id,
            tag_names: vault_item.tag_names.clone(),
            is_favorite: vault_item.is_favorite,
            has_password: vault_item.item_content.login_password().is_some(),
            has_totp: vault_item.item_content.totp_secret().is_some(),
            primary_website_url: vault_item.item_content.primary_website_url().map(str::to_string),
            attachment_count: vault_item.attachments.len(),
            updated_at: vault_item.updated_at,
            is_in_trash: vault_item.is_in_trash(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "scope", rename_all = "camelCase")]
pub enum FolderScope {
    #[default]
    AllFolders,
    WithoutFolder,
    #[serde(rename_all = "camelCase")]
    SpecificFolder {
        folder_id: Uuid,
    },
}

impl FolderScope {
    fn includes(self, vault_item: &VaultItem) -> bool {
        match self {
            Self::AllFolders => true,
            Self::WithoutFolder => vault_item.folder_id.is_none(),
            Self::SpecificFolder { folder_id } => vault_item.folder_id == Some(folder_id),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct VaultItemFilter {
    pub search_text: Option<String>,
    pub folder_scope: FolderScope,
    pub tag_name: Option<String>,
    pub content_type: Option<ItemContentType>,
    pub should_show_only_favorites: bool,
    pub should_show_trash: bool,
}

type ItemPredicate<'filter> = Box<dyn Fn(&VaultItem) -> bool + 'filter>;

impl VaultItemFilter {
    /// Cada criterio opcional se convierte en un predicado; los ausentes simplemente no se añaden.
    fn build_predicates(&self) -> Vec<ItemPredicate<'_>> {
        let mut item_predicates: Vec<ItemPredicate<'_>> = vec![
            Box::new(move |vault_item| vault_item.is_in_trash() == self.should_show_trash),
            Box::new(move |vault_item| self.folder_scope.includes(vault_item)),
            Box::new(move |vault_item| !self.should_show_only_favorites || vault_item.is_favorite),
        ];

        let normalized_search_text = self
            .search_text
            .as_deref()
            .map(str::trim)
            .filter(|search_text| !search_text.is_empty())
            .map(str::to_lowercase);
        item_predicates.extend(normalized_search_text.map(|search_text| -> ItemPredicate<'_> {
            Box::new(move |vault_item| item_matches_search_text(vault_item, &search_text))
        }));
        item_predicates.extend(self.tag_name.as_deref().map(|tag_name| -> ItemPredicate<'_> {
            Box::new(move |vault_item| {
                vault_item.tag_names.iter().any(|item_tag| item_tag.eq_ignore_ascii_case(tag_name))
            })
        }));
        item_predicates.extend(self.content_type.map(|content_type| -> ItemPredicate<'_> {
            Box::new(move |vault_item| vault_item.item_content.content_type() == content_type)
        }));
        item_predicates
    }
}

fn item_matches_search_text(vault_item: &VaultItem, lowercase_search_text: &str) -> bool {
    let searchable_text = format!(
        "{} {} {}",
        vault_item.title,
        vault_item.tag_names.join(" "),
        vault_item.item_content.searchable_text()
    )
    .to_lowercase();
    searchable_text.contains(lowercase_search_text)
}

pub fn search_items(vault_contents: &VaultContents, item_filter: &VaultItemFilter) -> Vec<VaultItemSummary> {
    let item_predicates = item_filter.build_predicates();
    let mut matching_summaries: Vec<VaultItemSummary> = vault_contents
        .vault_items
        .iter()
        .filter(|vault_item| item_predicates.iter().all(|item_predicate| item_predicate(vault_item)))
        .map(VaultItemSummary::from)
        .collect();

    matching_summaries.sort_by(|first_summary, second_summary| {
        second_summary
            .is_favorite
            .cmp(&first_summary.is_favorite)
            .then_with(|| first_summary.title.to_lowercase().cmp(&second_summary.title.to_lowercase()))
    });
    matching_summaries
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderOverview {
    pub folder_id: Uuid,
    pub folder_name: String,
    pub item_count: usize,
}

/// Datos para la barra lateral: contadores por carpeta, etiqueta, tipo, favoritos y papelera.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultOverview {
    pub total_item_count: usize,
    pub favorite_item_count: usize,
    pub trash_item_count: usize,
    pub items_without_folder_count: usize,
    pub folders: Vec<FolderOverview>,
    pub tag_item_counts: BTreeMap<String, usize>,
    pub content_type_item_counts: BTreeMap<ItemContentType, usize>,
}

pub fn build_overview(vault_contents: &VaultContents) -> VaultOverview {
    let active_items: Vec<&VaultItem> = vault_contents.active_items().collect();

    let mut tag_item_counts = BTreeMap::new();
    let mut content_type_item_counts = BTreeMap::new();
    for vault_item in &active_items {
        for tag_name in &vault_item.tag_names {
            *tag_item_counts.entry(tag_name.clone()).or_insert(0) += 1;
        }
        *content_type_item_counts.entry(vault_item.item_content.content_type()).or_insert(0) += 1;
    }

    let mut folders: Vec<FolderOverview> = vault_contents
        .folders
        .iter()
        .map(|folder| FolderOverview {
            folder_id: folder.folder_id,
            folder_name: folder.folder_name.clone(),
            item_count: active_items.iter().filter(|vault_item| vault_item.folder_id == Some(folder.folder_id)).count(),
        })
        .collect();
    folders.sort_by_key(|folder_overview| folder_overview.folder_name.to_lowercase());

    VaultOverview {
        total_item_count: active_items.len(),
        favorite_item_count: active_items.iter().filter(|vault_item| vault_item.is_favorite).count(),
        trash_item_count: vault_contents.vault_items.len() - active_items.len(),
        items_without_folder_count: active_items.iter().filter(|vault_item| vault_item.folder_id.is_none()).count(),
        folders,
        tag_item_counts,
        content_type_item_counts,
    }
}
