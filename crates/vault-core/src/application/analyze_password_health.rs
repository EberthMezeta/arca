//! Informe de salud: contraseñas débiles, repetidas y antiguas.
//! Cada comprobación es una regla independiente (Open/Closed: añadir una regla no toca las demás).

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::domain::password_strength::PasswordStrength;
use crate::domain::secret_text::SecretText;
use crate::domain::vault_contents::VaultContents;

const WEAK_PASSWORD_SCORE_THRESHOLD: u8 = 3;
const OLD_PASSWORD_AGE_DAYS: i64 = 365;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "issueKind", rename_all = "camelCase")]
pub enum PasswordHealthIssue {
    #[serde(rename_all = "camelCase")]
    WeakPassword { strength_score: u8 },
    #[serde(rename_all = "camelCase")]
    ReusedPassword { reuse_count: usize },
    #[serde(rename_all = "camelCase")]
    OldPassword { age_in_days: i64 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PasswordHealthFinding {
    pub item_id: Uuid,
    pub item_title: String,
    pub issue: PasswordHealthIssue,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PasswordHealthReport {
    pub analyzed_login_count: usize,
    pub healthy_login_count: usize,
    /// 0-100: porcentaje de inicios de sesión sin ningún problema.
    pub overall_health_score: u8,
    pub findings: Vec<PasswordHealthFinding>,
}

struct LoginPasswordEntry<'vault> {
    item_id: Uuid,
    item_title: &'vault str,
    password: &'vault SecretText,
    password_changed_at: Option<DateTime<Utc>>,
}

struct PasswordHealthContext<'vault> {
    login_entries: Vec<LoginPasswordEntry<'vault>>,
    password_usage_counts: HashMap<&'vault str, usize>,
    now: DateTime<Utc>,
}

trait PasswordHealthRule {
    fn find_issue(&self, login_entry: &LoginPasswordEntry<'_>, context: &PasswordHealthContext<'_>) -> Option<PasswordHealthIssue>;
}

struct WeakPasswordRule;

impl PasswordHealthRule for WeakPasswordRule {
    fn find_issue(&self, login_entry: &LoginPasswordEntry<'_>, _context: &PasswordHealthContext<'_>) -> Option<PasswordHealthIssue> {
        let strength_score = PasswordStrength::estimate(login_entry.password).score;
        (strength_score < WEAK_PASSWORD_SCORE_THRESHOLD).then_some(PasswordHealthIssue::WeakPassword { strength_score })
    }
}

struct ReusedPasswordRule;

impl PasswordHealthRule for ReusedPasswordRule {
    fn find_issue(&self, login_entry: &LoginPasswordEntry<'_>, context: &PasswordHealthContext<'_>) -> Option<PasswordHealthIssue> {
        let reuse_count = context.password_usage_counts.get(login_entry.password.expose_secret()).copied().unwrap_or(0);
        (reuse_count > 1).then_some(PasswordHealthIssue::ReusedPassword { reuse_count })
    }
}

struct OldPasswordRule;

impl PasswordHealthRule for OldPasswordRule {
    fn find_issue(&self, login_entry: &LoginPasswordEntry<'_>, context: &PasswordHealthContext<'_>) -> Option<PasswordHealthIssue> {
        let age_in_days = (context.now - login_entry.password_changed_at?).num_days();
        (age_in_days >= OLD_PASSWORD_AGE_DAYS).then_some(PasswordHealthIssue::OldPassword { age_in_days })
    }
}

fn all_health_rules() -> Vec<Box<dyn PasswordHealthRule>> {
    vec![Box::new(WeakPasswordRule), Box::new(ReusedPasswordRule), Box::new(OldPasswordRule)]
}

fn build_context(vault_contents: &VaultContents, now: DateTime<Utc>) -> PasswordHealthContext<'_> {
    let login_entries: Vec<LoginPasswordEntry<'_>> = vault_contents
        .active_items()
        .filter_map(|vault_item| {
            vault_item.item_content.login_password().map(|password| LoginPasswordEntry {
                item_id: vault_item.item_id,
                item_title: &vault_item.title,
                password,
                password_changed_at: vault_item.password_changed_at,
            })
        })
        .collect();

    let mut password_usage_counts = HashMap::new();
    for login_entry in &login_entries {
        *password_usage_counts.entry(login_entry.password.expose_secret()).or_insert(0) += 1;
    }
    PasswordHealthContext { login_entries, password_usage_counts, now }
}

pub fn execute(vault_contents: &VaultContents, now: DateTime<Utc>) -> PasswordHealthReport {
    let health_context = build_context(vault_contents, now);
    let health_rules = all_health_rules();

    let findings: Vec<PasswordHealthFinding> = health_context
        .login_entries
        .iter()
        .flat_map(|login_entry| {
            health_rules.iter().filter_map(|health_rule| {
                health_rule.find_issue(login_entry, &health_context).map(|issue| PasswordHealthFinding {
                    item_id: login_entry.item_id,
                    item_title: login_entry.item_title.to_string(),
                    issue,
                })
            })
        })
        .collect();

    let analyzed_login_count = health_context.login_entries.len();
    let healthy_login_count = health_context
        .login_entries
        .iter()
        .filter(|login_entry| findings.iter().all(|finding| finding.item_id != login_entry.item_id))
        .count();

    PasswordHealthReport {
        analyzed_login_count,
        healthy_login_count,
        overall_health_score: percentage(healthy_login_count, analyzed_login_count),
        findings,
    }
}

fn percentage(part_count: usize, total_count: usize) -> u8 {
    if total_count == 0 {
        return 100;
    }
    u8::try_from(part_count * 100 / total_count).unwrap_or(100)
}
