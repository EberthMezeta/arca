//! Informe de salud de las contraseñas, agrupado por tipo de problema.

use std::rc::Rc;

use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use uuid::Uuid;
use vault_core::application::analyze_password_health::{
    PasswordHealthFinding, PasswordHealthIssue, PasswordHealthReport,
};

use crate::presentation::app_context::{AppContext, AppEvent};
use crate::{AppWindow, HealthFinding, HealthGroup, HealthIssueKind, PasswordHealth, VaultSection};
use vault_core::application::vault_service::VaultItems;

/// Cómo se presenta cada tipo de problema (tabla en vez de condicionales).
struct IssuePresentation {
    issue_kind: HealthIssueKind,
    title_key: &'static str,
    explanation_key: &'static str,
    detail_key: &'static str,
    matches_issue: fn(&PasswordHealthIssue) -> bool,
    detail_values: fn(&PasswordHealthIssue) -> Vec<(&'static str, String)>,
}

const ISSUE_PRESENTATIONS: [IssuePresentation; 3] = [
    IssuePresentation {
        issue_kind: HealthIssueKind::Weak,
        title_key: "health.weakTitle",
        explanation_key: "health.weakExplanation",
        detail_key: "health.weakDetail",
        matches_issue: |issue| matches!(issue, PasswordHealthIssue::WeakPassword { .. }),
        detail_values: |_| Vec::new(),
    },
    IssuePresentation {
        issue_kind: HealthIssueKind::Reused,
        title_key: "health.reusedTitle",
        explanation_key: "health.reusedExplanation",
        detail_key: "health.reusedDetail",
        matches_issue: |issue| matches!(issue, PasswordHealthIssue::ReusedPassword { .. }),
        detail_values: |issue| match issue {
            PasswordHealthIssue::ReusedPassword { reuse_count } => vec![("reuseCount", reuse_count.to_string())],
            _ => Vec::new(),
        },
    },
    IssuePresentation {
        issue_kind: HealthIssueKind::Old,
        title_key: "health.oldTitle",
        explanation_key: "health.oldExplanation",
        detail_key: "health.oldDetail",
        matches_issue: |issue| matches!(issue, PasswordHealthIssue::OldPassword { .. }),
        detail_values: |issue| match issue {
            PasswordHealthIssue::OldPassword { age_in_days } => vec![("ageInDays", age_in_days.to_string())],
            _ => Vec::new(),
        },
    },
];

pub struct HealthController {
    context: Rc<AppContext>,
}

impl HealthController {
    pub fn install(context: &Rc<AppContext>, app_window: &AppWindow) -> Rc<Self> {
        let controller = Rc::new(Self { context: Rc::clone(context) });
        let changing_controller = Rc::clone(&controller);
        app_window.global::<PasswordHealth>().on_change_now(move |item_id| {
            if let Ok(item_id) = Uuid::parse_str(&item_id) {
                changing_controller.context.navigator.start_editing_item(item_id);
            }
        });
        let event_controller = Rc::clone(&controller);
        context.events.subscribe(move |app_event| match app_event {
            AppEvent::SectionChanged | AppEvent::VaultDataChanged | AppEvent::LanguageChanged => {
                event_controller.refresh();
            }
            AppEvent::SessionLocked => event_controller.clear(),
            _ => {}
        });
        controller
    }

    /// Solo se calcula con la sección abierta: zxcvbn sobre toda la bóveda no es gratis.
    fn refresh(&self) {
        if self.context.navigator.snapshot().section != VaultSection::Health {
            return;
        }
        self.context.with_window(|app_window| {
            let Ok(health_report) = self
                .context
                .shared_vault
                .lock_service()
                .and_then(|vault_service| Ok(vault_service.password_health_report()?))
            else {
                return;
            };
            let password_health = app_window.global::<PasswordHealth>();
            password_health.set_has_logins(health_report.analyzed_login_count > 0);
            password_health.set_overall_score(i32::from(health_report.overall_health_score));
            password_health.set_summary(
                self.context
                    .text_with(
                        "health.summary",
                        &[
                            ("healthyCount", &health_report.healthy_login_count.to_string()),
                            ("totalCount", &health_report.analyzed_login_count.to_string()),
                        ],
                    )
                    .into(),
            );
            password_health.set_issue_groups(ModelRc::new(VecModel::from(self.build_groups(&health_report))));
            password_health.set_is_loading(false);
        });
    }

    fn build_groups(&self, health_report: &PasswordHealthReport) -> Vec<HealthGroup> {
        ISSUE_PRESENTATIONS
            .iter()
            .map(|presentation| {
                let findings: Vec<HealthFinding> = health_report
                    .findings
                    .iter()
                    .filter(|finding| (presentation.matches_issue)(&finding.issue))
                    .map(|finding| self.describe_finding(presentation, finding))
                    .collect();
                HealthGroup {
                    issue_kind: presentation.issue_kind,
                    title: self.context.text(presentation.title_key).into(),
                    explanation: self.context.text(presentation.explanation_key).into(),
                    finding_count: i32::try_from(findings.len()).unwrap_or(i32::MAX),
                    findings: ModelRc::new(VecModel::from(findings)),
                }
            })
            .collect()
    }

    fn describe_finding(&self, presentation: &IssuePresentation, finding: &PasswordHealthFinding) -> HealthFinding {
        let detail_values = (presentation.detail_values)(&finding.issue);
        let interpolated_values: Vec<(&str, &str)> =
            detail_values.iter().map(|(placeholder, value)| (*placeholder, value.as_str())).collect();
        HealthFinding {
            item_id: finding.item_id.to_string().into(),
            item_title: finding.item_title.clone().into(),
            detail: self.context.text_with(presentation.detail_key, &interpolated_values).into(),
        }
    }

    fn clear(&self) {
        self.context.with_window(|app_window| {
            let password_health = app_window.global::<PasswordHealth>();
            password_health.set_is_loading(true);
            password_health.set_issue_groups(ModelRc::default());
            password_health.set_summary(SharedString::new());
        });
    }
}
